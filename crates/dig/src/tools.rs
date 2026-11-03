//! The app's own yt-dlp, for Macs, x86_64 Linux and x86_64 Windows without one (a Homebrew,
//! distribution, PATH or configured yt-dlp always wins and is never touched).
//!
//! On Windows it is yt-dlp's zip app run by python.org's embeddable Python, with Deno as its
//! JavaScript runtime, not `yt-dlp.exe`: every program in Python and Deno is signed, so Smart
//! App Control can let them run, and it starts faster than the one-file exe. Python and Deno
//! are pinned here ([`Pin`]) and only change with the app; each is unpacked into its own folder
//! of `tools/` the same all-or-nothing way.
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

/// Whether this system has an official yt-dlp build the app can manage.
pub const MANAGED: bool = cfg!(any(
    target_os = "macos",
    all(target_os = "linux", target_arch = "x86_64"),
    all(windows, target_arch = "x86_64")
));
/// Whether the app's copy is the zip app run by its own Python, with Deno (Windows).
pub const WITH_PYTHON: bool = cfg!(windows);
/// The release asset: yt-dlp's universal macOS build, its x86_64 Linux one, or (Windows) its
/// platform-independent zip app. Never taken from anything downloaded.
pub const ASSET: &str = if cfg!(target_os = "macos") {
    "yt-dlp_macos"
} else if WITH_PYTHON {
    "yt-dlp"
} else {
    "yt-dlp_linux"
};
const RELEASES: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest";
const SUMS: &str = "SHA2-256SUMS";
/// The download's size, as the question to download it says (on Windows, Python's 13 MB, the
/// zip app's 3 MB and Deno's 40 MB).
pub const SIZE_MB: u32 = if cfg!(target_os = "macos") {
    37
} else if WITH_PYTHON {
    56
} else {
    41
};

/// A program the app pins on Windows: a zip from a fixed address, checked against a fixed
/// SHA-256, unpacked into `tools/‹folder›/`, with `‹exe›` at its top.
#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    pub folder: &'static str,
    pub exe: &'static str,
    pub version: &'static str,
    pub url: String,
    pub sha256: [u8; 32],
    /// Its share of the download, for the progress.
    pub mb: u32,
}

/// The pinned Python and Deno. Checked on python.org's SBOM and Deno's release, and on Windows
/// 11 (every binary signed, see `openspec/changes/ytdlp-onboarding/findings.md`). Release
/// checklist: newer versions replace version, address and SHA-256 together, then
/// `cargo test -p dig -- --ignored real_pins`.
pub fn pins() -> Vec<Pin> {
    let pin = |folder, exe, version, url: &str, sum: &str, mb| Pin {
        folder,
        exe,
        version,
        url: url.to_owned(),
        sha256: hex32(sum).expect("a valid pinned SHA-256"),
        mb,
    };
    vec![
        pin(
            "python",
            "python.exe",
            "3.15.2",
            "https://www.python.org/ftp/python/3.15.2/python-3.15.2-embed-amd64.zip",
            "d012f3aa29cf07436dae8aea1fec0fd195c2612a0b059b1f915ae0afb9ad1f62",
            13,
        ),
        pin(
            "js",
            "deno.exe",
            "2.9.8",
            "https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip",
            "cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf",
            40,
        ),
    ]
}
/// Written into each unpacked pin, so a newer pin replaces it.
const STAMP: &str = "diggr-version";
/// The least time between two update checks.
pub const UPDATE_EVERY: Duration = Duration::from_secs(24 * 3600);
const MAX_BYTES: u64 = 200 << 20;

/// Where the release files come from, behind a trait so tests run offline.
pub trait ToolSource: Send + Sync {
    /// A file of yt-dlp's latest release, telling `progress` the percent done.
    fn get(&self, name: &str, progress: &dyn Fn(u8)) -> Result<Vec<u8>, String> {
        self.fetch(&format!("{RELEASES}/download/{name}"), progress)
    }
    /// The file at `url`, one of the fixed addresses here. An error names the host.
    fn fetch(&self, url: &str, progress: &dyn Fn(u8)) -> Result<Vec<u8>, String>;
    /// The pinned programs (tests use their own).
    fn pins(&self) -> Vec<Pin> {
        pins()
    }
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

/// `https://www.python.org/ftp/…` → `python.org`, for errors.
fn host_of(url: &str) -> &str {
    let host = url.split("://").nth(1).unwrap_or(url);
    let host = host.split('/').next().unwrap_or(host);
    host.strip_prefix("www.").unwrap_or(host)
}

impl ToolSource for GitHub {
    fn fetch(&self, url: &str, progress: &dyn Fn(u8)) -> Result<Vec<u8>, String> {
        let host = host_of(url);
        let resp = self
            .agent
            .get(url)
            .header("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| format!("couldn't reach {host}: {e}"))?;
        if resp.status() != 200 {
            let name = url.rsplit('/').next().unwrap_or(url);
            return Err(format!("{host} answered {} for {name}", resp.status()));
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
            let n = reader
                .read(&mut buf)
                .map_err(|e| format!("download from {host} broke off: {e}"))?;
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
    hex32(hex).ok_or(format!("{SUMS} has a malformed hash for {ASSET}"))
}

/// 64 hex digits → 32 bytes.
fn hex32(hex: &str) -> Option<[u8; 32]> {
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| {
            hex.get(i..i + 2)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        })
        .collect::<Option<Vec<u8>>>()?;
    <[u8; 32]>::try_from(bytes).ok()
}

/// The app's copy, in `dir` (the cache folder's `tools/`).
pub fn program(dir: &Path) -> PathBuf {
    dir.join("yt-dlp")
}

/// The Python beside the app's zip app (Windows), in `dir`.
pub fn python(dir: &Path) -> PathBuf {
    dir.join("python").join("python.exe")
}

/// The JavaScript runtime the app's Windows copy uses, in `dir`.
pub fn deno(dir: &Path) -> PathBuf {
    dir.join("js").join("deno.exe")
}

/// Whether `dir` holds `pin` at its pinned version.
pub fn current(dir: &Path, pin: &Pin) -> bool {
    let folder = dir.join(pin.folder);
    folder.join(pin.exe).is_file()
        && std::fs::read_to_string(folder.join(STAMP)).is_ok_and(|v| v.trim() == pin.version)
}

/// Unpacks the zip `bytes` into `to`. An entry that would land outside `to` (`..`, an absolute
/// path) fails the whole unpack.
pub fn unpack(bytes: &[u8], to: &Path) -> Result<(), String> {
    let broken = |e: &dyn std::fmt::Display| format!("couldn't unpack: {e}");
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| broken(&e))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| broken(&e))?;
        let Some(rel) = entry.enclosed_name() else {
            let name = String::from_utf8_lossy(entry.name_raw()).into_owned();
            return Err(broken(&format!("{name} is outside its folder")));
        };
        let path = to.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&path).map_err(|e| broken(&e))?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| broken(&e))?;
        }
        let mut file = std::fs::File::create(&path).map_err(|e| broken(&e))?;
        std::io::copy(&mut entry, &mut file).map_err(|e| broken(&e))?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::Permissions::from_mode(mode & 0o755);
            std::fs::set_permissions(&path, mode).map_err(|e| broken(&e))?;
        }
    }
    Ok(())
}

/// Downloads `pin` and unpacks it into `dir/‹folder›`, if its SHA-256 matches. It is unpacked
/// beside the old one first, so a failure at any step leaves the old one working.
fn install_pin(
    src: &dyn ToolSource,
    dir: &Path,
    pin: &Pin,
    progress: &dyn Fn(u8),
) -> Result<(), String> {
    let name = pin.exe.trim_end_matches(".exe");
    let bytes = src.fetch(&pin.url, progress)?;
    if <[u8; 32]>::from(Sha256::digest(&bytes)) != pin.sha256 {
        return Err(format!("the {name} download doesn't match its checksum"));
    }
    let part = dir.join(format!(".{}.part", pin.folder));
    let old = dir.join(format!(".{}.old", pin.folder));
    let current = dir.join(pin.folder);
    let _ = std::fs::remove_dir_all(&part);
    let _ = std::fs::remove_dir_all(&old);
    let result = std::fs::create_dir_all(dir)
        .map_err(|e| format!("couldn't install {name}: {e}"))
        .and_then(|()| unpack(&bytes, &part))
        .and_then(|()| {
            if !part.join(pin.exe).is_file() {
                return Err(format!(
                    "couldn't unpack: the {name} download has no {}",
                    pin.exe
                ));
            }
            let swap = || -> std::io::Result<()> {
                std::fs::write(part.join(STAMP), pin.version)?;
                if current.exists() {
                    std::fs::rename(&current, &old)?;
                }
                std::fs::rename(&part, &current)
            };
            swap().map_err(|e| format!("couldn't install {name}: {e}"))
        });
    let _ = std::fs::remove_dir_all(&part);
    // The old copy may still be in use by a running yt-dlp: removed now or at the next install.
    let _ = std::fs::remove_dir_all(&old);
    result
}

/// Downloads the latest release into `dir` and installs it, if its SHA-256 matches; a copy
/// that doesn't is never written. On Windows the pinned Python and Deno come first, when they
/// aren't there yet or are older pins; the progress covers all of it.
pub fn install(src: &dyn ToolSource, dir: &Path, progress: &dyn Fn(u8)) -> Result<PathBuf, String> {
    install_with(src, dir, WITH_PYTHON, progress)
}

/// [`install`], with the pinned programs (`pinned`, Windows) or without.
fn install_with(
    src: &dyn ToolSource,
    dir: &Path,
    pinned: bool,
    progress: &dyn Fn(u8),
) -> Result<PathBuf, String> {
    let missing: Vec<Pin> = if pinned {
        src.pins()
            .into_iter()
            .filter(|p| !current(dir, p))
            .collect()
    } else {
        Vec::new()
    };
    const ASSET_MB: u32 = 3;
    let total = missing.iter().map(|p| p.mb).sum::<u32>() + ASSET_MB;
    let mut done = 0;
    let share = |done: u32, mb: u32, p: u8| {
        progress(((done * 100 + mb * u32::from(p)) / total.max(1)).min(100) as u8)
    };
    for pin in &missing {
        install_pin(src, dir, pin, &|p| share(done, pin.mb, p))?;
        done += pin.mb;
    }
    let sums = src.get(SUMS, &|_| {})?;
    let expected = expected_sum(&String::from_utf8_lossy(&sums))?;
    let bytes = if missing.is_empty() {
        src.get(ASSET, progress)?
    } else {
        src.get(ASSET, &|p| share(done, ASSET_MB, p))?
    };
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
    // A newer app pins a newer Python or Deno: installed at once, with the latest zip app.
    if WITH_PYTHON && src.pins().iter().any(|p| !current(dir, p)) {
        return install(src, dir, &|_| {}).map(Some);
    }
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
        let mut files = vec![
            (SUMS.to_owned(), format!("{hex}  {ASSET}\n").into_bytes()),
            (ASSET.to_owned(), asset.to_vec()),
        ];
        for pin in FAKE_PINS {
            files.push((pin.2.to_owned(), fake_zip(pin.1)));
        }
        Self {
            files: std::sync::Mutex::new(files.into_iter().collect()),
            tag: std::sync::Mutex::new(tag.to_owned()),
            asked: Default::default(),
        }
    }
}

/// The fake pins: folder, exe, file name (at python.org and GitHub, as the real ones).
const FAKE_PINS: [(&str, &str, &str, &str); 2] = [
    (
        "python",
        "python.exe",
        "python-embed.zip",
        "https://www.python.org/ftp/python/fake/",
    ),
    (
        "js",
        "deno.exe",
        "deno.zip",
        "https://github.com/denoland/deno/releases/download/fake/",
    ),
];

/// A zip laid out as python.org's and Deno's are: `exe` at its top, and a library beside it.
pub fn fake_zip(exe: &str) -> Vec<u8> {
    use std::io::Write;
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    for (name, body) in [(exe, &b"MZ fake"[..]), ("lib.zip", b"PK lib")] {
        w.start_file(name, opts).expect("a zip entry");
        w.write_all(body).expect("written");
    }
    w.finish().expect("a zip").into_inner()
}

impl ToolSource for FakeSource {
    fn fetch(&self, url: &str, progress: &dyn Fn(u8)) -> Result<Vec<u8>, String> {
        let name = url.rsplit('/').next().unwrap_or(url);
        self.asked.lock().unwrap().push(name.to_owned());
        let f = self.files.lock().unwrap().get(name).cloned();
        progress(100);
        f.ok_or_else(|| format!("couldn't reach {}: no {name}", host_of(url)))
    }

    /// The fake pins, checked against the fake zips' own SHA-256 (a test may change a file).
    fn pins(&self) -> Vec<Pin> {
        FAKE_PINS
            .iter()
            .map(|&(folder, exe, file, base)| Pin {
                folder,
                exe,
                version: "1.0",
                url: format!("{base}{file}"),
                sha256: Sha256::digest(fake_zip(exe)).into(),
                mb: 1,
            })
            .collect()
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
    fn the_sum_is_read_for_exactly_this_systems_asset() {
        let a = "aa".repeat(32);
        let sums = [
            "yt-dlp_macos.zip",
            "yt-dlp",
            "yt-dlp_macos",
            "yt-dlp_linux",
            "yt-dlp_linux_aarch64",
        ]
        .iter()
        .enumerate()
        .map(|(i, name)| format!("{}  {name}\n", format!("{i:02x}").repeat(32)))
        .collect::<String>();
        let i = if cfg!(target_os = "macos") {
            2
        } else if WITH_PYTHON {
            1
        } else {
            3
        };
        assert_eq!(expected_sum(&sums).unwrap(), [i; 32]);
        assert!(
            expected_sum(&format!("{a}  yt-dlp_x86.exe\n"))
                .unwrap_err()
                .contains("no entry")
        );
        assert!(
            expected_sum(&format!("{}  {ASSET}\n", "ab".repeat(31)))
                .unwrap_err()
                .contains("malformed")
        );
        assert!(
            expected_sum(&format!("{}  {ASSET}\n", "zz".repeat(32)))
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

    fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, body) in entries {
            w.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(body).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn folders(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn the_real_pins_are_python_and_deno_from_their_own_sites() {
        let pins = pins();
        assert_eq!(pins.len(), 2);
        assert_eq!(host_of(&pins[0].url), "python.org");
        assert!(pins[0].url.contains(pins[0].version) && pins[0].url.ends_with("-embed-amd64.zip"));
        assert_eq!(host_of(&pins[1].url), "github.com");
        assert!(pins[1].url.contains(&format!("/v{}/", pins[1].version)));
        assert_eq!((pins[0].exe, pins[1].exe), ("python.exe", "deno.exe"));
        // The fetcher looks for them where they are unpacked.
        let dir = Path::new("tools");
        assert_eq!(dir.join(pins[0].folder).join(pins[0].exe), python(dir));
        assert_eq!(dir.join(pins[1].folder).join(pins[1].exe), deno(dir));
    }

    #[test]
    fn each_pin_is_unpacked_whole_and_stamped() {
        let dir = TestDir::new("tools-pins");
        let src = FakeSource::new("2026.08.19", SCRIPT, true);
        for pin in src.pins() {
            install_pin(&src, &dir, &pin, &|_| {}).unwrap();
            assert!(current(&dir, &pin), "{}", pin.folder);
            assert!(dir.join(pin.folder).join("lib.zip").is_file());
        }
        assert_eq!(
            folders(&dir),
            ["js", "python"],
            "no .part or .old folder is left"
        );
    }

    #[test]
    fn a_bad_pin_download_keeps_the_old_one() {
        let dir = TestDir::new("tools-pin-bad");
        let src = FakeSource::new("2026.08.19", SCRIPT, true);
        let pin = src.pins().remove(0);
        install_pin(&src, &dir, &pin, &|_| {}).unwrap();
        let file = pin.url.rsplit('/').next().unwrap().to_owned();

        // Wrong checksum: never unpacked.
        src.files
            .lock()
            .unwrap()
            .insert(file.clone(), b"not the pinned file".to_vec());
        let e = install_pin(&src, &dir, &pin, &|_| {}).unwrap_err();
        assert!(e.contains("python") && e.contains("checksum"), "{e}");
        assert!(current(&dir, &pin));

        // Missing: the error names the host.
        src.files.lock().unwrap().remove(&file);
        let e = install_pin(&src, &dir, &pin, &|_| {}).unwrap_err();
        assert!(e.contains("python.org"), "{e}");
        assert!(current(&dir, &pin));
        assert_eq!(folders(&dir), ["python"]);
    }

    #[test]
    fn a_pin_zip_without_its_exe_is_refused() {
        let dir = TestDir::new("tools-pin-empty");
        let src = FakeSource::new("2026.08.19", SCRIPT, true);
        let bytes = zip_of(&[("readme.txt", b"no exe here")]);
        let pin = Pin {
            sha256: Sha256::digest(&bytes).into(),
            ..src.pins().remove(1)
        };
        let file = pin.url.rsplit('/').next().unwrap().to_owned();
        src.files.lock().unwrap().insert(file, bytes);
        let e = install_pin(&src, &dir, &pin, &|_| {}).unwrap_err();
        assert!(e.contains("no deno.exe"), "{e}");
        assert!(std::fs::read_dir(&*dir).unwrap().next().is_none());
    }

    #[test]
    fn an_older_stamp_is_replaced_by_the_pinned_one() {
        let dir = TestDir::new("tools-pin-stamp");
        let src = FakeSource::new("2026.08.19", SCRIPT, true);
        let pin = src.pins().remove(1);
        install_pin(&src, &dir, &pin, &|_| {}).unwrap();
        std::fs::write(dir.join(pin.folder).join(STAMP), "0.9").unwrap();
        assert!(!current(&dir, &pin));
        install_pin(&src, &dir, &pin, &|_| {}).unwrap();
        assert!(current(&dir, &pin));
        assert_eq!(folders(&dir), ["js"]);
    }

    #[test]
    fn the_windows_install_puts_python_deno_and_the_zip_app_in_place() {
        let dir = TestDir::new("tools-windows");
        let src = FakeSource::new("2026.08.19", b"zip app", true);
        let seen = std::sync::Mutex::new(Vec::new());
        let path = install_with(&src, &dir, true, &|p| seen.lock().unwrap().push(p)).unwrap();
        assert_eq!(path, program(&dir));
        assert_eq!(folders(&dir), ["js", "python", "yt-dlp"]);
        assert!(src.pins().iter().all(|p| current(&dir, p)));
        let seen = seen.into_inner().unwrap();
        assert!(
            seen.windows(2).all(|w| w[0] <= w[1]),
            "progress only grows: {seen:?}"
        );
        assert_eq!(seen.last(), Some(&100));
        // Once in place, Python and Deno aren't fetched again.
        src.asked.lock().unwrap().clear();
        install_with(&src, &dir, true, &|_| {}).unwrap();
        let asked = src.asked.lock().unwrap().clone();
        assert_eq!(asked, [SUMS, ASSET], "only the zip app again");
    }

    #[test]
    fn a_failed_pin_stops_the_install_before_the_zip_app() {
        let dir = TestDir::new("tools-windows-fail");
        let src = FakeSource::new("2026.08.19", b"zip app", true);
        src.files.lock().unwrap().remove("deno.zip");
        let e = install_with(&src, &dir, true, &|_| {}).unwrap_err();
        assert!(e.contains("github.com"), "{e}");
        assert!(!program(&dir).exists());
        assert_eq!(folders(&dir), ["python"], "Python stays for the next try");
    }

    #[test]
    fn unpacking_refuses_entries_outside_the_folder() {
        let dir = TestDir::new("tools-unpack");
        let to = dir.join("out");
        let evil = zip_of(&[("ok.txt", b"ok"), ("../escaped.txt", b"x")]);
        assert!(unpack(&evil, &to).unwrap_err().contains("outside"));
        assert!(!dir.join("escaped.txt").exists());
        assert!(
            unpack(b"not a zip", &to)
                .unwrap_err()
                .contains("couldn't unpack")
        );
        unpack(&zip_of(&[("a/b/c.txt", b"c")]), &to).unwrap();
        assert_eq!(std::fs::read(to.join("a/b/c.txt")).unwrap(), b"c");
    }

    /// The real Python (13 MB) and Deno (40 MB) against their pins, after changing them:
    /// `cargo test -p dig -- --ignored real_pins`.
    #[test]
    #[ignore]
    fn real_pins_match_their_downloads() {
        let dir = TestDir::new("tools-real-pins");
        for pin in pins() {
            install_pin(&GitHub::default(), &dir, &pin, &|_| {}).unwrap();
            assert!(current(&dir, &pin), "{}", pin.folder);
        }
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
