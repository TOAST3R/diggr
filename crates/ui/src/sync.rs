//! SYNC: every track heard at one target tempo, the way it will sit in a set. Speed is
//! vinyl-style (pitch follows); a track the range can't reach plays at its own tempo.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncSettings {
    pub on: bool,
    /// The tempo every track is fitted to, in whole BPM: what older builds read and write.
    /// Use [`Self::cents`] for the tempo itself.
    pub target: u16,
    /// The tempo in hundredths of a BPM. It counts while it rounds to `target`; a file from an
    /// older build has none, and one an older build saved may hold a `target` that moved on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_cents: Option<u32>,
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
            target_cents: None,
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
        if self.target_cents.is_some() {
            let cents = self.cents();
            self.set_cents(cents);
        }
        self
    }

    /// The tempo every track is fitted to, in hundredths of a BPM.
    pub fn cents(&self) -> u32 {
        let whole = self.target as u32 * 100;
        match self.target_cents {
            Some(c) if (c + 50) / 100 == self.target as u32 => c,
            _ => whole,
        }
    }

    /// The tempo every track is fitted to, in BPM.
    pub fn target_bpm(&self) -> f64 {
        self.cents() as f64 / 100.0
    }

    /// Sets the tempo, in hundredths of a BPM, kept within [`TARGET`]; both fields follow.
    pub fn set_cents(&mut self, cents: u32) {
        let c = cents.clamp(TARGET.0 as u32 * 100, TARGET.1 as u32 * 100);
        self.target_cents = Some(c);
        self.target = ((c + 50) / 100) as u16;
    }

    /// The tempo a record of `bpm` can be fitted to, in hundredths: its range either side.
    pub fn range_cents(&self, bpm: u16) -> (u32, u32) {
        let p = if self.wide { 16 } else { 8 };
        (bpm as u32 * (100 - p), bpm as u32 * (100 + p))
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
            .map(|b| self.target_bpm() / b)
            .into_iter()
            .min_by(|x, y| (x - 1.0).abs().total_cmp(&(y - 1.0).abs()))?;
        // (A hair over, so a range edge like 124 → 133.92 still fits.)
        ((s - 1.0).abs() <= self.range() + 1e-9).then_some(s)
    }

    /// Where SYNC stands for the playing record (`bpm`, `analyzing` while its tempo is
    /// being read, `playing` for anything but stopped): what the turntable's state line says.
    pub fn fit(&self, bpm: Option<u16>, analyzing: bool, playing: bool) -> Fit {
        if !self.on {
            return Fit::Off;
        }
        if self.pending.is_some() || analyzing {
            return Fit::Analyzing;
        }
        if !playing {
            return Fit::Idle;
        }
        let Some(b) = bpm.filter(|&b| b > 0) else {
            return Fit::NoBpm;
        };
        let s = self.raw_speed(b);
        if (s - 1.0).abs() <= self.range() + 1e-9 {
            Fit::Locked(s)
        } else {
            Fit::Out(s)
        }
    }

    /// The speed that would fit a record of `bpm` to the target, whether or not the range
    /// allows it (see [`Self::speed`]).
    pub fn raw_speed(&self, bpm: u16) -> f64 {
        let b = bpm.max(1) as f64;
        [b, b * 2.0, b / 2.0]
            .map(|b| self.target_bpm() / b)
            .into_iter()
            .min_by(|x, y| (x - 1.0).abs().total_cmp(&(y - 1.0).abs()))
            .unwrap_or(1.0)
    }

    /// Steps the tempo by `cents` hundredths of a BPM (100 is 1 BPM).
    pub fn nudge(&mut self, cents: i32) {
        let c = (self.cents() as i64 + cents as i64).max(0) as u32;
        self.set_cents(c);
    }
}

/// SYNC's state for the playing record, as the turntable shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fit {
    /// SYNC is off.
    Off,
    /// On, nothing plays.
    Idle,
    /// Waiting for the record's BPM.
    Analyzing,
    /// Fitted to the target at this speed.
    Locked(f64),
    /// The speed that would fit it (this one) is outside the range: it plays at its own.
    Out(f64),
    /// The record has no BPM: it plays at its own tempo.
    NoBpm,
}

impl Fit {
    /// The turntable's state line (at most 17 characters).
    pub fn line(&self, range: f64) -> String {
        match self {
            Fit::Off => "SYNC OFF".into(),
            Fit::Idle => "SYNC ON".into(),
            Fit::Analyzing => "ANALYZING".into(),
            Fit::Locked(s) => match speed_label(*s) {
                Some(p) => format!("LOCKED {p}"),
                None => "LOCKED".into(),
            },
            Fit::Out(_) => format!("OWN · OUT OF ±{}%", (range * 100.0).round() as u32),
            Fit::NoBpm => "OWN · NO BPM".into(),
        }
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
            target_cents: None,
            wide: false,
            pending: None,
        }
    }

    #[test]
    fn the_tempo_is_kept_to_hundredths_and_old_files_load() {
        let mut s = on(124);
        assert_eq!((s.cents(), s.target_bpm()), (12_400, 124.0));
        s.set_cents(12_635);
        assert_eq!((s.cents(), s.target), (12_635, 126), "both fields follow");
        s.nudge(1);
        assert_eq!(s.cents(), 12_636);
        s.nudge(-100);
        assert_eq!(s.cents(), 12_536);
        s.set_cents(30_000);
        assert_eq!(s.cents(), 25_000, "kept within 250");
        s.set_cents(100);
        assert_eq!(s.cents(), 4_000, "and 40");
        // A record's range in hundredths: 123 at ±8% is 113.16 to 132.84, 124 tops at 133.92.
        assert_eq!(on(123).range_cents(123), (11_316, 13_284));
        assert_eq!(on(124).range_cents(124).1, 13_392);
        // An old file has only the whole tempo; a new one keeps both.
        let old: SyncSettings = ron::from_str("(on: true, target: 124)").unwrap();
        assert_eq!(old.cents(), 12_400);
        let mut new = on(124);
        new.set_cents(12_635);
        let text = ron::to_string(&new).unwrap();
        assert!(
            text.contains("target:126") && text.contains("target_cents:Some(12635)"),
            "{text}"
        );
        let back: SyncSettings = ron::from_str(&text).unwrap();
        assert_eq!(back.cents(), 12_635);
        // An older build saved a new whole tempo over it: the whole tempo wins.
        let moved = SyncSettings {
            target: 130,
            ..back
        };
        assert_eq!(moved.cents(), 13_000);
        // The speed follows the hundredths.
        let mut f = on(124);
        f.set_cents(12_435);
        assert!((f.speed(Some(124)).unwrap() - 124.35 / 124.0).abs() < 1e-12);
    }

    #[test]
    fn the_state_line_says_what_sync_is_doing() {
        let line = |s: SyncSettings, bpm, analyzing, playing| {
            s.fit(bpm, analyzing, playing).line(s.range())
        };
        assert_eq!(
            line(SyncSettings::default(), Some(124), false, true),
            "SYNC OFF"
        );
        assert_eq!(line(on(126), Some(122), false, true), "LOCKED +3.3%");
        assert_eq!(line(on(124), Some(124), false, true), "LOCKED");
        assert_eq!(line(on(102), Some(124), false, true), "OWN · OUT OF ±8%");
        assert_eq!(line(on(126), None, false, true), "OWN · NO BPM");
        assert_eq!(line(on(126), None, true, true), "ANALYZING");
        assert_eq!(line(on(126), None, false, false), "SYNC ON");
        let wide = SyncSettings {
            wide: true,
            ..on(102)
        };
        assert!(
            matches!(wide.fit(Some(124), false, true), Fit::Out(_)),
            "102/124 = −18%"
        );
        // The longest line fits the state line's inset (17 characters after the lamp), also in
        // the wide range.
        assert!(line(on(102), Some(124), false, true).chars().count() <= 17);
        assert!(
            Fit::Out(0.8)
                .line(
                    SyncSettings {
                        wide: true,
                        ..on(102)
                    }
                    .range()
                )
                .chars()
                .count()
                <= 17
        );
        // Out of range, which way: 102 against 124 is a slow-down.
        let Fit::Out(s) = on(102).fit(Some(124), false, true) else {
            panic!()
        };
        assert!(s < 1.0);
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
        s.nudge(500);
        assert_eq!(s.target, TARGET.1);
        assert_eq!(speed_label(126.0 / 123.0).as_deref(), Some("+2.4%"));
        assert_eq!(speed_label(0.97).as_deref(), Some("-3.0%"));
        assert_eq!(speed_label(1.0), None);
    }
}
