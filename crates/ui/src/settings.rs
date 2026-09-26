//! Persistent settings, playlist and EQ presets as RON files in the platform config directory.

use std::path::{Path, PathBuf};

use audio::{EqPresets, EqSettings};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        }
    }

    pub fn to_engine(self) -> audio::RepeatMode {
        match self {
            Repeat::Off => audio::RepeatMode::Off,
            Repeat::All => audio::RepeatMode::All,
            Repeat::One => audio::RepeatMode::One,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VisMode {
    #[default]
    Spectrum,
    Scope,
    Off,
}

impl VisMode {
    pub fn cycle(self) -> Self {
        match self {
            VisMode::Spectrum => VisMode::Scope,
            VisMode::Scope => VisMode::Off,
            VisMode::Off => VisMode::Spectrum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Skin pixels per UI point: 1 (classic size) or 2 (double size).
    pub scale: u8,
    pub window_pos: Option<(f32, f32)>,
    pub show_eq: bool,
    pub show_playlist: bool,
    /// Visible playlist rows.
    pub playlist_rows: u16,
    pub volume: f32,
    pub balance: f32,
    pub eq: EqSettings,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub time_remaining: bool,
    pub vis: VisMode,
    pub av_offset_ms: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scale: 2,
            window_pos: None,
            show_eq: true,
            show_playlist: true,
            playlist_rows: 10,
            volume: 0.8,
            balance: 0.0,
            eq: EqSettings::default(),
            shuffle: false,
            repeat: Repeat::Off,
            time_remaining: false,
            vis: VisMode::Spectrum,
            av_offset_ms: 0,
        }
    }
}

impl Settings {
    /// Clamps values that a hand-edited file could break.
    pub fn sanitized(mut self) -> Self {
        self.scale = self.scale.clamp(1, 3);
        self.playlist_rows = self.playlist_rows.clamp(4, 60);
        self.volume = self.volume.clamp(0.0, 1.0);
        self.balance = self.balance.clamp(-1.0, 1.0);
        self.eq = self.eq.clamped();
        self
    }
}

pub const SETTINGS_FILE: &str = "settings.ron";
pub const PLAYLIST_FILE: &str = "playlist.ron";
pub const PRESETS_FILE: &str = "eq_presets.ron";

/// Reads and writes RON files in one directory. Writes are atomic (temp file + rename).
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `~/Library/Application Support/winamp_rust` on macOS, the XDG/AppData equivalents
    /// elsewhere.
    pub fn platform_default() -> Option<Self> {
        dirs::config_dir().map(|d| Self::new(d.join("winamp_rust")))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Missing or unreadable files give the default (a broken file never blocks startup).
    pub fn load<T: DeserializeOwned + Default>(&self, name: &str) -> T {
        std::fs::read_to_string(self.dir.join(name))
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save<T: Serialize>(&self, name: &str, value: &T) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let text = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        let tmp = self.dir.join(format!(".{name}.tmp"));
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, self.dir.join(name))
    }

    pub fn load_settings(&self) -> Settings {
        self.load::<Settings>(SETTINGS_FILE).sanitized()
    }

    pub fn load_presets(&self) -> EqPresets {
        let p: EqPresets = self.load(PRESETS_FILE);
        if p.presets.is_empty() {
            EqPresets::default()
        } else {
            p
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio::EqPreset;

    fn store(name: &str) -> Store {
        let d = std::env::temp_dir().join(format!("ui-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        Store::new(d)
    }

    #[test]
    fn settings_round_trip_including_eq() {
        let s = store("settings");
        let mut settings = Settings::default();
        settings.eq.enabled = true;
        settings.eq.bands_db[3] = 6.0;
        settings.volume = 0.42;
        settings.repeat = Repeat::One;
        s.save(SETTINGS_FILE, &settings).unwrap();
        assert_eq!(s.load_settings(), settings);
    }

    #[test]
    fn missing_or_broken_files_fall_back_to_defaults() {
        let s = store("broken");
        assert_eq!(s.load_settings(), Settings::default());
        std::fs::create_dir_all(s.dir()).unwrap();
        std::fs::write(s.dir().join(SETTINGS_FILE), "not ron {").unwrap();
        assert_eq!(s.load_settings(), Settings::default());
        // Older files with missing fields keep working.
        std::fs::write(s.dir().join(SETTINGS_FILE), "(volume: 0.5)").unwrap();
        assert_eq!(s.load_settings().volume, 0.5);
        assert!(s.load_settings().show_playlist);
    }

    #[test]
    fn user_preset_survives_restart() {
        let s = store("presets");
        let mut presets = s.load_presets();
        assert!(presets.get("Techno").is_some());
        let mine = EqSettings {
            enabled: true,
            preamp_db: 2.0,
            bands_db: [3.0; 10],
        };
        presets.save(EqPreset::from_settings("My Club", &mine));
        s.save(PRESETS_FILE, &presets).unwrap();
        assert_eq!(s.load_presets().get("My Club").unwrap().preamp_db, 2.0);
    }

    #[test]
    fn values_are_sanitized() {
        let s = Settings {
            scale: 9,
            volume: 3.0,
            playlist_rows: 0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!((s.scale, s.volume, s.playlist_rows), (3, 1.0, 4));
    }

    #[test]
    fn cycles() {
        assert_eq!(Repeat::Off.cycle().cycle().cycle(), Repeat::Off);
        assert_eq!(VisMode::Spectrum.cycle(), VisMode::Scope);
    }
}
