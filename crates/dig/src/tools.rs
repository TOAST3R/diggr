//! The app's own yt-dlp, for Macs without one (a Homebrew or configured yt-dlp always wins and
//! is never touched).
//!
//! It is downloaded only when the user says yes, only from yt-dlp's official GitHub releases
//! over HTTPS, and installed only when its SHA-256 matches the one published with the release:
//! the file goes to `tools/.yt-dlp.part` and is renamed to `tools/yt-dlp`, so a failed or
//! interrupted download never replaces a working copy, and downloads already running keep the
//! old file. At most once a day, while previews are fetched, a newer release replaces it the
//! same way (`yt-dlp -U` isn't used, so every copy passes the same check).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::discogs::transport::USER_AGENT;

/// The release asset: yt-dlp's universal macOS build. Never taken from anything downloaded.
pub const ASSET: &str = "yt-dlp_macos";
const RELEASES: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest";
const SUMS: &str = "SHA2-256SUMS";
/// Its size, as the question to download it says.
pub const SIZE_MB: u32 = 37;
/// The least time between two update checks.
pub const UPDATE_EVERY: Duration = Duration::from_secs(24 * 3600);
const MAX_BYTES: u64 = 200 << 20;

/// Where the release files come from, behind a trait so tests run offline.
pub trait ToolSource: Send + Sync {
    /// A file of the latest release, telling `progress` the percent done.
    fn get(&self, name: &str, progress: &dyn Fn(u8)) -> Result<Vec<u8>, String>;
    /// The latest release's tag ("2026.08.19").
    fn latest_tag(&self) -> Result<String, String>;
}

/// The real thing: GitHub over HTTPS.
pub struct GitHub {
    agent: ureq::Agent,
    /// Doesn't follow redirects: the latest tag is in the `Location` of `releases/latest`.
    no_redirects: ureq::Agent,
}

impl Default for GitHub {
    fn default() -> Self {
        let config = || {
            ureq::Agent::config_builder()
                .timeout_connect(Some(Duration::from_secs(20)))
                .timeout_recv_body(Some(Duration::from_secs(60)))
                .http_status_as_error(false)
        };
        Self {
            agent: config().build().into(),
            no_redirects: config().max_redirects(0).build().into(),
        }
    }
}

impl ToolSource for GitHub {
    fn get(&self, name: &str, progress: &dyn Fn(u8)) -> Result<Vec<u8>, String> {
        let resp = self
            .agent
            .get(&format!("{RELEASES}/download/{name}"))
            .header("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| e.to_string())?;
        if resp.status() != 200 {
            return Err(format!("GitHub answered {} for {name}", resp.status()));
        }
        let total = resp.body().content_length();
        let mut reader = resp
            .into_body()
            .into_with_config()
            .limit(MAX_BYTES)
            .reader();
        let mut out = Vec::new();
        let mut buf = [0u8; 64 * 1024];
        let mut said = None;
        loop {
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                return Ok(out);
            }
            out.extend_from_slice(&buf[..n]);
            if let Some(t) = total.filter(|&t| t > 0) {
                let pct = (out.len() as u64 * 100 / t).min(100) as u8;
                if said != Some(pct) {
                    said = Some(pct);
                    progress(pct);
                }
            }
        }
    }

    fn latest_tag(&self) -> Result<String, String> {
        let resp = self
            .no_redirects
            .get(RELEASES)
            .header("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| e.to_string())?;
        let location = resp
            .headers()
            .get("location")
            .and_then(|l| l.to_str().ok())
            .ok_or("GitHub gave no latest release")?;
        tag_of(location).ok_or_else(|| format!("unexpected latest release: {location}"))
    }
}

/// `…/releases/tag/2026.08.19` → `2026.08.19`.
fn tag_of(location: &str) -> Option<String> {
    let tag = location.rsplit_once("/tag/")?.1;
    valid_version(tag).then(|| tag.to_owned())
}

/// yt-dlp versions are dates with an optional build number: `2026.08.19` or `2026.08.19.1`.
fn valid_version(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    (3..=4).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether release `tag` is newer than `version` (what `yt-dlp --version` printed).
pub fn newer(tag: &str, version: &str) -> bool {
    let parse = |v: &str| -> Option<Vec<u64>> {
        valid_version(v.trim()).then(|| {
            v.trim()
                .split('.')
                .map(|p| p.parse().unwrap_or(0))
                .collect()
        })
    };
    match (parse(tag), parse(version)) {
        (Some(t), Some(v)) => t > v,
        // A version we can't read is replaced by a release we can.
        (Some(_), None) => true,
        _ => false,
    }
}

/// The expected SHA-256 of [`ASSET`] in a `SHA2-256SUMS` file (`‹hex›  ‹name›` lines).
pub fn expected_sum(sums: &str) -> Result<[u8; 32], String> {
    let hex = sums
        .lines()
        .find_map(|l| {
            let (hash, name) = l.trim().split_once(char::is_whitespace)?;
            (name.trim().trim_start_matches('*') == ASSET).then_some(hash)
        })
        .ok_or(format!("{SUMS} has no entry for {ASSET}"))?;
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| {
            hex.get(i..i + 2)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        })
        .collect::<Option<Vec<u8>>>();
    bytes
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .ok_or(format!("{SUMS} has a malformed hash for {ASSET}"))
}

/// The app's copy, in `dir` (the cache folder's `tools/`).
pub fn program(dir: &Path) -> PathBuf {
    dir.join("yt-dlp")
}

/// Downloads the latest release into `dir` and installs it, if its SHA-256 matches; a copy
/// that doesn't is never written.
pub fn install(src: &dyn ToolSource, dir: &Path, progress: &dyn Fn(u8)) -> Result<PathBuf, String> {
    let sums = src.get(SUMS, &|_| {})?;
    let expected = expected_sum(&String::from_utf8_lossy(&sums))?;
    let bytes = src.get(ASSET, progress)?;
    if <[u8; 32]>::from(Sha256::digest(&bytes)) != expected {
        return Err("the download doesn't match its published checksum".into());
    }
    let part = dir.join(".yt-dlp.part");
    let write = || -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(&part, &bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755))?;
        }
        let to = program(dir);
        std::fs::rename(&part, &to)?;
        Ok(to)
    };
    write().map_err(|e| {
        let _ = std::fs::remove_file(&part);
        format!("couldn't install yt-dlp: {e}")
    })
}

/// Whether an update check is due: none in the last [`UPDATE_EVERY`] (`tools/checked`).
pub fn update_due(dir: &Path, now: u64) -> bool {
    let last = std::fs::read_to_string(dir.join("checked"))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok());
    last.is_none_or(|t| now.saturating_sub(t) >= UPDATE_EVERY.as_secs())
}

/// Checks for a newer release than `version` once a day (`now` in Unix seconds), installing
/// it: the new copy, or `None` when nothing was due or newer.
pub fn update(
    src: &dyn ToolSource,
    dir: &Path,
    version: &str,
    now: u64,
) -> Result<Option<PathBuf>, String> {
    if !update_due(dir, now) {
        return Ok(None);
    }
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join("checked"), now.to_string());
    let tag = src.latest_tag()?;
    if !newer(&tag, version) {
        return Ok(None);
    }
    install(src, dir, &|_| {}).map(Some)
}

/// A release that tests make up: these files, and this latest tag.
pub struct FakeSource {
    pub files: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    pub tag: std::sync::Mutex<String>,
    /// Requests made, by file name (and "latest" for the tag).
    pub asked: std::sync::Mutex<Vec<String>>,
}

impl FakeSource {
    /// A release `tag` whose asset is `asset`, with a matching (or, `honest: false`, wrong)
    /// checksum.
    pub fn new(tag: &str, asset: &[u8], honest: bool) -> Self {
        let mut sum: [u8; 32] = Sha256::digest(asset).into();
        if !honest {
            sum[0] ^= 1;
        }
        let hex: String = sum.iter().map(|b| format!("{b:02x}")).collect();
        let files = [
            (SUMS.to_owned(), format!("{hex}  {ASSET}\n").into_bytes()),
            (ASSET.to_owned(), asset.to_vec()),
        ];
        Self {
            files: std::sync::Mutex::new(files.into_iter().collect()),
            tag: std::sync::Mutex::new(tag.to_owned()),
            asked: Default::default(),
        }
    }
}

impl ToolSource for FakeSource {
    fn get(&self, name: &str, progress: &dyn Fn(u8)) -> Result<Vec<u8>, String> {
        self.asked.lock().unwrap().push(name.to_owned());
        let f = self.files.lock().unwrap().get(name).cloned();
        progress(100);
        f.ok_or_else(|| format!("no {name}"))
    }

    fn latest_tag(&self) -> Result<String, String> {
        self.asked.lock().unwrap().push("latest".into());
        Ok(self.tag.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::testing::TestDir;

    const SCRIPT: &[u8] = b"#!/bin/sh\necho 2026.08.19\n";

    #[test]
    fn the_sum_is_read_for_exactly_the_macos_asset() {
        let a = "aa".repeat(32);
        let b = "bb".repeat(32);
        let sums = format!("{a}  yt-dlp_macos.zip\n{b}  yt-dlp_macos\n{a}  yt-dlp\n");
        assert_eq!(expected_sum(&sums).unwrap(), [0xbb; 32]);
        assert!(
            expected_sum(&format!("{a}  yt-dlp\n"))
                .unwrap_err()
                .contains("no entry")
        );
        assert!(
            expected_sum(&format!("{}  yt-dlp_macos\n", "ab".repeat(31)))
                .unwrap_err()
                .contains("malformed")
        );
        assert!(
            expected_sum(&format!("{}  yt-dlp_macos\n", "zz".repeat(32)))
                .unwrap_err()
                .contains("malformed")
        );
    }

    #[test]
    fn tags_and_versions_compare_as_dates() {
        assert_eq!(
            tag_of("https://github.com/yt-dlp/yt-dlp/releases/tag/2026.08.19").as_deref(),
            Some("2026.08.19")
        );
        assert_eq!(
            tag_of("https://github.com/yt-dlp/yt-dlp/releases/tag/../../evil"),
            None
        );
        assert!(newer("2026.08.19", "2026.07.01"));
        assert!(newer("2026.08.19.1", "2026.08.19"));
        assert!(newer("2026.08.19", "garbage"));
        assert!(!newer("2026.08.19", "2026.08.19\n"));
        assert!(!newer("2026.07.01", "2026.08.19"));
    }

    #[test]
    fn a_matching_download_is_installed_and_runs() {
        let dir = TestDir::new("tools-install");
        let src = FakeSource::new("2026.08.19", SCRIPT, true);
        let path = install(&src, &dir, &|_| {}).unwrap();
        assert_eq!(path, program(&dir));
        let out = std::process::Command::new(&path)
            .arg("--version")
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "2026.08.19");
        assert!(!dir.join(".yt-dlp.part").exists());
    }

    #[test]
    fn a_mismatched_download_leaves_nothing_behind() {
        let dir = TestDir::new("tools-mismatch");
        let src = FakeSource::new("2026.08.19", SCRIPT, false);
        assert!(
            install(&src, &dir, &|_| {})
                .unwrap_err()
                .contains("checksum")
        );
        assert_eq!(std::fs::read_dir(&*dir).unwrap().count(), 0);
    }

    #[test]
    fn a_failed_download_keeps_the_old_copy() {
        let dir = TestDir::new("tools-interrupted");
        std::fs::write(program(&dir), b"old").unwrap();
        let src = FakeSource::new("2026.08.19", SCRIPT, true);
        src.files.lock().unwrap().remove(ASSET);
        assert!(install(&src, &dir, &|_| {}).is_err());
        assert_eq!(std::fs::read(program(&dir)).unwrap(), b"old");
    }

    /// The real release on GitHub (37 MB): `cargo test -p dig -- --ignored real_ytdlp`.
    #[test]
    #[ignore]
    fn real_ytdlp_release_installs_and_runs() {
        let dir = TestDir::new("tools-real");
        let src = GitHub::default();
        let tag = src.latest_tag().unwrap();
        let path = install(&src, &dir, &|_| {}).unwrap();
        let out = std::process::Command::new(&path)
            .arg("--version")
            .output()
            .unwrap();
        let version = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        assert_eq!(version, tag);
        assert!(!newer(&tag, &version));
    }

    #[test]
    fn updates_are_checked_once_a_day_and_only_install_newer_releases() {
        let dir = TestDir::new("tools-update");
        let src = FakeSource::new("2026.08.19", SCRIPT, true);
        let day = UPDATE_EVERY.as_secs();
        // Same version: checked, nothing installed.
        assert_eq!(update(&src, &dir, "2026.08.19", 1000).unwrap(), None);
        assert!(!program(&dir).exists());
        // Within a day of that check: not even asked.
        *src.tag.lock().unwrap() = "2026.09.01".into();
        src.asked.lock().unwrap().clear();
        assert_eq!(
            update(&src, &dir, "2026.08.19", 1000 + day - 1).unwrap(),
            None
        );
        assert!(src.asked.lock().unwrap().is_empty());
        // A day later, a newer release is installed.
        assert_eq!(
            update(&src, &dir, "2026.08.19", 1000 + day).unwrap(),
            Some(program(&dir))
        );
        // An older release never is.
        *src.tag.lock().unwrap() = "2026.01.01".into();
        assert_eq!(
            update(&src, &dir, "2026.09.01", 1000 + 3 * day).unwrap(),
            None
        );
    }
}
