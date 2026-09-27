//! Track overview: a native-rate stereo summary of a whole track for drawing waveforms. Built in
//! the background from the track's own decoder, published progressively, and cached by content
//! hash.
//!
//! Level 0 has one [`Block`] per 256 frames: per-channel min/max, RMS, and low/mid/high band
//! energy (Linkwitz–Riley crossovers at 200 Hz and 2.5 kHz). Each further level is 4× coarser,
//! so all levels together cost about 1.33× level 0: ~13 MB for a 2-hour mix at 44.1 kHz. The same
//! pass builds the bounded spectral overview and cutoff statistics ([`crate::spectral`], ≤ 2 MB).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use audio::decode::TrackDecoder;
use platform::{FileSource, Priority, Spawner, TrackRef};

use crate::spectral::{Spectral, SpectralBuilder};

/// Frames per level-0 block.
pub const BLOCK: usize = 256;
/// Each level is this many times coarser than the one below.
pub const FACTOR: usize = 4;
const LOW_HZ: f32 = 200.0;
const HIGH_HZ: f32 = 2500.0;
/// Band energies are stored as dB in [-FLOOR_DB, 0] mapped to 0..=255.
const FLOOR_DB: f32 = 60.0;
const CACHE_VERSION: u32 = 2;
const MAGIC: &[u8; 4] = b"WROV";

/// Summary of a run of frames.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Block {
    pub min_l: i8,
    pub max_l: i8,
    pub min_r: i8,
    pub max_r: i8,
    /// Linear RMS of both channels, 0..=255 for 0..=1.
    pub rms: u8,
    /// Band energy in dB, 0 = -60 dBFS or less, 255 = 0 dBFS.
    pub low: u8,
    pub mid: u8,
    pub high: u8,
}

impl Block {
    /// Merges a run of blocks: extremes of the peaks, mean of the levels.
    pub fn merge(blocks: &[Block]) -> Option<Block> {
        let first = *blocks.first()?;
        let n = blocks.len() as u32;
        let mean =
            |f: fn(&Block) -> u8| (blocks.iter().map(|b| f(b) as u32).sum::<u32>() / n) as u8;
        Some(Block {
            min_l: blocks.iter().map(|b| b.min_l).min().unwrap_or(first.min_l),
            max_l: blocks.iter().map(|b| b.max_l).max().unwrap_or(first.max_l),
            min_r: blocks.iter().map(|b| b.min_r).min().unwrap_or(first.min_r),
            max_r: blocks.iter().map(|b| b.max_r).max().unwrap_or(first.max_r),
            rms: mean(|b| b.rms),
            low: mean(|b| b.low),
            mid: mean(|b| b.mid),
            high: mean(|b| b.high),
        })
    }

    fn to_bytes(self) -> [u8; 8] {
        [
            self.min_l as u8,
            self.max_l as u8,
            self.min_r as u8,
            self.max_r as u8,
            self.rms,
            self.low,
            self.mid,
            self.high,
        ]
    }

    fn from_bytes(b: [u8; 8]) -> Self {
        Block {
            min_l: b[0] as i8,
            max_l: b[1] as i8,
            min_r: b[2] as i8,
            max_r: b[3] as i8,
            rms: b[4],
            low: b[5],
            mid: b[6],
            high: b[7],
        }
    }
}

/// The overview of a track, possibly still growing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overview {
    pub sample_rate: u32,
    /// Frames summarized so far.
    pub frames: u64,
    /// `levels[0]` has one block per [`BLOCK`] frames; `levels[k]` per `BLOCK × FACTOR^k`.
    pub levels: Vec<Vec<Block>>,
    /// The whole track has been summarized.
    pub complete: bool,
    /// The spectrogram at overview resolution, as far as it is built.
    pub spectral: Option<Arc<Spectral>>,
}

impl Overview {
    pub fn block_frames(level: usize) -> u64 {
        (BLOCK * FACTOR.pow(level as u32)) as u64
    }

    /// Seconds summarized so far.
    pub fn seconds(&self) -> f64 {
        self.frames as f64 / self.sample_rate.max(1) as f64
    }

    /// The coarsest level whose blocks are no longer than `frames_per_column` (so a column
    /// covers at least one block).
    pub fn level_for(&self, frames_per_column: f64) -> usize {
        let mut level = 0;
        while level + 1 < self.levels.len()
            && Self::block_frames(level + 1) as f64 <= frames_per_column
        {
            level += 1;
        }
        level
    }

    /// Summary of `[t0, t1)` seconds for one drawn column, or `None` where nothing is known yet.
    pub fn column(&self, t0: f64, t1: f64) -> Option<Block> {
        if self.levels.is_empty() || t1 <= 0.0 {
            return None;
        }
        let rate = self.sample_rate as f64;
        let level = self.level_for((t1 - t0) * rate);
        let bf = Self::block_frames(level) as f64;
        let blocks = &self.levels[level];
        let a = ((t0.max(0.0) * rate) / bf).floor() as usize;
        let b = (((t1 * rate) / bf).ceil() as usize)
            .max(a + 1)
            .min(blocks.len());
        if a >= b {
            return None;
        }
        Block::merge(&blocks[a..b])
    }

    /// Memory used by the block and spectral data.
    pub fn bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|l| l.len() * std::mem::size_of::<Block>())
            .sum::<usize>()
            + self.spectral.as_ref().map_or(0, |s| s.bytes())
    }

    fn rebuild_levels(&mut self) {
        self.levels.truncate(1);
        let mut k = 0;
        while self.levels[k].len() > FACTOR {
            let next: Vec<Block> = self.levels[k]
                .chunks(FACTOR)
                .filter_map(Block::merge)
                .collect();
            self.levels.push(next);
            k += 1;
        }
    }

    /// Serializes a complete overview (level 0 only; coarser levels are rebuilt on load).
    pub fn write_to(&self, w: &mut impl Write) -> std::io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&CACHE_VERSION.to_le_bytes())?;
        w.write_all(&self.sample_rate.to_le_bytes())?;
        w.write_all(&self.frames.to_le_bytes())?;
        let l0 = self.levels.first().map_or(&[][..], |l| l.as_slice());
        w.write_all(&(l0.len() as u64).to_le_bytes())?;
        let bytes: Vec<u8> = l0.iter().flat_map(|b| b.to_bytes()).collect();
        w.write_all(&bytes)?;
        match &self.spectral {
            Some(s) => {
                w.write_all(&[1])?;
                s.write_to(w)
            }
            None => w.write_all(&[0]),
        }
    }

    pub fn read_from(r: &mut impl Read) -> std::io::Result<Self> {
        let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "not a current overview");
        let mut head = [0u8; 28];
        r.read_exact(&mut head)?;
        let u32_at = |i: usize| u32::from_le_bytes(head[i..i + 4].try_into().expect("4 bytes"));
        let u64_at = |i: usize| u64::from_le_bytes(head[i..i + 8].try_into().expect("8 bytes"));
        if &head[..4] != MAGIC || u32_at(4) != CACHE_VERSION {
            return Err(bad());
        }
        let n = u64_at(20) as usize;
        let mut data = vec![0u8; n.checked_mul(8).ok_or_else(bad)?];
        r.read_exact(&mut data)?;
        let l0 = data
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| Block::from_bytes(*c))
            .collect();
        let mut flag = [0u8];
        r.read_exact(&mut flag)?;
        let spectral = match flag[0] {
            0 => None,
            _ => Some(Arc::new(Spectral::read_from(r, u32_at(8))?)),
        };
        let mut o = Overview {
            sample_rate: u32_at(8),
            frames: u64_at(12),
            levels: vec![l0],
            complete: true,
            spectral,
        };
        o.rebuild_levels();
        Ok(o)
    }
}

/// Direct form 1 biquad.
#[derive(Debug, Clone, Copy, Default)]
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

impl Biquad {
    /// Butterworth (Q = 1/√2) low- or high-pass; two in series make a Linkwitz–Riley filter.
    fn butterworth(rate: u32, hz: f32, high: bool) -> Self {
        let w = std::f32::consts::TAU * hz / rate as f32;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let b = if high {
            [(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0]
        } else {
            [(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0]
        };
        Biquad {
            b: b.map(|v| v / a0),
            a: [-2.0 * c / a0, (1.0 - alpha) / a0],
            ..Default::default()
        }
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        // Flush the decaying tail after silence or DC: denormals make each step ~100× slower.
        let y = if y.abs() < 1e-20 { 0.0 } else { y };
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

/// A fourth-order Linkwitz–Riley filter (two Butterworth biquads).
#[derive(Debug, Clone, Copy)]
struct Lr4([Biquad; 2]);

impl Lr4 {
    fn new(rate: u32, hz: f32, high: bool) -> Self {
        let b = Biquad::butterworth(rate, hz, high);
        Lr4([b, b])
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.0[0].run(x);
        self.0[1].run(y)
    }
}

fn peak(x: f32) -> i8 {
    (x * 127.0).round().clamp(-127.0, 127.0) as i8
}

fn db_byte(mean_square: f64) -> u8 {
    let db = 10.0 * (mean_square.max(1e-12)).log10() as f32;
    ((db + FLOOR_DB) / FLOOR_DB * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// Accumulates one block.
#[derive(Debug, Clone, Copy)]
struct Acc {
    n: usize,
    min: [f32; 2],
    max: [f32; 2],
    sq: f64,
    bands: [f64; 3],
}

impl Default for Acc {
    fn default() -> Self {
        Self {
            n: 0,
            min: [f32::MAX; 2],
            max: [f32::MIN; 2],
            sq: 0.0,
            bands: [0.0; 3],
        }
    }
}

impl Acc {
    fn block(&self) -> Block {
        let n = self.n.max(1) as f64;
        Block {
            min_l: peak(self.min[0]),
            max_l: peak(self.max[0]),
            min_r: peak(self.min[1]),
            max_r: peak(self.max[1]),
            rms: ((self.sq / (2.0 * n)).sqrt() * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8,
            low: db_byte(self.bands[0] / n),
            mid: db_byte(self.bands[1] / n),
            high: db_byte(self.bands[2] / n),
        }
    }
}

/// Builds an overview from interleaved stereo audio pushed in order.
pub struct OverviewBuilder {
    ov: Overview,
    acc: Acc,
    low: Lr4,
    mid_hp: Lr4,
    mid_lp: Lr4,
    high: Lr4,
    /// Blocks of each level not yet merged into the next.
    pending: Vec<usize>,
    spectral: SpectralBuilder,
}

impl OverviewBuilder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            ov: Overview {
                sample_rate,
                levels: vec![Vec::new()],
                ..Default::default()
            },
            acc: Acc::default(),
            low: Lr4::new(sample_rate, LOW_HZ, false),
            mid_hp: Lr4::new(sample_rate, LOW_HZ, true),
            mid_lp: Lr4::new(sample_rate, HIGH_HZ, false),
            high: Lr4::new(sample_rate, HIGH_HZ, true),
            pending: vec![0],
            spectral: SpectralBuilder::new(sample_rate),
        }
    }

    pub fn push(&mut self, interleaved: &[f32]) {
        for f in interleaved.as_chunks::<2>().0 {
            let a = &mut self.acc;
            for (c, &x) in f.iter().enumerate() {
                a.min[c] = a.min[c].min(x);
                a.max[c] = a.max[c].max(x);
            }
            a.sq += (f[0] * f[0] + f[1] * f[1]) as f64;
            let m = 0.5 * (f[0] + f[1]);
            let lo = self.low.run(m);
            let mi = self.mid_lp.run(self.mid_hp.run(m));
            let hi = self.high.run(m);
            a.bands[0] += (lo * lo) as f64;
            a.bands[1] += (mi * mi) as f64;
            a.bands[2] += (hi * hi) as f64;
            a.n += 1;
            self.spectral.push_frame(f[0], f[1]);
            if a.n == BLOCK {
                self.emit();
            }
        }
    }

    fn emit(&mut self) {
        let b = self.acc.block();
        self.ov.frames += self.acc.n as u64;
        self.acc = Acc::default();
        self.ov.levels[0].push(b);
        self.pending[0] += 1;
        let mut k = 0;
        while self.pending[k] == FACTOR {
            self.pending[k] = 0;
            let src = &self.ov.levels[k];
            let merged = Block::merge(&src[src.len() - FACTOR..]).expect("FACTOR blocks");
            if self.ov.levels.len() == k + 1 {
                self.ov.levels.push(Vec::new());
                self.pending.push(0);
            }
            self.ov.levels[k + 1].push(merged);
            self.pending[k + 1] += 1;
            k += 1;
        }
    }

    /// What has been built so far.
    pub fn snapshot(&self) -> Overview {
        Overview {
            spectral: Some(Arc::new(self.spectral.snapshot())),
            ..self.ov.clone()
        }
    }

    pub fn frames(&self) -> u64 {
        self.ov.frames
    }

    /// Flushes the last partial block and returns the complete overview.
    pub fn finish(mut self) -> Overview {
        if self.acc.n > 0 {
            self.emit();
        }
        self.ov.rebuild_levels();
        self.ov.complete = true;
        self.ov.spectral = Some(Arc::new(self.spectral.finish()));
        self.ov
    }
}

fn cache_path(dir: &Path, hash: u64) -> PathBuf {
    dir.join("overviews").join(format!("{hash:016x}.bin"))
}

/// A cached overview for this content, if one exists in the current format.
pub fn load_cached(dir: &Path, hash: u64) -> Option<Overview> {
    let mut f = std::io::BufReader::new(std::fs::File::open(cache_path(dir, hash)).ok()?);
    Overview::read_from(&mut f).ok()
}

pub fn save_cached(dir: &Path, hash: u64, o: &Overview) -> std::io::Result<()> {
    let path = cache_path(dir, hash);
    std::fs::create_dir_all(path.parent().expect("has parent"))?;
    let tmp = path.with_extension("tmp");
    {
        let mut w = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        o.write_to(&mut w)?;
        w.flush()?;
    }
    std::fs::rename(tmp, path)
}

/// How often a growing overview is republished.
const PUBLISH_EVERY: Duration = Duration::from_millis(250);
/// Tracks whose overviews are kept in memory.
const KEEP: usize = 6;

struct Entry {
    overview: ArcSwapOption<Overview>,
    running: AtomicBool,
    last_used: Mutex<Instant>,
}

/// Builds and caches overviews on low-priority background threads, one per requested track.
pub struct OverviewService {
    spawner: Arc<dyn Spawner>,
    files: Arc<dyn FileSource>,
    cache_dir: Option<PathBuf>,
    entries: Mutex<HashMap<TrackRef, Arc<Entry>>>,
}

impl OverviewService {
    pub fn new(
        spawner: Arc<dyn Spawner>,
        files: Arc<dyn FileSource>,
        cache_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            spawner,
            files,
            cache_dir,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Starts building the track's overview if it isn't built or building (cheap to call every
    /// frame). The caller decides when: the player asks only once the analyzer has its first
    /// horizon, so the overview never competes with playback start or the playhead analysis.
    pub fn request(&self, track: &TrackRef) {
        let entry = {
            let mut map = self.entries.lock().expect("not poisoned");
            if let Some(e) = map.get(track) {
                *e.last_used.lock().expect("not poisoned") = Instant::now();
                return;
            }
            if map.len() >= KEEP
                && let Some(oldest) = map
                    .iter()
                    .filter(|(_, e)| !e.running.load(Ordering::Acquire))
                    .min_by_key(|(_, e)| *e.last_used.lock().expect("not poisoned"))
                    .map(|(k, _)| k.clone())
            {
                map.remove(&oldest);
            }
            let e = Arc::new(Entry {
                overview: ArcSwapOption::empty(),
                running: AtomicBool::new(true),
                last_used: Mutex::new(Instant::now()),
            });
            map.insert(track.clone(), e.clone());
            e
        };
        let (files, dir, track2, e2) = (
            self.files.clone(),
            self.cache_dir.clone(),
            track.clone(),
            entry.clone(),
        );
        let spawned = self.spawner.spawn(
            "overview",
            Priority::Low,
            Box::new(move || {
                build(&*files, dir.as_deref(), &track2, &e2.overview);
                e2.running.store(false, Ordering::Release);
            }),
        );
        if spawned.is_err() {
            entry.running.store(false, Ordering::Release);
        }
    }

    /// The latest overview for a track (never blocks).
    pub fn get(&self, track: &TrackRef) -> Option<Arc<Overview>> {
        let map = self.entries.lock().expect("not poisoned");
        map.get(track)?.overview.load_full()
    }
}

fn build(
    files: &dyn FileSource,
    dir: Option<&Path>,
    track: &TrackRef,
    out: &ArcSwapOption<Overview>,
) {
    let hash = dir.and(crate::cache::content_hash(files, track));
    if let (Some(dir), Some(h)) = (dir, hash)
        && let Some(o) = load_cached(dir, h)
    {
        out.store(Some(Arc::new(o)));
        return;
    }
    let Ok(mut dec) = TrackDecoder::open(files, track) else {
        return;
    };
    let mut builder = OverviewBuilder::new(dec.src_rate());
    let mut buf = Vec::new();
    let mut last = Instant::now();
    loop {
        buf.clear();
        let more = dec.decode_next(&mut buf).unwrap_or(false);
        builder.push(&buf);
        if !more {
            break;
        }
        if last.elapsed() >= PUBLISH_EVERY {
            out.store(Some(Arc::new(builder.snapshot())));
            last = Instant::now();
            std::thread::yield_now();
        }
    }
    let done = builder.finish();
    if let (Some(dir), Some(h)) = (dir, hash) {
        let _ = save_cached(dir, h, &done);
    }
    out.store(Some(Arc::new(done)));
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 44_100;

    fn stereo(mono: impl IntoIterator<Item = f32>) -> Vec<f32> {
        mono.into_iter().flat_map(|x| [x, x]).collect()
    }

    fn sine(hz: f32, secs: f32, amp: f32) -> Vec<f32> {
        stereo(
            (0..(secs * SR as f32) as usize)
                .map(|i| amp * (std::f32::consts::TAU * hz * i as f32 / SR as f32).sin()),
        )
    }

    #[test]
    fn a_single_peak_lands_in_its_block() {
        let mut b = OverviewBuilder::new(SR);
        let mut x = vec![0.0f32; 12 * SR as usize];
        let at = 10 * SR as usize;
        x[at] = 1.0;
        b.push(&stereo(x));
        let o = b.finish();
        let blk = o.levels[0][at / BLOCK];
        assert_eq!((blk.max_l, blk.max_r), (127, 127));
        assert_eq!(
            o.levels[0][at / BLOCK + 2].max_l,
            0,
            "neighbours stay quiet"
        );
        // Coarser levels and a column spanning the peak keep it.
        assert_eq!(o.column(9.0, 11.0).unwrap().max_l, 127);
        assert_eq!(o.levels[2][at / (BLOCK * 16)].max_l, 127);
    }

    #[test]
    fn bands_follow_frequency() {
        let colour = |hz| {
            let mut b = OverviewBuilder::new(SR);
            b.push(&sine(hz, 2.0, 0.5));
            b.finish().column(0.5, 1.5).unwrap()
        };
        let bass = colour(60.0);
        assert!(
            bass.low > bass.mid + 40 && bass.low > bass.high + 40,
            "{bass:?}"
        );
        let mid = colour(800.0);
        assert!(mid.mid > mid.low + 40 && mid.mid > mid.high + 40, "{mid:?}");
        let hat = colour(9000.0);
        assert!(
            hat.high > hat.low + 40 && hat.high > hat.mid + 40,
            "{hat:?}"
        );
        assert!(
            bass.rms > 80 && bass.rms < 100,
            "0.5 amplitude sine ≈ 0.35 RMS: {}",
            bass.rms
        );
    }

    #[test]
    fn progressive_snapshots_cover_what_was_pushed() {
        let mut b = OverviewBuilder::new(SR);
        let chunk = sine(440.0, 1.0, 0.3);
        for _ in 0..60 {
            b.push(&chunk);
        }
        let snap = b.snapshot();
        assert!(!snap.complete);
        assert!((snap.seconds() - 60.0).abs() < 0.01);
        assert!(snap.column(59.0, 59.5).is_some());
        assert!(
            snap.column(61.0, 62.0).is_none(),
            "nothing past what was decoded"
        );
    }

    #[test]
    fn levels_pick_by_column_width() {
        let mut b = OverviewBuilder::new(SR);
        b.push(&vec![0.0; 2 * 60 * SR as usize]);
        let o = b.finish();
        assert_eq!(o.level_for(100.0), 0);
        assert_eq!(o.level_for(1024.0), 1);
        assert_eq!(o.level_for(5000.0), 2);
        assert_eq!(Overview::block_frames(3), 256 * 64);
    }

    #[test]
    fn cache_round_trip() {
        let mut b = OverviewBuilder::new(SR);
        b.push(&sine(100.0, 3.0, 0.8));
        let o = b.finish();
        let dir = std::env::temp_dir().join(format!("overview-cache-{}", std::process::id()));
        save_cached(&dir, 42, &o).unwrap();
        let back = load_cached(&dir, 42).unwrap();
        assert_eq!(back, o);
        assert!(load_cached(&dir, 43).is_none());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn two_hour_mix_fits_in_16_mb() {
        let frames = 2 * 3600 * SR as u64;
        let mut b = OverviewBuilder::new(SR);
        let chunk = vec![0.1f32; 2 * SR as usize * 10];
        let mut pushed = 0;
        while pushed < frames {
            b.push(&chunk);
            pushed += chunk.len() as u64 / 2;
        }
        let o = b.finish();
        assert!(o.bytes() <= 16 << 20, "{} bytes", o.bytes());
        assert!((o.seconds() - 7200.0).abs() < 1.0);
    }

    struct Threads;
    impl Spawner for Threads {
        fn spawn(
            &self,
            _: &str,
            _: Priority,
            f: Box<dyn FnOnce() + Send + 'static>,
        ) -> Result<(), platform::PlatformError> {
            std::thread::spawn(f);
            Ok(())
        }
    }

    #[test]
    fn service_builds_then_serves_from_cache() {
        let dir = std::env::temp_dir().join(format!("overview-svc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("t.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: SR,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&wav, spec).unwrap();
        for x in sine(60.0, 5.0, 0.5) {
            w.write_sample((x * 32767.0) as i16).unwrap();
        }
        w.finalize().unwrap();
        let track = TrackRef::new(wav.to_string_lossy());
        let files: Arc<dyn FileSource> = Arc::new(platform::native::NativeFileSource);
        let svc = OverviewService::new(Arc::new(Threads), files.clone(), Some(dir.clone()));
        svc.request(&track);
        let start = Instant::now();
        let done = loop {
            if let Some(o) = svc.get(&track).filter(|o| o.complete) {
                break o;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "built in time");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!((done.seconds() - 5.0).abs() < 0.01);
        // A fresh service finds it in the cache at once.
        let svc2 = OverviewService::new(Arc::new(Threads), files, Some(dir.clone()));
        svc2.request(&track);
        let t = Instant::now();
        while svc2.get(&track).is_none() {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(t.elapsed() < Duration::from_millis(500));
        assert_eq!(*svc2.get(&track).unwrap(), *done);
        std::fs::remove_dir_all(dir).ok();
    }
}
