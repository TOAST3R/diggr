//! Dig settings and the Discogs token, in `<config>/dig/`.
//!
//! The token sits alone in `dig/token`, written atomically and readable only by the user
//! (mode 0600 on Unix; the config folder is per-user on Windows). It is never logged, and the
//! UI only ever shows its last 4 characters.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DIR: &str = "dig";
pub const SETTINGS_FILE: &str = "settings.ron";
pub const TOKEN_FILE: &str = "token";
/// Preview cache limit when nothing else is set.
pub const DEFAULT_CACHE_GB: f32 = 2.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DigSettings {
    /// Leave out records with no vinyl format.
    pub vinyl_only: bool,
    /// Leave out clips the user passed on.
    pub skip_passed: bool,
    /// The preview cache limit, in GB.
    pub cache_gb: f32,
    /// yt-dlp to run instead of the one on the PATH.
    pub ytdlp_path: Option<String>,
    /// The wantlist crate, once it exists (older files call it the Keepers crate).
    #[serde(alias = "keepers")]
    pub wantlist: Option<u64>,
    /// The Keepers crate has been renamed "Wantlist" (done once, on the first launch after
    /// wanting replaced keeping).
    pub wantlist_named: bool,
    /// "Don't show this again" in the Connect to Discogs dialog.
    pub connect_hint_dismissed: bool,
}

impl Default for DigSettings {
    fn default() -> Self {
        Self {
            vinyl_only: true,
            skip_passed: true,
            cache_gb: DEFAULT_CACHE_GB,
            ytdlp_path: None,
            wantlist: None,
            wantlist_named: false,
            connect_hint_dismissed: false,
        }
    }
}

impl DigSettings {
    pub fn sanitized(mut self) -> Self {
        if !self.cache_gb.is_finite() {
            self.cache_gb = DEFAULT_CACHE_GB;
        }
        self.cache_gb = self.cache_gb.clamp(0.1, 1000.0);
        self.ytdlp_path = self.ytdlp_path.filter(|p| !p.trim().is_empty());
        self
    }

    pub fn cache_bytes(&self) -> u64 {
        (f64::from(self.cache_gb) * 1e9) as u64
    }
}

/// `<config>/dig/`.
pub fn dir(config: &Path) -> PathBuf {
    config.join(DIR)
}

pub fn load_settings(config: &Path) -> DigSettings {
    std::fs::read_to_string(dir(config).join(SETTINGS_FILE))
        .ok()
        .and_then(|s| ron::from_str::<DigSettings>(&s).ok())
        .unwrap_or_default()
        .sanitized()
}

pub fn save_settings(config: &Path, s: &DigSettings) -> std::io::Result<()> {
    let text = ron::ser::to_string_pretty(s, ron::ser::PrettyConfig::default())
        .map_err(std::io::Error::other)?;
    write_atomic(&dir(config).join(SETTINGS_FILE), text.as_bytes(), false)
}

pub fn load_token(config: &Path) -> Option<String> {
    let t = std::fs::read_to_string(dir(config).join(TOKEN_FILE)).ok()?;
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// Saves the token (or removes it with `None`), readable only by the user.
pub fn save_token(config: &Path, token: Option<&str>) -> std::io::Result<()> {
    let path = dir(config).join(TOKEN_FILE);
    match token.map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => write_atomic(&path, t.as_bytes(), true),
        None => match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    }
}

/// `••••abcd`: only the last 4 characters.
pub fn masked(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    let tail: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    format!("••••{tail}")
}

/// Temp file and rename; `private` makes the file readable only by its owner from the start.
pub fn write_atomic(path: &Path, bytes: &[u8], private: bool) -> std::io::Result<()> {
    let parent = path.parent().expect("has parent");
    std::fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name().expect("file").to_string_lossy()
    ));
    let _ = std::fs::remove_file(&tmp);
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut f = opts.open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_with_defaults() {
        let d = crate::test_dir("config-settings");
        assert_eq!(load_settings(&d), DigSettings::default());
        let s = DigSettings {
            vinyl_only: false,
            cache_gb: 5.0,
            ytdlp_path: Some("/opt/bin/yt-dlp".into()),
            wantlist: Some(7),
            connect_hint_dismissed: true,
            ..Default::default()
        };
        save_settings(&d, &s).unwrap();
        assert_eq!(load_settings(&d), s);
        // An older file's Keepers crate is the wantlist crate.
        std::fs::write(d.join("dig/settings.ron"), "(keepers: Some(9))").unwrap();
        assert_eq!(load_settings(&d).wantlist, Some(9));
        std::fs::write(d.join("dig/settings.ron"), "(cache_gb: -3.0)").unwrap();
        let s = load_settings(&d);
        assert_eq!((s.cache_gb, s.vinyl_only), (0.1, true));
    }

    #[test]
    fn the_token_is_private_and_masked() {
        let d = crate::test_dir("config-token");
        assert_eq!(load_token(&d), None);
        save_token(&d, Some(" abcdefgh1234 \n")).unwrap();
        assert_eq!(load_token(&d).as_deref(), Some("abcdefgh1234"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(d.join("dig/token"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(masked("abcdefgh1234"), "••••1234");
        assert_eq!(masked("ab"), "••••ab");
        save_token(&d, None).unwrap();
        assert_eq!(load_token(&d), None);
        save_token(&d, None).unwrap();
    }
}
