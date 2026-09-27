//! Builds one `SongScore` from the analyzed pieces of a file: live regions (a seek starts a new
//! one) and, when resuming from cache, the cached score as a frozen prefix.

use crate::region::RegionResult;
use crate::rhythm::Onsets;
use crate::score::{
    ALGORITHM_VERSION, BeatCurves, Events, IntervalSet, Section, SongScore, TempoSegment,
};
use crate::structure;

/// A contiguous analyzed stretch, independent of how it was produced.
#[derive(Debug, Clone, Default)]
pub struct Piece {
    pub start: f64,
    pub end: f64,
    pub segments: Vec<TempoSegment>,
    pub beats: Vec<f64>,
    pub downbeats: Vec<usize>,
    pub phrase_origin: u8,
    pub events: Events,
    pub curves: BeatCurves,
    pub sections: Vec<Section>,
    pub complete: bool,
}

fn events_of(o: &Onsets) -> Events {
    let times = |v: &[(f64, f32)]| v.iter().map(|&(t, _)| t).collect();
    Events {
        kick: times(&o.kick),
        snare: times(&o.snare),
        hat: times(&o.hat),
    }
}

impl From<&RegionResult> for Piece {
    fn from(r: &RegionResult) -> Self {
        let f = &r.feats;
        Piece {
            start: r.start,
            end: r.end,
            segments: r.segments.clone(),
            beats: r.beats.clone(),
            downbeats: r.downbeats.clone(),
            phrase_origin: r.phrase_origin,
            events: events_of(&r.onsets),
            curves: BeatCurves {
                energy: f.iter().map(|b| b.energy).collect(),
                bass: f.iter().map(|b| b.bass).collect(),
                mid: f.iter().map(|b| b.mid).collect(),
                treble: f.iter().map(|b| b.treble).collect(),
                brightness: f.iter().map(|b| b.brightness).collect(),
                flatness: f.iter().map(|b| b.flatness).collect(),
                tension: Vec::new(),
            },
            sections: r.sections.clone(),
            complete: r.complete,
        }
    }
}

impl From<&SongScore> for Piece {
    /// A cached score as one frozen piece (its first coverage span).
    fn from(s: &SongScore) -> Self {
        let (start, end) = s.coverage.spans().first().copied().unwrap_or((0.0, 0.0));
        Piece {
            start,
            end,
            segments: s.tempo_segments.clone(),
            beats: s.beats.clone(),
            downbeats: s.downbeats.clone(),
            phrase_origin: s.phrase_origin,
            events: s.events.clone(),
            curves: BeatCurves {
                tension: Vec::new(),
                ..s.curves.clone()
            },
            sections: s.sections.clone(),
            complete: s.complete,
        }
    }
}

/// Joins pieces in time order. Where pieces overlap, the earlier one wins.
pub fn assemble(mut pieces: Vec<Piece>, content_hash: u64) -> SongScore {
    pieces.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut s = SongScore {
        version: ALGORITHM_VERSION,
        content_hash,
        ..Default::default()
    };
    let mut covered_to = f64::NEG_INFINITY;
    for (k, p) in pieces.iter().enumerate() {
        s.coverage.insert(p.start, p.end);
        if k == 0 {
            s.phrase_origin = p.phrase_origin;
        }
        // Beats of this piece that are not already covered.
        let skip = p.beats.partition_point(|&t| t < covered_to);
        let offset = s.beats.len() as isize - skip as isize;
        s.beats.extend_from_slice(&p.beats[skip..]);
        let curve = |dst: &mut Vec<f32>, src: &[f32]| dst.extend(src.iter().skip(skip).copied());
        curve(&mut s.curves.energy, &p.curves.energy);
        curve(&mut s.curves.bass, &p.curves.bass);
        curve(&mut s.curves.mid, &p.curves.mid);
        curve(&mut s.curves.treble, &p.curves.treble);
        curve(&mut s.curves.brightness, &p.curves.brightness);
        curve(&mut s.curves.flatness, &p.curves.flatness);
        s.downbeats.extend(
            p.downbeats
                .iter()
                .filter(|&&i| i >= skip)
                .map(|&i| (i as isize + offset) as usize),
        );
        for sec in &p.sections {
            if sec.end_beat <= skip {
                continue;
            }
            s.sections.push(Section {
                start_beat: (sec.start_beat.max(skip) as isize + offset) as usize,
                end_beat: (sec.end_beat as isize + offset) as usize,
                ..*sec
            });
        }
        let after = |v: &[f64]| {
            v.iter()
                .copied()
                .filter(|&t| t >= covered_to)
                .collect::<Vec<_>>()
        };
        s.events.kick.extend(after(&p.events.kick));
        s.events.snare.extend(after(&p.events.snare));
        s.events.hat.extend(after(&p.events.hat));
        s.tempo_segments
            .extend(
                p.segments
                    .iter()
                    .filter(|g| g.end > covered_to)
                    .map(|g| TempoSegment {
                        start: g.start.max(covered_to),
                        ..*g
                    }),
            );
        covered_to = covered_to.max(p.end);
    }
    s.curves.tension = structure::tension(&s.sections, s.beats.len());
    // Complete when one span covers the whole file from the start and reached its end.
    s.complete = pieces.iter().any(|p| p.complete)
        && s.coverage.spans().len() == 1
        && s.coverage.spans()[0].0 <= 1e-6;
    s
}

/// A score covering nothing yet (the "analyzing…" state).
pub fn empty(content_hash: u64) -> SongScore {
    SongScore {
        version: ALGORITHM_VERSION,
        content_hash,
        coverage: IntervalSet::default(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::SR;
    use crate::region::Region;
    use crate::score::SectionKind;
    use crate::synth;

    #[test]
    fn a_seek_region_joins_the_first_region() {
        let s = synth::render(&synth::standard_track(128.0), SR, 3);
        let split = 90.0;
        let cut = (split * SR as f64) as usize;
        let mut a = Region::new(0.0);
        a.push(&s.samples[..cut]);
        a.update(false);
        let mut b = Region::new(split);
        b.push(&s.samples[cut..]);
        b.update(true);
        let score = assemble(vec![Piece::from(b.latest()), Piece::from(a.latest())], 42);
        assert_eq!(
            score.coverage.spans(),
            &[(0.0, s.samples.len() as f64 / SR as f64)]
        );
        assert!(
            score.beats.windows(2).all(|w| w[1] > w[0]),
            "beats strictly increasing"
        );
        assert!((score.beats.len() as i64 - s.beats.len() as i64).abs() <= 2);
        assert_eq!(score.curves.energy.len(), score.beats.len());
        assert!(
            score
                .sections
                .windows(2)
                .all(|w| w[0].end_beat <= w[1].start_beat + 1)
        );
        assert!(score.sections.iter().any(|x| x.kind == SectionKind::Drop));
        assert_eq!(score.content_hash, 42);
    }

    #[test]
    fn cached_score_round_trips_as_a_piece() {
        let s = synth::render(&synth::standard_track(124.0), SR, 3);
        let mut r = Region::new(0.0);
        r.push(&s.samples);
        r.update(true);
        let score = assemble(vec![Piece::from(r.latest())], 7);
        assert!(score.complete);
        let again = assemble(vec![Piece::from(&score)], 7);
        assert_eq!(again, score);
        // Drop countdown and tension come with it.
        let drop = score
            .sections
            .iter()
            .find(|x| x.kind == SectionKind::Drop)
            .unwrap()
            .start_beat;
        assert_eq!(score.drop_in(drop - 8), Some(8));
        assert!(score.tension(drop - 8) > 0.8);
    }
}
