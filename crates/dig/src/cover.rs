//! Record covers for record rows and the record card: each fetched once from Discogs' or
//! Bandcamp's image host (never the Discogs API, so covers cost none of its request budget),
//! shrunk to a thumbnail, and kept on disk under `<cache>/covers/`. The card's large cover
//! (asked for only when the pointer rests on a record) is kept under `<cache>/covers/large/`.
//!
//! One low-priority thread fetches one cover at a time, at most 4 a second, whatever its size.
//! Two kinds of request wait: the hovered record's (only the newest, so sweeping the pointer
//! down a list fetches only where it stops: its thumbnail, then its large cover), which goes
//! first, and the record rows in view (thumbnails only), top first, replaced whenever the rows
//! in view change. The UI never reads these files itself; it gets
//! pixels back. An address that fails isn't tried again this session, and a 429 pauses
//! fetching for a minute.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use platform::{Priority, Spawner};

use crate::clock::Clock;
use crate::discogs::model::RecordKey;

/// Whose cover: a Discogs release or master, or a Bandcamp album by its art's id
/// (`a0161524395` in `https://f4.bcbits.com/img/a0161524395_5.jpg`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverKey {
    Record(RecordKey),
    BandcampArt(u64),
}

impl From<RecordKey> for CoverKey {
    fn from(k: RecordKey) -> Self {
        CoverKey::Record(k)
    }
}

/// A cover's size: the thumbnail rows draw, or the card's large cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverSize {
    Thumb,
    Large,
}

impl CoverSize {
    /// Stored at most this many pixels wide and high.
    pub fn max_px(self) -> u32 {
        match self {
            CoverSize::Thumb => MAX_PX,
            CoverSize::Large => LARGE_PX,
        }
    }
}

/// A Bandcamp cover at its 700 px size (`_16`), from any of its sizes' addresses (the art id
/// and host kept). `None` for an address that isn't Bandcamp art.
pub fn bandcamp_large(url: &str) -> Option<String> {
    bandcamp_art(url)?;
    if !allowed(url) {
        return None;
    }
    let (dir, name) = url.rsplit_once('/')?;
    let (stem, ext) = name.rsplit_once('.')?;
    let (art, _) = stem.split_once('_')?;
    Some(format!("{dir}/{art}_16.{ext}"))
}

/// A Bandcamp cover's art id, from its address.
pub fn bandcamp_art(url: &str) -> Option<u64> {
    let name = url.rsplit('/').next()?;
    let digits = name.strip_prefix('a')?.split('_').next()?;
    digits.parse().ok()
}
use crate::discogs::transport::USER_AGENT;

pub const DIR: &str = "covers";
/// Covers are stored at most this many pixels wide and high.
pub const MAX_PX: u32 = 150;
/// Large covers (the record card's) are stored at most this many pixels wide and high.
pub const LARGE_PX: u32 = 600;
/// Large covers' folder, under [`DIR`].
pub const LARGE_DIR: &str = "large";
/// Fetches start at least this far apart (4 a second).
const MIN_GAP: Duration = Duration::from_millis(250);
/// After a 429, nothing is fetched for this long.
const BACKOFF: Duration = Duration::from_secs(60);
/// A thumbnail is a few KB; anything this big isn't one.
const MAX_BYTES: u64 = 2 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    Status(u16),
    Net(String),
}

/// Fetches an image's bytes, behind a trait so tests run offline.
pub trait ImageSource: Send + Sync {
    fn get(&self, url: &str) -> Result<Vec<u8>, ImageError>;
}

/// The real thing: `ureq`, with the app's user agent.
pub struct UreqImages {
    agent: ureq::Agent,
}

impl Default for UreqImages {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl ImageSource for UreqImages {
    fn get(&self, url: &str) -> Result<Vec<u8>, ImageError> {
        let resp = self
            .agent
            .get(url)
            .header("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| ImageError::Net(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 {
            return Err(ImageError::Status(status));
        }
        resp.into_body()
            .with_config()
            .limit(MAX_BYTES)
            .read_to_vec()
            .map_err(|e| ImageError::Net(e.to_string()))
    }
}

/// Only Discogs' and Bandcamp's image hosts, over https: an address from cached JSON never sends a request
/// anywhere else.
pub fn allowed(url: &str) -> bool {
    url.strip_prefix("https://")
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .is_some_and(|host| {
            matches!(host, "i.discogs.com" | "img.discogs.com")
                // Bandcamp's image hosts: f1.bcbits.com … f4.bcbits.com.
                || host
                    .strip_suffix(".bcbits.com")
                    .and_then(|h| h.strip_prefix('f'))
                    .is_some_and(|n| n.len() == 1 && n.bytes().all(|b| b.is_ascii_digit()))
        })
}

/// A cover's pixels, unpremultiplied RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cover {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// One colour standing for the cover (see [`tint`]), worked out here, off the UI thread.
    pub tint: [u8; 3],
}

impl Cover {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let tint = tint(&rgba, width, height);
        Self {
            width,
            height,
            rgba,
            tint,
        }
    }
}

/// Samples along each side at most: enough for a colour, cheap on a 600 px cover, and the same
/// grid on a thumbnail and its large cover, so both give nearly the same colour.
const TINT_GRID: u32 = 32;

/// One colour for a cover, to tint its record's label: the mean of a grid of samples, each
/// weighted by how colourful it is (saturation, and lightness near the middle), so a white
/// border or black type doesn't wash it out. Its lightness is kept to 0.35–0.75, so it shows
/// on a black disc and isn't white.
pub fn tint(rgba: &[u8], width: u32, height: u32) -> [u8; 3] {
    let (mut sum, mut total) = ([0.0f32; 3], 0.0f32);
    let n = |len: u32| len.clamp(1, TINT_GRID);
    let (nx, ny) = (n(width), n(height));
    for j in 0..ny {
        for i in 0..nx {
            // The middle of each cell of the grid.
            let x = ((2 * i + 1) * width / (2 * nx)).min(width.saturating_sub(1));
            let y = ((2 * j + 1) * height / (2 * ny)).min(height.saturating_sub(1));
            let at = ((y * width + x) * 4) as usize;
            let Some(px) = rgba.get(at..at + 4) else {
                continue;
            };
            let c = [px[0], px[1], px[2]].map(|v| f32::from(v) / 255.0);
            let (_, s, l) = to_hsl(c);
            let w = (s * (1.0 - (l - 0.5).abs()) + 0.02) * f32::from(px[3]) / 255.0;
            for k in 0..3 {
                sum[k] += c[k] * w;
            }
            total += w;
        }
    }
    if total <= 0.0 {
        return from_hsl(0.0, 0.0, 0.35);
    }
    let (h, s, l) = to_hsl(sum.map(|v| v / total));
    from_hsl(h, s, l.clamp(0.35, 0.75))
}

/// Hue (0..1), saturation and lightness of an RGB colour in 0..1.
fn to_hsl([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d <= f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs()).max(f32::EPSILON);
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    (h, s.min(1.0), l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h * 6.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r, g, b].map(|v| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoverResult {
    Ready(CoverKey, Arc<Cover>),
    /// No cover this session (no image, a broken one, or a failed fetch).
    Failed(CoverKey),
    /// The address is gone (403/404): the record's data may hold a newer one.
    Stale(CoverKey),
    /// A large cover.
    LargeReady(CoverKey, Arc<Cover>),
    /// No large cover this session (the card keeps the thumbnail; a gone address isn't looked
    /// up again for it).
    LargeFailed(CoverKey),
}

/// The synchronous core (tests drive it directly); [`CoverHandle`] runs it on a thread.
pub struct Covers {
    dir: Option<PathBuf>,
    source: Arc<dyn ImageSource>,
    clock: Arc<dyn Clock>,
    /// Addresses that failed this session.
    failed: HashSet<String>,
    last_fetch: Option<Duration>,
    paused_until: Option<Duration>,
}

impl Covers {
    /// `cache_root` is the app's cache folder; covers go in its `covers/` (`None` keeps none).
    pub fn new(
        cache_root: Option<&Path>,
        source: Arc<dyn ImageSource>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            dir: cache_root.map(|r| r.join(DIR)),
            source,
            clock,
            failed: HashSet::new(),
            last_fetch: None,
            paused_until: None,
        }
    }

    pub fn path(&self, key: CoverKey) -> Option<PathBuf> {
        self.sized_path(key, CoverSize::Thumb)
    }

    pub fn sized_path(&self, key: CoverKey, size: CoverSize) -> Option<PathBuf> {
        let name = match key {
            CoverKey::Record(RecordKey::Release(id)) => format!("release-{id}.png"),
            CoverKey::Record(RecordKey::Master(id)) => format!("master-{id}.png"),
            CoverKey::BandcampArt(id) => format!("bandcamp-{id}.png"),
        };
        let dir = self.dir.as_ref()?;
        Some(match size {
            CoverSize::Thumb => dir.join(name),
            CoverSize::Large => dir.join(LARGE_DIR).join(name),
        })
    }

    /// `key`'s thumbnail: from disk, else fetched from `url`, shrunk, and stored.
    pub fn load(&mut self, key: CoverKey, url: &str) -> CoverResult {
        self.load_sized(key, CoverSize::Thumb, url)
    }

    /// `key`'s cover at `size`: from disk, else fetched from `url`, shrunk, and stored.
    pub fn load_sized(&mut self, key: CoverKey, size: CoverSize, url: &str) -> CoverResult {
        let large = size == CoverSize::Large;
        let ready = |c: Cover| {
            if large {
                CoverResult::LargeReady(key, Arc::new(c))
            } else {
                CoverResult::Ready(key, Arc::new(c))
            }
        };
        let failed = || {
            if large {
                CoverResult::LargeFailed(key)
            } else {
                CoverResult::Failed(key)
            }
        };
        if let Some(c) = self.sized_path(key, size).and_then(|p| read_png(&p)) {
            return ready(c);
        }
        if !allowed(url) || self.failed.contains(url) {
            return failed();
        }
        let now = self.clock.now();
        if self.paused_until.is_some_and(|t| now < t) {
            return failed();
        }
        if let Some(next) = self.last_fetch.map(|t| t + MIN_GAP)
            && now < next
        {
            self.clock.sleep(next - now);
        }
        self.last_fetch = Some(self.clock.now());
        match self.source.get(url) {
            Ok(bytes) => match decode(&bytes, size.max_px()) {
                Some(c) => {
                    if let Some(p) = self.sized_path(key, size) {
                        write_png(&p, &c);
                    }
                    ready(c)
                }
                None => {
                    self.failed.insert(url.to_owned());
                    failed()
                }
            },
            Err(ImageError::Status(403 | 404 | 410)) if !large => {
                self.failed.insert(url.to_owned());
                CoverResult::Stale(key)
            }
            Err(ImageError::Status(429)) => {
                self.paused_until = Some(self.clock.now() + BACKOFF);
                failed()
            }
            Err(_) => {
                self.failed.insert(url.to_owned());
                failed()
            }
        }
    }
}

/// JPEG, WebP or PNG, shrunk to fit `max` pixels (never enlarged): a thumbnail quickly, a
/// large cover smoothly.
fn decode(bytes: &[u8], max: u32) -> Option<Cover> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = if img.width() <= max && img.height() <= max {
        img
    } else if max <= MAX_PX {
        img.thumbnail(max, max)
    } else {
        img.resize(max, max, image::imageops::FilterType::Triangle)
    };
    let rgba = img.to_rgba8();
    Some(Cover::new(rgba.width(), rgba.height(), rgba.into_raw()))
}

fn read_png(path: &Path) -> Option<Cover> {
    let bytes = std::fs::read(path).ok()?;
    let rgba = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    Some(Cover::new(rgba.width(), rgba.height(), rgba.into_raw()))
}

/// Writes atomically; failures are ignored (the folder is only a cache).
fn write_png(path: &Path, c: &Cover) {
    let tmp = path.with_extension("tmp");
    let ok = path
        .parent()
        .is_some_and(|d| std::fs::create_dir_all(d).is_ok())
        && image::save_buffer_with_format(
            &tmp,
            &c.rgba,
            c.width,
            c.height,
            image::ExtendedColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .is_ok();
    if ok {
        let _ = std::fs::rename(&tmp, path);
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Answers every address with the same result (or a routed one), and logs the addresses asked
/// for (tests).
pub struct FakeImages {
    answer: Mutex<Result<Vec<u8>, ImageError>>,
    routes: Mutex<std::collections::HashMap<String, Result<Vec<u8>, ImageError>>>,
    log: Mutex<Vec<String>>,
}

impl FakeImages {
    pub fn new(answer: Result<Vec<u8>, ImageError>) -> Arc<Self> {
        Arc::new(Self {
            answer: Mutex::new(answer),
            routes: Mutex::default(),
            log: Mutex::new(Vec::new()),
        })
    }

    pub fn set_answer(&self, answer: Result<Vec<u8>, ImageError>) {
        *self.answer.lock().unwrap() = answer;
    }

    /// Answers `url` with `answer` instead.
    pub fn route(&self, url: &str, answer: Result<Vec<u8>, ImageError>) {
        self.routes.lock().unwrap().insert(url.to_owned(), answer);
    }

    /// The addresses asked for, in order.
    pub fn log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.log.lock().unwrap().len()
    }
}

impl ImageSource for FakeImages {
    fn get(&self, url: &str) -> Result<Vec<u8>, ImageError> {
        self.log.lock().unwrap().push(url.to_owned());
        match self.routes.lock().unwrap().get(url) {
            Some(a) => a.clone(),
            None => self.answer.lock().unwrap().clone(),
        }
    }
}

/// A `w`×`h` JPEG of one colour (tests).
pub fn test_jpeg(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 40, 10]));
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Jpeg)
        .expect("in memory");
    out.into_inner()
}

/// One cover to load.
type Job = (CoverKey, CoverSize, String);

enum Request {
    /// The hovered record's covers (its thumbnail, then its large one): they go first,
    /// replacing a hovered record's still waiting.
    Hover(Vec<Job>),
    /// The covers of the rows in view, top first, replacing the list still waiting.
    InView(Vec<(CoverKey, String)>),
}

/// The worker thread's end of the channels.
pub struct CoverHandle {
    requests: Sender<Request>,
    results: Receiver<CoverResult>,
}

impl CoverHandle {
    /// Runs `covers` on a low-priority thread; `wake` is called when a result is ready.
    pub fn start(
        spawner: &dyn Spawner,
        mut covers: Covers,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let (req_tx, req_rx) = channel::<Request>();
        let (res_tx, res_rx) = channel();
        spawner.spawn(
            "covers",
            Priority::Low,
            Box::new(move || {
                let mut hover: VecDeque<Job> = VecDeque::new();
                let mut in_view: VecDeque<(CoverKey, String)> = VecDeque::new();
                let take = |r: Request,
                            hover: &mut VecDeque<Job>,
                            in_view: &mut VecDeque<(CoverKey, String)>| {
                    match r {
                        Request::Hover(jobs) => *hover = jobs.into(),
                        Request::InView(list) => *in_view = list.into(),
                    }
                };
                loop {
                    if hover.is_empty() && in_view.is_empty() {
                        match req_rx.recv() {
                            Ok(r) => take(r, &mut hover, &mut in_view),
                            Err(_) => return,
                        }
                    }
                    loop {
                        match req_rx.try_recv() {
                            Ok(r) => take(r, &mut hover, &mut in_view),
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => return,
                        }
                    }
                    let next = hover
                        .pop_front()
                        .or_else(|| in_view.pop_front().map(|(k, u)| (k, CoverSize::Thumb, u)));
                    let Some((key, size, url)) = next else {
                        continue;
                    };
                    if size == CoverSize::Thumb {
                        in_view.retain(|(k, _)| *k != key);
                    }
                    if res_tx.send(covers.load_sized(key, size, &url)).is_err() {
                        return;
                    }
                    wake();
                }
            }),
        )?;
        Ok(Self {
            requests: req_tx,
            results: res_rx,
        })
    }

    /// Asks for the hovered row's cover first, replacing a hovered one still waiting.
    pub fn request(&self, key: CoverKey, url: &str) {
        self.request_record(key, Some(url), None);
    }

    /// Asks for the hovered record's covers first (its thumbnail, then its large one, each
    /// when given), replacing a hovered record's still waiting.
    pub fn request_record(&self, key: CoverKey, thumb: Option<&str>, large: Option<&str>) {
        let jobs = [(CoverSize::Thumb, thumb), (CoverSize::Large, large)]
            .into_iter()
            .filter_map(|(size, url)| Some((key, size, url?.to_owned())))
            .collect();
        let _ = self.requests.send(Request::Hover(jobs));
    }

    /// The covers of the rows in view, top first: they replace the list still waiting.
    pub fn want(&self, list: Vec<(CoverKey, String)>) {
        let _ = self.requests.send(Request::InView(list));
    }

    pub fn poll(&self) -> Vec<CoverResult> {
        self.results.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;

    const URL: &str = "https://i.discogs.com/abc/rs:fit/w:150/R-1001.jpeg";

    fn covers(root: &Path, images: &Arc<FakeImages>, clock: &Arc<FakeClock>) -> Covers {
        Covers::new(Some(root), images.clone(), clock.clone())
    }

    /// An image of `w` × `h` coloured by `f(x, y)` (fractions of the side), opaque.
    fn image(w: u32, h: u32, f: impl Fn(f32, f32) -> [u8; 3]) -> Vec<u8> {
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let [r, g, b] = f(x as f32 / w as f32, y as f32 / h as f32);
                out.extend_from_slice(&[r, g, b, 255]);
            }
        }
        out
    }

    fn lightness(c: [u8; 3]) -> f32 {
        to_hsl(c.map(|v| f32::from(v) / 255.0)).2
    }

    #[test]
    fn a_red_cover_tints_red() {
        let t = tint(&image(64, 64, |_, _| [220, 20, 30]), 64, 64);
        assert!(t[0] > 150 && t[1] < 60 && t[2] < 60, "{t:?}");
    }

    #[test]
    fn a_white_border_does_not_wash_the_tint_out() {
        // A red square in the middle third, white all around: most of the cover is white.
        let red = |x: f32, y: f32| {
            let inside = (0.33..0.67).contains(&x) && (0.33..0.67).contains(&y);
            if inside {
                [200, 30, 30]
            } else {
                [255, 255, 255]
            }
        };
        let t = tint(&image(90, 90, red), 90, 90);
        assert!(t[0] > t[1] + 60 && t[0] > t[2] + 60, "{t:?}");
    }

    #[test]
    fn a_black_cover_still_shows_on_the_disc() {
        let t = tint(&image(40, 40, |_, _| [0, 0, 0]), 40, 40);
        assert!(lightness(t) >= 0.34, "{t:?}");
        let t = tint(&image(40, 40, |_, _| [255, 255, 255]), 40, 40);
        assert!(lightness(t) <= 0.76, "{t:?}");
    }

    #[test]
    fn a_thumbnail_and_its_large_cover_tint_alike() {
        let art = |x: f32, y: f32| [(x * 255.0) as u8, (y * 200.0) as u8, 90];
        let small = tint(&image(150, 150, art), 150, 150);
        let large = tint(&image(600, 600, art), 600, 600);
        for k in 0..3 {
            assert!(small[k].abs_diff(large[k]) <= 8, "{small:?} {large:?}");
        }
    }

    #[test]
    fn an_empty_or_short_buffer_gives_a_grey() {
        assert_eq!(tint(&[], 0, 0), tint(&[], 4, 4));
        assert!(lightness(tint(&[], 4, 4)) >= 0.34);
    }

    #[test]
    fn a_cover_is_fetched_once_shrunk_and_then_read_from_disk() {
        let root = crate::test_dir("covers-once");
        let images = FakeImages::new(Ok(test_jpeg(600, 300)));
        let clock = Arc::new(FakeClock::default());
        let mut c = covers(&root, &images, &clock);
        let key = CoverKey::Record(RecordKey::Release(1001));
        let CoverResult::Ready(_, cover) = c.load(key, URL) else {
            panic!("a cover");
        };
        assert_eq!(
            (cover.width, cover.height),
            (150, 75),
            "fits 150 px, aspect kept"
        );
        assert!(root.join("covers/release-1001.png").exists());

        // Next session: from disk, no request.
        let mut c = covers(&root, &images, &clock);
        assert!(matches!(c.load(key, URL), CoverResult::Ready(..)));
        assert_eq!(images.count(), 1);
    }

    #[test]
    fn a_large_cover_is_kept_apart_at_600_px_and_its_failure_spares_the_thumbnail() {
        let root = crate::test_dir("covers-large");
        let images = FakeImages::new(Ok(test_jpeg(1200, 900)));
        let clock = Arc::new(FakeClock::default());
        let mut c = covers(&root, &images, &clock);
        let key = CoverKey::Record(RecordKey::Release(1));
        let CoverResult::LargeReady(_, big) = c.load_sized(key, CoverSize::Large, URL) else {
            panic!("a large cover");
        };
        assert_eq!((big.width, big.height), (600, 450));
        assert!(root.join("covers/large/release-1.png").exists());
        assert!(
            !root.join("covers/release-1.png").exists(),
            "no thumbnail made"
        );
        let mut c = covers(&root, &images, &clock);
        assert!(matches!(
            c.load_sized(key, CoverSize::Large, URL),
            CoverResult::LargeReady(..)
        ));
        assert_eq!(images.count(), 1, "from disk next session");

        // Gone: failed for the session, never stale (no lookup for a large cover).
        let gone = "https://i.discogs.com/gone/R-2.jpeg";
        images.route(gone, Err(ImageError::Status(404)));
        let two = CoverKey::Record(RecordKey::Release(2));
        assert_eq!(
            c.load_sized(two, CoverSize::Large, gone),
            CoverResult::LargeFailed(two)
        );
        assert_eq!(
            c.load_sized(two, CoverSize::Large, gone),
            CoverResult::LargeFailed(two)
        );
        assert_eq!(images.count(), 2, "not asked again");
        assert!(
            matches!(c.load(two, URL), CoverResult::Ready(..)),
            "the thumbnail still loads"
        );
    }

    #[test]
    fn bandcamp_large_art_is_the_same_art_at_700_px() {
        assert_eq!(
            bandcamp_large("https://f4.bcbits.com/img/a0161524395_7.jpg").as_deref(),
            Some("https://f4.bcbits.com/img/a0161524395_16.jpg")
        );
        assert_eq!(bandcamp_large("https://i.discogs.com/x/R-1.jpeg"), None);
        assert_eq!(
            bandcamp_large("https://f4.bcbits.com.evil.net/img/a1_7.jpg"),
            None
        );
    }

    #[test]
    fn failures_are_remembered_and_gone_addresses_are_stale() {
        let root = crate::test_dir("covers-fail");
        let clock = Arc::new(FakeClock::default());
        let key = CoverKey::Record(RecordKey::Master(9));

        let images = FakeImages::new(Err(ImageError::Net("down".into())));
        let mut c = covers(&root, &images, &clock);
        assert_eq!(c.load(key, URL), CoverResult::Failed(key));
        assert_eq!(c.load(key, URL), CoverResult::Failed(key));
        assert_eq!(images.count(), 1, "not tried again this session");

        let images = FakeImages::new(Err(ImageError::Status(404)));
        let mut c = covers(&root, &images, &clock);
        assert_eq!(c.load(key, URL), CoverResult::Stale(key));
        let newer = "https://i.discogs.com/new/R-9.jpeg";
        assert_eq!(
            c.load(key, newer),
            CoverResult::Stale(key),
            "a new address is tried"
        );
        assert_eq!(images.count(), 2);

        let images = FakeImages::new(Ok(b"not an image".to_vec()));
        let mut c = covers(&root, &images, &clock);
        assert_eq!(c.load(key, URL), CoverResult::Failed(key));
        assert!(!root.join("covers/master-9.png").exists());
    }

    #[test]
    fn bandcamp_covers_are_keyed_by_their_art() {
        assert_eq!(
            bandcamp_art("https://f4.bcbits.com/img/a0161524395_5.jpg"),
            Some(161524395)
        );
        assert_eq!(bandcamp_art("https://f4.bcbits.com/img/x.jpg"), None);
        assert!(allowed("https://f4.bcbits.com/img/a0161524395_5.jpg"));
        for bad in [
            "https://f44.bcbits.com/x.jpg",
            "https://bcbits.com/x.jpg",
            "https://f4.bcbits.com.evil.net/x.jpg",
            "http://f4.bcbits.com/x.jpg",
        ] {
            assert!(!allowed(bad), "{bad}");
        }
    }

    #[test]
    fn only_discogs_image_hosts_are_asked() {
        let root = crate::test_dir("covers-hosts");
        let images = FakeImages::new(Ok(test_jpeg(10, 10)));
        let clock = Arc::new(FakeClock::default());
        let mut c = covers(&root, &images, &clock);
        for url in [
            "http://i.discogs.com/x.jpeg",
            "https://evil.com/i.discogs.com/x.jpeg",
            "https://i.discogs.com.evil.com/x.jpeg",
            "https://api.discogs.com/images/x.jpeg",
            "",
        ] {
            assert_eq!(
                c.load(CoverKey::Record(RecordKey::Release(1)), url),
                CoverResult::Failed(CoverKey::Record(RecordKey::Release(1))),
                "{url}"
            );
        }
        assert_eq!(images.count(), 0);
        assert!(allowed("https://img.discogs.com/x.jpg"));
        assert!(allowed(URL));
    }

    #[test]
    fn fetches_are_paced_and_a_429_pauses_them() {
        let root = crate::test_dir("covers-pace");
        let images = FakeImages::new(Ok(test_jpeg(10, 10)));
        let clock = Arc::new(FakeClock::default());
        let mut c = Covers::new(None, images.clone(), clock.clone());
        for id in 0..5 {
            c.load(CoverKey::Record(RecordKey::Release(id)), URL);
        }
        assert_eq!(images.count(), 5);
        assert_eq!(clock.now(), Duration::from_secs(1), "4 a second");
        // Large covers share the pace.
        for id in 0..4 {
            c.load_sized(
                CoverKey::Record(RecordKey::Release(id)),
                CoverSize::Large,
                URL,
            );
        }
        assert_eq!(images.count(), 9);
        assert_eq!(clock.now(), Duration::from_secs(2), "still 4 a second");

        let images = FakeImages::new(Err(ImageError::Status(429)));
        let mut c = covers(&root, &images, &clock);
        c.load(CoverKey::Record(RecordKey::Release(1)), URL);
        c.load(
            CoverKey::Record(RecordKey::Release(2)),
            "https://i.discogs.com/2.jpeg",
        );
        assert_eq!(images.count(), 1, "paused after a 429");
        clock.advance(BACKOFF);
        c.load(
            CoverKey::Record(RecordKey::Release(3)),
            "https://i.discogs.com/3.jpeg",
        );
        assert_eq!(images.count(), 2, "and fetching again a minute later");
    }

    #[test]
    fn the_worker_fetches_only_the_newest_waiting_request() {
        struct Slow(Arc<FakeImages>);
        impl ImageSource for Slow {
            fn get(&self, url: &str) -> Result<Vec<u8>, ImageError> {
                std::thread::sleep(Duration::from_millis(50));
                self.0.get(url)
            }
        }
        let images = FakeImages::new(Ok(test_jpeg(10, 10)));
        let covers = Covers::new(
            None,
            Arc::new(Slow(images.clone())),
            Arc::new(crate::clock::RealClock::default()),
        );
        let h = CoverHandle::start(&platform::native::NativeSpawner, covers, || {}).unwrap();
        // The first starts at once; the next 39 wait behind it and only the last is fetched.
        for id in 0..40 {
            h.request(
                CoverKey::Record(RecordKey::Release(id)),
                &format!("https://i.discogs.com/{id}.jpeg"),
            );
        }
        let key = |r: &CoverResult| match r {
            CoverResult::Ready(k, _)
            | CoverResult::Failed(k)
            | CoverResult::Stale(k)
            | CoverResult::LargeReady(k, _)
            | CoverResult::LargeFailed(k) => *k,
        };
        let mut keys = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while keys.last() != Some(&CoverKey::Record(RecordKey::Release(39))) {
            assert!(std::time::Instant::now() < deadline, "timed out: {keys:?}");
            keys.extend(h.poll().iter().map(key));
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(images.count() <= 2, "{} fetched", images.count());
    }

    #[test]
    fn a_hovered_record_loads_its_thumbnail_then_its_large_cover() {
        struct Slow(Arc<FakeImages>);
        impl ImageSource for Slow {
            fn get(&self, url: &str) -> Result<Vec<u8>, ImageError> {
                std::thread::sleep(Duration::from_millis(50));
                self.0.get(url)
            }
        }
        let images = FakeImages::new(Ok(test_jpeg(10, 10)));
        let covers = Covers::new(
            None,
            Arc::new(Slow(images.clone())),
            Arc::new(crate::clock::RealClock::default()),
        );
        let h = CoverHandle::start(&platform::native::NativeSpawner, covers, || {}).unwrap();
        // Something in front, so A's and B's requests wait together.
        h.request(
            CoverKey::Record(RecordKey::Release(9)),
            "https://i.discogs.com/9.jpeg",
        );
        std::thread::sleep(Duration::from_millis(10));
        for id in [1, 2] {
            h.request_record(
                CoverKey::Record(RecordKey::Release(id)),
                Some(&format!("https://i.discogs.com/{id}-150.jpeg")),
                Some(&format!("https://i.discogs.com/{id}-big.jpeg")),
            );
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut large = false;
        while !large {
            assert!(std::time::Instant::now() < deadline, "timed out");
            large = h
                .poll()
                .iter()
                .any(|r| matches!(r, CoverResult::LargeReady(..)));
            std::thread::sleep(Duration::from_millis(5));
        }
        // Whether 9 got started before the records came in depends on the thread; after it,
        // only the newest record's covers, thumbnail first.
        let after: Vec<String> = images
            .log()
            .into_iter()
            .filter(|u| !u.ends_with("/9.jpeg"))
            .collect();
        assert_eq!(
            after,
            [
                "https://i.discogs.com/2-150.jpeg",
                "https://i.discogs.com/2-big.jpeg"
            ],
            "only the newest record, thumbnail first"
        );
    }

    #[test]
    fn rows_in_view_load_top_first_and_a_hover_jumps_the_queue() {
        let dir = crate::test_dir("covers-in-view");
        let images = FakeImages::new(Ok(test_jpeg(10, 10)));
        let clock = Arc::new(FakeClock::default());
        let covers = covers(dir.path(), &images, &clock);
        let h = CoverHandle::start(&platform::native::NativeSpawner, covers, || {}).unwrap();
        let url = |id: u64| format!("https://i.discogs.com/{id}.jpeg");
        // Rows 1..=12 in view, then scrolled: 20..=23 replace whatever still waits.
        h.want(
            (1..=12)
                .map(|id| (CoverKey::Record(RecordKey::Release(id)), url(id)))
                .collect(),
        );
        h.want(
            (20..=23)
                .map(|id| (CoverKey::Record(RecordKey::Release(id)), url(id)))
                .collect(),
        );
        h.request(CoverKey::Record(RecordKey::Release(99)), &url(99));
        let key = |r: &CoverResult| match r {
            CoverResult::Ready(k, _)
            | CoverResult::Failed(k)
            | CoverResult::Stale(k)
            | CoverResult::LargeReady(k, _)
            | CoverResult::LargeFailed(k) => *k,
        };
        let mut keys = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while keys.last() != Some(&CoverKey::Record(RecordKey::Release(23))) {
            assert!(std::time::Instant::now() < deadline, "timed out: {keys:?}");
            keys.extend(h.poll().iter().map(key));
            std::thread::sleep(Duration::from_millis(2));
        }
        let ids: Vec<u64> = keys
            .iter()
            .map(|k| match k {
                CoverKey::Record(RecordKey::Release(id) | RecordKey::Master(id))
                | CoverKey::BandcampArt(id) => *id,
            })
            .collect();
        let at = |id: u64| ids.iter().position(|&i| i == id).unwrap();
        assert!(at(99) < at(20), "the hover first: {ids:?}");
        assert!(at(20) < at(21) && at(21) < at(22), "top first: {ids:?}");
        assert!(!ids.contains(&12), "scrolled past before its turn: {ids:?}");
        // Cached on disk now: asked again, they come back without a fetch.
        let fetched = images.count();
        h.want(vec![(CoverKey::Record(RecordKey::Release(20)), url(20))]);
        while !h
            .poll()
            .iter()
            .any(|r| key(r) == CoverKey::Record(RecordKey::Release(20)))
        {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(images.count(), fetched);
    }
}
