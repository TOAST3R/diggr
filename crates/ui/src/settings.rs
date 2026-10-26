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

/// A Discogs group in the crate sidebar: at most one is open at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideGroup {
    Labels,
    Sellers,
    Friends,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Skin pixels per UI point: 1 (classic size) or 2 (double size).
    pub scale: u8,
    /// The EQ window is open, and where it was last put.
    pub show_eq: bool,
    pub eq_window_pos: Option<(f32, f32)>,
    /// The waveform in the band (`W`).
    pub show_waveform: bool,
    /// Bars visible in the waveform's zoomed row.
    pub waveform_bars: f32,
    /// The playlist's columns, shown once it is wide enough.
    pub columns: crate::columns::ColumnSettings,
    pub volume: f32,
    pub eq: EqSettings,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub time_remaining: bool,
    pub av_offset_ms: i64,
    /// The spectrogram window's mode, channel, dB range and size.
    pub spectrogram: crate::spectrogram::SpectroSettings,
    /// The open Discogs group in the crate sidebar (LABELS, TOP SELLERS or FRIENDS), if any.
    pub side_open: Option<SideGroup>,
    /// CRATES (the user's own crates) is folded in the crate sidebar.
    pub crates_folded: bool,
    /// SYNC: on, its target tempo and range.
    pub sync: crate::sync::SyncSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scale: 2,
            show_eq: false,
            eq_window_pos: None,
            show_waveform: true,
            waveform_bars: crate::waveform::DEFAULT_BARS,
            columns: Default::default(),
            volume: 0.8,
            eq: EqSettings::default(),
            shuffle: false,
            repeat: Repeat::Off,
            time_remaining: false,
            av_offset_ms: 0,
            spectrogram: Default::default(),
            side_open: None,
            crates_folded: false,
            sync: Default::default(),
        }
    }
}

impl Settings {
    /// Clamps values that a hand-edited file could break.
    pub fn sanitized(mut self) -> Self {
        self.scale = self.scale.clamp(1, 3);
        self.columns = self.columns.sanitized();
        self.volume = self.volume.clamp(0.0, 1.0);
        self.eq = self.eq.clamped();
        self.spectrogram = self.spectrogram.sanitized();
        self.sync = self.sync.sanitized();
        self
    }
}

pub const SETTINGS_FILE: &str = "settings.ron";
pub const PLAYLIST_FILE: &str = "playlist.ron";
pub const PRESETS_FILE: &str = "eq_presets.ron";
const VISUALS_DIR: &str = "visuals";

/// Reads and writes RON files in one directory. Writes are atomic (temp file + rename).
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `~/Library/Application Support/Diggr` on macOS, the XDG/AppData equivalents
    /// elsewhere.
    pub fn platform_default() -> Option<Self> {
        dirs::config_dir().map(|d| Self::new(d.join(platform::APP_DIR)))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Deletes the `visuals` folder earlier versions kept the user's scenes and variants in:
    /// nothing reads it any more. Only that folder; a failure leaves it.
    pub fn remove_old_visuals(&self) {
        let _ = std::fs::remove_dir_all(self.dir.join(VISUALS_DIR));
    }

    /// Missing or unreadable files give the default (a broken file never blocks startup).
    pub fn load<T: DeserializeOwned + Default>(&self, name: &str) -> T {
        std::fs::read_to_string(self.dir.join(name))
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// `Ok(None)` if the file doesn't exist; `Err` if it exists but can't be read or parsed.
    pub fn try_load<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, String> {
        let text = match std::fs::read_to_string(self.dir.join(name)) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        ron::from_str(&text).map(Some).map_err(|e| e.to_string())
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

/// A store in its own test folder, deleted with it.
#[cfg(test)]
pub(crate) struct TestStore {
    store: Store,
    _dir: platform::testing::TestDir,
}

#[cfg(test)]
impl TestStore {
    pub(crate) fn new(name: &str) -> Self {
        let dir = platform::testing::TestDir::new(name);
        Self {
            store: Store::new(dir.path()),
            _dir: dir,
        }
    }
}

#[cfg(test)]
impl std::ops::Deref for TestStore {
    type Target = Store;

    fn deref(&self) -> &Store {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio::EqPreset;

    fn store(name: &str) -> TestStore {
        TestStore::new(&format!("ui-store-{name}"))
    }

    #[test]
    fn the_old_visuals_folder_goes_and_nothing_else() {
        let s = store("old-visuals");
        let scenes = s.dir().join("visuals/scenes");
        std::fs::create_dir_all(&scenes).unwrap();
        std::fs::write(scenes.join("tunnel.ron"), "()").unwrap();
        s.save(SETTINGS_FILE, &Settings::default()).unwrap();
        std::fs::write(s.dir().join("visuals.txt"), "keep").unwrap();
        s.remove_old_visuals();
        assert!(!s.dir().join("visuals").exists());
        assert!(s.dir().join(SETTINGS_FILE).exists());
        assert!(s.dir().join("visuals.txt").exists());
        // Nothing there: nothing happens, nothing is created.
        s.remove_old_visuals();
        assert!(!s.dir().join("visuals").exists());
    }

    #[test]
    fn settings_round_trip_including_eq() {
        let s = store("settings");
        let mut settings = Settings::default();
        settings.eq.enabled = true;
        settings.eq.bands_db[3] = 6.0;
        settings.volume = 0.42;
        settings.repeat = Repeat::One;
        settings.sync.on = true;
        settings.sync.target = 126;
        settings.sync.wide = true;
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
        assert!(!s.load_settings().show_eq, "the EQ window starts closed");
        // The balance they may hold is gone: the file still loads, and playback is centred.
        std::fs::write(
            s.dir().join(SETTINGS_FILE),
            "(volume: 0.5, balance: -0.6, repeat: One)",
        )
        .unwrap();
        let old = s.load_settings();
        assert_eq!((old.volume, old.repeat), (0.5, Repeat::One));
        // So are the show-render options the player used to remember.
        std::fs::write(
            s.dir().join(SETTINGS_FILE),
            "(volume: 0.5, render_size: (1280, 720), render_fps: 30, render_overlay: true, \
             repeat: One)",
        )
        .unwrap();
        let old = s.load_settings();
        assert_eq!((old.volume, old.repeat), (0.5, Repeat::One));
        // The old LABELS fold is ignored: every Discogs group starts closed.
        std::fs::write(
            s.dir().join(SETTINGS_FILE),
            "(volume: 0.5, labels_folded: true)",
        )
        .unwrap();
        let old = s.load_settings();
        assert_eq!((old.volume, old.side_open), (0.5, None));
    }

    #[test]
    fn the_classic_layouts_settings_load_and_are_ignored() {
        let s = store("classic-layout");
        std::fs::create_dir_all(s.dir()).unwrap();
        std::fs::write(
            s.dir().join(SETTINGS_FILE),
            "(volume: 0.5, show_eq: true, show_crates: true, show_playlist: false, \
             playlist_maximized: true, playlist_rows: 20, playlist_width: 700, vis: Scope, \
             window_pos: Some((10.0, 20.0)))",
        )
        .unwrap();
        let old = s.load_settings();
        assert_eq!(old.volume, 0.5, "the file loads");
        assert!(old.show_eq, "an older EQ section opens the EQ window");
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
            ..Default::default()
        }
        .sanitized();
        assert_eq!((s.scale, s.volume), (3, 1.0));
    }

    #[test]
    fn cycles() {
        assert_eq!(Repeat::Off.cycle().cycle().cycle(), Repeat::Off);
    }
}
