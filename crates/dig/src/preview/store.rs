//! The preview folder, `<cache>/previews/`, kept within its size limit (2 GB by default).
//!
//! A preview's modification time is set to now when it starts playing, so the oldest files are
//! the least recently played; those go first when a download takes the folder over its limit.
//! The playing, armed and next previews are never deleted. The folder holds a few hundred
//! files at most, so it is simply scanned each time.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const DIR: &str = "previews";

/// `<cache>/previews`.
pub fn dir(cache_root: &Path) -> PathBuf {
    cache_root.join(DIR)
}

/// Marks a preview as just played.
pub fn touch(path: &Path) {
    if let Ok(f) = std::fs::File::options().write(true).open(path) {
        let _ = f.set_modified(SystemTime::now());
    }
}

/// Deletes the least recently played previews until the folder fits in `limit` bytes, never
/// one whose clip is in `protected`. Returns the deleted files.
pub fn enforce_limit(dir: &Path, limit: u64, protected: &HashSet<String>) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(SystemTime, u64, PathBuf, String)> = read
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            if !m.is_file() {
                return None;
            }
            let path = e.path();
            let clip = path
                .file_name()?
                .to_string_lossy()
                .split('.')
                .next()?
                .to_owned();
            Some((m.modified().ok()?, m.len(), path, clip))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    let mut deleted = Vec::new();
    for (_, len, path, clip) in files {
        if total <= limit {
            break;
        }
        if protected.contains(&clip) {
            continue;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
            deleted.push(path);
        }
    }
    deleted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn file(dir: &Path, clip: &str, kb: usize, age_secs: u64) {
        let p = dir.join(format!("{clip}.m4a"));
        std::fs::write(&p, vec![0u8; kb * 1000]).unwrap();
        let f = std::fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(age_secs))
            .unwrap();
    }

    #[test]
    fn the_least_recently_played_go_first_and_protected_ones_stay() {
        let d = crate::test_dir("store");
        // Oldest first: a, b, c, d, e; 100 KB each.
        for (i, c) in [
            "aaaaaaaaaaa",
            "bbbbbbbbbbb",
            "ccccccccccc",
            "ddddddddddd",
            "eeeeeeeeeee",
        ]
        .iter()
        .enumerate()
        {
            file(&d, c, 100, 1000 - i as u64 * 100);
        }
        // "a" was played just now, and "b" is the playing entry's.
        touch(&d.join("aaaaaaaaaaa.m4a"));
        let protected: HashSet<String> = ["bbbbbbbbbbb".to_owned()].into();
        let deleted = enforce_limit(&d, 300_000, &protected);
        let names: Vec<String> = deleted
            .iter()
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["ccccccccccc", "ddddddddddd"]);
        assert!(d.join("aaaaaaaaaaa.m4a").exists());
        assert!(d.join("bbbbbbbbbbb.m4a").exists());
        assert!(
            enforce_limit(&d, 300_000, &protected).is_empty(),
            "fits now"
        );
    }

    #[test]
    fn protected_previews_are_kept_even_over_the_limit() {
        let d = crate::test_dir("store-protected");
        file(&d, "aaaaaaaaaaa", 100, 10);
        file(&d, "bbbbbbbbbbb", 100, 5);
        let all: HashSet<String> = ["aaaaaaaaaaa".into(), "bbbbbbbbbbb".into()].into();
        assert!(enforce_limit(&d, 0, &all).is_empty());
        assert!(enforce_limit(&d.join("missing"), 0, &all).is_empty());
    }
}
