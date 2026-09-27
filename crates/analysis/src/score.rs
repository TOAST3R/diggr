//! `SongScore`: what the analyzer knows about a track, as an immutable snapshot.

use serde::{Deserialize, Serialize};

/// Bump when analysis results change meaningfully; cached scores of other versions are ignored.
pub const ALGORITHM_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SectionKind {
    Intro,
    Build,
    Drop,
    Breakdown,
    Groove,
    Outro,
}

impl SectionKind {
    pub const ALL: [SectionKind; 6] = [
        SectionKind::Intro,
        SectionKind::Build,
        SectionKind::Drop,
        SectionKind::Breakdown,
        SectionKind::Groove,
        SectionKind::Outro,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SectionKind::Intro => "intro",
            SectionKind::Build => "build",
            SectionKind::Drop => "drop",
            SectionKind::Breakdown => "breakdown",
            SectionKind::Groove => "groove",
            SectionKind::Outro => "outro",
        }
    }
}

/// Sorted, non-overlapping `[start, end)` intervals in seconds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntervalSet {
    spans: Vec<(f64, f64)>,
}

impl IntervalSet {
    pub fn insert(&mut self, start: f64, end: f64) {
        if end <= start {
            return;
        }
        self.spans.push((start, end));
        self.spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = Vec::with_capacity(self.spans.len());
        for &(s, e) in &self.spans {
            match merged.last_mut() {
                Some(last) if s <= last.1 + 1e-6 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        self.spans = merged;
    }

    pub fn contains(&self, t: f64) -> bool {
        self.spans.iter().any(|&(s, e)| t >= s && t < e)
    }

    /// End of the covered span containing `t`, if any.
    pub fn covered_until(&self, t: f64) -> Option<f64> {
        self.spans
            .iter()
            .find(|&&(s, e)| t >= s && t < e)
            .map(|&(_, e)| e)
    }

    pub fn spans(&self) -> &[(f64, f64)] {
        &self.spans
    }
}

/// A stretch of steady tempo: beat `n` is at `t0 + n * period`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempoSegment {
    pub start: f64,
    pub end: f64,
    pub t0: f64,
    pub period: f64,
    pub confidence: f32,
}

impl TempoSegment {
    pub fn bpm(&self) -> f64 {
        60.0 / self.period
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Events {
    pub kick: Vec<f64>,
    pub snare: Vec<f64>,
    pub hat: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Kick,
    Snare,
    Hat,
    Downbeat,
}

/// Per-beat curves, indexed like `SongScore::beats`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BeatCurves {
    pub energy: Vec<f32>,
    pub bass: Vec<f32>,
    pub mid: Vec<f32>,
    pub treble: Vec<f32>,
    pub brightness: Vec<f32>,
    pub flatness: Vec<f32>,
    pub tension: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Index into `SongScore::beats` of the first beat.
    pub start_beat: usize,
    /// One past the last beat.
    pub end_beat: usize,
    /// Similar sections share a label: 0 = "A", 1 = "B", …
    pub label: u8,
    pub kind: SectionKind,
    /// Provisional sections may still change as analysis continues.
    pub is_final: bool,
}

impl Section {
    pub fn label_name(&self) -> char {
        (b'A' + self.label.min(25)) as char
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SongScore {
    pub version: u32,
    pub content_hash: u64,
    /// Seconds of the file that have been analyzed.
    pub coverage: IntervalSet,
    pub tempo_segments: Vec<TempoSegment>,
    /// Beat times in seconds, ascending.
    pub beats: Vec<f64>,
    /// Indices into `beats` that start a bar.
    pub downbeats: Vec<usize>,
    /// Bar index (counting from the first downbeat) where 8-bar phrases start, mod 8.
    pub phrase_origin: u8,
    pub events: Events,
    pub curves: BeatCurves,
    pub sections: Vec<Section>,
    /// The whole file has been analyzed.
    pub complete: bool,
}

impl SongScore {
    /// Fractional beat index at time `t` (e.g. 12.5 = halfway between beats 12 and 13), if `t`
    /// lies within the beat grid.
    pub fn beat_at(&self, t: f64) -> Option<f64> {
        let b = &self.beats;
        if b.len() < 2 || t < b[0] || t > *b.last()? + (b[b.len() - 1] - b[b.len() - 2]) {
            return None;
        }
        let i = b.partition_point(|&x| x <= t).saturating_sub(1);
        // Past the last beat, extrapolate one period (i ≥ 1 here since b.len() ≥ 2).
        let next = b
            .get(i + 1)
            .copied()
            .unwrap_or_else(|| b[i] + (b[i] - b[i - 1]));
        Some(i as f64 + (t - b[i]) / (next - b[i]))
    }

    /// Event times in `[a, b)`.
    pub fn events_in(&self, kind: EventKind, a: f64, b: f64) -> Vec<f64> {
        let list: Vec<f64> = match kind {
            EventKind::Kick => self.events.kick.clone(),
            EventKind::Snare => self.events.snare.clone(),
            EventKind::Hat => self.events.hat.clone(),
            EventKind::Downbeat => self.downbeats.iter().map(|&i| self.beats[i]).collect(),
        };
        list.into_iter().filter(|&t| t >= a && t < b).collect()
    }

    pub fn section_at(&self, beat: usize) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| beat >= s.start_beat && beat < s.end_beat)
    }

    /// Beats until the next drop starts, if one is known.
    pub fn drop_in(&self, beat: usize) -> Option<usize> {
        self.sections
            .iter()
            .filter(|s| s.kind == SectionKind::Drop && s.start_beat > beat)
            .map(|s| s.start_beat - beat)
            .min()
    }

    pub fn tension(&self, beat: usize) -> f32 {
        self.curves.tension.get(beat).copied().unwrap_or(0.0)
    }

    /// Tempo (BPM) at time `t`.
    pub fn bpm_at(&self, t: f64) -> Option<f64> {
        self.tempo_segments
            .iter()
            .find(|s| t >= s.start && t < s.end)
            .map(TempoSegment::bpm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_set_merges() {
        let mut c = IntervalSet::default();
        c.insert(10.0, 20.0);
        c.insert(0.0, 5.0);
        c.insert(4.0, 11.0);
        c.insert(30.0, 40.0);
        assert_eq!(c.spans(), &[(0.0, 20.0), (30.0, 40.0)]);
        assert!(c.contains(19.9) && !c.contains(25.0));
        assert_eq!(c.covered_until(3.0), Some(20.0));
        assert_eq!(c.covered_until(25.0), None);
    }

    fn score() -> SongScore {
        let beats: Vec<f64> = (0..64).map(|i| i as f64 * 0.5).collect();
        SongScore {
            beats,
            downbeats: (0..16).map(|i| i * 4).collect(),
            sections: vec![
                Section {
                    start_beat: 0,
                    end_beat: 32,
                    label: 0,
                    kind: SectionKind::Build,
                    is_final: true,
                },
                Section {
                    start_beat: 32,
                    end_beat: 64,
                    label: 1,
                    kind: SectionKind::Drop,
                    is_final: false,
                },
            ],
            curves: BeatCurves {
                tension: (0..64)
                    .map(|i| if i < 32 { i as f32 / 32.0 } else { 0.0 })
                    .collect(),
                ..Default::default()
            },
            events: Events {
                kick: vec![16.0, 16.5, 17.0],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn queries() {
        let s = score();
        assert_eq!(s.beat_at(1.25), Some(2.5));
        // Inside the first beat (regression: this used to index beats[-1]).
        assert_eq!(s.beat_at(0.0), Some(0.0));
        assert_eq!(s.beat_at(0.25), Some(0.5));
        // Just past the last beat extrapolates one period.
        assert_eq!(s.beat_at(31.75), Some(63.5));
        assert_eq!(s.beat_at(-1.0), None);
        assert_eq!(s.section_at(40).unwrap().kind, SectionKind::Drop);
        assert_eq!(s.drop_in(24), Some(8));
        assert_eq!(s.drop_in(40), None);
        assert!((s.tension(24) - 0.75).abs() < 1e-6);
        assert_eq!(s.events_in(EventKind::Kick, 16.2, 17.5), vec![16.5, 17.0]);
        assert_eq!(s.events_in(EventKind::Downbeat, 0.0, 4.0), vec![0.0, 2.0]);
        assert_eq!(s.sections[1].label_name(), 'B');
    }

    #[test]
    fn serializes_compactly() {
        let s = score();
        let bytes = postcard::to_stdvec(&s).unwrap();
        let back: SongScore = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, s);
    }
}
