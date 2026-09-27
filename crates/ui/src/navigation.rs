//! Structure navigation: where section and drop jumps go, the downbeats they are quantized to,
//! and bar-aligned loop regions — all from the analyzed [`SongScore`].

use analysis::SongScore;

/// A boundary counts as a "drop" when its section is this much louder than the one before.
pub const RISE_DB: f32 = 4.0;
/// Pressing `[` within this many bars of a section start goes to the previous section.
const BACK_GRACE_BARS: usize = 1;
/// Longest section loop.
pub const MAX_LOOP_BARS: usize = 32;
/// Candidate downbeats sent with a quantized jump (the engine picks the first it can reach).
pub const CANDIDATES: usize = 4;

fn beat_secs(score: &SongScore, beat: usize) -> Option<f64> {
    score.beats.get(beat).copied()
}

/// Index of the beat at or before `t`.
fn beat_index(score: &SongScore, t: f64) -> Option<usize> {
    let i = score.beats.partition_point(|&b| b <= t);
    i.checked_sub(1)
}

/// Index (into `score.sections`) of the section containing `t`.
pub fn section_index(score: &SongScore, t: f64) -> Option<usize> {
    let beat = beat_index(score, t)?;
    score
        .sections
        .iter()
        .position(|s| s.start_beat <= beat && beat < s.end_beat)
}

fn section_start(score: &SongScore, i: usize) -> Option<f64> {
    beat_secs(score, score.sections.get(i)?.start_beat)
}

fn section_energy(score: &SongScore, i: usize) -> Option<f32> {
    let s = score.sections.get(i)?;
    let e = &score.curves.energy;
    let (a, b) = (s.start_beat.min(e.len()), s.end_beat.min(e.len()));
    (b > a).then(|| e[a..b].iter().sum::<f32>() / (b - a) as f32)
}

/// Start of the section after the one playing at `t`.
pub fn next_section(score: &SongScore, t: f64) -> Option<f64> {
    match section_index(score, t) {
        Some(i) => section_start(score, i + 1),
        // Before the first section (or no sections known here yet): the first one ahead.
        None => score
            .sections
            .iter()
            .filter_map(|s| beat_secs(score, s.start_beat))
            .find(|&s| s > t),
    }
}

/// Start of the current section, or of the previous one within the first bar of a section.
pub fn previous_section(score: &SongScore, t: f64) -> Option<f64> {
    let i = section_index(score, t)?;
    let start = section_start(score, i)?;
    let grace_end = score
        .downbeats
        .iter()
        .filter_map(|&d| beat_secs(score, d))
        .filter(|&d| d > start)
        .nth(BACK_GRACE_BARS - 1)
        .unwrap_or(start);
    if t < grace_end && i > 0 {
        section_start(score, i - 1)
    } else {
        Some(start)
    }
}

/// The next boundary whose section is at least [`RISE_DB`] louder than the one before it,
/// falling back to the next section.
pub fn next_rise(score: &SongScore, t: f64) -> Option<f64> {
    let from = section_index(score, t).map_or(0, |i| i + 1);
    (from.max(1)..score.sections.len())
        .find(
            |&i| match (section_energy(score, i - 1), section_energy(score, i)) {
                (Some(a), Some(b)) => b - a >= RISE_DB,
                _ => false,
            },
        )
        .and_then(|i| section_start(score, i))
        .or_else(|| next_section(score, t))
}

/// Times of the next `n` downbeats strictly after `t`.
pub fn downbeats_after(score: &SongScore, t: f64, n: usize) -> Vec<f64> {
    score
        .downbeats
        .iter()
        .filter_map(|&d| beat_secs(score, d))
        .filter(|&d| d > t)
        .take(n)
        .collect()
}

/// The downbeat at or before `t`.
fn downbeat_at_or_before(score: &SongScore, t: f64) -> Option<(usize, f64)> {
    score
        .downbeats
        .iter()
        .enumerate()
        .filter_map(|(k, &d)| Some((k, beat_secs(score, d)?)))
        .take_while(|&(_, d)| d <= t)
        .last()
}

/// Whether jumps at `t` can be quantized: the analysis covers `t` and a downbeat follows
/// within two bars' worth of seconds (a beatless or unanalyzed stretch jumps immediately).
pub fn grid_ok(score: &SongScore, t: f64) -> bool {
    if !score.coverage.contains(t) {
        return false;
    }
    let bar = score.bpm_at(t).map_or(2.0, |bpm| 240.0 / bpm);
    downbeats_after(score, t, 1)
        .first()
        .is_some_and(|&d| d - t <= 2.0 * bar + 0.05)
}

/// A loop over the section playing at `t`, capped at [`MAX_LOOP_BARS`]: the stretch of at most
/// that many bars, from the section start, that contains `t`.
pub fn section_loop(score: &SongScore, t: f64) -> Option<(f64, f64)> {
    let i = section_index(score, t)?;
    let s = score.sections[i];
    let bars: Vec<f64> = score
        .downbeats
        .iter()
        .filter(|&&d| d >= s.start_beat && d < s.end_beat)
        .filter_map(|&d| beat_secs(score, d))
        .collect();
    // The last section ends one beat after the final beat.
    let end_secs = beat_secs(score, s.end_beat).or_else(|| match score.beats.as_slice() {
        [.., a, b] => Some(b + (b - a)),
        _ => None,
    })?;
    let start = *bars.first()?;
    // Which MAX_LOOP_BARS-sized stretch of the section contains t.
    let k = bars
        .iter()
        .take_while(|&&b| b <= t)
        .count()
        .saturating_sub(1)
        / MAX_LOOP_BARS;
    let a = bars.get(k * MAX_LOOP_BARS).copied().unwrap_or(start);
    let b = bars
        .get((k + 1) * MAX_LOOP_BARS)
        .copied()
        .unwrap_or(end_secs);
    (b > a).then_some((a, b))
}

/// A loop of `bars` bars starting at the downbeat at or before `t`.
pub fn bar_loop(score: &SongScore, t: f64, bars: usize) -> Option<(f64, f64)> {
    let (k, a) = downbeat_at_or_before(score, t)?;
    let b = beat_secs(score, *score.downbeats.get(k + bars)?)?;
    Some((a, b))
}

/// The next loop length for `Shift+L`: 4 → 8 → 16 → 4.
pub fn next_loop_bars(current: Option<usize>) -> usize {
    match current {
        Some(4) => 8,
        Some(8) => 16,
        _ => 4,
    }
}

/// Energy-rise boundaries (for drawing drop markers): section start times.
pub fn rise_marks(score: &SongScore) -> Vec<f64> {
    (1..score.sections.len())
        .filter(|&i| matches!((section_energy(score, i - 1), section_energy(score, i)), (Some(a), Some(b)) if b - a >= RISE_DB))
        .filter_map(|i| section_start(score, i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysis::SectionKind;
    use analysis::score::{IntervalSet, Section, TempoSegment};

    /// 120 BPM (0.5 s beats), 4/4, sections of `bars` bars at the given energies (dB).
    fn score(sections: &[(usize, f32)]) -> SongScore {
        let beats_total: usize = sections.iter().map(|(b, _)| b * 4).sum();
        let mut s = SongScore {
            beats: (0..beats_total).map(|i| i as f64 * 0.5).collect(),
            ..Default::default()
        };
        s.downbeats = (0..beats_total).step_by(4).collect();
        s.tempo_segments = vec![TempoSegment {
            start: 0.0,
            end: beats_total as f64 * 0.5,
            t0: 0.0,
            period: 0.5,
            confidence: 1.0,
        }];
        let mut coverage = IntervalSet::default();
        coverage.insert(0.0, beats_total as f64 * 0.5);
        s.coverage = coverage;
        let mut at = 0;
        for (label, &(bars, db)) in sections.iter().enumerate() {
            let n = bars * 4;
            s.sections.push(Section {
                start_beat: at,
                end_beat: at + n,
                label: label as u8,
                kind: SectionKind::Groove,
                is_final: true,
            });
            s.curves.energy.extend(std::iter::repeat_n(db, n));
            at += n;
        }
        s
    }

    #[test]
    fn section_jumps() {
        // Sections start at 0, 16 s, 32 s, 48 s (8 bars each).
        let s = score(&[(8, -20.0), (8, -20.0), (8, -20.0), (8, -20.0)]);
        assert_eq!(next_section(&s, 20.0), Some(32.0));
        assert_eq!(next_section(&s, 50.0), None, "no section after the last");
        assert_eq!(
            previous_section(&s, 20.0),
            Some(16.0),
            "start of the current section"
        );
        assert_eq!(
            previous_section(&s, 17.0),
            Some(0.0),
            "within the first bar: previous"
        );
        assert_eq!(previous_section(&s, 1.0), Some(0.0), "first section stays");
    }

    #[test]
    fn drop_jump_skips_quiet_boundaries() {
        // groove, breakdown (quieter), drop (+8 dB), outro.
        let s = score(&[(8, -14.0), (8, -22.0), (8, -14.0), (8, -18.0)]);
        assert_eq!(next_rise(&s, 5.0), Some(32.0));
        assert_eq!(rise_marks(&s), [32.0]);
        // No rise ahead: the next section.
        assert_eq!(next_rise(&s, 40.0), Some(48.0));
    }

    #[test]
    fn quantization_candidates() {
        let s = score(&[(8, -20.0), (8, -20.0)]);
        assert_eq!(downbeats_after(&s, 5.1, 3), [6.0, 8.0, 10.0]);
        assert_eq!(downbeats_after(&s, 6.0, 1), [8.0], "strictly after");
        assert!(grid_ok(&s, 5.1));
        assert!(!grid_ok(&s, 100.0), "not analyzed there");
    }

    #[test]
    fn loops() {
        let s = score(&[(8, -20.0), (40, -20.0)]);
        assert_eq!(section_loop(&s, 5.0), Some((0.0, 16.0)));
        // Second section is 40 bars: its first 32 bars, then the remaining 8.
        assert_eq!(section_loop(&s, 20.0), Some((16.0, 16.0 + 64.0)));
        assert_eq!(
            section_loop(&s, 16.0 + 70.0),
            Some((16.0 + 64.0, 16.0 + 80.0))
        );
        assert_eq!(bar_loop(&s, 5.1, 4), Some((4.0, 12.0)));
        assert_eq!(next_loop_bars(None), 4);
        assert_eq!(next_loop_bars(Some(4)), 8);
        assert_eq!(next_loop_bars(Some(16)), 4);
    }
}
