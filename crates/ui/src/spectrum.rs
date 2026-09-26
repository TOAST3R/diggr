//! Mini visualizer: 19-bar spectrum with falling peaks, and an oscilloscope.
//!
//! Samples come from the engine's tap, stamped with track frames. Each frame the analyzer takes
//! the 1024 samples that end at the *audible* frame reported by the playback clock, so bars
//! move with what is heard, not with what was just decoded.

use std::collections::VecDeque;
use std::sync::Arc;

use audio::{PlayState, Position, TapChunk, TrackId};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

pub const BARS: usize = 19;
const FFT: usize = 1024;
const MIN_HZ: f32 = 30.0;
const MAX_HZ: f32 = 16_000.0;
const FLOOR_DB: f32 = -60.0;
/// Full-scale bar height fall per second.
const BAR_FALL: f32 = 2.5;
const PEAK_HOLD: f32 = 0.4;
const PEAK_FALL: f32 = 0.8;

pub struct Analyzer {
    rate: u32,
    track: TrackId,
    /// Track frame of `history[0]`.
    start: u64,
    history: VecDeque<f32>,
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    output: Vec<Complex<f32>>,
    edges: [usize; BARS + 1],
    bars: [f32; BARS],
    peaks: [f32; BARS],
    peak_age: [f32; BARS],
}

impl Analyzer {
    pub fn new(rate: u32) -> Self {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT);
        let window = (0..FFT)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FFT as f32).cos())
            .collect();
        let mut a = Self {
            rate,
            track: 0,
            start: 0,
            history: VecDeque::with_capacity(rate as usize * 2),
            input: fft.make_input_vec(),
            output: fft.make_output_vec(),
            fft,
            window,
            edges: [0; BARS + 1],
            bars: [0.0; BARS],
            peaks: [0.0; BARS],
            peak_age: [0.0; BARS],
        };
        a.set_rate(rate);
        a
    }

    pub fn set_rate(&mut self, rate: u32) {
        self.rate = rate;
        let max = MAX_HZ.min(rate as f32 * 0.45);
        let mut last = 0;
        for (i, e) in self.edges.iter_mut().enumerate() {
            let hz = MIN_HZ * (max / MIN_HZ).powf(i as f32 / BARS as f32);
            let bin = ((hz * FFT as f32 / rate as f32).round() as usize).clamp(1, FFT / 2);
            // Every bar gets at least one bin.
            *e = if i == 0 { bin } else { bin.max(last + 1) };
            last = *e;
        }
        self.history.clear();
    }

    /// Frequency range (Hz) covered by bar `i`.
    pub fn band_hz(&self, i: usize) -> (f32, f32) {
        let f = |bin: usize| bin as f32 * self.rate as f32 / FFT as f32;
        (f(self.edges[i]), f(self.edges[i + 1]))
    }

    pub fn push(&mut self, chunk: &TapChunk) {
        let contiguous = chunk.track == self.track
            && chunk.start_frame == self.start + self.history.len() as u64;
        if !contiguous {
            self.history.clear();
            self.track = chunk.track;
            self.start = chunk.start_frame;
        }
        self.history
            .extend(chunk.data().chunks(2).map(|f| 0.5 * (f[0] + f[1])));
        let cap = self.rate as usize * 2;
        if self.history.len() > cap {
            let drop = self.history.len() - cap;
            self.history.drain(..drop);
            self.start += drop as u64;
        }
    }

    /// History indices of the `n` samples ending at the audible frame, if covered.
    fn window_at(&self, pos: &Position, n: usize) -> Option<std::ops::Range<usize>> {
        if pos.track != self.track || pos.frame < self.start + n as u64 {
            return None;
        }
        let end = (pos.frame - self.start) as usize;
        (end <= self.history.len()).then(|| end - n..end)
    }

    /// Recomputes the bars for the audible position; `dt` is seconds since the last update.
    pub fn update(&mut self, pos: &Position, dt: f32) {
        let mut target = [0.0f32; BARS];
        if pos.state == PlayState::Playing
            && let Some(range) = self.window_at(pos, FFT)
        {
            let samples = self.history.range(range);
            for ((x, s), w) in self.input.iter_mut().zip(samples).zip(&self.window) {
                *x = s * w;
            }
            if self.fft.process(&mut self.input, &mut self.output).is_ok() {
                for (i, t) in target.iter_mut().enumerate() {
                    let band = &self.output[self.edges[i]..self.edges[i + 1]];
                    let mag = band.iter().map(|c| c.norm()).fold(0.0, f32::max);
                    // A full-scale sine has magnitude ≈ FFT/4 with a Hann window.
                    let db = 20.0 * (mag / (FFT as f32 / 4.0)).max(1e-9).log10();
                    *t = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
                }
            }
        }
        #[allow(clippy::needless_range_loop)] // four parallel arrays: indexing is clearest
        for i in 0..BARS {
            self.bars[i] = target[i].max(self.bars[i] - BAR_FALL * dt);
            if self.bars[i] >= self.peaks[i] {
                self.peaks[i] = self.bars[i];
                self.peak_age[i] = 0.0;
            } else {
                self.peak_age[i] += dt;
                if self.peak_age[i] > PEAK_HOLD {
                    self.peaks[i] = (self.peaks[i] - PEAK_FALL * dt).max(self.bars[i]);
                }
            }
        }
    }

    /// Bar heights, 0..=1.
    pub fn bars(&self) -> &[f32; BARS] {
        &self.bars
    }

    pub fn peaks(&self) -> &[f32; BARS] {
        &self.peaks
    }

    /// Whether anything is still visible (lets the UI stop repainting when all is quiet).
    pub fn is_active(&self) -> bool {
        self.peaks.iter().any(|&p| p > 0.001)
    }

    /// `n` samples (−1..=1) ending at the audible frame, for the oscilloscope.
    pub fn scope(&self, pos: &Position, n: usize) -> Vec<f32> {
        match self.window_at(pos, n) {
            Some(r) if pos.state == PlayState::Playing => self.history.range(r).copied().collect(),
            _ => vec![0.0; n],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio::tap::TAP_CHUNK_FRAMES;

    const RATE: u32 = 48_000;

    fn chunk(track: TrackId, start: u64, f: impl Fn(u64) -> f32) -> TapChunk {
        let mut samples = [0.0; TAP_CHUNK_FRAMES * 2];
        for i in 0..TAP_CHUNK_FRAMES {
            let v = f(start + i as u64);
            samples[i * 2] = v;
            samples[i * 2 + 1] = v;
        }
        TapChunk {
            track,
            start_frame: start,
            frames: TAP_CHUNK_FRAMES,
            samples,
        }
    }

    fn feed(a: &mut Analyzer, frames: u64, f: impl Fn(u64) -> f32 + Copy) {
        let mut at = 0;
        while at < frames {
            a.push(&chunk(1, at, f));
            at += TAP_CHUNK_FRAMES as u64;
        }
    }

    fn playing(frame: u64) -> Position {
        Position {
            track: 1,
            frame,
            sample_rate: RATE,
            state: PlayState::Playing,
            discontinuity: false,
        }
    }

    fn loudest(a: &Analyzer) -> usize {
        (0..BARS)
            .max_by(|&x, &y| a.bars()[x].total_cmp(&a.bars()[y]))
            .unwrap()
    }

    fn bar_for(a: &Analyzer, hz: f32) -> usize {
        (0..BARS)
            .find(|&i| {
                let (lo, hi) = a.band_hz(i);
                hz >= lo && hz < hi
            })
            .unwrap()
    }

    #[test]
    fn bands_are_increasing_and_cover_the_range() {
        let a = Analyzer::new(RATE);
        for i in 0..BARS {
            let (lo, hi) = a.band_hz(i);
            assert!(hi > lo, "bar {i}");
        }
        assert!(a.band_hz(0).0 < 50.0 && a.band_hz(BARS - 1).1 > 14_000.0);
    }

    #[test]
    fn a_tone_lights_its_own_bar() {
        for hz in [100.0f32, 1_000.0, 5_000.0] {
            let mut a = Analyzer::new(RATE);
            let w = 2.0 * std::f32::consts::PI * hz / RATE as f32;
            feed(&mut a, 8_192, |f| (f as f32 * w).sin() * 0.5);
            a.update(&playing(8_000), 1.0 / 30.0);
            assert_eq!(loudest(&a), bar_for(&a, hz), "{hz} Hz");
            assert!(a.bars()[loudest(&a)] > 0.8, "a -6 dB sine is near the top");
        }
    }

    #[test]
    fn bars_follow_the_audible_frame_not_the_newest_samples() {
        let mut a = Analyzer::new(RATE);
        // Silence until frame 6000, then a loud burst: the tap already has it, but it is not
        // audible yet at frame 5000.
        feed(&mut a, 8_192, |f| {
            if f >= 6_000 {
                ((f as f32) * 0.3).sin()
            } else {
                0.0
            }
        });
        a.update(&playing(5_000), 1.0 / 30.0);
        assert!(a.bars().iter().all(|&b| b == 0.0), "not heard yet");
        a.update(&playing(7_100), 1.0 / 30.0);
        assert!(a.bars().iter().any(|&b| b > 0.5), "heard now");
    }

    #[test]
    fn bars_fall_and_peaks_hold() {
        let mut a = Analyzer::new(RATE);
        feed(&mut a, 4_096, |f| (f as f32 * 0.05).sin());
        a.update(&playing(4_000), 0.0);
        let i = loudest(&a);
        let (bar, peak) = (a.bars()[i], a.peaks()[i]);
        let stopped = Position {
            state: PlayState::Stopped,
            ..playing(4_000)
        };
        a.update(&stopped, 0.1);
        assert!((a.bars()[i] - (bar - 0.25)).abs() < 1e-5, "falls at 2.5/s");
        assert_eq!(a.peaks()[i], peak, "peak holds briefly");
        for _ in 0..30 {
            a.update(&stopped, 0.1);
        }
        assert!(!a.is_active(), "everything settles to zero");
    }

    #[test]
    fn scope_and_discontinuities() {
        let mut a = Analyzer::new(RATE);
        feed(&mut a, 1_024, |f| f as f32 / 10_000.0);
        let s = a.scope(&playing(1_000), 4);
        assert_eq!(s, [0.0996, 0.0997, 0.0998, 0.0999]);
        // A seek (new start frame) resets the history instead of mixing old samples in.
        a.push(&chunk(1, 90_000, |_| 0.25));
        assert_eq!(a.scope(&playing(1_000), 4), [0.0; 4]);
        assert_eq!(a.scope(&playing(90_256), 2), [0.25, 0.25]);
    }
}
