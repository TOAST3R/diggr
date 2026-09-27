//! The signal bus: everything the visuals know about the music this frame, sampled once into an
//! immutable [`Signals`] snapshot.
//!
//! Musical time comes from the analyzed beat grid at the *audible* position. Until the score
//! covers the playhead, a nominal 120 BPM grid and live spectrum levels stand in; the switch is
//! phase-locked and levels crossfade, so nothing jumps.

use analysis::{SectionKind, SongScore};
use audio::{PlayState, Position, TrackId};

pub const NOMINAL_BPM: f64 = 120.0;
/// Value used for "no drop known" (shaders need finite numbers).
pub const FAR: f32 = 9_999.0;
const LIVE_BANDS: usize = 19;

/// Events crossed since the previous frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Trigger {
    /// How many (dense hats between two frames are all counted).
    pub count: u8,
    /// Seconds since the most recent one, at the audible time of this frame.
    pub ago: f32,
}

impl Trigger {
    pub fn fired(&self) -> bool {
        self.count > 0
    }

    fn add(&mut self, ago: f64) {
        self.count = self.count.saturating_add(1);
        self.ago = if self.count == 1 {
            ago as f32
        } else {
            self.ago.min(ago as f32)
        };
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Triggers {
    pub kick: Trigger,
    pub snare: Trigger,
    pub hat: Trigger,
    pub beat: Trigger,
    pub downbeat: Trigger,
    pub section_change: Trigger,
    pub drop: Trigger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Signals {
    pub playing: bool,
    /// Audible position in seconds.
    pub seconds: f64,
    /// Continuous musical time in beats (motion should be driven by this, not wall time).
    pub beats: f64,
    pub tempo_bpm: f32,
    /// Phases 0..1.
    pub beat: f32,
    pub bar: f32,
    pub phrase: f32,
    pub section_progress: f32,
    pub triggers: Triggers,
    /// Levels 0..1.
    pub energy: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub brightness: f32,
    pub tension: f32,
    /// Beats until the next known drop (`FAR` if none).
    pub drop_in: f32,
    pub section_kind: Option<SectionKind>,
    pub section_label: u8,
    /// Index of the current section in the score (changes on every boundary).
    pub section_index: Option<usize>,
    /// 1 on an analyzed beat grid, 0 on the nominal one.
    pub beat_confidence: f32,
    /// 0 = live levels only, 1 = analyzed levels only.
    pub score_mix: f32,
}

impl Default for Signals {
    fn default() -> Self {
        Self {
            playing: false,
            seconds: 0.0,
            beats: 0.0,
            tempo_bpm: NOMINAL_BPM as f32,
            beat: 0.0,
            bar: 0.0,
            phrase: 0.0,
            section_progress: 0.0,
            triggers: Triggers::default(),
            energy: 0.0,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            brightness: 0.0,
            tension: 0.0,
            drop_in: FAR,
            section_kind: None,
            section_label: 0,
            section_index: None,
            beat_confidence: 0.0,
            score_mix: 0.0,
        }
    }
}

/// 5th/95th percentile of a curve, for normalizing it to 0..1 per track.
#[derive(Debug, Clone, Copy, Default)]
struct Norm {
    lo: f32,
    hi: f32,
}

impl Norm {
    fn of(v: &[f32]) -> Self {
        if v.is_empty() {
            return Self { lo: 0.0, hi: 1.0 };
        }
        let mut s: Vec<f32> = v.iter().copied().filter(|x| x.is_finite()).collect();
        s.sort_by(f32::total_cmp);
        let at = |p: f32| s[((s.len() - 1) as f32 * p) as usize];
        let (lo, hi) = (at(0.05), at(0.95));
        Self {
            lo,
            hi: if hi - lo < 1e-6 { lo + 1.0 } else { hi },
        }
    }

    fn apply(&self, x: f32) -> f32 {
        ((x - self.lo) / (self.hi - self.lo)).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Default)]
struct Norms {
    key: (u64, usize),
    energy: Norm,
    bass: Norm,
    mid: Norm,
    treble: Norm,
    brightness: Norm,
}

#[derive(Debug, Default)]
pub struct SignalBus {
    track: TrackId,
    last_secs: Option<f64>,
    beats: f64,
    locked: bool,
    mix: f32,
    norms: Norms,
    prev_low: f32,
    live_kick_ago: f32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Crossings of sorted `times` in `(a, b]`, reported into `trig` with their age at `b`.
fn crossings(times: &[f64], a: f64, b: f64, trig: &mut Trigger) {
    let i = times.partition_point(|&t| t <= a);
    for &t in times[i..].iter().take_while(|&&t| t <= b) {
        trig.add(b - t);
    }
}

impl SignalBus {
    pub fn update(
        &mut self,
        pos: &Position,
        score: Option<&SongScore>,
        bars: &[f32; LIVE_BANDS],
        dt: f32,
    ) -> Signals {
        let playing = pos.state == PlayState::Playing;
        let secs = pos.seconds();
        let mut s = Signals {
            playing,
            seconds: secs,
            ..Default::default()
        };
        if pos.track != self.track || pos.discontinuity {
            self.track = pos.track;
            self.last_secs = None;
            self.locked = false;
        }
        let covered = score.filter(|sc| sc.coverage.contains(secs));
        let score_beat = covered.and_then(|sc| sc.beat_at(secs));
        let tempo = covered
            .and_then(|sc| sc.bpm_at(secs))
            .unwrap_or(NOMINAL_BPM);
        s.tempo_bpm = tempo as f32;

        // Musical time: integrate at the tempo while playing, and phase-lock to the analyzed grid.
        if playing {
            self.beats += dt as f64 * tempo / 60.0;
        }
        match score_beat {
            Some(target) => {
                let err = target - self.beats;
                if !self.locked || err.abs() > 2.0 {
                    self.beats = target; // first lock, or a seek: jump
                    self.locked = true;
                } else if playing {
                    // Converge smoothly within about a quarter second.
                    self.beats += err * (dt as f64 * 4.0).min(1.0);
                }
                s.beat_confidence = 1.0;
            }
            None => {
                self.locked = false;
                if !playing && self.last_secs.is_none() {
                    self.beats = secs * NOMINAL_BPM / 60.0;
                }
            }
        }
        s.beats = self.beats;
        s.beat = self.beats.rem_euclid(1.0) as f32;
        let (bar_offset, phrase_origin) = covered
            .and_then(|sc| {
                sc.downbeats
                    .first()
                    .map(|&d| ((d % 4) as f64, sc.phrase_origin as f64))
            })
            .unwrap_or((0.0, 0.0));
        let bars_elapsed = (self.beats - bar_offset) / 4.0;
        s.bar = bars_elapsed.rem_euclid(1.0) as f32;
        s.phrase = ((bars_elapsed - phrase_origin) / 8.0).rem_euclid(1.0) as f32;

        // Triggers: score events crossed since the last frame (none across seeks or pauses).
        if let (Some(sc), Some(last)) = (covered, self.last_secs)
            && playing
            && secs > last
            && secs - last < 0.5
        {
            let t = &mut s.triggers;
            crossings(&sc.events.kick, last, secs, &mut t.kick);
            crossings(&sc.events.snare, last, secs, &mut t.snare);
            crossings(&sc.events.hat, last, secs, &mut t.hat);
            crossings(&sc.beats, last, secs, &mut t.beat);
            let downs: Vec<f64> = sc
                .downbeats
                .iter()
                .filter_map(|&i| sc.beats.get(i).copied())
                .collect();
            crossings(&downs, last, secs, &mut t.downbeat);
            for sec in sc.sections.iter().skip(1) {
                if let Some(&start) = sc.beats.get(sec.start_beat)
                    && start > last
                    && start <= secs
                {
                    t.section_change.add(secs - start);
                    if sec.kind == SectionKind::Drop {
                        t.drop.add(secs - start);
                    }
                }
            }
        }
        self.last_secs = Some(secs);

        // Levels: analyzed curves (normalized per track) blended with live spectrum bars.
        let live_low = bars[..4].iter().sum::<f32>() / 4.0;
        let live_mid = bars[4..12].iter().sum::<f32>() / 8.0;
        let live_high = bars[12..].iter().sum::<f32>() / 7.0;
        let live_energy = bars.iter().sum::<f32>() / LIVE_BANDS as f32;
        if score_beat.is_none()
            && playing
            && live_low - self.prev_low > 0.15
            && self.live_kick_ago > 0.1
        {
            s.triggers.kick.add(0.0);
            self.live_kick_ago = 0.0;
        }
        self.live_kick_ago += dt;
        self.prev_low = live_low;

        let target_mix = if score_beat.is_some() { 1.0 } else { 0.0 };
        self.mix += (target_mix - self.mix).clamp(-dt, dt); // 1 s crossfade
        s.score_mix = self.mix;
        let (mut e, mut b, mut m, mut tr, mut br, mut tension) =
            (live_energy, live_low, live_mid, live_high, live_high, 0.0);
        if let (Some(sc), Some(beat)) = (covered, score_beat) {
            let key = (sc.content_hash, sc.beats.len());
            if self.norms.key != key {
                let c = &sc.curves;
                self.norms = Norms {
                    key,
                    energy: Norm::of(&c.energy),
                    bass: Norm::of(&c.bass),
                    mid: Norm::of(&c.mid),
                    treble: Norm::of(&c.treble),
                    brightness: Norm::of(&c.brightness),
                };
            }
            let i = (beat.floor() as usize).min(sc.beats.len().saturating_sub(1));
            let at = |v: &[f32]| v.get(i).copied().unwrap_or(0.0);
            let n = &self.norms;
            let mx = self.mix;
            e = lerp(live_energy, n.energy.apply(at(&sc.curves.energy)), mx);
            b = lerp(live_low, n.bass.apply(at(&sc.curves.bass)), mx);
            m = lerp(live_mid, n.mid.apply(at(&sc.curves.mid)), mx);
            tr = lerp(live_high, n.treble.apply(at(&sc.curves.treble)), mx);
            br = lerp(live_high, n.brightness.apply(at(&sc.curves.brightness)), mx);
            tension = sc.tension(i) * mx;
            s.drop_in = sc.drop_in(i).map_or(FAR, |d| d as f32);
            if let Some((k, sec)) = sc
                .sections
                .iter()
                .enumerate()
                .find(|(_, x)| i >= x.start_beat && i < x.end_beat)
            {
                s.section_kind = Some(sec.kind);
                s.section_label = sec.label;
                s.section_index = Some(k);
                let len = (sec.end_beat - sec.start_beat).max(1) as f64;
                s.section_progress = ((beat - sec.start_beat as f64) / len).clamp(0.0, 1.0) as f32;
            }
        }
        s.energy = e;
        s.bass = b;
        s.mid = m;
        s.treble = tr;
        s.brightness = br;
        s.tension = tension;
        s
    }
}

/// A deterministic per-track seed: the analyzed content hash, or a hash of the title before the
/// track has been analyzed.
pub fn track_seed(score: Option<&SongScore>, title: &str) -> u64 {
    match score.map(|s| s.content_hash).filter(|&h| h != 0) {
        Some(h) => h,
        None => title.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysis::score::{BeatCurves, Events, IntervalSet, Section, TempoSegment};

    const RATE: u32 = 48_000;

    fn pos(secs: f64, state: PlayState) -> Position {
        Position {
            track: 1,
            frame: (secs * RATE as f64).round() as u64,
            sample_rate: RATE,
            state,
            discontinuity: false,
        }
    }

    /// 128 BPM from t = 0, downbeats every 4 beats, sections every 32 beats.
    fn score() -> SongScore {
        let period = 60.0 / 128.0;
        let beats: Vec<f64> = (0..256).map(|i| i as f64 * period).collect();
        let mut coverage = IntervalSet::default();
        coverage.insert(0.0, 256.0 * period);
        let kinds = [
            SectionKind::Intro,
            SectionKind::Build,
            SectionKind::Drop,
            SectionKind::Breakdown,
        ];
        SongScore {
            content_hash: 42,
            coverage,
            tempo_segments: vec![TempoSegment {
                start: 0.0,
                end: 256.0 * period,
                t0: 0.0,
                period,
                confidence: 1.0,
            }],
            downbeats: (0..64).map(|i| i * 4).collect(),
            events: Events {
                kick: beats.clone(),
                snare: vec![],
                hat: beats
                    .iter()
                    .flat_map(|&t| {
                        [
                            t,
                            t + period / 4.0,
                            t + period / 2.0,
                            t + 3.0 * period / 4.0,
                        ]
                    })
                    .collect(),
            },
            curves: BeatCurves {
                energy: (0..256).map(|i| -30.0 + (i / 64) as f32 * 5.0).collect(),
                bass: vec![1.0; 256],
                mid: vec![1.0; 256],
                treble: vec![1.0; 256],
                brightness: vec![1.0; 256],
                flatness: vec![0.0; 256],
                tension: (0..256)
                    .map(|i| {
                        if (32..64).contains(&i) {
                            (i - 32) as f32 / 32.0
                        } else {
                            0.0
                        }
                    })
                    .collect(),
            },
            sections: (0..8)
                .map(|k| Section {
                    start_beat: k * 32,
                    end_beat: (k + 1) * 32,
                    label: (k % 2) as u8,
                    kind: kinds[k % 4],
                    is_final: true,
                })
                .collect(),
            beats,
            ..Default::default()
        }
    }

    fn run(
        bus: &mut SignalBus,
        sc: Option<&SongScore>,
        from: f64,
        to: f64,
        fps: f64,
    ) -> Vec<Signals> {
        let dt = 1.0 / fps;
        let mut out = Vec::new();
        let mut t = from;
        while t <= to + 1e-9 {
            out.push(bus.update(&pos(t, PlayState::Playing), sc, &[0.0; 19], dt as f32));
            t += dt;
        }
        out
    }

    #[test]
    fn musical_time_follows_the_grid_and_tempo() {
        let sc = score();
        let mut bus = SignalBus::default();
        let s = run(&mut bus, Some(&sc), 0.0, 10.0, 60.0);
        let last = s.last().unwrap();
        assert!(
            (last.beats - 10.0 * 128.0 / 60.0).abs() < 0.02,
            "beats {}",
            last.beats
        );
        assert_eq!(last.tempo_bpm, 128.0);
        assert_eq!(last.beat_confidence, 1.0);
        // Rotating once per bar is faster at 174 BPM than at 120: beats advance with tempo.
        let mut slow = SignalBus::default();
        let n = run(&mut slow, None, 0.0, 10.0, 60.0);
        assert!(
            (n.last().unwrap().beats - 20.0).abs() < 0.05,
            "nominal 120 BPM"
        );
    }

    #[test]
    fn pause_freezes_musical_time() {
        let sc = score();
        let mut bus = SignalBus::default();
        run(&mut bus, Some(&sc), 0.0, 2.0, 60.0);
        let a = bus.update(
            &pos(2.0, PlayState::Paused),
            Some(&sc),
            &[0.0; 19],
            1.0 / 60.0,
        );
        let b = bus.update(&pos(2.0, PlayState::Paused), Some(&sc), &[0.0; 19], 1.0);
        assert_eq!(a.beats, b.beats);
    }

    #[test]
    fn every_hat_is_counted_even_at_low_frame_rates() {
        let sc = score();
        let mut bus = SignalBus::default();
        // 16th hats at 128 BPM = 8.5 per second; at 30 fps some frames get two.
        let s = run(&mut bus, Some(&sc), 0.0, 20.0, 30.0);
        let counted: u32 = s.iter().map(|x| x.triggers.hat.count as u32).sum();
        let truth = sc
            .events
            .hat
            .iter()
            .filter(|&&t| t > 0.0 && t <= s.last().unwrap().seconds)
            .count() as u32;
        assert_eq!(counted, truth);
        let kicks: u32 = s.iter().map(|x| x.triggers.kick.count as u32).sum();
        assert!((42..=43).contains(&kicks), "{kicks}");
    }

    #[test]
    fn kick_trigger_lands_in_the_frame_of_the_beat() {
        let sc = score();
        let mut bus = SignalBus::default();
        let period = 60.0 / 128.0;
        let s = run(&mut bus, Some(&sc), 0.0, 3.0, 60.0);
        for x in s.iter().filter(|x| x.triggers.kick.fired()) {
            assert!(
                x.triggers.kick.ago < 1.0 / 60.0 + 1e-6,
                "fired {} s late",
                x.triggers.kick.ago
            );
            let nearest = (x.seconds / period).floor() * period;
            assert!((x.seconds - nearest - x.triggers.kick.ago as f64).abs() < 1e-6);
        }
    }

    #[test]
    fn sections_levels_and_countdown() {
        let sc = score();
        let mut bus = SignalBus::default();
        let period = 60.0 / 128.0;
        let s = run(&mut bus, Some(&sc), 0.0, 60.0 * period, 60.0);
        let at_40 = s.iter().find(|x| x.seconds >= 40.0 * period).unwrap();
        assert_eq!(at_40.section_kind, Some(SectionKind::Build));
        assert!((at_40.section_progress - 0.25).abs() < 0.05);
        assert_eq!(at_40.drop_in, 24.0);
        assert!(at_40.tension > 0.2);
        let changes = s
            .iter()
            .filter(|x| x.triggers.section_change.fired())
            .count();
        assert_eq!(changes, 1, "one boundary (beat 32) crossed");
        // Bar phase restarts on downbeats.
        let on_bar = s.iter().find(|x| x.seconds >= 8.0 * period).unwrap();
        assert!(on_bar.bar < 0.05 || on_bar.bar > 0.95, "{}", on_bar.bar);
    }

    #[test]
    fn switching_from_live_to_score_does_not_jump() {
        let full = score();
        let mut partial = full.clone();
        partial.coverage = IntervalSet::default();
        partial.coverage.insert(0.0, 0.0);
        let mut bus = SignalBus::default();
        // First 3 s without coverage (nominal grid), then the score arrives.
        let a = run(&mut bus, Some(&partial), 0.0, 3.0, 60.0);
        let b = run(&mut bus, Some(&full), 3.0 + 1.0 / 60.0, 5.0, 60.0);
        let before = a.last().unwrap();
        let after = &b[0];
        assert!((after.beats - before.beats).abs() > 0.0);
        assert_eq!(before.score_mix, 0.0);
        assert!(after.score_mix < 0.1, "levels crossfade, not switch");
        assert!(b.last().unwrap().score_mix > 0.99);
        // Once locked, the phase matches the analyzed grid.
        let end = b.last().unwrap();
        assert!((end.beats - 5.0 * 128.0 / 60.0).abs() < 0.02);
    }

    #[test]
    fn seek_does_not_fire_everything_in_between() {
        let sc = score();
        let mut bus = SignalBus::default();
        run(&mut bus, Some(&sc), 0.0, 1.0, 60.0);
        let s = bus.update(
            &pos(50.0, PlayState::Playing),
            Some(&sc),
            &[0.0; 19],
            1.0 / 60.0,
        );
        assert_eq!(s.triggers.kick.count, 0);
        assert!(
            (s.beats - 50.0 * 128.0 / 60.0).abs() < 1e-6,
            "jumps to the new position"
        );
    }

    #[test]
    fn seeds_are_stable() {
        let sc = score();
        assert_eq!(track_seed(Some(&sc), "x"), 42);
        assert_eq!(track_seed(None, "Echo"), track_seed(None, "Echo"));
        assert_ne!(track_seed(None, "Echo"), track_seed(None, "Siren"));
    }
}
