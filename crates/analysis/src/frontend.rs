//! Feature extraction: mono audio at 22,050 Hz → STFT frames (1024-sample Hann window,
//! 256-sample hop ≈ 11.6 ms) with band onset strengths and timbre/harmony features.

use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

pub const SR: u32 = 22_050;
pub const N: usize = 1024;
pub const HOP: usize = 256;
pub const FPS: f64 = SR as f64 / HOP as f64;
pub const MEL_BANDS: usize = 32;

const LOG_GAIN: f32 = 10.0;

/// Features of one STFT frame. `t` is the time of the window center in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub t: f64,
    /// Half-wave rectified log-spectral flux in the kick (40–120 Hz), snare (150–400 Hz and
    /// 2–5 kHz) and hat (> 6 kHz) bands, and over the whole spectrum.
    pub kick: f32,
    pub snare: f32,
    pub hat: f32,
    pub full: f32,
    pub mel: [f32; MEL_BANDS],
    pub chroma: [f32; 12],
    pub rms: f32,
    /// Spectral centroid in Hz (brightness).
    pub centroid: f32,
    /// Spectral flatness 0..1 (1 = noise-like).
    pub flatness: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
}

fn bin(hz: f32) -> usize {
    ((hz * N as f32 / SR as f32).round() as usize).clamp(1, N / 2)
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10f32.powf(mel / 2595.0) - 1.0)
}

pub struct FrontEnd {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    /// Samples not yet consumed; `pending[0]` is sample `pending_start` of the file (negative
    /// while the half-window of leading silence is pending).
    pending: Vec<f32>,
    pending_start: i64,
    prev_log: Vec<f32>,
    mel_filters: Vec<Vec<(usize, f32)>>,
    chroma_bins: Vec<(usize, usize)>,
}

impl FrontEnd {
    /// `start_sample` is the file position (at 22,050 Hz) of the first sample pushed. At the
    /// start of a file, half a window of silence is prepended so the first frame is centered at
    /// t = 0 and an onset right at the start can be located.
    pub fn new(start_sample: u64) -> Self {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(N);
        let window = (0..N)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos())
            .collect();
        let (lo, hi) = (hz_to_mel(30.0), hz_to_mel(10_000.0));
        let edges: Vec<f32> = (0..MEL_BANDS + 2)
            .map(|i| {
                mel_to_hz(lo + (hi - lo) * i as f32 / (MEL_BANDS + 1) as f32) * N as f32 / SR as f32
            })
            .collect();
        let mel_filters = (0..MEL_BANDS)
            .map(|m| {
                let (a, c, b) = (edges[m], edges[m + 1], edges[m + 2]);
                (a.floor() as usize..=b.ceil() as usize)
                    .filter(|&k| (1..=N / 2).contains(&k))
                    .filter_map(|k| {
                        let x = k as f32;
                        let w = if x < c {
                            (x - a) / (c - a)
                        } else {
                            (b - x) / (b - c)
                        };
                        (w > 0.0).then_some((k, w))
                    })
                    .collect()
            })
            .collect();
        let chroma_bins = (bin(80.0)..=bin(5_000.0))
            .map(|k| {
                let hz = k as f32 * SR as f32 / N as f32;
                let midi = 69.0 + 12.0 * (hz / 440.0).log2();
                (k, (midi.round() as i32).rem_euclid(12) as usize)
            })
            .collect();
        Self {
            input: fft.make_input_vec(),
            spectrum: fft.make_output_vec(),
            fft,
            window,
            pending: if start_sample == 0 {
                vec![0.0; N / 2]
            } else {
                Vec::new()
            },
            pending_start: start_sample as i64 - if start_sample == 0 { N as i64 / 2 } else { 0 },
            prev_log: vec![0.0; N / 2 + 1],
            mel_filters,
            chroma_bins,
        }
    }

    /// Adds samples and appends every complete frame to `out`.
    pub fn push(&mut self, samples: &[f32], out: &mut Vec<Frame>) {
        self.pending.extend_from_slice(samples);
        let mut used = 0;
        while self.pending.len() - used >= N {
            let frame = &self.pending[used..used + N];
            let t = (self.pending_start + used as i64 + N as i64 / 2) as f64 / SR as f64;
            let mut sq = 0.0;
            for ((x, s), w) in self.input.iter_mut().zip(frame).zip(&self.window) {
                *x = s * w;
                sq += s * s;
            }
            let rms = (sq / N as f32).sqrt();
            self.fft
                .process(&mut self.input, &mut self.spectrum)
                .expect("fft sizes");
            out.push(self.features(t, rms));
            used += HOP;
        }
        self.pending.drain(..used);
        self.pending_start += used as i64;
    }

    fn features(&mut self, t: f64, rms: f32) -> Frame {
        let mag: Vec<f32> = self
            .spectrum
            .iter()
            .map(|c| c.norm() / (N as f32 / 4.0))
            .collect();
        let flux = |prev: &[f32], lo: usize, hi: usize| -> f32 {
            (lo..=hi)
                .map(|k| ((1.0 + LOG_GAIN * mag[k]).ln() - prev[k]).max(0.0))
                .sum::<f32>()
                / (hi - lo + 1) as f32
        };
        let kick = flux(&self.prev_log, bin(40.0), bin(120.0));
        let snare = 0.5 * flux(&self.prev_log, bin(150.0), bin(400.0))
            + 0.5 * flux(&self.prev_log, bin(2_000.0), bin(5_000.0));
        let hat = flux(&self.prev_log, bin(6_000.0), N / 2);
        let full = flux(&self.prev_log, 1, N / 2);
        for (p, m) in self.prev_log.iter_mut().zip(&mag) {
            *p = (1.0 + LOG_GAIN * m).ln();
        }

        let mut mel = [0.0; MEL_BANDS];
        for (m, filt) in mel.iter_mut().zip(&self.mel_filters) {
            let e: f32 = filt.iter().map(|&(k, w)| w * mag[k] * mag[k]).sum();
            *m = (1e-8 + e).ln();
        }
        let mut chroma = [0.0; 12];
        for &(k, c) in &self.chroma_bins {
            chroma[c] += mag[k] * mag[k];
        }
        let total: f32 = chroma.iter().sum::<f32>().max(1e-12);
        chroma.iter_mut().for_each(|c| *c /= total);

        let power: Vec<f32> = mag.iter().map(|m| m * m + 1e-12).collect();
        let sum: f32 = power[1..].iter().sum();
        let centroid = power[1..]
            .iter()
            .enumerate()
            .map(|(i, p)| (i + 1) as f32 * p)
            .sum::<f32>()
            / sum
            * SR as f32
            / N as f32;
        let n = (power.len() - 1) as f32;
        let geo = (power[1..].iter().map(|p| p.ln()).sum::<f32>() / n).exp();
        let flatness = (geo / (sum / n)).clamp(0.0, 1.0);
        let band = |lo: f32, hi: f32| (1e-8 + power[bin(lo)..=bin(hi)].iter().sum::<f32>()).ln();
        Frame {
            t,
            kick,
            snare,
            hat,
            full,
            mel,
            chroma,
            rms,
            centroid,
            flatness,
            bass: band(20.0, 250.0),
            mid: band(250.0, 4_000.0),
            treble: band(4_000.0, 11_000.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(samples: &[f32]) -> Vec<Frame> {
        let mut fe = FrontEnd::new(0);
        let mut out = Vec::new();
        for chunk in samples.chunks(3000) {
            fe.push(chunk, &mut out);
        }
        out
    }

    fn sine(hz: f32, secs: f32) -> Vec<f32> {
        (0..(secs * SR as f32) as usize)
            .map(|i| (std::f32::consts::TAU * hz * i as f32 / SR as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn frames_are_evenly_timed_regardless_of_chunking() {
        let s = sine(440.0, 2.0);
        let frames = run(&s);
        assert_eq!(frames.len(), (s.len() + N / 2 - N) / HOP + 1);
        assert_eq!(frames[0].t, 0.0, "first frame centered on the first sample");
        assert!((frames[1].t - frames[0].t - HOP as f64 / SR as f64).abs() < 1e-9);
        let mut fe = FrontEnd::new(0);
        let mut one = Vec::new();
        fe.push(&s, &mut one);
        assert_eq!(one, frames, "chunk size does not change results");
    }

    #[test]
    fn centroid_chroma_and_flatness() {
        let f = &run(&sine(1_000.0, 1.0))[20];
        assert!((f.centroid - 1_000.0).abs() < 60.0, "{}", f.centroid);
        let top = (0..12)
            .max_by(|&a, &b| f.chroma[a].total_cmp(&f.chroma[b]))
            .unwrap();
        assert_eq!(top, 11, "1 kHz is closest to B"); // C=0 … B=11
        assert!(f.flatness < 0.1);
        let mut x = 1u64;
        let noise: Vec<f32> = (0..SR)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 40) as f32 / (1u64 << 23) as f32 - 1.0
            })
            .collect();
        assert!(run(&noise)[20].flatness > 0.4);
    }

    #[test]
    fn a_kick_shows_up_in_the_kick_band_only() {
        let s = crate::synth::render(
            &[crate::synth::part(crate::synth::Pattern::Outro, 1, 120.0)],
            SR,
            1,
        );
        let frames = run(&s.samples);
        let near_first_kick = frames
            .iter()
            .filter(|f| f.t < 0.1)
            .map(|f| f.kick)
            .fold(0.0, f32::max);
        let between = frames
            .iter()
            .filter(|f| f.t > 0.3 && f.t < 0.45)
            .map(|f| f.kick)
            .fold(0.0, f32::max);
        assert!(
            near_first_kick > 5.0 * between,
            "{near_first_kick} vs {between}"
        );
    }
}
