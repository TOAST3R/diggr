//! Fetching one clip's audio. The real fetcher runs the user's own yt-dlp; the app never
//! bundles, downloads or updates it.
//!
//! yt-dlp gets an argument list (never a shell), only a validated 11-character id, a URL
//! rebuilt from that id, `--ignore-config` so a user config can't change paths or formats, and
//! an output template inside the preview folder. It writes `.part` files and renames them when
//! complete, so a half-downloaded preview is never played.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::discogs::model::valid_clip_id;

/// The audio format asked for: AAC in M4A, which the decoder plays and YouTube offers for
/// nearly every clip.
pub const EXT: &str = "m4a";
const FORMAT: &str = "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// The id isn't exactly 11 of `A–Z a–z 0–9 - _`: never run.
    InvalidId,
    /// yt-dlp couldn't be started.
    NoProgram,
    Cancelled,
    /// yt-dlp failed, or produced no file; the text is its last error line.
    Failed(String),
}

pub trait Fetcher: Send + Sync {
    /// Fetches `clip` into `dir/<clip>.m4a`, reporting percent done, until done or `cancel`.
    fn fetch(
        &self,
        clip: &str,
        dir: &Path,
        progress: &dyn Fn(u8),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, FetchError>;

    /// The fetcher's version, if it can run at all.
    fn version(&self) -> Option<String>;
}

/// Where a clip's preview lives.
pub fn preview_path(dir: &Path, clip: &str) -> PathBuf {
    dir.join(format!("{clip}.{EXT}"))
}

/// The address a clip is played from (and exported as).
pub fn clip_url(clip: &str) -> String {
    format!("https://www.youtube.com/watch?v={clip}")
}

pub struct YtDlp {
    pub program: PathBuf,
}

/// Where yt-dlp usually is, for when the app is started without the shell's PATH (a macOS app
/// opened from the Finder sees only `/usr/bin:/bin:/usr/sbin:/sbin`).
pub fn candidates(configured: Option<&str>) -> Vec<PathBuf> {
    if let Some(p) = configured {
        return vec![PathBuf::from(p)];
    }
    let mut c = vec![PathBuf::from("yt-dlp")];
    for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
        c.push(Path::new(dir).join("yt-dlp"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        c.push(Path::new(&home).join(".local/bin/yt-dlp"));
    }
    c
}

/// The first candidate that runs, with its version.
pub fn find(configured: Option<&str>) -> Option<(YtDlp, String)> {
    candidates(configured).into_iter().find_map(|program| {
        let y = YtDlp { program };
        let v = y.version()?;
        Some((y, v))
    })
}

impl YtDlp {
    /// The whole argument list for one clip.
    pub fn args(clip: &str, dir: &Path) -> Vec<String> {
        vec![
            "--ignore-config".into(),
            "--no-playlist".into(),
            "--no-mtime".into(),
            "--no-warnings".into(),
            "--newline".into(),
            "-f".into(),
            FORMAT.into(),
            // `download:` selects the template for downloads; the line itself reads
            // `progress  40.2%`.
            "--progress-template".into(),
            "download:progress %(progress._percent_str)s".into(),
            "-o".into(),
            dir.join(format!("{clip}.%(ext)s"))
                .to_string_lossy()
                .into_owned(),
            "--".into(),
            clip_url(clip),
        ]
    }
}

/// `progress  40.2%` → 40.
pub fn parse_progress(line: &str) -> Option<u8> {
    let pct = line
        .trim()
        .strip_prefix("progress")?
        .trim()
        .strip_suffix('%')?;
    let pct: String = pct
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let v: f32 = pct.parse().ok()?;
    Some(v.clamp(0.0, 100.0) as u8)
}

impl Fetcher for YtDlp {
    fn fetch(
        &self,
        clip: &str,
        dir: &Path,
        progress: &dyn Fn(u8),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, FetchError> {
        if !valid_clip_id(clip) {
            return Err(FetchError::InvalidId);
        }
        std::fs::create_dir_all(dir).map_err(|e| FetchError::Failed(e.to_string()))?;
        let dest = preview_path(dir, clip);
        let mut child = Command::new(&self.program)
            .args(Self::args(clip, dir))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| FetchError::NoProgram)?;
        let (tx, rx) = std::sync::mpsc::channel();
        let stdout = child.stdout.take().expect("piped");
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(p) = parse_progress(&line)
                    && tx.send(p).is_err()
                {
                    return;
                }
            }
        });
        let mut stderr = child.stderr.take().expect("piped");
        let err_reader = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = stderr.read_to_string(&mut s);
            s
        });
        let status = loop {
            for p in rx.try_iter() {
                progress(p);
            }
            if cancel.load(Ordering::Acquire) {
                let _ = child.kill();
                let _ = child.wait();
                remove_partial(dir, clip);
                return Err(FetchError::Cancelled);
            }
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => return Err(FetchError::Failed(e.to_string())),
            }
        };
        for p in rx.try_iter() {
            progress(p);
        }
        let errors = err_reader.join().unwrap_or_default();
        let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        if status.success() && size > 0 {
            Ok(dest)
        } else {
            remove_partial(dir, clip);
            let last = errors.lines().rev().find(|l| !l.trim().is_empty());
            Err(FetchError::Failed(
                last.unwrap_or("yt-dlp failed").trim().to_owned(),
            ))
        }
    }

    fn version(&self) -> Option<String> {
        let mut child = Command::new(&self.program)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match child.try_wait().ok()? {
                Some(s) if s.success() => break,
                Some(_) => return None,
                None if Instant::now() > deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                None => std::thread::sleep(Duration::from_millis(20)),
            }
        }
        let mut out = String::new();
        child.stdout.take()?.read_to_string(&mut out).ok()?;
        let v = out.lines().next()?.trim();
        (!v.is_empty()).then(|| v.to_owned())
    }
}

/// Deletes what an unfinished download leaves behind (`<id>.*.part`, `<id>.*.ytdl`, …).
pub fn remove_partial(dir: &Path, clip: &str) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for e in read.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{clip}.")) && name != format!("{clip}.{EXT}") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// A fetcher for tests: copies a fixture file, with scripted progress, delays and failures.
pub struct FakeFetcher {
    pub source: PathBuf,
    /// Clips that fail this many times before working (`u32::MAX`: always).
    pub failures: std::sync::Mutex<std::collections::HashMap<String, u32>>,
    /// Clips that never finish until cancelled.
    pub hangs: std::sync::Mutex<std::collections::HashSet<String>>,
    pub delay: Duration,
    pub available: AtomicBool,
    /// Every fetch started, in order.
    pub started: std::sync::Mutex<Vec<String>>,
}

impl FakeFetcher {
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            failures: Default::default(),
            hangs: Default::default(),
            delay: Duration::from_millis(20),
            available: AtomicBool::new(true),
            started: Default::default(),
        }
    }

    pub fn fail(&self, clip: &str, times: u32) {
        self.failures.lock().unwrap().insert(clip.into(), times);
    }

    pub fn hang(&self, clip: &str) {
        self.hangs.lock().unwrap().insert(clip.into());
    }

    pub fn started(&self) -> Vec<String> {
        self.started.lock().unwrap().clone()
    }
}

impl Fetcher for FakeFetcher {
    fn fetch(
        &self,
        clip: &str,
        dir: &Path,
        progress: &dyn Fn(u8),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, FetchError> {
        if !valid_clip_id(clip) {
            return Err(FetchError::InvalidId);
        }
        if !self.available.load(Ordering::SeqCst) {
            return Err(FetchError::NoProgram);
        }
        self.started.lock().unwrap().push(clip.to_owned());
        std::fs::create_dir_all(dir).map_err(|e| FetchError::Failed(e.to_string()))?;
        let part = dir.join(format!("{clip}.{EXT}.part"));
        let _ = std::fs::write(&part, b"partial");
        let hang = self.hangs.lock().unwrap().contains(clip);
        for step in 0..=4u8 {
            if cancel.load(Ordering::Acquire) {
                remove_partial(dir, clip);
                return Err(FetchError::Cancelled);
            }
            progress(step * 25);
            std::thread::sleep(self.delay / 4);
            while hang && !cancel.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        {
            let mut f = self.failures.lock().unwrap();
            if let Some(n) = f.get_mut(clip)
                && *n > 0
            {
                *n = n.saturating_sub(1);
                remove_partial(dir, clip);
                return Err(FetchError::Failed("ERROR: Video unavailable".into()));
            }
        }
        let dest = preview_path(dir, clip);
        std::fs::copy(&self.source, &dest).map_err(|e| FetchError::Failed(e.to_string()))?;
        let _ = std::fs::remove_file(part);
        Ok(dest)
    }

    fn version(&self) -> Option<String> {
        self.available
            .load(Ordering::SeqCst)
            .then(|| "2026.01.01-fake".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_argument_list_is_fixed_and_confined() {
        let args = YtDlp::args("dQw4w9WgXcQ", Path::new("/cache/previews"));
        assert_eq!(args[0], "--ignore-config");
        let o = args.iter().position(|a| a == "-o").unwrap();
        assert_eq!(args[o + 1], "/cache/previews/dQw4w9WgXcQ.%(ext)s");
        assert_eq!(
            &args[args.len() - 2..],
            ["--", "https://www.youtube.com/watch?v=dQw4w9WgXcQ"]
        );
        let f = args.iter().position(|a| a == "-f").unwrap();
        assert_eq!(args[f + 1], "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]");
        for flag in ["--no-playlist", "--newline", "--no-mtime"] {
            assert!(args.contains(&flag.to_owned()), "{flag}");
        }
    }

    #[test]
    fn malformed_ids_are_never_run() {
        let y = YtDlp {
            program: PathBuf::from("/definitely/not/here/yt-dlp"),
        };
        let dir = crate::test_dir("fetch-invalid");
        let never = AtomicBool::new(false);
        for id in ["abc;rm -rf", "--exec=rm-x", "dQw4w9WgXc", "../../etc/pw"] {
            assert_eq!(
                y.fetch(id, &dir, &|_| {}, &never),
                Err(FetchError::InvalidId),
                "{id}"
            );
        }
        assert_eq!(
            y.fetch("dQw4w9WgXcQ", &dir, &|_| {}, &never),
            Err(FetchError::NoProgram)
        );
        assert_eq!(y.version(), None);
    }

    #[test]
    fn progress_lines() {
        assert_eq!(parse_progress("progress  40.2%"), Some(40));
        assert_eq!(parse_progress("progress 100.0%"), Some(100));
        assert_eq!(parse_progress("[youtube] dQw4w9WgXcQ: Downloading"), None);
        assert_eq!(parse_progress("progress    N/A%"), None);
    }

    #[test]
    fn a_configured_path_overrides_the_search() {
        assert_eq!(
            candidates(Some("/opt/x/yt-dlp")),
            [PathBuf::from("/opt/x/yt-dlp")]
        );
        assert_eq!(candidates(None)[0], PathBuf::from("yt-dlp"));
    }

    /// Needs a real yt-dlp and the network: `cargo test -p dig -- --ignored real_ytdlp`.
    #[test]
    #[ignore]
    fn real_ytdlp_downloads_a_clip() {
        let (y, version) = find(None).expect("yt-dlp installed");
        assert!(!version.is_empty());
        let dir = crate::test_dir("fetch-real");
        let pcts = std::sync::Mutex::new(Vec::new());
        let path = y
            .fetch(
                "jNQXAC9IVRw",
                &dir,
                &|p| pcts.lock().unwrap().push(p),
                &AtomicBool::new(false),
            )
            .expect("downloaded");
        assert!(path.ends_with("jNQXAC9IVRw.m4a"));
        assert!(std::fs::metadata(&path).unwrap().len() > 10_000);
        assert!(!pcts.lock().unwrap().is_empty(), "progress was reported");
    }
}
