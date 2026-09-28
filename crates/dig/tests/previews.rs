//! The preview scheduler with a fake fetcher: the horizon, 2 slots, the armed entry first,
//! cancelling, retrying, the timeout, a yt-dlp that appears later, and the cache limit.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use dig::preview::fetcher::{FakeFetcher, Fetcher};
use dig::preview::scheduler::{Config, Finder, PreviewCommand, PreviewEvent, PreviewHandle};
use platform::native::NativeSpawner;
use platform::testing::TestDir;

fn dir(name: &str) -> TestDir {
    TestDir::new(&format!("dig-previews-{name}"))
}

fn fixture() -> PathBuf {
    format!(
        "{}/../audio/tests/fixtures/tone.m4a",
        env!("CARGO_MANIFEST_DIR")
    )
    .into()
}

fn clip(n: usize) -> String {
    format!("clip{n:07}")
}

fn start(
    name: &str,
    fake: &Arc<FakeFetcher>,
    tweak: impl FnOnce(&mut Config),
) -> (PreviewHandle, TestDir) {
    let d = dir(name);
    let mut cfg = Config::new(d.to_path_buf(), 2_000_000_000, None);
    cfg.recheck = Duration::from_millis(100);
    tweak(&mut cfg);
    let f = fake.clone();
    let finder: Finder = Arc::new(move |_| {
        let v = f.version()?;
        Some((f.clone() as Arc<dyn Fetcher>, v))
    });
    (
        PreviewHandle::start(Arc::new(NativeSpawner), cfg, finder, || {}).unwrap(),
        d,
    )
}

fn want(h: &PreviewHandle, wanted: &[String]) {
    h.send(PreviewCommand::Want {
        wanted: wanted.to_vec(),
        protected: wanted.to_vec(),
    });
}

/// Collects events until `done` says so (or 5 s pass).
fn until(h: &PreviewHandle, mut done: impl FnMut(&[PreviewEvent]) -> bool) -> Vec<PreviewEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut all = Vec::new();
    while Instant::now() < deadline {
        all.extend(h.poll());
        if done(&all) {
            return all;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out; got {all:?}");
}

fn done_count(ev: &[PreviewEvent]) -> usize {
    ev.iter()
        .filter(|e| matches!(e, PreviewEvent::Done(..)))
        .count()
}

#[test]
fn the_horizon_downloads_two_at_a_time_in_order() {
    let fake = Arc::new({
        let mut f = FakeFetcher::new(fixture());
        f.delay = Duration::from_millis(80);
        f
    });
    let (h, d) = start("slots", &fake, |_| {});
    let wanted: Vec<String> = (3..7).map(clip).collect();
    want(&h, &wanted);
    // While the first two download, the others wait.
    let first = until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::Progress(..)))
    });
    assert!(first.iter().all(|e| !matches!(e, PreviewEvent::Done(..))));
    let pair = |mut v: Vec<String>| {
        v.sort();
        v
    };
    assert_eq!(pair(fake.started()), [clip(3), clip(4)]);
    let ev = until(&h, |ev| done_count(ev) == 4);
    let started = fake.started();
    assert_eq!(
        pair(started[2..].to_vec()),
        [clip(5), clip(6)],
        "in priority order"
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Progress(c, 50) if *c == clip(3)))
    );
    for c in &wanted {
        assert!(d.join(format!("{c}.m4a")).exists());
    }
    assert!(
        !d.join(format!("{}.m4a", clip(7))).exists(),
        "nothing past the horizon"
    );
    // Wanting them again downloads nothing.
    want(&h, &wanted);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(fake.started().len(), 4);
}

#[test]
fn the_armed_entry_takes_the_next_free_slot() {
    let fake = Arc::new({
        let mut f = FakeFetcher::new(fixture());
        f.delay = Duration::from_millis(60);
        f
    });
    let (h, _) = start("armed", &fake, |_| {});
    let mut wanted: Vec<String> = (3..7).map(clip).collect();
    want(&h, &wanted);
    until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::Progress(..)))
    });
    wanted.insert(0, clip(29));
    want(&h, &wanted);
    until(&h, |ev| done_count(ev) >= 5);
    assert!(
        fake.started()[..3].contains(&clip(29)),
        "before any other waiting preview: {:?}",
        fake.started()
    );
}

#[test]
fn leaving_the_horizon_cancels_and_deletes_the_partial_file() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.hang(&clip(1));
    let (h, d) = start("cancel", &fake, |_| {});
    want(&h, &[clip(1)]);
    until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::Progress(..)))
    });
    assert!(d.join(format!("{}.m4a.part", clip(1))).exists());
    want(&h, &[clip(2)]);
    until(&h, |ev| done_count(ev) == 1);
    std::thread::sleep(Duration::from_millis(50));
    assert!(!d.join(format!("{}.m4a.part", clip(1))).exists());
    assert!(!d.join(format!("{}.m4a", clip(1))).exists());
}

#[test]
fn a_failure_is_retried_once_then_given_up() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.fail(&clip(1), 1);
    fake.fail(&clip(2), u32::MAX);
    let (h, _) = start("retry", &fake, |_| {});
    want(&h, &[clip(1), clip(2)]);
    let ev = until(&h, |ev| {
        done_count(ev) == 1 && ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..)))
    });
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Done(c, _) if *c == clip(1)))
    );
    assert!(ev.contains(&PreviewEvent::Failed(clip(2), "clip failed".into())));
    let tries = fake.started().iter().filter(|c| **c == clip(2)).count();
    assert_eq!(tries, 2);
}

#[test]
fn a_hanging_download_times_out_and_is_retried_once() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.hang(&clip(1));
    let (h, _) = start("timeout", &fake, |c| c.timeout = Duration::from_millis(150));
    want(&h, &[clip(1)]);
    let ev = until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..)))
    });
    assert!(ev.contains(&PreviewEvent::Failed(clip(1), "clip failed".into())));
    assert_eq!(fake.started(), [clip(1), clip(1)]);
}

#[test]
fn missing_yt_dlp_waits_and_downloads_once_installed() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.available.store(false, Ordering::SeqCst);
    let (h, _) = start("missing", &fake, |_| {});
    want(&h, &[clip(1)]);
    let ev = until(&h, |ev| ev.contains(&PreviewEvent::NeedsYtDlp));
    std::thread::sleep(Duration::from_millis(250));
    let later = h.poll();
    assert!(
        !later.contains(&PreviewEvent::NeedsYtDlp) && !ev[1..].contains(&PreviewEvent::NeedsYtDlp),
        "said once"
    );
    fake.available.store(true, Ordering::SeqCst);
    let ev = until(&h, |ev| done_count(ev) == 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::YtDlp(v) if v.contains("fake")))
    );
}

#[test]
fn the_cache_limit_evicts_all_but_the_protected() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let size = std::fs::metadata(fixture()).unwrap().len();
    // Room for 3 previews (and the partial file of one still downloading).
    let (h, d) = start("limit", &fake, |c| c.limit = size * 3 + 1000);
    let first: Vec<String> = (0..3).map(clip).collect();
    want(&h, &first);
    until(&h, |ev| done_count(ev) == 3);
    for c in &first {
        let _ = std::fs::File::options()
            .write(true)
            .open(d.join(format!("{c}.m4a")))
            .unwrap()
            .set_modified(std::time::SystemTime::now() - Duration::from_secs(100));
    }
    h.send(PreviewCommand::Played(d.join(format!("{}.m4a", clip(0)))));
    let next: Vec<String> = (3..5).map(clip).collect();
    want(&h, &next);
    let mut ev = until(&h, |ev| done_count(ev) == 2);
    std::thread::sleep(Duration::from_millis(50));
    ev.extend(h.poll());
    let evicted: Vec<&PreviewEvent> = ev
        .iter()
        .filter(|e| matches!(e, PreviewEvent::Evicted(_)))
        .collect();
    assert_eq!(
        evicted,
        [
            &PreviewEvent::Evicted(clip(1)),
            &PreviewEvent::Evicted(clip(2))
        ],
        "least recently played first; clip 0 was just played"
    );
    assert!(d.join(format!("{}.m4a", clip(0))).exists());
    // Needed again: downloaded again.
    want(&h, &[clip(1)]);
    until(&h, |ev| done_count(ev) == 1);
    assert_eq!(fake.started().iter().filter(|c| **c == clip(1)).count(), 2);
}
