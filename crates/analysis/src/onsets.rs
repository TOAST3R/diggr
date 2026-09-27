//! Onset picking on the band flux curves: adaptive threshold, local maxima, minimum spacing,
//! sub-frame refinement.

use crate::frontend::{FPS, Frame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Kick,
    Snare,
    Hat,
}

impl Band {
    pub fn value(self, f: &Frame) -> f32 {
        match self {
            Band::Kick => f.kick,
            Band::Snare => f.snare,
            Band::Hat => f.hat,
        }
    }

    /// Fraction of the ±30 s band level an onset must exceed. Higher for kicks: basslines share
    /// the kick band and must not read as kicks.
    fn level_ratio(self) -> f32 {
        match self {
            Band::Kick => 0.45,
            Band::Snare | Band::Hat => 0.25,
        }
    }

    fn min_gap(self) -> f64 {
        match self {
            Band::Kick => 0.09,
            Band::Snare => 0.06,
            Band::Hat => 0.05,
        }
    }
}

/// Seconds between the true onset and the frame time where the flux peaks (measured on
/// synthetic drums; subtracted from detected times).
pub const DETECTION_DELAY: f64 = -0.005;

/// Onsets as `(time, strength)`.
pub fn pick(frames: &[Frame], band: Band) -> Vec<(f64, f32)> {
    let x: Vec<f32> = frames.iter().map(|f| band.value(f)).collect();
    let n = x.len();
    if n < 3 {
        return Vec::new();
    }
    let local = 16; // ±186 ms for the median
    // Level of the band over ±30 s: quiet sections (no kick) must not promote weak bumps from
    // bass notes or noise into onsets.
    let wide = (30.0 * FPS) as usize;
    let mut out: Vec<(f64, f32)> = Vec::new();
    let mut sorted = Vec::with_capacity(2 * local + 1);
    let at = |i: isize| {
        if i < 0 || i as usize >= n {
            0.0
        } else {
            x[i as usize]
        }
    };
    for i in 0..n {
        if x[i] < at(i as isize - 1) || x[i] < at(i as isize + 1) {
            continue;
        }
        let (a, b) = (i.saturating_sub(3), (i + 4).min(n));
        if x[a..b].iter().any(|&v| v > x[i]) {
            continue;
        }
        sorted.clear();
        sorted.extend_from_slice(&x[i.saturating_sub(local)..(i + local + 1).min(n)]);
        sorted.sort_by(f32::total_cmp);
        let median = sorted[sorted.len() / 2];
        let level = x[i.saturating_sub(wide)..(i + wide).min(n)]
            .iter()
            .copied()
            .fold(0.0, f32::max);
        if x[i] <= median * 1.5 + band.level_ratio() * level || x[i] < 1e-3 {
            continue;
        }
        // Parabolic peak interpolation for sub-frame timing.
        let (l, c, r) = (at(i as isize - 1), x[i], at(i as isize + 1));
        let denom = l - 2.0 * c + r;
        let shift = if denom.abs() > 1e-9 {
            (0.5 * (l - r) / denom).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let t = frames[i].t + shift as f64 / FPS - DETECTION_DELAY;
        match out.last_mut() {
            Some(last) if t - last.0 < band.min_gap() => {
                if c > last.1 {
                    *last = (t, c);
                }
            }
            _ => out.push((t, c)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{FrontEnd, SR};
    use crate::synth::{self, Pattern, part};

    fn frames(parts: &[synth::Part]) -> (Vec<Frame>, synth::Synth) {
        let s = synth::render(parts, SR, 11);
        let mut fe = FrontEnd::new(0);
        let mut out = Vec::new();
        fe.push(&s.samples, &mut out);
        (out, s)
    }

    /// For each truth time, the error to the nearest detection.
    fn errors(truth: &[f64], found: &[(f64, f32)]) -> Vec<f64> {
        truth
            .iter()
            .map(|&t| {
                found
                    .iter()
                    .map(|&(f, _)| f - t)
                    .min_by(|a, b| a.abs().total_cmp(&b.abs()))
                    .unwrap_or(f64::MAX)
            })
            .collect()
    }

    #[test]
    fn kicks_are_found_within_20_ms() {
        let (fr, s) = frames(&synth::standard_track(128.0));
        let kicks = pick(&fr, Band::Kick);
        let errs = errors(&s.kicks, &kicks);
        let mean = errs.iter().sum::<f64>() / errs.len() as f64;
        let worst = errs.iter().fold(0.0f64, |m, e| m.max(e.abs()));
        eprintln!(
            "kick timing: mean {:.2} ms, worst {:.2} ms, {} found / {} true",
            mean * 1e3,
            worst * 1e3,
            kicks.len(),
            s.kicks.len()
        );
        assert!(worst <= 0.020, "worst kick error {:.1} ms", worst * 1e3);
        // Few spurious kicks (e.g. from bass notes).
        assert!(
            kicks.len() <= s.kicks.len() + s.kicks.len() / 20,
            "{} detections",
            kicks.len()
        );
    }

    #[test]
    fn snares_and_hats_are_found() {
        let (fr, s) = frames(&[part(Pattern::Drop, 8, 128.0)]);
        let beat = 60.0 / 128.0;
        let snare_truth: Vec<f64> = s
            .beats
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .map(|(_, &t)| t)
            .collect();
        let snares = pick(&fr, Band::Snare);
        let errs = errors(&snare_truth, &snares);
        assert!(
            errs.iter().all(|e| e.abs() < 0.025),
            "snare errors {:?}",
            &errs[..4]
        );
        let hats = pick(&fr, Band::Hat);
        let hat_truth: Vec<f64> = s.beats.iter().flat_map(|&t| [t, t + beat / 2.0]).collect();
        let errs = errors(&hat_truth, &hats);
        let found = errs.iter().filter(|e| e.abs() < 0.025).count();
        assert!(
            found as f64 >= 0.9 * hat_truth.len() as f64,
            "{found}/{}",
            hat_truth.len()
        );
    }
}
