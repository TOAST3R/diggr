//! The DJ filter: a resonant low-pass over the whole output, swept from one knob.
//!
//! The knob (0..1) maps logarithmically to 60 Hz..20 kHz, and 1.0 is "off": the samples pass
//! through untouched, not through a filter at 20 kHz, so having the filter costs nothing when
//! it's off. Moving the knob never jumps: its position is smoothed (about 20 ms to arrive),
//! coefficients follow every 32 frames, and going on or off crossfades with the dry signal
//! over 20 ms. Everything is preallocated: this runs in the audio callback.

use crate::ring::CHANNELS;

/// The knob's "off" position.
pub const OFF: f32 = 1.0;
pub const MIN_HZ: f32 = 60.0;
pub const MAX_HZ: f32 = 20_000.0;
/// A slight resonant peak: musical, never self-oscillating.
const Q: f32 = 1.2;
/// Coefficients are recomputed this often (frames).
const UPDATE_EVERY: usize = 32;
/// The knob's smoothing time constant: it is within 2% of where it was turned after 20 ms.
const SMOOTH_SECS: f32 = 0.005;
/// Going on or off crossfades over this long.
const FADE_SECS: f32 = 0.02;

/// The cutoff for a knob position, or `None` when the knob is off.
pub fn cutoff_hz(knob: f32) -> Option<f32> {
    (knob < OFF).then(|| MIN_HZ * (MAX_HZ / MIN_HZ).powf(knob.clamp(0.0, 1.0)))
}

#[derive(Debug, Clone, Copy, Default)]
struct Coefs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

/// RBJ cookbook low-pass.
fn lowpass(hz: f32, sample_rate: f32) -> Coefs {
    let hz = hz.min(sample_rate * 0.45);
    let w0 = std::f32::consts::TAU * hz / sample_rate;
    let (sin, cos) = w0.sin_cos();
    let alpha = sin / (2.0 * Q);
    let a0 = 1.0 + alpha;
    let b1 = (1.0 - cos) / a0;
    Coefs {
        b0: b1 / 2.0,
        b1,
        b2: b1 / 2.0,
        a1: -2.0 * cos / a0,
        a2: (1.0 - alpha) / a0,
    }
}

pub struct LowPass {
    sample_rate: f32,
    /// The smoothed knob position (0..1) the coefficients are for.
    pos: f32,
    /// The wet share: 0 is off (bypassed exactly), 1 is fully filtered.
    mix: f32,
    coefs: Coefs,
    /// Transposed direct form II state, per channel.
    z: [[f32; 2]; CHANNELS],
    /// Frames until the next coefficient update.
    countdown: usize,
}

impl LowPass {
    pub fn new(sample_rate: u32) -> Self {
        let mut f = Self {
            sample_rate: sample_rate.max(1) as f32,
            pos: OFF,
            mix: 0.0,
            coefs: Coefs::default(),
            z: [[0.0; 2]; CHANNELS],
            countdown: 0,
        };
        f.update_coefs();
        f
    }

    /// A new device rate (off the audio thread, while the callback is idle).
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate.max(1) as f32;
        self.update_coefs();
    }

    pub fn is_off(&self) -> bool {
        self.mix == 0.0
    }

    fn update_coefs(&mut self) {
        let hz = MIN_HZ * (MAX_HZ / MIN_HZ).powf(self.pos.clamp(0.0, 1.0));
        self.coefs = lowpass(hz, self.sample_rate);
    }

    /// Filters interleaved stereo `buf` in place, moving toward knob position `target`.
    pub fn process(&mut self, buf: &mut [f32], target: f32) {
        let on = target < OFF;
        if !on && self.mix == 0.0 {
            return; // off: untouched
        }
        if on && self.mix == 0.0 {
            // Coming on: start from the top and sweep down, with fresh state.
            self.pos = OFF;
            self.z = [[0.0; 2]; CHANNELS];
            self.countdown = 0;
        }
        let goal = if on { target.clamp(0.0, 1.0) } else { OFF };
        let follow = 1.0 - (-(UPDATE_EVERY as f32) / (SMOOTH_SECS * self.sample_rate)).exp();
        let fade = 1.0 / (FADE_SECS * self.sample_rate);
        let mix_goal = if on { 1.0 } else { 0.0 };
        for frame in buf.as_chunks_mut::<CHANNELS>().0 {
            if self.countdown == 0 {
                self.pos += (goal - self.pos) * follow;
                self.update_coefs();
                self.countdown = UPDATE_EVERY;
            }
            self.countdown -= 1;
            self.mix = if mix_goal > self.mix {
                (self.mix + fade).min(1.0)
            } else {
                (self.mix - fade).max(0.0)
            };
            let c = self.coefs;
            for (x, z) in frame.iter_mut().zip(&mut self.z) {
                let y = c.b0 * *x + z[0];
                z[0] = c.b1 * *x - c.a1 * y + z[1];
                z[1] = c.b2 * *x - c.a2 * y;
                *x += (y - *x) * self.mix;
            }
        }
        if self.mix == 0.0 {
            self.z = [[0.0; 2]; CHANNELS];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn sine(hz: f32, frames: usize, amp: f32) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let s = amp * (std::f32::consts::TAU * hz * i as f32 / SR as f32).sin();
                [s, s]
            })
            .collect()
    }

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
    }

    /// The largest jump between consecutive samples of the left channel.
    fn max_step(buf: &[f32]) -> f32 {
        buf.as_chunks::<2>()
            .0
            .iter()
            .map(|f| f[0])
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn the_knob_maps_logarithmically_and_full_right_is_off() {
        assert_eq!(cutoff_hz(OFF), None);
        assert!((cutoff_hz(0.0).unwrap() - MIN_HZ).abs() < 0.01);
        assert!((cutoff_hz(0.999_999).unwrap() - MAX_HZ).abs() < 1.0);
        let mid = cutoff_hz(0.5).unwrap();
        assert!((mid - (MIN_HZ * MAX_HZ).sqrt()).abs() < 1.0, "{mid}");
    }

    #[test]
    fn off_is_sample_identical() {
        let mut f = LowPass::new(SR);
        let input: Vec<f32> = (0..4096)
            .map(|i| ((i * 7919) % 2001) as f32 / 1000.0 - 1.0)
            .collect();
        let mut buf = input.clone();
        f.process(&mut buf, OFF);
        assert_eq!(buf, input);
        // And again after having been on and faded out.
        f.process(&mut sine(440.0, 4800, 0.5), 0.3);
        f.process(&mut sine(440.0, 4800, 0.5), OFF);
        assert!(f.is_off());
        let mut buf = input.clone();
        f.process(&mut buf, OFF);
        assert_eq!(buf, input);
    }

    #[test]
    fn a_turn_takes_the_highs_away_within_20_ms() {
        let mut f = LowPass::new(SR);
        let mut buf = sine(5_000.0, SR as usize / 10, 0.5);
        f.process(&mut buf, 0.3); // ~340 Hz
        let at = |ms: usize| ms * SR as usize / 1000 * 2;
        let before = rms(&sine(5_000.0, 480, 0.5));
        let after = rms(&buf[at(20)..at(30)]);
        assert!(after < before * 0.1, "{after} vs {before}");
    }

    #[test]
    fn sweeps_and_switching_never_jump() {
        // A 1 kHz sine moves at most 0.066 per sample; the resonance may add a little.
        let mut f = LowPass::new(SR);
        let limit = 0.12;
        for target in [0.2, 0.9, 0.05, OFF, 0.6, OFF] {
            let mut buf = sine(1_000.0, 4_800, 0.5);
            f.process(&mut buf, target);
            assert!(max_step(&buf) < limit, "to {target}: {}", max_step(&buf));
        }
        // Small blocks, as a device asks for, behave the same.
        let mut f = LowPass::new(SR);
        let mut all = Vec::new();
        for (i, chunk) in sine(1_000.0, 9_600, 0.5).chunks(2 * 64).enumerate() {
            let mut c = chunk.to_vec();
            f.process(&mut c, if i < 40 { 0.4 } else { OFF });
            all.extend(c);
        }
        assert!(max_step(&all) < limit, "{}", max_step(&all));
        assert!(f.is_off());
    }

    #[test]
    fn a_new_sample_rate_keeps_the_cutoff() {
        let mut f = LowPass::new(44_100);
        f.set_sample_rate(SR);
        let mut buf = sine(5_000.0, SR as usize / 10, 0.5);
        f.process(&mut buf, 0.3);
        let tail = &buf[buf.len() - 2 * 480..];
        assert!(rms(tail) < 0.05, "{}", rms(tail));
    }
}
