//! The level meter's ballistics: how its bars move, as a hardware LED meter's do. A bar jumps
//! up to a new peak at once and falls back at a steady rate in dB, so hits read instantly and
//! the bar never jitters; a peak dot marks the highest level for a moment, then falls too.
//!
//! The levels come from the audio engine as the loudest peak since the last frame
//! (`audio::Engine::levels`), so even a transient shorter than a frame reaches the bar.

/// The bottom of the scale: below this a bar is empty.
pub const FLOOR_DB: f32 = -48.0;
/// The scale's segments, 2 dB each from [`FLOOR_DB`] to 0 dBFS.
pub const SEGMENTS: usize = 24;
const SEGMENT_DB: f32 = -FLOOR_DB / SEGMENTS as f32;
/// How fast a bar and a falling peak dot drop: 20 dB in 1.5 s.
pub const RELEASE_DB_PER_SEC: f32 = 20.0 / 1.5;
/// How long a peak dot hangs before it falls.
pub const HOLD_SECS: f32 = 1.0;
/// Where the colours change: amber from −12 dB, red from −3 dB.
const AMBER_DB: f32 = -12.0;
const RED_DB: f32 = -3.0;

/// A linear peak (1.0 is full scale) in dBFS, [`FLOOR_DB`] for silence.
pub fn db(linear: f32) -> f32 {
    if linear <= 0.0 || !linear.is_finite() {
        FLOOR_DB
    } else {
        (20.0 * linear.log10()).clamp(FLOOR_DB, 0.0)
    }
}

/// A segment's colour (0 is the bottom one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Led {
    Green,
    Amber,
    Red,
}

impl Led {
    pub fn of(segment: usize) -> Self {
        let centre = FLOOR_DB + (segment as f32 + 0.5) * SEGMENT_DB;
        if centre >= RED_DB {
            Led::Red
        } else if centre >= AMBER_DB {
            Led::Amber
        } else {
            Led::Green
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Led::Green => "green",
            Led::Amber => "amber",
            Led::Red => "red",
        }
    }
}

/// One channel's bar: its level and its peak dot, in dBFS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    pub level: f32,
    pub peak: f32,
    /// How long the peak dot still hangs, in seconds.
    hold: f32,
}

impl Default for Bar {
    fn default() -> Self {
        Self {
            level: FLOOR_DB,
            peak: FLOOR_DB,
            hold: 0.0,
        }
    }
}

impl Bar {
    /// Moves the bar `dt` seconds on, with `peak` (linear) the loudest heard since the last step.
    pub fn step(&mut self, peak: f32, dt: f32) {
        let p = db(peak);
        let dt = dt.max(0.0);
        let fall = RELEASE_DB_PER_SEC * dt;
        self.level = if p >= self.level {
            p
        } else {
            (self.level - fall).max(p)
        };
        if p >= self.peak {
            self.peak = p;
            self.hold = HOLD_SECS;
        } else if self.hold >= dt {
            self.hold -= dt;
        } else {
            // The hold ran out within this step: fall for what's left of it.
            let falling = dt - self.hold;
            self.hold = 0.0;
            self.peak = (self.peak - RELEASE_DB_PER_SEC * falling).max(self.level);
        }
    }

    /// The bar without ballistics (Reduce motion): just the level, no dot.
    pub fn set(&mut self, peak: f32) {
        self.level = db(peak);
        self.peak = FLOOR_DB;
        self.hold = 0.0;
    }

    /// Empty, with no dot: nothing left to draw or move.
    pub fn settled(&self) -> bool {
        self.level <= FLOOR_DB && self.peak <= FLOOR_DB
    }

    /// How many segments are lit, from the bottom.
    pub fn lit(&self) -> usize {
        segments_to(self.level)
    }

    /// The segment the peak dot is in, when it shows above the bar.
    pub fn dot(&self) -> Option<usize> {
        let n = segments_to(self.peak);
        (n > self.lit()).then(|| n - 1)
    }
}

/// How many segments a level fills.
fn segments_to(level: f32) -> usize {
    (((level - FLOOR_DB) / SEGMENT_DB).ceil().max(0.0) as usize).min(SEGMENTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    fn lin(db: f32) -> f32 {
        10f32.powf(db / 20.0)
    }

    #[test]
    fn attack_is_instant() {
        let mut b = Bar::default();
        b.step(0.0, FRAME);
        b.step(lin(-6.0), FRAME);
        assert!((b.level - -6.0).abs() < 1e-3, "{b:?}");
        assert!((b.peak - -6.0).abs() < 1e-3);
    }

    #[test]
    fn release_falls_20_db_in_a_second_and_a_half_and_the_dot_hangs_a_second() {
        let mut b = Bar::default();
        b.step(lin(-6.0), FRAME);
        let mut t = 0.0;
        let mut dot_at_1s = None;
        while t < 1.5 - 1e-4 {
            b.step(0.0, FRAME);
            t += FRAME;
            if dot_at_1s.is_none() && t >= 0.99 {
                dot_at_1s = Some(b.peak);
            }
        }
        assert!((b.level - -26.0).abs() < 0.25, "after 1.5 s: {b:?}");
        assert!(
            (dot_at_1s.unwrap() - -6.0).abs() < 1e-3,
            "the dot hung for a second"
        );
        assert!(b.peak < -6.0 && b.peak > b.level, "then falls: {b:?}");
    }

    #[test]
    fn a_very_short_transient_shows_on_the_next_frame() {
        // The engine keeps the loudest peak between two frames; one step sees it.
        let mut b = Bar::default();
        b.step(lin(-3.5), FRAME);
        assert_eq!(b.lit(), segments_to(-3.5));
        assert!(b.lit() >= 22);
    }

    #[test]
    fn a_bar_empties_and_settles() {
        let mut b = Bar::default();
        b.step(1.0, FRAME);
        let mut frames = 0;
        while !b.settled() {
            b.step(0.0, FRAME);
            frames += 1;
            assert!(frames < 400);
        }
        // 48 dB at 13.3 dB/s, after the dot's second: within 3.6 s + 1 s.
        assert!(frames as f32 * FRAME <= 4.7, "{frames} frames");
    }

    #[test]
    fn segments_and_colours() {
        assert_eq!(db(0.0), FLOOR_DB);
        assert_eq!(db(1.0), 0.0);
        assert_eq!(Bar::default().lit(), 0);
        let mut b = Bar::default();
        b.set(1.0);
        assert_eq!(b.lit(), SEGMENTS);
        assert_eq!(b.dot(), None, "no dot without ballistics");
        assert_eq!(Led::of(0), Led::Green);
        assert_eq!(Led::of(17), Led::Green, "-14..-12 dB");
        assert_eq!(Led::of(18), Led::Amber, "-12..-10 dB");
        assert_eq!(Led::of(21), Led::Amber, "-6..-4 dB");
        assert_eq!(Led::of(22), Led::Red, "-4..-2 dB, centred on -3");
        assert_eq!(Led::of(23), Led::Red);
    }
}
