//! When an automaton scene steps. Steps are counted in musical time, not frames, so the same
//! stretch of music runs the same steps at 30 or 144 fps (and offline). The rate may change on
//! each beat (the scene's `step_rate` param, sampled when the beat starts and snapped to a power
//! of two with hysteresis), which keeps steps evenly spaced within a beat and on the beat grid.

/// Steps run in one frame at most; the rest carry over to the next frames.
pub const MAX_CATCHUP: u32 = 8;
/// Default steps run after a reset (a scene can ask for more), so a fresh grid isn't empty.
pub const PREROLL: u32 = 16;
/// A larger move of musical time (either way) is a seek: the grid starts over.
pub const JUMP_BEATS: f64 = 2.0;
/// How far (in octaves) `step_rate` must move from the current rate before the rate follows.
pub const HYSTERESIS: f32 = 0.6;
/// Rates are 2^level: ×0.5, ×1, ×2, ×4.
const LEVELS: std::ops::RangeInclusive<i32> = -1..=2;

/// The automaton's work for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AutoFrame {
    /// Clear the grid before stepping.
    pub reset: bool,
    /// Index of the first step to run (steps are `first..first + steps`).
    pub first: i64,
    pub steps: u32,
    /// Fraction of the current step elapsed, for gliding between steps.
    pub phase: f32,
    /// Index of the first step after the last reset (step `origin` starts from an empty grid,
    /// so a rule can seed its initial state there).
    pub origin: i64,
}

/// The rate level (log2) for a `step_rate` value, staying at `current` unless the value is more
/// than `HYSTERESIS` octaves from it. Jumps may cross several levels.
pub fn snap_level(step_rate: f32, current: Option<i32>) -> i32 {
    let v = if step_rate.is_finite() {
        step_rate
    } else {
        1.0
    };
    let l = v.max(1e-3).log2();
    match current {
        Some(c) if (l - c as f32).abs() <= HYSTERESIS => c,
        _ => (l.round() as i32).clamp(*LEVELS.start(), *LEVELS.end()),
    }
}

fn rate(level: i32) -> f64 {
    2f64.powi(level)
}

/// Step bookkeeping for one drawn automaton (one per layer).
#[derive(Debug, Clone)]
pub struct AutomatonClock {
    steps_per_beat: f64,
    level: i32,
    /// The beat `base` starts at.
    beat: i64,
    /// Step position at the start of `beat`.
    base: f64,
    /// Furthest musical time seen since the last reset (the automaton never runs backwards).
    high: f64,
    /// Musical time of the previous frame; `None` forces a reset.
    last: Option<f64>,
    /// The last step handed out.
    done: i64,
    /// Steps run after a reset.
    preroll: u32,
    /// The first step after the last reset.
    origin: i64,
}

impl AutomatonClock {
    pub fn new(steps_per_beat: f32, preroll: u32) -> Self {
        Self {
            steps_per_beat: steps_per_beat as f64,
            preroll,
            origin: 0,
            level: 0,
            beat: 0,
            base: 0.0,
            high: 0.0,
            last: None,
            done: 0,
        }
    }

    /// Starts over on the next frame (track change).
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// The current rate multiplier.
    pub fn rate(&self) -> f32 {
        rate(self.level) as f32
    }

    /// The steps due at musical time `beats`, with `step_rate` the scene's current value (1
    /// when it has none).
    pub fn advance(&mut self, beats: f64, step_rate: f32) -> AutoFrame {
        let reset = self
            .last
            .is_none_or(|l| (beats - l).abs() > JUMP_BEATS || !beats.is_finite());
        self.last = Some(beats);
        let spb = self.steps_per_beat;
        if reset {
            self.level = snap_level(step_rate, None);
            self.beat = beats.floor() as i64;
            self.base = self.beat as f64 * spb * rate(self.level);
            self.high = beats;
            let pos = self.base + beats.fract() * spb * rate(self.level);
            self.done = pos.floor() as i64 - self.preroll as i64;
            self.origin = self.done + 1;
        }
        let t = beats.max(self.high);
        self.high = t;
        while self.beat < t.floor() as i64 {
            self.base += spb * rate(self.level);
            self.beat += 1;
            self.level = snap_level(step_rate, Some(self.level));
        }
        let pos = self.base + (t - self.beat as f64) * spb * rate(self.level);
        let steps = (pos.floor() as i64 - self.done).clamp(0, MAX_CATCHUP as i64);
        let first = self.done + 1;
        self.done += steps;
        AutoFrame {
            reset,
            first,
            steps: steps as u32,
            phase: pos.fract() as f32,
            origin: self.origin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `secs` of 120 BPM (2 beats/s) at `fps`, returning each frame's beats and work.
    fn run(fps: f64, secs: f64, step_rate: impl Fn(f64) -> f32) -> Vec<(f64, AutoFrame)> {
        let mut c = AutomatonClock::new(4.0, PREROLL);
        (0..=(secs * fps).round() as u64)
            .map(|n| {
                let beats = n as f64 / fps * 2.0;
                (beats, c.advance(beats, step_rate(beats)))
            })
            .collect()
    }

    /// Steps run up to and including the frame at musical time `beats` (after the pre-roll).
    fn done_at(frames: &[(f64, AutoFrame)], beats: f64) -> i64 {
        frames
            .iter()
            .take_while(|(b, _)| *b <= beats + 1e-9)
            .flat_map(|(_, f)| f.first..f.first + f.steps as i64)
            .filter(|&k| k > 0)
            .count() as i64
    }

    #[test]
    fn steps_land_on_musical_time_at_any_frame_rate() {
        for fps in [30.0, 144.0] {
            let frames = run(fps, 4.0, |_| 1.0);
            assert!(frames[0].1.reset);
            assert_eq!(done_at(&frames, 8.0), 32, "{fps} fps");
            // Each step runs on the first frame whose musical time reaches it.
            for k in 1..=32i64 {
                let (at, _) = frames
                    .iter()
                    .find(|(_, f)| (f.first..f.first + f.steps as i64).contains(&k))
                    .unwrap();
                let first = frames.iter().find(|(b, _)| *b >= k as f64 / 4.0).unwrap().0;
                assert_eq!(*at, first, "{fps} fps, step {k}");
            }
        }
    }

    #[test]
    fn pre_roll_is_spread_over_frames() {
        let mut c = AutomatonClock::new(4.0, PREROLL);
        let a = c.advance(0.0, 1.0);
        let b = c.advance(0.01, 1.0);
        let d = c.advance(0.02, 1.0);
        assert_eq!((a.reset, a.steps), (true, MAX_CATCHUP));
        assert_eq!((b.reset, b.steps), (false, PREROLL - MAX_CATCHUP));
        assert_eq!(d.steps, 0);
        assert_eq!(b.first, a.first + a.steps as i64, "contiguous");
        assert_eq!(
            (a.origin, b.origin),
            (a.first, a.first),
            "steps count from the reset"
        );
    }

    #[test]
    fn a_long_pre_roll_fills_the_grid_over_frames() {
        let mut c = AutomatonClock::new(4.0, 256);
        let steps: Vec<u32> = (0..40)
            .map(|n| c.advance(n as f64 * 1e-3, 1.0).steps)
            .collect();
        assert!(steps[..32].iter().all(|&s| s == MAX_CATCHUP), "{steps:?}");
        assert!(steps[32..].iter().all(|&s| s == 0), "{steps:?}");
    }

    #[test]
    fn a_drop_at_rate_2_doubles_the_steps() {
        for fps in [30.0, 144.0] {
            let frames = run(fps, 4.0, |b| if b < 4.0 { 1.0 } else { 2.0 });
            assert_eq!(done_at(&frames, 4.0), 16, "{fps} fps groove");
            assert_eq!(done_at(&frames, 8.0), 16 + 32, "{fps} fps drop");
        }
    }

    #[test]
    fn a_rate_change_applies_from_the_next_beat() {
        let frames = run(60.0, 1.0, |b| if b < 0.5 { 1.0 } else { 2.0 });
        assert_eq!(done_at(&frames, 1.0), 4, "beat 0 keeps rate 1");
        assert_eq!(done_at(&frames, 2.0), 4 + 8);
    }

    #[test]
    fn a_strong_drop_changes_speed_several_times() {
        let rate = |b: f64| match b {
            b if b < 4.0 => 2.0,
            b if b < 8.0 => 4.0,
            _ => 2.0,
        };
        for fps in [30.0, 144.0] {
            let frames = run(fps, 6.0, rate);
            assert_eq!(done_at(&frames, 4.0), 32, "{fps}");
            assert_eq!(done_at(&frames, 8.0), 32 + 64, "{fps}");
            assert_eq!(done_at(&frames, 12.0), 32 + 64 + 32, "{fps}");
        }
    }

    #[test]
    fn hysteresis_stops_flicker_but_allows_big_jumps() {
        let frames = run(60.0, 4.0, |b| if (b as i64) % 2 == 0 { 1.35 } else { 1.5 });
        assert_eq!(done_at(&frames, 8.0), 32, "stays at ×1");
        assert_eq!(snap_level(1.5, Some(0)), 0);
        assert_eq!(snap_level(1.6, Some(0)), 1);
        assert_eq!(snap_level(4.0, Some(0)), 2, "×1 → ×4 in one move");
        assert_eq!(snap_level(2.5, Some(2)), 1, "eases back from ×4");
        assert_eq!(snap_level(100.0, Some(0)), 2);
        assert_eq!(snap_level(0.01, None), -1);
        assert_eq!(snap_level(f32::NAN, None), 0);
    }

    #[test]
    fn holds_while_paused_or_corrected_backwards() {
        let mut c = AutomatonClock::new(4.0, PREROLL);
        for n in 0..=150 {
            c.advance(n as f64 / 50.0, 1.0);
        }
        // Musical time 3.0, all steps run.
        for _ in 0..300 {
            assert_eq!(c.advance(3.0, 1.0).steps, 0, "paused");
        }
        let back = c.advance(2.9, 1.0);
        assert_eq!((back.reset, back.steps), (false, 0));
        assert_eq!(c.advance(3.0, 1.0).steps, 0);
        assert_eq!(c.advance(3.25, 1.0).steps, 1, "resumes past the last step");
    }

    #[test]
    fn seeks_and_track_changes_start_over() {
        let mut c = AutomatonClock::new(4.0, PREROLL);
        for n in 0..=20 {
            c.advance(10.0 + n as f64 * 0.05, 1.0);
        }
        let seek = c.advance(100.0, 1.0);
        assert!(seek.reset);
        assert_eq!(
            seek.steps + c.advance(100.0, 1.0).steps,
            PREROLL,
            "two frames"
        );
        assert_eq!(c.advance(100.0, 1.0).steps, 0);
        assert!(
            c.advance(98.5, 1.0).steps == 0,
            "1.5 beats back is a correction"
        );
        assert!(c.advance(90.0, 1.0).reset, "a seek back");
        c.advance(90.0, 1.0);
        c.reset();
        assert!(c.advance(90.0, 1.0).reset, "track change");
    }
}
