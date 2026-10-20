//! The preview folder, `<cache>/previews/`, and the overviews next to it in
//! `<cache>/overviews/`, kept together within the preview cache size (10 GB by default).
//!
//! A preview's modification time is set to now when it starts playing, and an overview's when
//! it is loaded, so the oldest files are the least recently played; those go first when a
//! download takes the two folders over their limit. A deleted preview's overview goes with it.
//! The playing, armed and next previews (and their overviews) are never deleted. The folders
//! hold a few thousand files at most, so they are simply scanned each time.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use platform::TrackRef;
use platform::native::NativeFileSource;

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

/// `<cache>/overviews`, next to the preview folder `dir`.
fn overviews(dir: &Path) -> Option<PathBuf> {
    Some(overview_path(dir, 0)?.parent()?.to_owned())
}

fn overview_path(dir: &Path, hash: u64) -> Option<PathBuf> {
    Some(analysis::overview::cache_path(dir.parent()?, hash))
}

/// The overview built from the preview at `path`, named by its content.
fn overview_of(dir: &Path, path: &Path) -> Option<PathBuf> {
    let file = TrackRef::new(path.to_string_lossy());
    overview_path(
        dir,
        analysis::cache::content_hash(&NativeFileSource, &file)?,
    )
}

/// Files in `dir`: modification time, length, path.
fn files_in(dir: &Path) -> Vec<(SystemTime, u64, PathBuf)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            m.is_file()
                .then_some((m.modified().ok()?, m.len(), e.path()))
        })
        .collect()
}

/// The bytes the previews in `dir` and the overviews next to it take.
pub fn total_size(dir: &Path) -> u64 {
    let ov = overviews(dir).map(|d| files_in(&d)).unwrap_or_default();
    files_in(dir).iter().chain(&ov).map(|f| f.1).sum()
}

fn clip_of(path: &Path) -> Option<String> {
    Some(
        path.file_name()?
            .to_string_lossy()
            .split('.')
            .next()?
            .to_owned(),
    )
}

/// Deletes the least recently used previews and overviews until both fit in `limit` bytes,
/// never a preview whose clip is in `protected` or its overview. Returns the deleted previews.
pub fn enforce_limit(dir: &Path, limit: u64, protected: &HashSet<String>) -> Vec<PathBuf> {
    let previews = files_in(dir);
    let ov_dir = overviews(dir);
    let mut files: Vec<(SystemTime, u64, PathBuf, bool)> = previews
        .iter()
        .map(|(t, len, p)| (*t, *len, p.clone(), true))
        .chain(
            ov_dir
                .iter()
                .flat_map(|d| files_in(d))
                .map(|(t, len, p)| (t, len, p, false)),
        )
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= limit {
        return Vec::new();
    }
    // Only the few protected previews are hashed to find their overviews.
    let mut gone: HashSet<PathBuf> = previews
        .iter()
        .filter(|f| clip_of(&f.2).is_some_and(|c| protected.contains(&c)))
        .flat_map(|f| [Some(f.2.clone()), overview_of(dir, &f.2)])
        .flatten()
        .collect();
    files.sort_by_key(|f| f.0);
    let mut deleted = Vec::new();
    for (_, len, path, is_preview) in files {
        if total <= limit {
            break;
        }
        if gone.contains(&path) {
            continue;
        }
        // Hashed before deleting: the overview is named by the preview's content.
        let overview = is_preview.then(|| overview_of(dir, &path)).flatten();
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
            if let Some(o) = overview
                && let Ok(m) = std::fs::metadata(&o)
                && std::fs::remove_file(&o).is_ok()
            {
                total = total.saturating_sub(m.len());
                gone.insert(o);
            }
            if is_preview {
                deleted.push(path);
            }
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
    fn a_deleted_previews_overview_goes_with_it() {
        let root = crate::test_dir("store-overviews");
        let d = dir(&root);
        std::fs::create_dir_all(&d).unwrap();
        let overview = |clip: &str, byte: u8| {
            let p = d.join(format!("{clip}.m4a"));
            std::fs::write(&p, vec![byte; 100_000]).unwrap();
            let h = analysis::cache::content_hash(
                &NativeFileSource,
                &TrackRef::new(p.to_string_lossy()),
            )
            .unwrap();
            let o = root.join("overviews").join(format!("{h:016x}.bin"));
            std::fs::create_dir_all(o.parent().unwrap()).unwrap();
            std::fs::write(&o, b"x").unwrap();
            o
        };
        let old = overview("aaaaaaaaaaa", 1);
        let f = std::fs::File::options()
            .write(true)
            .open(d.join("aaaaaaaaaaa.m4a"))
            .unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(100))
            .unwrap();
        let kept = overview("bbbbbbbbbbb", 2);
        assert_eq!(enforce_limit(&d, 150_000, &HashSet::new()).len(), 1);
        assert!(!old.exists(), "the evicted preview's overview is deleted");
        assert!(kept.exists(), "the kept preview's overview stays");
    }

    #[test]
    fn overviews_count_toward_the_limit_and_the_oldest_go_first() {
        let root = crate::test_dir("store-overviews-limit");
        let d = dir(&root);
        let ov = root.join("overviews");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::create_dir_all(&ov).unwrap();
        let aged = |p: &Path, age_secs: u64| {
            let f = std::fs::File::options().write(true).open(p).unwrap();
            f.set_modified(SystemTime::now() - Duration::from_secs(age_secs))
                .unwrap();
        };
        // The protected preview's overview is the oldest file, then an orphan overview (a
        // local track's), then an unprotected preview.
        let playing = d.join("aaaaaaaaaaa.m4a");
        std::fs::write(&playing, vec![1u8; 100_000]).unwrap();
        let playing_ov = overview_of(&d, &playing).unwrap();
        std::fs::write(&playing_ov, vec![0u8; 50_000]).unwrap();
        aged(&playing_ov, 300);
        let orphan = ov.join("00000000000000ff.bin");
        std::fs::write(&orphan, vec![0u8; 50_000]).unwrap();
        aged(&orphan, 200);
        file(&d, "bbbbbbbbbbb", 100, 100);
        assert_eq!(total_size(&d), 300_000);
        let protected: HashSet<String> = ["aaaaaaaaaaa".to_owned()].into();
        assert!(enforce_limit(&d, 250_000, &protected).is_empty());
        assert!(!orphan.exists(), "the oldest unprotected file goes first");
        assert!(playing_ov.exists(), "a protected preview's overview stays");
        assert_eq!(enforce_limit(&d, 150_000, &protected).len(), 1);
        assert!(playing.exists() && playing_ov.exists());
        assert_eq!(total_size(&d), 150_000);
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
