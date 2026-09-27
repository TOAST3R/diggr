//! CPU side of the compositor: transition timing and the dynamic-resolution governor.

/// How the show moves from one look to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransitionKind {
    /// Swap in the frame where the section's first beat is audible.
    Cut,
    /// Two layers rendered and mixed.
    Crossfade,
    /// Same scene; parameters interpolate from the old values to the new ones.
    Morph,
}

/// A transition in musical time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    pub kind: TransitionKind,
    pub start_beats: f64,
    pub length_beats: f64,
}

impl Transition {
    /// Eased 0..1 progress at `beats`; `None` once finished.
    pub fn progress(&self, beats: f64) -> Option<f32> {
        if self.kind == TransitionKind::Cut || self.length_beats <= 0.0 {
            return None;
        }
        let t = (beats - self.start_beats) / self.length_beats;
        if t >= 1.0 {
            return None;
        }
        let t = t.clamp(0.0, 1.0) as f32;
        Some(t * t * (3.0 - 2.0 * t))
    }
}

/// The start of the next bar at or after `beats`, given the bar phase at that time.
pub fn next_downbeat(beats: f64, bar_phase: f32) -> f64 {
    if bar_phase < 1e-3 {
        return beats;
    }
    beats + (1.0 - bar_phase as f64) * 4.0
}

/// Interpolates two parameter sets (morph).
pub fn lerp_params(a: &[[f32; 4]], b: &[[f32; 4]], t: f32) -> Vec<[f32; 4]> {
    a.iter()
        .zip(b)
        .map(|(x, y)| std::array::from_fn(|i| x[i] + (y[i] - x[i]) * t))
        .collect()
}

pub const MIN_SCALE: f32 = 0.5;
/// Frames per measurement window.
const WINDOW: u32 = 60;
/// Missed-frame share that calls for a lower scale.
const HEAVY: f32 = 0.3;
/// Missed-frame share that counts as clean.
const CLEAN: f32 = 0.02;
/// Clean windows (≈ 10 s) before trying a higher scale.
const RAISE_AFTER: u32 = 10;
/// After lowering did not help: frames (≈ 5 min) at full scale before trying again.
const HOLD: u32 = 18_000;

/// Holds the frame rate by lowering the render scale when frames are missed — but only while
/// lowering actually helps.
///
/// The host presents with vsync, so a frame's wall time sits at the refresh interval whenever
/// rendering keeps up; a *miss* is a frame that took noticeably longer than that interval.
/// The interval itself is learned as the shortest recent frame time. Misses are counted per
/// one-second window, and the scale steps down 0.1 per heavy window. If even the floor does not
/// cut the miss rate, the bottleneck is elsewhere (CPU, compositor, display): the governor goes
/// back to the scale it started from instead of trading picture quality for nothing.
#[derive(Debug, Clone)]
pub struct Governor {
    pub scale: f32,
    pub max_scale: f32,
    interval: f32,
    frames: u32,
    misses: u32,
    clean_windows: u32,
    /// Scale before the current series of steps down, and the miss rate before the last step.
    stepping: Option<(f32, f32)>,
    hold: u32,
    /// Frames to leave out of the statistics (one-off hitches such as a new scene's first use).
    ignore: u32,
}

impl Default for Governor {
    fn default() -> Self {
        Self {
            scale: 1.0,
            max_scale: 1.0,
            interval: 1.0 / 60.0,
            frames: 0,
            misses: 0,
            clean_windows: 0,
            stepping: None,
            hold: 0,
            ignore: 0,
        }
    }
}

impl Governor {
    /// Feeds one frame's wall time (seconds); returns the scale to render the next frame at.
    pub fn update(&mut self, dt: f32) -> f32 {
        if !(0.0..0.5).contains(&dt) {
            return self.scale; // stalls (window moves, sleep) say nothing about GPU load
        }
        // Learn the refresh interval: track the minimum, forgetting slowly (display changes).
        self.interval = if dt < self.interval {
            dt.max(1.0 / 480.0)
        } else {
            self.interval * 0.999 + dt * 0.001
        };
        self.hold = self.hold.saturating_sub(1);
        if self.ignore > 0 {
            self.ignore -= 1;
            return self.scale.min(self.max_scale);
        }
        self.frames += 1;
        self.misses += (dt > self.interval * 1.4) as u32;
        if self.frames >= WINDOW {
            let rate = self.misses as f32 / self.frames as f32;
            self.frames = 0;
            self.misses = 0;
            self.window(rate);
        }
        self.scale = self.scale.min(self.max_scale);
        self.scale
    }

    fn window(&mut self, rate: f32) {
        if rate > HEAVY {
            self.clean_windows = 0;
            match self.stepping {
                // Down to the floor and still missing as much as before: the GPU is not the
                // bottleneck. Undo the whole series and stop trying for a while.
                Some((from, before)) if self.scale <= MIN_SCALE && rate >= before * 0.75 => {
                    self.scale = from;
                    self.stepping = None;
                    self.hold = HOLD;
                }
                _ if self.hold > 0 || self.scale <= MIN_SCALE => {}
                prev => {
                    self.stepping = Some(prev.unwrap_or((self.scale, rate)));
                    self.scale = (self.scale - 0.1).max(MIN_SCALE);
                }
            }
        } else {
            // Acceptable now: the steps taken so far helped; keep them.
            self.stepping = None;
            if rate < CLEAN {
                self.clean_windows += 1;
                if self.clean_windows >= RAISE_AFTER {
                    self.scale = (self.scale + 0.1).min(self.max_scale);
                    self.clean_windows = 0;
                }
            } else {
                self.clean_windows = 0;
            }
        }
    }

    /// Leave the next second of frames out: a scene switch (new pipeline, new targets) causes a
    /// one-off hitch that says nothing about steady-state load.
    pub fn settle(&mut self) {
        self.ignore = WINDOW;
        self.frames = 0;
        self.misses = 0;
    }

    /// The display's refresh interval as learned from frame times (seconds).
    pub fn refresh_interval(&self) -> f32 {
        self.interval
    }

    /// A crossfade renders two scenes: step down before it starts rather than after it stutters.
    pub fn pre_lower(&mut self) {
        self.scale = (self.scale - 0.1).max(MIN_SCALE);
        self.clean_windows = 0;
    }

    /// Render size for a screen size (pixels) at the current scale.
    pub fn size(&self, screen: (u32, u32)) -> (u32, u32) {
        let s = self.scale.min(self.max_scale);
        (
            ((screen.0 as f32 * s) as u32).max(1),
            ((screen.1 as f32 * s) as u32).max(1),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_ease_and_finish() {
        let x = Transition {
            kind: TransitionKind::Crossfade,
            start_beats: 16.0,
            length_beats: 4.0,
        };
        assert_eq!(x.progress(16.0), Some(0.0));
        assert_eq!(x.progress(18.0), Some(0.5));
        assert_eq!(x.progress(20.0), None, "lands on the downbeat");
        assert_eq!(
            Transition {
                kind: TransitionKind::Cut,
                ..x
            }
            .progress(16.0),
            None
        );
        assert_eq!(next_downbeat(13.0, 0.25), 16.0);
        assert_eq!(next_downbeat(16.0, 0.0), 16.0);
        assert_eq!(
            lerp_params(&[[0.0, 2.0, 0.0, 0.0]], &[[1.0, 4.0, 0.0, 0.0]], 0.5),
            [[0.5, 3.0, 0.0, 0.0]]
        );
    }

    /// Frames that miss vsync while the scale is above `fits`, as a GPU-bound scene would.
    fn gpu_bound(g: &mut Governor, fits: f32, frames: usize) {
        for _ in 0..frames {
            let dt = if g.scale > fits + 1e-4 {
                2.0 / 60.0
            } else {
                1.0 / 60.0
            };
            g.update(dt);
        }
    }

    #[test]
    fn governor_steps_down_while_it_helps_and_back_up_when_clean() {
        let mut g = Governor::default();
        gpu_bound(&mut g, 1.0, 600);
        assert_eq!(g.scale, 1.0);
        gpu_bound(&mut g, 0.7, 600);
        assert!(
            (g.scale - 0.7).abs() < 1e-4,
            "settles where frames fit: {}",
            g.scale
        );
        gpu_bound(&mut g, 1.0, 60 * RAISE_AFTER as usize * 3 + 60);
        assert_eq!(g.scale, 1.0, "raises slowly once clean");
    }

    #[test]
    fn governor_ignores_the_hitch_after_a_scene_switch() {
        let mut g = Governor::default();
        gpu_bound(&mut g, 1.0, 120);
        g.settle();
        for _ in 0..WINDOW {
            g.update(3.0 / 60.0);
        }
        gpu_bound(&mut g, 1.0, 600);
        assert_eq!(g.scale, 1.0);
    }

    #[test]
    fn governor_undoes_steps_that_do_not_help() {
        // Every frame misses whatever the scale: the bottleneck is not the GPU.
        let mut g = Governor::default();
        let mut lowest: f32 = 1.0;
        for _ in 0..1200 {
            g.update(2.0 / 60.0);
            g.update(1.0 / 60.0); // keeps the learned interval at 60 Hz
            lowest = lowest.min(g.scale);
        }
        assert_eq!(lowest, MIN_SCALE, "tried down to the floor");
        assert_eq!(g.scale, 1.0, "and went back to full quality");
    }

    #[test]
    fn governor_ignores_stalls_and_respects_caps() {
        let mut g = Governor {
            max_scale: 0.75,
            ..Default::default()
        };
        g.update(3.0);
        assert_eq!(g.update(1.0 / 60.0), 0.75);
        assert_eq!(g.size((1920, 1080)), (1440, 810));
        g.pre_lower();
        assert!((g.scale - 0.65).abs() < 1e-6);
        // A 120 Hz display: 8.3 ms frames are not misses of a 60 Hz budget.
        let mut g = Governor::default();
        for _ in 0..300 {
            g.update(1.0 / 120.0);
        }
        assert_eq!(g.scale, 1.0);
    }
}
