//! 10-band graphic equalizer + preamp: RBJ peaking biquads, Direct Form II Transposed.
//!
//! Coefficients are computed off the audio thread ([`EqCoefs::from_settings`]) and handed to the
//! callback, which ramps from the old to the new coefficients over ~5 ms to avoid zipper noise.

use serde::{Deserialize, Serialize};

use crate::ring::CHANNELS;

pub const BAND_COUNT: usize = 10;
/// Classic 10-band graphic EQ centers.
pub const BAND_HZ: [f32; BAND_COUNT] = [
    60.0, 170.0, 310.0, 600.0, 1_000.0, 3_000.0, 6_000.0, 12_000.0, 14_000.0, 16_000.0,
];
pub const MAX_DB: f32 = 12.0;
const Q: f32 = std::f32::consts::SQRT_2;
const RAMP_SECONDS: f32 = 0.005;
/// Bands at or above this fraction of the sample rate are bypassed (Nyquist is 0.5).
const MAX_CENTER_RATIO: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EqSettings {
    pub enabled: bool,
    pub preamp_db: f32,
    pub bands_db: [f32; BAND_COUNT],
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            preamp_db: 0.0,
            bands_db: [0.0; BAND_COUNT],
        }
    }
}

impl EqSettings {
    pub fn clamped(mut self) -> Self {
        self.preamp_db = self.preamp_db.clamp(-MAX_DB, MAX_DB);
        for b in &mut self.bands_db {
            *b = b.clamp(-MAX_DB, MAX_DB);
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Biquad {
    const IDENTITY: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    fn peaking(sample_rate: f32, f0: f32, gain_db: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * f0 / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * Q);
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }

    fn lerp(&self, to: &Self, t: f32) -> Self {
        let l = |x: f32, y: f32| x + (y - x) * t;
        Self {
            b0: l(self.b0, to.b0),
            b1: l(self.b1, to.b1),
            b2: l(self.b2, to.b2),
            a1: l(self.a1, to.a1),
            a2: l(self.a2, to.a2),
        }
    }
}

/// Everything the callback needs; `Copy` so it can travel through an SPSC ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqCoefs {
    pub enabled: bool,
    preamp: f32,
    bands: [Biquad; BAND_COUNT],
    active: [bool; BAND_COUNT],
}

impl EqCoefs {
    pub fn from_settings(s: &EqSettings, sample_rate: u32) -> Self {
        let s = s.clamped();
        let fs = sample_rate as f32;
        let mut bands = [Biquad::IDENTITY; BAND_COUNT];
        let mut active = [false; BAND_COUNT];
        for i in 0..BAND_COUNT {
            if BAND_HZ[i] < fs * MAX_CENTER_RATIO {
                bands[i] = Biquad::peaking(fs, BAND_HZ[i], s.bands_db[i]);
                active[i] = true;
            }
        }
        Self {
            enabled: s.enabled,
            preamp: 10f32.powf(s.preamp_db / 20.0),
            bands,
            active,
        }
    }

    /// Which bands are processed at this sample rate.
    pub fn active_bands(&self) -> [bool; BAND_COUNT] {
        self.active
    }
}

/// The real-time part. Never allocates.
pub struct Equalizer {
    current: EqCoefs,
    ramp_from: EqCoefs,
    ramp_pos: u32,
    ramp_len: u32,
    state: [[[f32; 2]; CHANNELS]; BAND_COUNT],
}

impl Equalizer {
    pub fn new(coefs: EqCoefs, sample_rate: u32) -> Self {
        Self {
            current: coefs,
            ramp_from: coefs,
            ramp_pos: 0,
            ramp_len: ((sample_rate as f32 * RAMP_SECONDS) as u32).max(1),
            state: [[[0.0; 2]; CHANNELS]; BAND_COUNT],
        }
    }

    pub fn set(&mut self, coefs: EqCoefs) {
        if coefs.enabled != self.current.enabled || coefs.active != self.current.active {
            // Switching on/off or changing the band layout: jump, starting from clean state.
            self.state = [[[0.0; 2]; CHANNELS]; BAND_COUNT];
            self.current = coefs;
            self.ramp_from = coefs;
            self.ramp_pos = 0;
            return;
        }
        // Start the new ramp from wherever the old one had got to.
        self.ramp_from = self.effective(self.ramp_progress());
        self.current = coefs;
        self.ramp_pos = self.ramp_len;
    }

    /// Resets the ramp length for a new sample rate (call with coefficients for that rate).
    pub fn reconfigure(&mut self, coefs: EqCoefs, sample_rate: u32) {
        *self = Self::new(coefs, sample_rate);
    }

    fn ramp_progress(&self) -> f32 {
        1.0 - self.ramp_pos as f32 / self.ramp_len as f32
    }

    fn effective(&self, t: f32) -> EqCoefs {
        if t >= 1.0 {
            return self.current;
        }
        let mut c = self.current;
        c.preamp = self.ramp_from.preamp + (self.current.preamp - self.ramp_from.preamp) * t;
        for i in 0..BAND_COUNT {
            c.bands[i] = self.ramp_from.bands[i].lerp(&self.current.bands[i], t);
        }
        c
    }

    /// Processes interleaved stereo in place.
    pub fn process(&mut self, buf: &mut [f32]) {
        if !self.current.enabled {
            return;
        }
        let mut coefs = self.current;
        for frame in buf.as_chunks_mut::<CHANNELS>().0 {
            if self.ramp_pos > 0 {
                self.ramp_pos -= 1;
                coefs = self.effective(self.ramp_progress());
            }
            for (ch, sample) in frame.iter_mut().enumerate() {
                let mut x = *sample * coefs.preamp;
                for b in 0..BAND_COUNT {
                    if !coefs.active[b] {
                        continue;
                    }
                    let q = &coefs.bands[b];
                    let s = &mut self.state[b][ch];
                    let y = q.b0 * x + s[0];
                    s[0] = q.b1 * x - q.a1 * y + s[1];
                    s[1] = q.b2 * x - q.a2 * y;
                    x = y;
                }
                *sample = x;
            }
        }
    }
}

/// A named set of EQ values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqPreset {
    pub name: String,
    pub preamp_db: f32,
    pub bands_db: [f32; BAND_COUNT],
}

impl EqPreset {
    /// Applies this preset's values, keeping the on/off state.
    pub fn apply_to(&self, settings: &EqSettings) -> EqSettings {
        EqSettings {
            enabled: settings.enabled,
            preamp_db: self.preamp_db,
            bands_db: self.bands_db,
        }
        .clamped()
    }

    pub fn from_settings(name: impl Into<String>, s: &EqSettings) -> Self {
        Self {
            name: name.into(),
            preamp_db: s.preamp_db,
            bands_db: s.bands_db,
        }
    }
}

/// Built-in presets in the spirit of classic players' defaults.
pub fn builtin_presets() -> Vec<EqPreset> {
    let p = |name: &str, bands_db: [f32; BAND_COUNT]| EqPreset {
        name: name.into(),
        preamp_db: 0.0,
        bands_db,
    };
    vec![
        p("Flat", [0.0; BAND_COUNT]),
        p(
            "Rock",
            [4.8, 2.9, -3.4, -4.8, -1.9, 2.4, 5.3, 6.7, 6.7, 6.7],
        ),
        p(
            "Pop",
            [-1.0, 2.9, 4.3, 4.8, 3.4, 0.0, -1.4, -1.4, -1.0, -1.0],
        ),
        p(
            "Dance",
            [5.8, 4.3, 1.4, 0.0, 0.0, -3.4, -4.3, -4.3, 0.0, 0.0],
        ),
        p(
            "Techno",
            [4.8, 3.4, 0.0, -3.4, -2.9, 0.0, 4.8, 5.8, 5.8, 5.3],
        ),
        p(
            "Full Bass",
            [5.8, 5.8, 5.8, 3.4, 1.0, -2.4, -4.8, -6.3, -6.7, -6.7],
        ),
        p(
            "Full Treble",
            [-5.8, -5.8, -5.8, -2.4, 1.4, 6.7, 9.6, 9.6, 9.6, 10.1],
        ),
    ]
}

/// An editable preset library, persisted as RON by the caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqPresets {
    pub presets: Vec<EqPreset>,
}

impl Default for EqPresets {
    fn default() -> Self {
        Self {
            presets: builtin_presets(),
        }
    }
}

impl EqPresets {
    pub fn get(&self, name: &str) -> Option<&EqPreset> {
        self.presets
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Adds or replaces a preset by name.
    pub fn save(&mut self, preset: EqPreset) {
        match self
            .presets
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(&preset.name))
        {
            Some(existing) => *existing = preset,
            None => self.presets.push(preset),
        }
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.presets.len();
        self.presets.retain(|p| !p.name.eq_ignore_ascii_case(name));
        self.presets.len() != before
    }

    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: u32 = 48_000;

    fn sine(freq: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = (2.0 * std::f32::consts::PI * freq * i as f32 / FS as f32).sin() * 0.25;
                [v, v]
            })
            .collect()
    }

    fn rms_db(buf: &[f32]) -> f32 {
        let tail = &buf[buf.len() / 2..]; // skip the filter's settling time
        let ms = tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32;
        10.0 * ms.log10()
    }

    fn gain_at(settings: &EqSettings, freq: f32) -> f32 {
        let mut eq = Equalizer::new(EqCoefs::from_settings(settings, FS), FS);
        let input = sine(freq, FS as usize);
        let mut out = input.clone();
        eq.process(&mut out);
        rms_db(&out) - rms_db(&input)
    }

    #[test]
    fn band_boost_is_accurate_at_center() {
        for (i, &hz) in BAND_HZ.iter().enumerate().take(7) {
            let mut s = EqSettings {
                enabled: true,
                ..Default::default()
            };
            s.bands_db[i] = 12.0;
            let g = gain_at(&s, hz);
            assert!((g - 12.0).abs() <= 0.5, "band {hz} Hz gave {g:.2} dB");
        }
    }

    #[test]
    fn cut_and_preamp() {
        let mut s = EqSettings {
            enabled: true,
            ..Default::default()
        };
        s.bands_db[4] = -12.0;
        assert!((gain_at(&s, 1_000.0) + 12.0).abs() <= 0.5);
        let s = EqSettings {
            enabled: true,
            preamp_db: 6.0,
            ..Default::default()
        };
        assert!((gain_at(&s, 440.0) - 6.0).abs() <= 0.1);
    }

    #[test]
    fn disabled_is_bit_identical() {
        let mut s = EqSettings {
            enabled: false,
            preamp_db: 12.0,
            ..Default::default()
        };
        s.bands_db = [12.0; BAND_COUNT];
        let mut eq = Equalizer::new(EqCoefs::from_settings(&s, FS), FS);
        let input = sine(1_000.0, 4_096);
        let mut out = input.clone();
        eq.process(&mut out);
        assert_eq!(out, input);
    }

    #[test]
    fn bands_above_nyquist_are_bypassed() {
        let s = EqSettings {
            enabled: true,
            bands_db: [12.0; BAND_COUNT],
            ..Default::default()
        };
        let c = EqCoefs::from_settings(&s, 22_050);
        assert_eq!(&c.active_bands()[7..], &[false, false, false]);
        assert!(c.active_bands()[..7].iter().all(|&a| a));
        // And it stays stable.
        let mut eq = Equalizer::new(c, 22_050);
        let mut buf: Vec<f32> = (0..44_100)
            .map(|i| ((i * 7919) % 200) as f32 / 100.0 - 1.0)
            .collect();
        eq.process(&mut buf);
        assert!(buf.iter().all(|x| x.is_finite() && x.abs() < 100.0));
    }

    #[test]
    fn sweeping_a_band_has_no_clicks() {
        // A slow sine keeps sample-to-sample steps small; a click would show up as a big jump.
        let mut eq = Equalizer::new(
            EqCoefs::from_settings(
                &EqSettings {
                    enabled: true,
                    ..Default::default()
                },
                FS,
            ),
            FS,
        );
        let mut buf = sine(100.0, FS as usize);
        let block = 256 * CHANNELS;
        let mut max_step = 0f32;
        let mut prev = 0f32;
        for (n, chunk) in buf.chunks_mut(block).enumerate() {
            // Jump the slider between extremes every block (as fast as a UI could).
            let db = if n % 2 == 0 { 12.0 } else { -12.0 };
            let mut s = EqSettings {
                enabled: true,
                ..Default::default()
            };
            s.bands_db[0] = db;
            s.bands_db[1] = db;
            eq.set(EqCoefs::from_settings(&s, FS));
            eq.process(chunk);
            for f in chunk.chunks(CHANNELS) {
                max_step = max_step.max((f[0] - prev).abs());
                prev = f[0];
            }
        }
        // The unfiltered 100 Hz sine at 0.25 amplitude steps at most ~0.0033 per sample; even
        // with +12 dB of gain the step stays well below a click.
        assert!(max_step < 0.05, "max step {max_step}");
    }

    #[test]
    fn presets_round_trip_and_edit() {
        let mut lib = EqPresets::default();
        let names: Vec<_> = lib.presets.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Flat",
                "Rock",
                "Pop",
                "Dance",
                "Techno",
                "Full Bass",
                "Full Treble"
            ]
        );
        let techno = lib.get("techno").unwrap().apply_to(&EqSettings::default());
        assert_eq!(techno.bands_db, lib.get("Techno").unwrap().bands_db);

        let mine = EqSettings {
            enabled: true,
            preamp_db: -3.0,
            bands_db: [1.0; BAND_COUNT],
        };
        lib.save(EqPreset::from_settings("My Club", &mine));
        let restored = EqPresets::from_ron(&lib.to_ron().unwrap()).unwrap();
        assert_eq!(restored, lib);
        assert_eq!(restored.get("My Club").unwrap().preamp_db, -3.0);
        assert!(lib.remove("My Club"));
        assert!(lib.get("My Club").is_none());
    }
}
