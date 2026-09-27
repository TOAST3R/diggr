//! Synthetic electronica for tests and evaluation: a 4/4 arrangement of parts (intro, build,
//! drop, …) rendered to mono audio, together with the exact ground truth (beats, downbeats,
//! kicks, section starts). Deterministic for a given seed.

use crate::score::SectionKind;

/// What plays in a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    /// Pads only: no percussion at all.
    Beatless,
    /// Kick on every beat, 8th-note hats, no bass.
    Intro,
    /// No kick; snare roll accelerating from 1/4 to 1/16 notes; rising noise riser.
    Build,
    /// Kick on every beat, snare on 2 and 4, 8th hats, loud bass, pads.
    Drop,
    /// Pads and soft hats, no kick.
    Breakdown,
    /// Kick, hats and quieter bass.
    Groove,
    /// Kick and hats only.
    Outro,
}

impl Pattern {
    /// The section kind a listener would give this part.
    pub fn kind(self) -> SectionKind {
        match self {
            Pattern::Beatless | Pattern::Intro => SectionKind::Intro,
            Pattern::Build => SectionKind::Build,
            Pattern::Drop => SectionKind::Drop,
            Pattern::Breakdown => SectionKind::Breakdown,
            Pattern::Groove => SectionKind::Groove,
            Pattern::Outro => SectionKind::Outro,
        }
    }

    fn has_beat(self) -> bool {
        self != Pattern::Beatless
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Part {
    pub pattern: Pattern,
    pub bars: u32,
    pub bpm: f64,
}

pub fn part(pattern: Pattern, bars: u32, bpm: f64) -> Part {
    Part { pattern, bars, bpm }
}

/// Rendered audio and its ground truth (all times in seconds).
#[derive(Debug, Clone, Default)]
pub struct Synth {
    pub rate: u32,
    pub samples: Vec<f32>,
    /// Every beat of every part that has a beat.
    pub beats: Vec<f64>,
    pub downbeats: Vec<f64>,
    pub kicks: Vec<f64>,
    /// Start time and kind of each part.
    pub sections: Vec<(f64, SectionKind)>,
}

/// A typical track: intro, build, drop, breakdown, build, drop, outro.
pub fn standard_track(bpm: f64) -> Vec<Part> {
    use Pattern::*;
    vec![
        part(Intro, 16, bpm),
        part(Build, 8, bpm),
        part(Drop, 16, bpm),
        part(Breakdown, 16, bpm),
        part(Build, 8, bpm),
        part(Drop, 16, bpm),
        part(Outro, 8, bpm),
    ]
}

struct Rng(u64);

impl Rng {
    fn noise(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 23) as f32 - 1.0
    }
}

const TAU: f32 = std::f32::consts::TAU;

/// Chord roots (semitones above A) per bar, cycling. Drops share one progression, breakdowns
/// another, so repeated sections sound alike and different sections differ.
fn progression(p: Pattern) -> &'static [i32] {
    match p {
        Pattern::Drop | Pattern::Build => &[0, 0, 5, 7],
        Pattern::Breakdown | Pattern::Beatless => &[3, 8, 10, 5],
        _ => &[0, 0, 0, 0],
    }
}

fn hz(semitones_above_a1: i32) -> f32 {
    55.0 * 2f32.powf(semitones_above_a1 as f32 / 12.0)
}

pub fn render(parts: &[Part], rate: u32, seed: u64) -> Synth {
    let total: f64 = parts
        .iter()
        .map(|p| p.bars as f64 * 4.0 * 60.0 / p.bpm)
        .sum();
    let n = (total * rate as f64).ceil() as usize + rate as usize;
    let mut out = vec![0.0f32; n];
    let mut rng = Rng(seed | 1);
    let mut truth = Synth {
        rate,
        ..Default::default()
    };
    let sr = rate as f32;
    let mut t0 = 0.0f64;
    let mut riser_lp = 0.0f32;

    for p in parts {
        let beat = 60.0 / p.bpm;
        let beats = p.bars * 4;
        truth.sections.push((t0, p.pattern.kind()));
        let start = (t0 * rate as f64) as usize;
        let len = (beats as f64 * beat * rate as f64) as usize;

        // Continuous layers: pads (and the build's riser).
        for i in 0..len {
            let t = i as f32 / sr;
            let bar = (t as f64 / (beat * 4.0)) as usize;
            let root = progression(p.pattern)[bar % 4];
            let pad_gain = match p.pattern {
                Pattern::Beatless | Pattern::Breakdown => 0.12,
                Pattern::Drop => 0.05,
                Pattern::Build => 0.06,
                _ => 0.0,
            };
            let mut v = 0.0;
            if pad_gain > 0.0 {
                for iv in [24, 28, 31] {
                    v += (TAU * hz(root + iv) * t).sin() * pad_gain / 3.0;
                }
            }
            if p.pattern == Pattern::Build {
                // Noise riser that gets louder and brighter towards the drop.
                let prog = i as f32 / len as f32;
                let open = 0.02 + 0.6 * prog;
                riser_lp += open * (rng.noise() - riser_lp);
                v += riser_lp * (0.05 + 0.35 * prog);
            }
            out[start + i] += v;
        }

        if !p.pattern.has_beat() {
            t0 += beats as f64 * beat;
            continue;
        }

        for b in 0..beats {
            let bt = t0 + b as f64 * beat;
            truth.beats.push(bt);
            if b % 4 == 0 {
                truth.downbeats.push(bt);
            }
            let bar = (b / 4) as usize;
            let pos = b % 4;
            let at = |secs: f64| (secs * rate as f64) as usize;
            let kick = matches!(
                p.pattern,
                Pattern::Intro | Pattern::Drop | Pattern::Groove | Pattern::Outro
            );
            if kick {
                truth.kicks.push(bt);
                add_kick(&mut out, at(bt), sr);
            }
            // Hats on 8ths (softer in breakdowns).
            let hat_gain = match p.pattern {
                Pattern::Breakdown => 0.08,
                Pattern::Build => 0.0,
                _ => 0.2,
            };
            if hat_gain > 0.0 {
                for half in 0..2 {
                    add_hat(
                        &mut out,
                        at(bt + half as f64 * beat / 2.0),
                        sr,
                        hat_gain,
                        &mut rng,
                    );
                }
            }
            if p.pattern == Pattern::Drop && (pos == 1 || pos == 3) {
                add_snare(&mut out, at(bt), sr, 0.5, &mut rng);
            }
            if p.pattern == Pattern::Build {
                // 1/4 notes, then 1/8, then 1/16 over the part.
                let q = b as f32 / beats as f32;
                let div = if q < 0.5 {
                    1
                } else if q < 0.75 {
                    2
                } else {
                    4
                };
                for k in 0..div {
                    let g = 0.15 + 0.35 * q;
                    add_snare(
                        &mut out,
                        at(bt + k as f64 * beat / div as f64),
                        sr,
                        g,
                        &mut rng,
                    );
                }
            }
            let bass_gain = match p.pattern {
                Pattern::Drop => 0.35,
                Pattern::Groove => 0.2,
                _ => 0.0,
            };
            if bass_gain > 0.0 {
                let f = hz(progression(p.pattern)[bar % 4]);
                let s = at(bt + beat * 0.25);
                let l = at(beat * 0.6);
                for i in 0..l {
                    let t = i as f32 / sr;
                    out[s + i] += (TAU * f * t).sin() * bass_gain * (1.0 - i as f32 / l as f32);
                }
            }
        }
        t0 += beats as f64 * beat;
    }
    for s in &mut out {
        *s = s.clamp(-1.0, 1.0);
    }
    out.truncate((total * rate as f64) as usize);
    truth.samples = out;
    truth
}

fn add_kick(out: &mut [f32], at: usize, sr: f32) {
    let len = (0.25 * sr) as usize;
    let mut phase = 0.0f32;
    for i in 0..len.min(out.len().saturating_sub(at)) {
        let t = i as f32 / sr;
        let f = 50.0 + 110.0 * (-t * 30.0).exp();
        phase += TAU * f / sr;
        out[at + i] += phase.sin() * (-t * 12.0).exp() * 0.9;
    }
}

fn add_snare(out: &mut [f32], at: usize, sr: f32, gain: f32, rng: &mut Rng) {
    let len = (0.18 * sr) as usize;
    for i in 0..len.min(out.len().saturating_sub(at)) {
        let t = i as f32 / sr;
        let body = (TAU * 190.0 * t).sin() * (-t * 22.0).exp() * 0.5;
        let noise = rng.noise() * (-t * 20.0).exp();
        out[at + i] += (body + noise) * gain;
    }
}

fn add_hat(out: &mut [f32], at: usize, sr: f32, gain: f32, rng: &mut Rng) {
    let len = (0.04 * sr) as usize;
    let mut prev = 0.0;
    for i in 0..len.min(out.len().saturating_sub(at)) {
        let t = i as f32 / sr;
        let n = rng.noise();
        out[at + i] += (n - prev) * (-t * 90.0).exp() * gain;
        prev = n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_truth_matches_the_arrangement() {
        let s = render(&standard_track(128.0), 22_050, 7);
        let beat = 60.0 / 128.0;
        assert_eq!(s.beats.len(), 88 * 4);
        assert_eq!(s.downbeats.len(), 88);
        assert!((s.beats[1] - beat).abs() < 1e-9);
        assert_eq!(s.sections.len(), 7);
        assert_eq!(s.sections[2].1, SectionKind::Drop);
        assert!((s.sections[2].0 - 24.0 * 4.0 * beat).abs() < 1e-9);
        // Builds and breakdowns have no kicks.
        assert_eq!(s.kicks.len(), (16 + 16 + 16 + 8) * 4);
        assert!((s.samples.len() as f64 / 22_050.0 - 88.0 * 4.0 * beat).abs() < 0.01);
        assert!(s.samples.iter().all(|x| x.abs() <= 1.0));
    }

    #[test]
    fn deterministic_per_seed() {
        let a = render(&standard_track(124.0), 22_050, 3);
        let b = render(&standard_track(124.0), 22_050, 3);
        assert_eq!(a.samples, b.samples);
    }
}
