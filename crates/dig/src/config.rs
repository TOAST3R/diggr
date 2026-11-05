//! Dig settings and the Discogs account, in `<config>/dig/`.
//!
//! A personal token sits alone in `dig/token`; an account connected with Discogs keeps its
//! access token and secret in `dig/oauth` (two lines). Both are written atomically and readable
//! only by the user (mode 0600 on Unix; the config folder is per-user on Windows), never
//! logged, and the UI only ever shows a token's last 4 characters. They are exclusive: saving
//! one removes the other.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discogs::oauth::{Consumer, Credentials};
use crate::discogs::transport::Auth;

pub const DIR: &str = "dig";
pub const SETTINGS_FILE: &str = "settings.ron";
pub const TOKEN_FILE: &str = "token";
pub const OAUTH_FILE: &str = "oauth";
/// Preview cache limit when nothing else is set.
pub const DEFAULT_CACHE_GB: f32 = 10.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DigSettings {
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
    /// The first launch's "Previews need yt-dlp" question was answered (the guided setup).
    pub ytdlp_setup_asked: bool,
    /// The setup steps the user said no to, and whether the welcome opens at launch.
    pub welcome: Welcome,
}

/// What the user answered in the setup panel (see `welcome-setup`). Whether a step is done is
/// read from the app (a token, a paired browser), never stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Welcome {
    /// "I don't have an account".
    pub no_discogs: bool,
    /// "I don't use Chrome".
    pub no_browser: bool,
    /// The welcome opens at launch while a step is open.
    pub at_launch: bool,
}

impl Default for Welcome {
    fn default() -> Self {
        Self {
            no_discogs: false,
            no_browser: false,
            at_launch: true,
        }
    }
}

impl Default for DigSettings {
    fn default() -> Self {
        Self {
            skip_passed: true,
            cache_gb: DEFAULT_CACHE_GB,
            ytdlp_path: None,
            wantlist: None,
            wantlist_named: false,
            connect_hint_dismissed: false,
            ytdlp_setup_asked: false,
            welcome: Welcome::default(),
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
        None => remove(&path),
    }
}

fn remove(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// The saved account: one connected with Discogs (when this build can sign for it, with
/// `consumer`), else a personal token. A broken `dig/oauth` counts as none.
pub fn load_auth(config: &Path, consumer: Option<&Consumer>) -> Option<Auth> {
    let oauth = consumer.and_then(|c| {
        let text = std::fs::read_to_string(dir(config).join(OAUTH_FILE)).ok()?;
        let mut lines = text.lines().map(str::trim);
        let (token, secret) = (lines.next()?, lines.next()?);
        (!token.is_empty() && !secret.is_empty()).then(|| {
            Auth::OAuth(Credentials::account(
                c.clone(),
                token.to_owned(),
                secret.to_owned(),
            ))
        })
    });
    oauth.or_else(|| load_token(config).map(Auth::Token))
}

/// Saves the account (one way removes the other's file), or removes both with `None`
/// (Disconnect).
pub fn save_auth(config: &Path, auth: Option<&Auth>) -> std::io::Result<()> {
    let oauth = dir(config).join(OAUTH_FILE);
    match auth {
        Some(Auth::Token(t)) => {
            save_token(config, Some(t))?;
            remove(&oauth)
        }
        Some(Auth::OAuth(c)) => {
            let token = c.token.as_deref().unwrap_or_default();
            write_atomic(&oauth, format!("{token}\n{}\n", c.secret).as_bytes(), true)?;
            save_token(config, None)
        }
        None => {
            save_token(config, None)?;
            remove(&oauth)
        }
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
            skip_passed: false,
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
        assert_eq!((s.cache_gb, s.skip_passed), (0.1, true));
        // Vinyl only is gone: a file that has it still loads.
        std::fs::write(
            d.join("dig/settings.ron"),
            "(vinyl_only: true, cache_gb: 3.0)",
        )
        .unwrap();
        assert_eq!(load_settings(&d).cache_gb, 3.0);
    }

    #[test]
    fn the_welcome_answers_round_trip_and_fail_soft() {
        let d = crate::test_dir("config-welcome");
        let w = load_settings(&d).welcome;
        assert!(
            !w.no_discogs && !w.no_browser && w.at_launch,
            "every step open"
        );
        let s = DigSettings {
            welcome: Welcome {
                no_discogs: true,
                no_browser: false,
                at_launch: false,
            },
            ..Default::default()
        };
        save_settings(&d, &s).unwrap();
        assert_eq!(load_settings(&d).welcome, s.welcome);
        // An older file without it, and a broken one, leave every step open.
        std::fs::write(d.join("dig/settings.ron"), "(cache_gb: 3.0)").unwrap();
        assert_eq!(load_settings(&d).welcome, Welcome::default());
        std::fs::write(d.join("dig/settings.ron"), "(welcome: (no_discogs: 7))").unwrap();
        assert_eq!(load_settings(&d).welcome, Welcome::default());
    }

    #[test]
    fn saving_over_a_file_replaces_it_in_any_folder() {
        // Windows won't rename onto a file some programs keep open, and folders there often
        // have spaces and accents (a user called Ana María).
        let d = crate::test_dir("config María García");
        let path = d.join("dig/some file.ron");
        write_atomic(&path, b"first", false).unwrap();
        write_atomic(&path, b"second", true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        let left: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["some file.ron"], "no temporary file is left");
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

    #[test]
    fn one_account_at_a_time_private_and_failing_soft() {
        let d = crate::test_dir("config-oauth");
        let consumer = Consumer {
            key: "ck".into(),
            secret: "cs".into(),
        };
        let account = Auth::OAuth(Credentials::account(
            consumer.clone(),
            "at".into(),
            "as".into(),
        ));
        assert_eq!(load_auth(&d, Some(&consumer)), None);
        save_auth(&d, Some(&Auth::Token("tok".into()))).unwrap();
        assert_eq!(
            load_auth(&d, Some(&consumer)),
            Some(Auth::Token("tok".into()))
        );

        // Connecting with Discogs replaces the token.
        save_auth(&d, Some(&account)).unwrap();
        assert_eq!(load_auth(&d, Some(&consumer)), Some(account.clone()));
        assert!(!d.join("dig/token").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(d.join("dig/oauth"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        // A build that can't sign (no consumer) can't use it.
        assert_eq!(load_auth(&d, None), None);

        // A token replaces the connection.
        save_auth(&d, Some(&Auth::Token("tok2".into()))).unwrap();
        assert!(!d.join("dig/oauth").exists());
        assert_eq!(
            load_auth(&d, Some(&consumer)),
            Some(Auth::Token("tok2".into()))
        );

        // Disconnect removes both.
        save_auth(&d, Some(&account)).unwrap();
        std::fs::write(d.join("dig/token"), "stale").unwrap();
        save_auth(&d, None).unwrap();
        assert!(!d.join("dig/oauth").exists() && !d.join("dig/token").exists());
        assert_eq!(load_auth(&d, Some(&consumer)), None);

        // A broken file is no account.
        std::fs::write(d.join("dig/oauth"), "only-one-line").unwrap();
        assert_eq!(load_auth(&d, Some(&consumer)), None);
        save_auth(&d, None).unwrap();
    }
}
