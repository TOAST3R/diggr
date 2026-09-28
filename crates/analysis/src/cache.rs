//! On-disk score cache, keyed by a sampled content hash and the algorithm version.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use platform::{FileSource, TrackRef};
use xxhash_rust::xxh3::Xxh3;

use crate::score::{ALGORITHM_VERSION, SongScore};

const SAMPLE: u64 = 256 * 1024;

/// File size plus xxh3 of the first, middle and last 256 KB: fast even for multi-GB mixes, and
/// stable across renames and moves.
pub fn content_hash(files: &dyn FileSource, track: &TrackRef) -> Option<u64> {
    let mut media = files.open(track).ok()?;
    let len = media
        .byte_len()
        .or_else(|| media.seek(SeekFrom::End(0)).ok())?;
    let mut h = Xxh3::new();
    h.update(&len.to_le_bytes());
    let mut buf = vec![0u8; SAMPLE as usize];
    for at in [
        0,
        len.saturating_sub(SAMPLE) / 2,
        len.saturating_sub(SAMPLE),
    ] {
        media.seek(SeekFrom::Start(at)).ok()?;
        let n = read_up_to(&mut media, &mut buf)?;
        h.update(&buf[..n]);
    }
    Some(h.digest())
}

fn read_up_to(r: &mut impl Read, buf: &mut [u8]) -> Option<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(_) => return None,
        }
    }
    Some(n)
}

#[derive(Debug, Clone)]
pub struct ScoreCache {
    dir: PathBuf,
}

impl ScoreCache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `WINAMP_CACHE_DIR`, else the platform cache directory (`~/Library/Caches/winamp_rust`).
    pub fn platform_default() -> Option<Self> {
        std::env::var_os("WINAMP_CACHE_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::cache_dir().map(|d| d.join("winamp_rust")))
            .map(Self::new)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, hash: u64) -> PathBuf {
        self.dir
            .join("scores")
            .join(format!("{hash:016x}.postcard"))
    }

    /// A cached score for this content, if one exists for the current algorithm version.
    pub fn load(&self, hash: u64) -> Option<SongScore> {
        let bytes = std::fs::read(self.path(hash)).ok()?;
        let s: SongScore = postcard::from_bytes(&bytes).ok()?;
        (s.version == ALGORITHM_VERSION && s.content_hash == hash).then_some(s)
    }

    pub fn save(&self, score: &SongScore) -> std::io::Result<()> {
        let path = self.path(score.content_hash);
        std::fs::create_dir_all(path.parent().expect("has parent"))?;
        let bytes = postcard::to_stdvec(score).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::native::NativeFileSource;

    fn tmp(name: &str) -> platform::testing::TestDir {
        platform::testing::TestDir::new(&format!("analysis-cache-{name}"))
    }

    #[test]
    fn hash_follows_content_not_name() {
        let d = tmp("hash");
        let data: Vec<u8> = (0..2_000_000u32).map(|i| (i * 7 % 251) as u8).collect();
        std::fs::write(d.join("a.mp3"), &data).unwrap();
        std::fs::write(d.join("renamed.mp3"), &data).unwrap();
        let mut other = data.clone();
        other[1_000_000] ^= 1; // inside the middle sample
        std::fs::write(d.join("b.mp3"), &other).unwrap();
        let h = |n: &str| {
            content_hash(
                &NativeFileSource,
                &TrackRef::new(d.join(n).to_string_lossy()),
            )
            .unwrap()
        };
        assert_eq!(h("a.mp3"), h("renamed.mp3"));
        assert_ne!(h("a.mp3"), h("b.mp3"));
    }

    #[test]
    fn save_load_and_version_mismatch() {
        let d = tmp("store");
        let c = ScoreCache::new(d.path());
        let s = SongScore {
            version: ALGORITHM_VERSION,
            content_hash: 99,
            beats: vec![0.5, 1.0],
            ..Default::default()
        };
        c.save(&s).unwrap();
        assert_eq!(c.load(99), Some(s.clone()));
        assert_eq!(c.load(100), None);
        let old = SongScore {
            version: ALGORITHM_VERSION + 1,
            ..s
        };
        c.save(&old).unwrap();
        assert_eq!(c.load(99), None, "other algorithm versions are ignored");
    }
}
