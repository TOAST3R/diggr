//! SYNC: every track heard at one target tempo, the way it will sit in a set. Speed is
//! vinyl-style (pitch follows); a track the range can't reach plays at its own tempo.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncSettings {
    pub on: bool,
    /// The tempo every track is fitted to, in BPM.
    pub target: u16,
    /// ±16% instead of ±8%.
    pub wide: bool,
    /// Turned on while the playing track's BPM wasn't known: that track (its engine id),
    /// whose BPM becomes the target once analyzed. Never saved.
    #[serde(skip)]
    pub pending: Option<u64>,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            on: false,
            target: 124,
            wide: false,
            pending: None,
        }
    }
}

/// The target's limits, in BPM.
pub const TARGET: (u16, u16) = (40, 250);

impl SyncSettings {
    pub fn sanitized(mut self) -> Self {
        self.target = self.target.clamp(TARGET.0, TARGET.1);
        self
    }

    /// How far from its own speed a track may play.
    pub fn range(&self) -> f64 {
        if self.wide { 0.16 } else { 0.08 }
    }

    /// The speed a track of `bpm` plays at to sound at the target: of its BPM, double and
    /// half, the one needing the speed closest to 1. `None` (its own tempo) when SYNC is off,
    /// it has no BPM, or that speed is out of range.
    pub fn speed(&self, bpm: Option<u16>) -> Option<f64> {
        let b = bpm.filter(|&b| self.on && b > 0)? as f64;
        let s = [b, b * 2.0, b / 2.0]
            .map(|b| self.target as f64 / b)
            .into_iter()
            .min_by(|x, y| (x - 1.0).abs().total_cmp(&(y - 1.0).abs()))?;
        // (A hair over, so a range edge like 124 → 133.92 still fits.)
        ((s - 1.0).abs() <= self.range() + 1e-9).then_some(s)
    }

    pub fn nudge(&mut self, bpm: i32) {
        self.target = (self.target as i32 + bpm).clamp(TARGET.0 as i32, TARGET.1 as i32) as u16;
    }
}

/// The main window's speed readout: "+2.4%", "-3.0%", or nothing at the track's own speed.
pub fn speed_label(speed: f64) -> Option<String> {
    let pct = (speed - 1.0) * 100.0;
    (pct.abs() >= 0.05).then(|| format!("{pct:+.1}%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(target: u16) -> SyncSettings {
        SyncSettings {
            on: true,
            target,
            wide: false,
            pending: None,
        }
    }

    #[test]
    fn fitted() {
        let s = on(126).speed(Some(122)).unwrap();
        assert!((s - 1.0328).abs() < 1e-4, "{s}");
        assert_eq!(on(124).speed(Some(124)), Some(1.0));
    }

    #[test]
    fn half_and_double_time() {
        // 64 BPM fits 126 as 128 (0.984), not as 1.97.
        let s = on(126).speed(Some(64)).unwrap();
        assert!((s - 0.984375).abs() < 1e-9, "{s}");
        let s = on(87).speed(Some(172)).unwrap();
        assert!((s - 87.0 / 86.0).abs() < 1e-9, "{s}");
    }

    #[test]
    fn out_of_range_and_no_bpm_play_at_their_own_tempo() {
        assert_eq!(on(126).speed(Some(140)), None);
        let wide = SyncSettings {
            wide: true,
            ..on(126)
        };
        assert!(wide.speed(Some(140)).is_some(), "126/140 = −10% fits ±16%");
        assert_eq!(on(126).speed(None), None);
        assert_eq!(SyncSettings::default().speed(Some(124)), None, "off");
    }

    #[test]
    fn target_and_label() {
        let mut s = on(249);
        s.nudge(5);
        assert_eq!(s.target, TARGET.1);
        assert_eq!(speed_label(126.0 / 123.0).as_deref(), Some("+2.4%"));
        assert_eq!(speed_label(0.97).as_deref(), Some("-3.0%"));
        assert_eq!(speed_label(1.0), None);
    }
}
