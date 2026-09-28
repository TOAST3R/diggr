//! Tuning on real music: annotations you record while listening, and metrics comparing them with
//! what the analyzer finds.

use std::path::{Path, PathBuf};

use platform::{FileSource, TrackRef};
use serde::{Deserialize, Serialize};

use crate::assemble::{Piece, assemble};
use crate::region::{Region, UPDATE_EVERY_SECS};
use crate::score::{SectionKind, SongScore};
use crate::source::Source;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Boundary {
    pub t: f64,
    pub kind: SectionKind,
}

/// What a listener marked on one track (times are audible positions from the playback clock).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Annotations {
    pub track: String,
    pub content_hash: u64,
    pub beats: Vec<f64>,
    pub boundaries: Vec<Boundary>,
}

impl Annotations {
    pub fn new(track: &TrackRef, content_hash: u64) -> Self {
        Self {
            track: track.0.clone(),
            content_hash,
            ..Default::default()
        }
    }

    pub fn tap(&mut self, t: f64) {
        let i = self.beats.partition_point(|&x| x < t);
        self.beats.insert(i, t);
    }

    pub fn boundary(&mut self, t: f64, kind: SectionKind) {
        let i = self.boundaries.partition_point(|b| b.t < t);
        self.boundaries.insert(i, Boundary { t, kind });
    }

    /// `<dir>/<hash>.json`
    pub fn path_in(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{:016x}.json", self.content_hash))
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let path = self.path_in(dir);
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)?;
        Ok(path)
    }

    pub fn load(path: &Path) -> Option<Self> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// Existing annotations for this content in `dir`, or a fresh set.
    pub fn load_or_new(dir: &Path, track: &TrackRef, content_hash: u64) -> Self {
        let fresh = Self::new(track, content_hash);
        Self::load(&fresh.path_in(dir)).unwrap_or(fresh)
    }
}

/// Harmonic mean of precision and recall, matching within `tol` seconds.
pub fn f_measure(truth: &[f64], found: &[f64], tol: f64) -> f64 {
    if truth.is_empty() || found.is_empty() {
        return 0.0;
    }
    let near = |list: &[f64], t: f64| {
        let i = list.partition_point(|&x| x < t - tol);
        list.get(i).is_some_and(|&x| x <= t + tol)
    };
    let recall = truth.iter().filter(|&&t| near(found, t)).count() as f64 / truth.len() as f64;
    let precision = found.iter().filter(|&&t| near(truth, t)).count() as f64 / found.len() as f64;
    if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackEval {
    pub track: String,
    /// Beat F-measure (±70 ms) within the stretch that was tapped.
    pub beat_f: Option<f64>,
    /// Share of annotated boundaries with a detected boundary within ±1 bar.
    pub boundary_hits: Option<f64>,
    pub taps: usize,
    pub boundaries: usize,
}

/// Runs of taps without pauses, as `(start, end)` padded by half a tap interval. A pause is a gap
/// longer than twice the median tap interval.
pub fn tapped_spans(taps: &[f64]) -> Vec<(f64, f64)> {
    if taps.len() < 2 {
        return Vec::new();
    }
    let mut gaps: Vec<f64> = taps.windows(2).map(|w| w[1] - w[0]).collect();
    gaps.sort_by(f64::total_cmp);
    let typical = gaps[gaps.len() / 2];
    let mut spans = Vec::new();
    let mut start = taps[0];
    for w in taps.windows(2) {
        if w[1] - w[0] > 2.0 * typical {
            spans.push((start - typical / 2.0, w[0] + typical / 2.0));
            start = w[1];
        }
    }
    spans.push((start - typical / 2.0, taps[taps.len() - 1] + typical / 2.0));
    spans
}

/// Analyzes a whole file offline (same code path as playback, without the horizon).
pub fn analyze_file(files: &dyn FileSource, track: &TrackRef) -> Option<SongScore> {
    let hash = crate::cache::content_hash(files, track).unwrap_or(0);
    let mut src = Source::open(files, track, 0.0).ok()?;
    let mut region = Region::new(0.0);
    while let Some(chunk) = src.read(UPDATE_EVERY_SECS) {
        region.push(&chunk);
        region.update(false);
    }
    region.update(true);
    Some(assemble(vec![Piece::from(region.latest())], hash))
}

pub fn evaluate(score: &SongScore, ann: &Annotations) -> TrackEval {
    let beat_f = (ann.beats.len() >= 4).then(|| {
        // Only score the stretches that were actually tapped: a pause in tapping (a gap of more
        // than twice the typical tap interval) must not count the analyzer's beats there as wrong.
        let spans = tapped_spans(&ann.beats);
        let found: Vec<f64> = score
            .beats
            .iter()
            .copied()
            .filter(|&t| spans.iter().any(|&(a, b)| t >= a && t <= b))
            .collect();
        f_measure(&ann.beats, &found, 0.07)
    });
    let found: Vec<f64> = score
        .sections
        .iter()
        .skip(1)
        .map(|s| score.beats[s.start_beat.min(score.beats.len() - 1)])
        .collect();
    let boundary_hits = (!ann.boundaries.is_empty() && !score.beats.is_empty()).then(|| {
        let hits = ann
            .boundaries
            .iter()
            .filter(|b| {
                let bar = 4.0 * 60.0 / score.bpm_at(b.t).unwrap_or(120.0);
                found.iter().any(|&f| (f - b.t).abs() <= bar)
            })
            .count();
        hits as f64 / ann.boundaries.len() as f64
    });
    TrackEval {
        track: ann.track.clone(),
        beat_f,
        boundary_hits,
        taps: ann.beats.len(),
        boundaries: ann.boundaries.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth;
    use platform::native::NativeFileSource;

    #[test]
    fn a_two_second_track_analyzes_without_panicking() {
        // Too short for any section: the structure step must cope with none.
        let track = TrackRef::new(format!(
            "{}/../audio/tests/fixtures/tone.flac",
            env!("CARGO_MANIFEST_DIR")
        ));
        assert!(analyze_file(&NativeFileSource, &track).is_some());
    }

    #[test]
    fn f_measure_basics() {
        let truth = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(f_measure(&truth, &truth, 0.07), 1.0);
        assert_eq!(f_measure(&truth, &[1.05, 2.05, 3.05, 4.05], 0.07), 1.0);
        assert_eq!(f_measure(&truth, &[1.1, 2.1, 3.1, 4.1], 0.07), 0.0);
        let half = f_measure(&truth, &[1.0, 2.0], 0.07); // precision 1, recall 0.5
        assert!((half - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn pauses_in_tapping_are_not_scored() {
        // Two tapped stretches at 120 BPM with a 10 s pause in between.
        let taps: Vec<f64> = (0..8)
            .map(|i| 10.0 + i as f64 * 0.5)
            .chain((0..8).map(|i| 24.0 + i as f64 * 0.5))
            .collect();
        assert_eq!(tapped_spans(&taps), vec![(9.75, 13.75), (23.75, 27.75)]);
        // An analyzer that has beats everywhere (including the pause) scores 100%.
        let score = SongScore {
            beats: (0..80).map(|i| i as f64 * 0.5).collect(),
            ..Default::default()
        };
        let ann = Annotations {
            beats: taps,
            ..Default::default()
        };
        assert_eq!(evaluate(&score, &ann).beat_f, Some(1.0));
    }

    #[test]
    fn annotations_round_trip_sorted() {
        let dir = platform::testing::TestDir::new("analysis-ann");
        let mut a = Annotations::new(&TrackRef::new("/m/a.mp3"), 0xabc);
        a.tap(2.0);
        a.tap(1.0);
        a.boundary(92.0, SectionKind::Drop);
        a.boundary(30.0, SectionKind::Build);
        assert_eq!(a.beats, [1.0, 2.0]);
        assert_eq!(a.boundaries[0].kind, SectionKind::Build);
        let path = a.save(&dir).unwrap();
        assert!(path.ends_with("0000000000000abc.json"));
        assert_eq!(
            Annotations::load_or_new(&dir, &TrackRef::new("/m/a.mp3"), 0xabc),
            a
        );
    }

    #[test]
    fn evaluating_a_synthetic_track_against_its_truth() {
        let dir = platform::testing::TestDir::new("analysis-eval");
        let s = synth::render(&synth::standard_track(126.0), 44_100, 13);
        let path = dir.join("t.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for &x in &s.samples {
            w.write_sample((x * 32_000.0) as i16).unwrap();
        }
        w.finalize().unwrap();
        let track = TrackRef::new(path.to_string_lossy());
        let score = analyze_file(&NativeFileSource, &track).unwrap();
        let mut ann = Annotations::new(&track, score.content_hash);
        // A listener tapping 16 bars of the first drop, and marking every section change.
        for &t in s.beats.iter().filter(|&&t| t > 46.0 && t < 76.0) {
            ann.tap(t + 0.02); // human taps are a little late
        }
        for &(t, kind) in &s.sections[1..] {
            ann.boundary(t, kind);
        }
        let e = evaluate(&score, &ann);
        assert!(e.beat_f.unwrap() > 0.97, "{e:?}");
        assert_eq!(e.boundary_hits, Some(1.0), "{e:?}");
    }
}
