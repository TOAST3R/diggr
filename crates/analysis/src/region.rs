//! Incremental analysis of one contiguous stretch of a file.
//!
//! Audio arrives in chunks. Each `update` re-analyzes a trailing window (tempo, beats, structure)
//! and merges it with what is already final. Results more than 32 bars behind the analyzed edge
//! become final and are never recomputed; frames older than that (plus context) are dropped, so
//! memory stays bounded however long the file is.

use crate::frontend::{FPS, Frame, FrontEnd, SR};
use crate::rhythm::{self, Onsets};
use crate::score::{Section, TempoSegment};
use crate::structure::{self, BeatFeat};

/// Results older than this many bars behind the analyzed edge are final.
const FINAL_LAG_BARS: f64 = 32.0;
/// Frames kept before the final cut, for context.
const CONTEXT_SECS: f64 = 30.0;
/// Novelty kernel half-size in beats (4 bars).
const KERNEL: usize = 16;

#[derive(Debug, Clone, Default)]
pub struct RegionResult {
    pub start: f64,
    pub end: f64,
    pub segments: Vec<TempoSegment>,
    pub beats: Vec<f64>,
    pub downbeats: Vec<usize>,
    pub phrase_origin: u8,
    pub onsets: Onsets,
    pub feats: Vec<BeatFeat>,
    pub sections: Vec<Section>,
    pub complete: bool,
}

pub struct Region {
    start: f64,
    frontend: FrontEnd,
    frames: Vec<Frame>,
    end: f64,
    complete: bool,
    /// Everything before `cut` (seconds) is final. The result below grows in place: each update
    /// truncates its provisional tail and appends, so nothing final is ever copied again.
    cut: f64,
    final_segments: usize,
    final_beats: usize,
    final_sections: usize,
    prototypes: Vec<Vec<f32>>,
    phase: Option<u8>,
    origin: Option<u8>,
    latest: RegionResult,
}

/// Keeps the entries before `cut` (lists are sorted by time) and appends the fresh ones after.
fn splice_onsets(list: &mut Vec<(f64, f32)>, fresh: &[(f64, f32)], cut: f64) {
    let keep = list.partition_point(|(t, _)| *t < cut);
    list.truncate(keep);
    list.extend(fresh.iter().copied().filter(|(t, _)| *t >= cut));
}

fn same_grid(a: &TempoSegment, b: &TempoSegment) -> bool {
    if (a.period / b.period - 1.0).abs() >= 0.004 {
        return false;
    }
    let n = (b.t0 - a.t0) / a.period;
    (n - n.round()).abs() * a.period < 0.02
}

impl Region {
    /// A region starting `start` seconds into the file.
    pub fn new(start: f64) -> Self {
        Self {
            start,
            frontend: FrontEnd::new((start * SR as f64).round() as u64),
            frames: Vec::new(),
            end: start,
            complete: false,
            cut: start,
            final_segments: 0,
            final_beats: 0,
            final_sections: 0,
            prototypes: Vec::new(),
            phase: None,
            origin: None,
            latest: RegionResult {
                start,
                end: start,
                ..Default::default()
            },
        }
    }

    pub fn start(&self) -> f64 {
        self.start
    }

    /// Seconds of audio pushed so far (end of the region).
    pub fn end(&self) -> f64 {
        self.end
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Frames currently held in memory (bounded).
    pub fn frames_held(&self) -> usize {
        self.frames.len()
    }

    /// Mono samples at 22,050 Hz, continuing the region.
    pub fn push(&mut self, mono: &[f32]) {
        self.frontend.push(mono, &mut self.frames);
        self.end += mono.len() as f64 / SR as f64;
    }

    /// Re-analyzes the open part of the region. `eof`: the file ended, so everything is final.
    pub fn update(&mut self, eof: bool) -> &RegionResult {
        self.complete |= eof;
        if self.frames.is_empty() {
            return &self.latest;
        }
        let frames = &self.frames;
        let cut = self.cut;
        let r = &mut self.latest;
        r.start = self.start;
        r.end = self.end;
        r.complete = self.complete;

        // Onsets: final ones before the cut, fresh ones after.
        let fresh = rhythm::detect(frames);
        splice_onsets(&mut r.onsets.kick, &fresh.kick, cut);
        splice_onsets(&mut r.onsets.snare, &fresh.snare, cut);
        splice_onsets(&mut r.onsets.hat, &fresh.hat, cut);

        // Tempo segments: final ones, then the window's segments from the cut on.
        r.segments.truncate(self.final_segments);
        if let Some(last) = r.segments.last_mut() {
            last.end = last.end.min(cut.max(last.start));
        }
        for mut s in rhythm::tempo_segments(frames, &fresh) {
            if s.end <= cut {
                continue;
            }
            match r.segments.last_mut() {
                Some(last) if same_grid(last, &s) => last.end = last.end.max(s.end),
                Some(last) => {
                    s.start = s.start.max(last.end);
                    r.segments.push(s);
                }
                None => r.segments.push(s),
            }
        }
        // Electronica keeps its tempo through breakdowns. Carry the grid to the edges of what we
        // know: forward to the analyzed end while the region is open (provisional until
        // confirmed), and back to the region start when the region begins mid-file (a seek into
        // a breakdown). A beatless intro at the very start of a file keeps no grid.
        if let Some(last) = r.segments.last_mut()
            && !self.complete
            && last.confidence >= 0.8
        {
            last.end = last.end.max(self.end);
        }
        if self.start > 0.0
            && let Some(first) = r.segments.first_mut()
            && first.confidence >= 0.8
            && first.start > self.start
        {
            first.start = self.start;
        }

        // Beats: final ones, then new ones after the last final beat.
        r.beats.truncate(self.final_beats);
        r.feats.truncate(self.final_beats);
        let last_final = r.beats.last().copied();
        let open_segments = &r.segments[self.final_segments.saturating_sub(1)..];
        let new_beats: Vec<f64> = rhythm::beats(open_segments)
            .into_iter()
            .filter(|&t| last_final.is_none_or(|l| t > l + 0.1) && t >= cut - 1e-9)
            .collect();
        r.feats
            .extend(structure::beat_features(&new_beats, frames, &fresh));
        r.beats.extend(new_beats);

        let phase = self
            .phase
            .unwrap_or_else(|| rhythm::downbeat_phase(&r.beats, frames, &r.onsets));
        let bar_of = |beat: usize| (beat as f64 - phase as f64) / 4.0;

        // Structure over the open part (from the start of the last final section).
        let finals = &r.sections[..self.final_sections.min(r.sections.len())];
        let open_from = finals.last().map_or(0, |s| s.start_beat);
        let nov = structure::novelty(&r.feats[open_from..], KERNEL);
        let mut raw_bars: Vec<f64> = structure::peaks(&nov, KERNEL)
            .iter()
            .map(|&b| bar_of(b + open_from))
            .collect();
        let origin = self.origin.unwrap_or_else(|| {
            let mut all: Vec<f64> = finals
                .iter()
                .skip(1)
                .map(|s| bar_of(s.start_beat))
                .collect();
            all.extend(&raw_bars);
            structure::phrase_origin(&all)
        });
        raw_bars.retain(|&b| b > 0.0);
        let mut bounds: Vec<usize> = finals.iter().map(|s| s.start_beat).collect();
        bounds.push(finals.last().map_or(0, |s| s.end_beat));
        for b in raw_bars {
            let beat = (structure::snap(b, origin) * 4 + phase as i64).max(0) as usize;
            if beat > *bounds.last().expect("non-empty") + 8 && beat + 8 < r.feats.len() {
                bounds.push(beat);
            }
        }
        if r.feats.len() > *bounds.last().expect("non-empty") {
            bounds.push(r.feats.len());
        }
        bounds.dedup();

        let n_final = finals.len();
        let kinds = structure::classify(&r.feats, &bounds, self.complete);
        let mut protos = self.prototypes.clone();
        let labels = structure::label(&r.feats, &bounds[n_final..], &mut protos);
        r.sections.truncate(n_final);
        for (i, w) in bounds.windows(2).enumerate().skip(n_final) {
            r.sections.push(Section {
                start_beat: w[0],
                end_beat: w[1],
                label: labels[i - n_final],
                kind: kinds[i],
                is_final: false,
            });
        }

        // Finalize everything well behind the analyzed edge.
        let period = r.segments.last().map_or(0.5, |s| s.period);
        let new_cut = if self.complete {
            f64::INFINITY
        } else {
            self.end - FINAL_LAG_BARS * 4.0 * period
        };
        let final_beats = r.beats.partition_point(|&t| t < new_cut);
        let n_final_sections = r
            .sections
            .iter()
            .take_while(|s| s.end_beat <= final_beats)
            .count();
        for s in &mut r.sections[..n_final_sections] {
            s.is_final = true;
        }
        if n_final_sections > n_final {
            // Commit the prototypes of the newly final sections.
            let b: Vec<usize> = r.sections[n_final..n_final_sections]
                .iter()
                .map(|s| s.start_beat)
                .chain([r.sections[n_final_sections - 1].end_beat])
                .collect();
            structure::label(&r.feats, &b, &mut self.prototypes);
            self.final_sections = n_final_sections;
            self.phase = Some(phase);
            self.origin = Some(origin);
        }
        if new_cut.is_finite() && new_cut > cut {
            self.final_segments = r.segments.iter().filter(|s| s.start < new_cut).count();
            self.final_beats = final_beats;
            self.cut = new_cut;
            let n = self
                .frames
                .partition_point(|f| f.t < new_cut - CONTEXT_SECS);
            self.frames.drain(..n);
        }

        r.downbeats.clear();
        r.downbeats
            .extend((phase as usize..r.beats.len()).step_by(4));
        r.phrase_origin = origin;
        &self.latest
    }

    pub fn latest(&self) -> &RegionResult {
        &self.latest
    }
}

/// Seconds of audio per `update` when streaming.
pub const UPDATE_EVERY_SECS: f64 = 8.0;

/// Frames per second of the front end (re-exported for memory bounds in tests).
pub const FRAMES_PER_SEC: f64 = FPS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::SectionKind;
    use crate::synth::{self, Pattern, part};

    fn stream(samples: &[f32], chunk_secs: f64) -> (Region, usize) {
        let mut r = Region::new(0.0);
        let chunk = (chunk_secs * SR as f64) as usize;
        let mut max_frames = 0;
        for c in samples.chunks(chunk) {
            r.push(c);
            r.update(false);
            max_frames = max_frames.max(r.frames_held());
        }
        r.update(true);
        (r, max_frames)
    }

    fn kinds(r: &RegionResult) -> Vec<SectionKind> {
        r.sections.iter().map(|s| s.kind).collect()
    }

    #[test]
    fn streaming_matches_the_arrangement_and_finalizes() {
        let s = synth::render(&synth::standard_track(128.0), SR, 3);
        let (r, _) = stream(&s.samples, UPDATE_EVERY_SECS);
        let res = r.latest();
        assert_eq!(
            kinds(res),
            s.sections.iter().map(|x| x.1).collect::<Vec<_>>()
        );
        assert!(res.sections.iter().all(|x| x.is_final), "complete ⇒ final");
        assert_eq!(res.beats.len(), s.beats.len());
        for (a, b) in res.beats.iter().zip(&s.beats) {
            assert!((a - b).abs() < 0.01);
        }
        for &d in &res.downbeats[..10] {
            assert!(s.downbeats.iter().any(|&t| (t - res.beats[d]).abs() < 0.01));
        }
    }

    #[test]
    fn early_results_are_provisional_near_the_edge() {
        let s = synth::render(&synth::standard_track(128.0), SR, 3);
        let mut r = Region::new(0.0);
        let upto = (100.0 * SR as f64) as usize; // part way through
        for c in s.samples[..upto].chunks((8.0 * SR as f64) as usize) {
            r.push(c);
            r.update(false);
        }
        let res = r.latest();
        let last = res.sections.last().unwrap();
        assert!(
            !last.is_final,
            "the section at the analyzed edge is provisional"
        );
        assert!(
            res.sections.first().unwrap().is_final,
            "the intro is well behind the edge"
        );
    }

    #[test]
    fn memory_is_bounded_on_a_long_mix() {
        let one = synth::standard_track(126.0);
        let parts: Vec<synth::Part> = (0..5).flat_map(|_| one.clone()).collect(); // ≈ 14 min
        let s = synth::render(&parts, SR, 4);
        let (r, max_frames) = stream(&s.samples, UPDATE_EVERY_SECS);
        let limit = ((FINAL_LAG_BARS * 4.0 * 60.0 / 126.0 + CONTEXT_SECS + 2.0 * UPDATE_EVERY_SECS)
            * FPS) as usize;
        assert!(
            max_frames <= limit,
            "held {max_frames} frames, limit {limit}"
        );
        assert_eq!(r.latest().beats.len(), s.beats.len());
        let drops = r
            .latest()
            .sections
            .iter()
            .filter(|x| x.kind == SectionKind::Drop)
            .count();
        assert_eq!(drops, 10);
        let drop_labels: std::collections::BTreeSet<u8> = r
            .latest()
            .sections
            .iter()
            .filter(|x| x.kind == SectionKind::Drop)
            .map(|x| x.label)
            .collect();
        assert_eq!(
            drop_labels.len(),
            1,
            "all drops share one label across the whole mix"
        );
    }

    /// Spec: "a 2-hour mix plays to the end → memory stays bounded". Rendered and streamed track
    /// by track (the synthetic input alone would otherwise need ~600 MB).
    /// `cargo test -p analysis --release -- --ignored two_hour`
    #[test]
    #[ignore = "long: about a minute in release"]
    fn two_hour_mix_keeps_memory_bounded() {
        let mut r = Region::new(0.0);
        let mut max_frames = 0;
        let (mut secs, mut tracks, mut true_beats) = (0.0, 0u64, 0usize);
        while secs < 7_200.0 {
            let bpm = 124.0 + (tracks % 5) as f64; // tempo drifts between tracks, like a DJ set
            let t = synth::render(&synth::standard_track(bpm), SR, tracks + 1);
            true_beats += t.beats.len();
            for c in t.samples.chunks((UPDATE_EVERY_SECS * SR as f64) as usize) {
                r.push(c);
                r.update(false);
                max_frames = max_frames.max(r.frames_held());
            }
            secs += t.samples.len() as f64 / SR as f64;
            tracks += 1;
        }
        let res = r.update(true);
        let limit = ((FINAL_LAG_BARS * 4.0 * 60.0 / 124.0 + CONTEXT_SECS + 2.0 * UPDATE_EVERY_SECS)
            * FPS) as usize;
        eprintln!(
            "{tracks} tracks, {:.0} min: max frames held {max_frames} (limit {limit}), {} beats (truth {true_beats}), {} sections, {} tempo segments",
            secs / 60.0,
            res.beats.len(),
            res.sections.len(),
            res.segments.len()
        );
        assert!(max_frames <= limit, "frames held grew to {max_frames}");
        // Per-beat data is all that is kept for the past: ~15k beats, not 600k frames.
        assert!((res.beats.len() as f64 - true_beats as f64).abs() / (true_beats as f64) < 0.01);
    }

    #[test]
    fn region_starting_mid_file() {
        let s = synth::render(&[part(Pattern::Drop, 32, 124.0)], SR, 2);
        let start = 20.0;
        let mut r = Region::new(start);
        r.push(&s.samples[(start * SR as f64) as usize..]);
        let res = r.update(true);
        assert!(res.beats[0] >= start - 0.05);
        let truth: Vec<f64> = s
            .beats
            .iter()
            .copied()
            .filter(|&t| t >= start + 0.05)
            .collect();
        assert!((res.beats.len() as i64 - truth.len() as i64).abs() <= 1);
        let first = res.beats.iter().find(|&&b| b >= truth[0] - 0.05).unwrap();
        assert!(
            (first - truth[0]).abs() < 0.01,
            "beat times are in file time"
        );
    }
}
