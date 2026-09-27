//! Tempo, beat grid, tempo segments, confidence, and downbeats.
//!
//! Electronica is quantized, so instead of tracking beats one by one we fit a straight line
//! `beat_n = t0 + n · period` to the onsets in overlapping windows, and merge windows that agree
//! into tempo segments. Windows without enough evidence (a breakdown with soft hats) keep the
//! running grid if the next evidence agrees; a region with no grid at all is beatless.

use crate::frontend::{FPS, Frame};
use crate::onsets::{self, Band};
use crate::score::TempoSegment;

pub const MIN_BPM: f64 = 60.0;
pub const MAX_BPM: f64 = 200.0;
/// Center of the tempo prior (log-Gaussian, one octave ≈ 2σ).
pub const PRIOR_BPM: f64 = 125.0;

const WINDOW_SECS: f64 = 16.0;
const STEP_SECS: f64 = 4.0;

#[derive(Debug, Clone, Default)]
pub struct Onsets {
    pub kick: Vec<(f64, f32)>,
    pub snare: Vec<(f64, f32)>,
    pub hat: Vec<(f64, f32)>,
}

pub fn detect(frames: &[Frame]) -> Onsets {
    Onsets {
        kick: onsets::pick(frames, Band::Kick),
        snare: onsets::pick(frames, Band::Snare),
        hat: onsets::pick(frames, Band::Hat),
    }
}

impl Onsets {
    /// Weighted onsets used to fit the grid (kicks count most, hats least).
    fn weighted(&self, a: f64, b: f64) -> Vec<(f64, f64)> {
        let mut v: Vec<(f64, f64)> = Vec::new();
        for (list, w) in [(&self.kick, 1.0), (&self.snare, 0.6), (&self.hat, 0.25)] {
            v.extend(
                list.iter()
                    .filter(|(t, _)| *t >= a && *t < b)
                    .map(|&(t, _)| (t, w)),
            );
        }
        v.sort_by(|x, y| x.0.total_cmp(&y.0));
        v
    }
}

/// Onset-strength envelope for tempo estimation, with the slow trend removed.
fn envelope(frames: &[Frame]) -> Vec<f64> {
    let raw: Vec<f64> = frames
        .iter()
        .map(|f| (f.kick + 0.6 * f.snare + 0.4 * f.hat) as f64)
        .collect();
    let w = FPS as usize; // 1 s moving average
    let mut out = Vec::with_capacity(raw.len());
    let mut sum = 0.0;
    for i in 0..raw.len() {
        sum += raw[i];
        if i >= w {
            sum -= raw[i - w];
        }
        let mean = sum / (i + 1).min(w) as f64;
        out.push((raw[i] - mean).max(0.0));
    }
    out
}

fn prior(bpm: f64) -> f64 {
    let x = (bpm / PRIOR_BPM).log2() / 0.5;
    (-0.5 * x * x).exp()
}

fn autocorr(env: &[f64], lag: f64) -> f64 {
    let n = env.len();
    let l0 = lag.floor() as usize;
    let frac = lag - l0 as f64;
    if l0 + 1 >= n {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n - l0 - 1 {
        let v = env[i + l0] * (1.0 - frac) + env[i + l0 + 1] * frac;
        s += env[i] * v;
    }
    s / (n - l0) as f64
}

/// Tempo (BPM) and a 0..1 periodicity strength for a stretch of frames.
pub fn tempo(frames: &[Frame], onsets: &Onsets) -> Option<(f64, f32)> {
    let env = envelope(frames);
    let energy = autocorr(&env, 0.0);
    if env.len() < (4.0 * FPS) as usize || energy <= 1e-9 {
        return None;
    }
    let score = |bpm: f64| {
        let lag = FPS * 60.0 / bpm;
        // Reinforce with the bar (4 beats) and half-beat so the fundamental beat wins.
        autocorr(&env, lag) + 0.5 * autocorr(&env, 2.0 * lag) + 0.25 * autocorr(&env, 4.0 * lag)
    };
    let mut best = (0.0, 0.0);
    let mut bpm = MIN_BPM;
    while bpm <= MAX_BPM {
        let s = score(bpm) * prior(bpm);
        if s > best.1 {
            best = (bpm, s);
        }
        bpm += 0.25;
    }
    let (mut bpm, _) = best;
    // Octave check with hat density: at the true tempo, 8th/16th hats give 2–4 hats per beat;
    // more than that means we picked half the tempo.
    let secs = frames.last()?.t - frames.first()?.t;
    let hats_per_beat = onsets
        .hat
        .iter()
        .filter(|(t, _)| *t >= frames[0].t && *t <= frames[0].t + secs)
        .count() as f64
        / (secs * bpm / 60.0);
    if hats_per_beat > 4.5 && bpm * 2.0 <= MAX_BPM {
        bpm *= 2.0;
    }
    // Refine to 0.05 BPM around the winner.
    let mut fine = (bpm, score(bpm));
    let mut b = bpm - 0.5;
    while b <= bpm + 0.5 {
        let s = score(b);
        if s > fine.1 {
            fine = (b, s);
        }
        b += 0.05;
    }
    let strength = (score(fine.0) / energy).clamp(0.0, 1.0) as f32;
    Some((fine.0, strength))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub t0: f64,
    pub period: f64,
    /// RMS timing error of the onsets that fit the grid, in seconds.
    pub residual: f64,
    /// Share of on-beat onset weight within 30 ms of the grid.
    pub fit: f64,
    /// Weight of onsets that support the grid.
    pub support: f64,
}

/// Robust straight-line fit of `t0 + n · period` to weighted onsets, starting from `period`.
pub fn fit_grid(onsets: &[(f64, f64)], period: f64) -> Option<Grid> {
    if onsets.len() < 4 {
        return None;
    }
    // Phase: the offset that puts most onset weight near grid lines.
    let mut best = (0.0, -1.0);
    let steps = 200;
    for k in 0..steps {
        let phase = period * k as f64 / steps as f64;
        let s: f64 = onsets
            .iter()
            .map(|&(t, w)| {
                let d = ((t - phase) / period).fract().rem_euclid(1.0);
                let d = d.min(1.0 - d) * period;
                w * (-(d * d) / (2.0 * 0.012f64.powi(2))).exp()
            })
            .sum();
        if s > best.1 {
            best = (phase, s);
        }
    }
    let (mut t0, mut p) = (best.0, period);
    let origin = onsets[0].0;
    t0 += ((origin - t0) / p).round() * p; // anchor near the data for numerical stability
    for _ in 0..6 {
        // Weighted least squares on (n, t) with Huber weights, on-beat onsets only.
        let (mut sw, mut sn, mut st, mut snn, mut snt) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for &(t, w) in onsets {
            let n = ((t - t0) / p).round();
            let r = t - (t0 + n * p);
            if r.abs() > 0.2 * p {
                continue;
            }
            let huber = if r.abs() <= 0.015 {
                1.0
            } else {
                0.015 / r.abs()
            };
            let w = w * huber;
            sw += w;
            sn += w * n;
            st += w * t;
            snn += w * n * n;
            snt += w * n * t;
        }
        let det = sw * snn - sn * sn;
        if sw <= 0.0 || det.abs() < 1e-9 {
            return None;
        }
        p = (sw * snt - sn * st) / det;
        t0 = (st - p * sn) / sw;
    }
    let (mut on, mut total, mut sq, mut cnt) = (0.0, 0.0, 0.0, 0.0);
    for &(t, w) in onsets {
        let n = ((t - t0) / p).round();
        let r = t - (t0 + n * p);
        if r.abs() > 0.2 * p {
            continue; // off-beat (8th hats, syncopation): no evidence either way
        }
        total += w;
        if r.abs() < 0.03 {
            on += w;
            sq += r * r;
            cnt += 1.0;
        }
    }
    Some(Grid {
        t0,
        period: p,
        residual: if cnt > 0.0 { (sq / cnt).sqrt() } else { 1.0 },
        fit: if total > 0.0 { on / total } else { 0.0 },
        support: on,
    })
}

#[derive(Debug, Clone, Copy)]
struct WindowFit {
    start: f64,
    end: f64,
    grid: Option<Grid>,
}

fn consistent(a: &Grid, b: &Grid) -> bool {
    if (a.period / b.period - 1.0).abs() > 0.004 {
        return false;
    }
    let n = (b.t0 - a.t0) / a.period;
    (n - n.round()).abs() * a.period < 0.02
}

/// Tempo segments for a stretch of frames. Windows of 16 s every 4 s are fitted
/// independently; agreeing windows merge; a window that disagrees starts a new segment.
pub fn tempo_segments(frames: &[Frame], onsets: &Onsets) -> Vec<TempoSegment> {
    let (Some(first), Some(last)) = (frames.first(), frames.last()) else {
        return Vec::new();
    };
    let mut fits = Vec::new();
    let mut a = first.t;
    loop {
        let b = (a + WINDOW_SECS).min(last.t);
        if b - a < 6.0 {
            break;
        }
        let i0 = frames.partition_point(|f| f.t < a);
        let i1 = frames.partition_point(|f| f.t < b);
        let window = &frames[i0..i1];
        let grid = tempo(window, onsets).and_then(|(bpm, strength)| {
            let g = fit_grid(&onsets.weighted(a, b), 60.0 / bpm)?;
            let beats = (b - a) / g.period;
            // Evidence: most on-beat onsets fit tightly and there are enough of them.
            (g.fit > 0.6 && g.support >= 0.35 * beats && strength > 0.05).then_some(g)
        });
        fits.push(WindowFit {
            start: a,
            end: b,
            grid,
        });
        if b >= last.t {
            break;
        }
        a += STEP_SECS;
    }

    // Windows that straddle a tempo change fit worse than steady ones: drop windows whose timing
    // error is well above the typical one (relative, so looser real-world timing still passes).
    let mut residuals: Vec<f64> = fits
        .iter()
        .filter_map(|w| w.grid.map(|g| g.residual))
        .collect();
    residuals.sort_by(f64::total_cmp);
    let limit = residuals
        .get(residuals.len() / 2)
        .map_or(1.0, |m| (2.5 * m).max(0.003));

    // Group windows into segments.
    let mut groups: Vec<(f64, f64, Grid)> = Vec::new();
    for w in &fits {
        let Some(g) = w.grid.filter(|g| g.residual <= limit) else {
            continue;
        };
        match groups.last_mut() {
            Some(last) if consistent(&last.2, &g) => last.1 = w.end,
            _ => groups.push((w.start, w.end, g)),
        }
    }
    let mut segments: Vec<TempoSegment> = Vec::new();
    for (i, &(start, end, g)) in groups.iter().enumerate() {
        // Refit over the whole group for the most precise line.
        let refined = fit_grid(&onsets.weighted(start, end), g.period)
            .filter(|r| consistent(r, &g))
            .unwrap_or(g);
        // Boundaries: where this grid's supporting onsets begin and end. Where groups overlap
        // (a tempo change), split at the midpoint of the overlap.
        let supported: Vec<f64> = onsets
            .weighted(start, end)
            .into_iter()
            .map(|(t, _)| t)
            .filter(|&t| {
                let n = ((t - refined.t0) / refined.period).round();
                (t - (refined.t0 + n * refined.period)).abs() < 0.03
            })
            .collect();
        let (Some(&s0), Some(&s1)) = (supported.first(), supported.last()) else {
            continue;
        };
        let mut seg_start = s0 - 0.03;
        if let Some(prev) = segments.last_mut()
            && prev.end > seg_start
        {
            let mid = 0.5 * (prev.end + seg_start);
            prev.end = mid;
            seg_start = mid;
        }
        let is_last = i + 1 == groups.len();
        let seg_end = if is_last {
            end.max(s1 + refined.period * 0.5)
        } else {
            s1 + refined.period * 0.5
        };
        segments.push(TempoSegment {
            start: seg_start,
            end: seg_end,
            t0: refined.t0,
            period: refined.period,
            confidence: (refined.fit as f32).clamp(0.0, 1.0),
        });
    }
    segments
}

/// Beat times of the segments.
pub fn beats(segments: &[TempoSegment]) -> Vec<f64> {
    let mut out = Vec::new();
    for s in segments {
        let first = ((s.start - s.t0) / s.period).ceil() as i64;
        let last = ((s.end - s.t0) / s.period).floor() as i64;
        for n in first..=last {
            let t = s.t0 + n as f64 * s.period;
            if t >= s.start && t < s.end && out.last().is_none_or(|&l: &f64| t - l > 0.5 * s.period)
            {
                out.push(t);
            }
        }
    }
    out
}

fn strength_near(list: &[(f64, f32)], t: f64, tol: f64) -> f32 {
    let i = list.partition_point(|&(x, _)| x < t - tol);
    list[i..]
        .iter()
        .take_while(|&&(x, _)| x <= t + tol)
        .map(|&(_, s)| s)
        .fold(0.0, f32::max)
}

/// Which of every 4 beats starts a bar (0..=3), from snare backbeats (snares on 2 and 4) and
/// chord changes at bar starts.
pub fn downbeat_phase(beats: &[f64], frames: &[Frame], onsets: &Onsets) -> u8 {
    if beats.len() < 8 {
        return 0;
    }
    let chroma_at = |t: f64| -> [f32; 12] {
        let i = frames.partition_point(|f| f.t < t);
        let mut c = [0.0; 12];
        for f in &frames[i.saturating_sub(0)..(i + 8).min(frames.len())] {
            for (a, b) in c.iter_mut().zip(&f.chroma) {
                *a += b;
            }
        }
        c
    };
    let change: Vec<f32> = beats
        .windows(2)
        .map(|w| {
            let (a, b) = (chroma_at(w[0]), chroma_at(w[1]));
            let (na, nb) = (
                a.iter().map(|x| x * x).sum::<f32>().sqrt(),
                b.iter().map(|x| x * x).sum::<f32>().sqrt(),
            );
            if na < 1e-9 || nb < 1e-9 {
                0.0
            } else {
                1.0 - a.iter().zip(&b).map(|(x, y)| x * y).sum::<f32>() / (na * nb)
            }
        })
        .collect();
    let snare: Vec<f32> = beats
        .iter()
        .map(|&t| strength_near(&onsets.snare, t, 0.03))
        .collect();
    let snare_total: f32 = snare.iter().sum::<f32>().max(1e-9);
    let change_total: f32 = change.iter().sum::<f32>().max(1e-9);
    let mut best = (0u8, f32::MIN);
    for phase in 0..4usize {
        let mut s = 0.0;
        for (i, &v) in snare.iter().enumerate() {
            // Backbeat: snares on beats 2 and 4 of the bar.
            s += if (i + 4 - phase) % 2 == 1 { v } else { -v } / snare_total;
        }
        for (i, &v) in change.iter().enumerate() {
            // `change[i]` is the harmony change arriving at beat i + 1.
            if (i + 1 + 4 - phase) % 4 == 0 {
                s += v / change_total;
            }
        }
        if s > best.1 {
            best = (phase as u8, s);
        }
    }
    best.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{FrontEnd, SR};
    use crate::synth::{self, Pattern, part};

    fn analyze(parts: &[synth::Part]) -> (Vec<Frame>, Onsets, synth::Synth) {
        let s = synth::render(parts, SR, 5);
        let mut fe = FrontEnd::new(0);
        let mut fr = Vec::new();
        fe.push(&s.samples, &mut fr);
        let on = detect(&fr);
        (fr, on, s)
    }

    /// Share of true beats with a detected beat within `tol` seconds (and vice versa).
    fn f_measure(truth: &[f64], found: &[f64], tol: f64) -> f64 {
        let hit = |a: &[f64], b: &[f64]| {
            a.iter()
                .filter(|&&t| b.iter().any(|&x| (x - t).abs() <= tol))
                .count() as f64
        };
        let (p, r) = (
            hit(found, truth) / found.len().max(1) as f64,
            hit(truth, found) / truth.len().max(1) as f64,
        );
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }

    #[test]
    fn tempo_for_common_genres() {
        for bpm in [124.0, 128.0, 140.0, 174.0] {
            let (fr, on, _) = analyze(&[part(Pattern::Drop, 16, bpm)]);
            // The autocorrelation estimate only seeds the grid fit: within 1%.
            let (coarse, strength) = tempo(&fr, &on).unwrap();
            assert!(
                (coarse / bpm - 1.0).abs() < 0.01,
                "{bpm} BPM read as {coarse}"
            );
            assert!(strength > 0.05);
            // The fitted grid is what the score reports.
            let segs = tempo_segments(&fr, &on);
            assert!(
                (segs[0].bpm() - bpm).abs() < 0.1,
                "{bpm}: fitted {}",
                segs[0].bpm()
            );
        }
    }

    #[test]
    fn grid_is_precise_on_a_full_track() {
        for bpm in [124.0, 128.0, 174.0] {
            let (fr, on, s) = analyze(&synth::standard_track(bpm));
            let segs = tempo_segments(&fr, &on);
            assert_eq!(segs.len(), 1, "{bpm}: one steady tempo, got {segs:?}");
            assert!(
                (segs[0].bpm() - bpm).abs() < 0.1,
                "{bpm}: {}",
                segs[0].bpm()
            );
            let b = beats(&segs);
            let f = f_measure(&s.beats, &b, 0.07);
            assert!(f > 0.97, "{bpm} BPM: beat F-measure {f:.3}");
            let worst = s
                .beats
                .iter()
                .filter_map(|&t| b.iter().map(|&x| (x - t).abs()).min_by(f64::total_cmp))
                .fold(0.0, f64::max);
            assert!(
                worst < 0.01,
                "{bpm}: worst beat error {:.1} ms",
                worst * 1e3
            );
        }
    }

    #[test]
    fn tempo_change_starts_a_new_segment() {
        let parts = [
            part(Pattern::Drop, 24, 122.0),
            part(Pattern::Drop, 24, 128.0),
        ];
        let (fr, on, s) = analyze(&parts);
        let segs = tempo_segments(&fr, &on);
        assert_eq!(segs.len(), 2, "{segs:?}");
        assert!((segs[0].bpm() - 122.0).abs() < 0.1 && (segs[1].bpm() - 128.0).abs() < 0.1);
        let change = 24.0 * 4.0 * 60.0 / 122.0;
        assert!(
            (segs[1].start - change).abs() < 4.0 * 60.0 / 122.0,
            "switch at {:.2}, truth {change:.2}",
            segs[1].start
        );
        let b = beats(&segs);
        let after: Vec<f64> = s
            .beats
            .iter()
            .copied()
            .filter(|&t| t > change + 8.0)
            .collect();
        assert!(
            f_measure(
                &after,
                &b.iter()
                    .copied()
                    .filter(|&t| t > change + 8.0)
                    .collect::<Vec<_>>(),
                0.02
            ) > 0.98
        );
    }

    #[test]
    fn beatless_intro_has_no_grid() {
        let parts = [
            part(Pattern::Beatless, 32, 124.0),
            part(Pattern::Drop, 16, 124.0),
        ];
        let (fr, on, s) = analyze(&parts);
        let segs = tempo_segments(&fr, &on);
        let beat_start = s.beats[0];
        assert!(!segs.is_empty());
        assert!(
            segs[0].start > beat_start - 2.0 * 60.0 / 124.0,
            "grid starts at {:.2}, beat enters at {beat_start:.2}",
            segs[0].start
        );
        assert!(
            beats(&segs).iter().all(|&t| t > beat_start - 0.05),
            "no beats in the beatless intro"
        );
    }

    #[test]
    fn downbeats_line_up_with_bars() {
        let (fr, on, s) = analyze(&synth::standard_track(128.0));
        let b = beats(&tempo_segments(&fr, &on));
        let phase = downbeat_phase(&b, &fr, &on) as usize;
        let first_downbeat = b
            .iter()
            .skip(phase)
            .step_by(4)
            .copied()
            .take(20)
            .collect::<Vec<_>>();
        for t in first_downbeat {
            assert!(
                s.downbeats.iter().any(|&d| (d - t).abs() < 0.01),
                "{t:.3} is not a downbeat"
            );
        }
    }
}
