//! Zoomed spectrograms computed on demand: the visible time range only, at the track's native
//! rate, from its own decoder on a low-priority worker, delivered in strips as they are ready.
//!
//! The FFT size follows the zoom (time and frequency resolution trade off): short ranges get
//! short windows so 10 ms events stay apart, long ranges get up to 8192 points. When columns
//! are far apart, the worker seeks from window to window instead of decoding the gap, so a
//! wide view of a 2-hour mix costs about as much as a narrow one.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use audio::decode::TrackDecoder;
use platform::{FileSource, Priority, Spawner, TrackRef};

use crate::spectral::{Channel, RowMap, Stft, power_to_byte};

const MIN_FFT: usize = 512;
const MAX_FFT: usize = 8192;
/// Results kept for going back to a previous view.
const KEEP: usize = 4;
/// A strip is published at least this often while the worker runs.
const PUBLISH_EVERY: Duration = Duration::from_millis(40);

/// What to compute: rows are log-spaced from `lo_hz` to `hi_hz`.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailRequest {
    pub track: TrackRef,
    pub t0: f64,
    pub t1: f64,
    pub columns: usize,
    pub rows: usize,
    pub lo_hz: f32,
    pub hi_hz: f32,
    pub channel: Channel,
}

impl DetailRequest {
    /// Frames between column centres at `sample_rate`.
    pub fn hop_frames(&self, sample_rate: u32) -> f64 {
        (self.t1 - self.t0) * sample_rate as f64 / self.columns.max(1) as f64
    }

    /// The smallest power of two at least 4× the column hop, within 512–8192 points.
    pub fn fft_size(&self, sample_rate: u32) -> usize {
        ((4.0 * self.hop_frames(sample_rate)).ceil() as usize)
            .next_power_of_two()
            .clamp(MIN_FFT, MAX_FFT)
    }
}

/// A (possibly partial) result.
#[derive(Debug, Clone, PartialEq)]
pub struct Detail {
    pub request: DetailRequest,
    pub sample_rate: u32,
    pub fft: usize,
    /// The frequency range the rows actually cover (the request's, limited by Nyquist).
    pub lo_hz: f32,
    pub hi_hz: f32,
    /// Columns computed so far, from the left.
    pub columns_done: usize,
    /// Column-major dB bytes (see [`crate::spectral::byte_to_db`]), `rows` per column.
    pub data: Vec<u8>,
    /// Seeks made to skip gaps between windows (for tests and diagnostics).
    pub seeks: u32,
    /// The track could not be opened or decoded.
    pub failed: bool,
}

impl Detail {
    pub fn complete(&self) -> bool {
        self.failed || self.columns_done == self.request.columns
    }

    pub fn column(&self, c: usize) -> Option<&[u8]> {
        let rows = self.request.rows;
        (c < self.columns_done).then(|| &self.data[c * rows..(c + 1) * rows])
    }
}

/// Runs one detail computation at a time; a new request supersedes the running one.
pub struct DetailService {
    spawner: Arc<dyn Spawner>,
    files: Arc<dyn FileSource>,
    current: Arc<ArcSwapOption<Detail>>,
    generation: Arc<AtomicU64>,
    /// The request being computed or shown.
    wanted: Mutex<Option<DetailRequest>>,
    done: Arc<Mutex<VecDeque<Arc<Detail>>>>,
}

impl DetailService {
    pub fn new(spawner: Arc<dyn Spawner>, files: Arc<dyn FileSource>) -> Self {
        Self {
            spawner,
            files,
            current: Arc::new(ArcSwapOption::empty()),
            generation: Arc::new(AtomicU64::new(0)),
            wanted: Mutex::new(None),
            done: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Asks for `req` (cheap to call every frame with the same request).
    pub fn request(&self, req: DetailRequest) {
        let mut wanted = self.wanted.lock().expect("not poisoned");
        if wanted.as_ref() == Some(&req) {
            return;
        }
        *wanted = Some(req.clone());
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        if let Some(hit) = self
            .done
            .lock()
            .expect("not poisoned")
            .iter()
            .find(|d| d.request == req)
        {
            self.current.store(Some(hit.clone()));
            return;
        }
        self.current.store(None);
        let (files, current, gen_now, done) = (
            self.files.clone(),
            self.current.clone(),
            self.generation.clone(),
            self.done.clone(),
        );
        let _ = self.spawner.spawn(
            "spectrogram detail",
            Priority::Low,
            Box::new(move || {
                let live = || gen_now.load(Ordering::Acquire) == generation;
                let publish = |d: &Detail| {
                    if live() {
                        current.store(Some(Arc::new(d.clone())));
                    }
                };
                if let Some(d) = compute(&*files, req, &live, &publish) {
                    let d = Arc::new(d);
                    let mut done = done.lock().expect("not poisoned");
                    if done.len() == KEEP {
                        done.pop_front();
                    }
                    done.push_back(d.clone());
                    if live() {
                        current.store(Some(d));
                    }
                }
            }),
        );
    }

    /// The latest result for the current request, as far as it is computed (never blocks).
    pub fn get(&self) -> Option<Arc<Detail>> {
        self.current.load_full()
    }
}

/// Computes `req`, publishing strips as it goes. Returns `None` when superseded.
pub fn compute(
    files: &dyn FileSource,
    req: DetailRequest,
    live: &dyn Fn() -> bool,
    publish: &dyn Fn(&Detail),
) -> Option<Detail> {
    let Ok(mut dec) = TrackDecoder::open(files, &req.track) else {
        let d = Detail {
            request: req,
            sample_rate: 0,
            fft: 0,
            lo_hz: 0.0,
            hi_hz: 0.0,
            columns_done: 0,
            data: Vec::new(),
            seeks: 0,
            failed: true,
        };
        publish(&d);
        return Some(d);
    };
    let rate = dec.src_rate();
    let n = req.fft_size(rate);
    let hop = req.hop_frames(rate);
    let hi = req.hi_hz.min(rate as f32 / 2.0);
    let lo = req.lo_hz.min(hi * 0.5);
    let map = RowMap::new(n, rate, req.rows, lo, hi);
    let mut stft = Stft::new(n);
    let mut out = Detail {
        sample_rate: rate,
        fft: n,
        lo_hz: lo,
        hi_hz: hi,
        columns_done: 0,
        data: Vec::with_capacity(req.columns * req.rows),
        seeks: 0,
        failed: false,
        request: req.clone(),
    };
    let mut rows = vec![0.0f32; req.rows];
    // `buf` holds the selected channel from frame `buf_start` on; `next` is the next frame the
    // decoder will produce.
    let mut buf: Vec<f32> = Vec::new();
    let mut buf_start: i64 = 0;
    let mut next: i64 = 0;
    let mut stereo = Vec::new();
    let mut eof = false;
    // Seeking costs about a packet's decode; skipping a gap this long is worth it.
    let seek_gap = (2 * n as i64).max(rate as i64 / 4);
    let mut last_publish = Instant::now();
    let mut seeks = 0;
    for c in 0..req.columns {
        if !live() {
            return None;
        }
        let centre = req.t0 * rate as f64 + (c as f64 + 0.5) * hop;
        let start = centre.round() as i64 - n as i64 / 2;
        let end = start + n as i64;
        // Far ahead: seek instead of decoding the gap.
        if start - next > seek_gap && !eof {
            let at = start.max(0);
            if dec.seek(at as f64 / rate as f64).is_ok() {
                seeks += 1;
                buf.clear();
                buf_start = at;
                next = at;
            }
        }
        // Drop what no later window needs.
        let keep_from = (start - buf_start).clamp(0, buf.len() as i64) as usize;
        buf.drain(..keep_from);
        buf_start += keep_from as i64;
        if buf.is_empty() {
            buf_start = next;
        }
        while next < end && !eof {
            stereo.clear();
            eof = !dec.decode_next(&mut stereo).unwrap_or(false);
            let frames = stereo.as_chunks::<2>().0;
            buf.extend(frames.iter().map(|f| req.channel.of(f[0], f[1])));
            next += frames.len() as i64;
        }
        // Samples before the track or past its end are silence.
        let at = |i: i64| -> f32 {
            let k = i - buf_start;
            if k >= 0 {
                buf.get(k as usize).copied().unwrap_or(0.0)
            } else {
                0.0
            }
        };
        let power = stft.power((start..end).map(at));
        map.map(power, &mut rows);
        out.data.extend(rows.iter().map(|&p| power_to_byte(p)));
        out.columns_done = c + 1;
        out.seeks = seeks;
        if last_publish.elapsed() >= PUBLISH_EVERY {
            publish(&out);
            last_publish = Instant::now();
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::byte_to_db;

    const SR: u32 = 44_100;

    fn wav(name: &str, mono: &[f32]) -> TrackRef {
        let dir = std::env::temp_dir().join(format!("detail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: SR,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for &x in mono {
            let s = (x * 32767.0) as i16;
            w.write_sample(s).unwrap();
            w.write_sample(s).unwrap();
        }
        w.finalize().unwrap();
        TrackRef::new(path.to_string_lossy())
    }

    fn req(track: TrackRef, t0: f64, t1: f64, columns: usize) -> DetailRequest {
        DetailRequest {
            track,
            t0,
            t1,
            columns,
            rows: 64,
            lo_hz: 20.0,
            hi_hz: 22_050.0,
            channel: Channel::Mid,
        }
    }

    fn run(r: DetailRequest) -> Detail {
        compute(&platform::native::NativeFileSource, r, &|| true, &|_| {}).unwrap()
    }

    /// Broadband level per column in dB.
    fn loudness(d: &Detail) -> Vec<f32> {
        (0..d.columns_done)
            .map(|c| {
                let col = d.column(c).unwrap();
                let p: f32 = col.iter().map(|&b| 10f32.powf(byte_to_db(b) / 10.0)).sum();
                10.0 * (p / col.len() as f32).max(1e-20).log10()
            })
            .collect()
    }

    #[test]
    fn fft_size_follows_the_zoom() {
        let t = TrackRef::new("x.wav");
        assert_eq!(req(t.clone(), 0.0, 2.0, 1000).fft_size(SR), 512);
        assert_eq!(req(t.clone(), 0.0, 10.0, 1000).fft_size(SR), 2048);
        assert_eq!(req(t, 0.0, 600.0, 1000).fft_size(SR), 8192);
    }

    #[test]
    fn clicks_10_ms_apart_are_resolved() {
        // Clicks every 10 ms from 0.5 s to 2.5 s.
        let mut x = vec![0.0f32; 3 * SR as usize];
        let mut clicks = 0;
        let mut t = 0.5;
        while t < 2.5 {
            x[(t * SR as f64) as usize] = 0.9;
            clicks += 1;
            t += 0.010;
        }
        let d = run(req(wav("clicks.wav", &x), 0.5, 2.5, 1000));
        assert!(d.complete() && !d.failed);
        assert_eq!(d.seeks, 1, "one seek to the range, then straight through");
        assert_eq!(d.fft, 512);
        let l = loudness(&d);
        // Count separate bursts: rises through the midpoint between floor and peak.
        let (lo, hi) = l
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        let mid = 0.5 * (lo + hi);
        let bursts = l.windows(2).filter(|w| w[0] < mid && w[1] >= mid).count();
        assert!(hi - lo > 12.0, "clear dips between clicks: {lo}..{hi} dB");
        assert!(
            (bursts as i32 - clicks).abs() <= 2,
            "{bursts} bursts for {clicks} clicks"
        );
    }

    #[test]
    fn a_tone_lands_on_its_row_in_a_sparse_view() {
        // 60 s of 440 Hz with a silent gap in the middle; 100 columns (0.6 s apart), so the
        // worker seeks between windows.
        let mut x: Vec<f32> = (0..60 * SR as usize)
            .map(|i| 0.5 * (std::f32::consts::TAU * 440.0 * i as f32 / SR as f32).sin())
            .collect();
        x[25 * SR as usize..35 * SR as usize].fill(0.0);
        let mut r = req(wav("tone.wav", &x), 0.0, 60.0, 100);
        r.rows = 256;
        let d = run(r);
        assert_eq!(d.fft, 8192);
        assert!(d.seeks > 90, "skipped the gaps: {} seeks", d.seeks);
        let want = crate::spectral::hz_to_row(440.0, 256, SR as f32 / 2.0).floor() as usize;
        for c in [5, 20, 70, 95] {
            let col = d.column(c).unwrap();
            let top = (0..256).max_by_key(|&r| col[r]).unwrap();
            assert!(
                top.abs_diff(want) <= 1,
                "column {c}: row {top}, want {want}"
            );
        }
        let gap = d.column(50).unwrap();
        assert!(gap.iter().all(|&b| b < 10), "silence in the gap");
    }

    #[test]
    fn past_the_end_is_silence() {
        let x = vec![0.5f32; SR as usize];
        let d = run(req(wav("short.wav", &x), 0.5, 2.0, 30));
        assert!(d.complete());
        assert!(d.column(29).unwrap().iter().all(|&b| b < 10));
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
    fn service_supersedes_and_caches() {
        let x: Vec<f32> = (0..10 * SR as usize)
            .map(|i| 0.3 * (i as f32 * 0.05).sin())
            .collect();
        let t = wav("svc.wav", &x);
        let svc = DetailService::new(
            Arc::new(Threads),
            Arc::new(platform::native::NativeFileSource),
        );
        let wait = |r: &DetailRequest| {
            let start = Instant::now();
            loop {
                if let Some(d) = svc.get().filter(|d| d.complete() && &d.request == r) {
                    return d;
                }
                assert!(start.elapsed() < Duration::from_secs(10));
                std::thread::sleep(Duration::from_millis(2));
            }
        };
        let a = req(t.clone(), 0.0, 10.0, 400);
        let b = req(t, 2.0, 4.0, 400);
        svc.request(a.clone());
        svc.request(b.clone());
        let got_b = wait(&b);
        assert_eq!(got_b.request, b, "the later request wins");
        svc.request(a.clone());
        wait(&a);
        // Going back to b is served from the cache at once.
        svc.request(b.clone());
        assert_eq!(svc.get().map(|d| d.request.clone()), Some(b));
    }
}
