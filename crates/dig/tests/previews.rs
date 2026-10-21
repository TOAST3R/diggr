//! The preview scheduler with a fake fetcher: the horizon, 2 slots, the armed entry first,
//! cancelling, retrying, the timeout, a yt-dlp that appears later, the app's own yt-dlp
//! (downloaded on request, never preferred to the user's), the cache limit, and Juno samples
//! (fetched without yt-dlp).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use dig::bandcamp::{self, BandcampAlbum, BandcampTrack, Listing};
use dig::preview::clip::Source;
use dig::preview::fetcher::{FakeFetcher, Fetcher};
use dig::preview::sample::SampleFetcher;
use dig::preview::scheduler::{Config, Finder, PreviewCommand, PreviewEvent, PreviewHandle, Tools};
use dig::tools::{self, FakeSource};
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
    assert!(ev.iter().any(
        |e| matches!(e, PreviewEvent::YtDlp { version, managed: false } if version.contains("fake"))
    ));
}

/// A scheduler whose user has no yt-dlp, and whose own copy (a stand-in script) comes from
/// `source`; the fake fetcher answers only for the app's copy, once it exists.
fn start_with_tools(
    name: &str,
    fake: &Arc<FakeFetcher>,
    source: &Arc<FakeSource>,
) -> (PreviewHandle, TestDir) {
    let d = dir(name);
    let mut cfg = Config::new(d.join("previews"), 2_000_000_000, None);
    cfg.recheck = Duration::from_millis(100);
    cfg.tools = Some(Tools {
        dir: d.join("tools"),
        source: source.clone(),
    });
    let f = fake.clone();
    let finder: Finder = Arc::new(move |path| {
        let found = path.is_some_and(|p| std::path::Path::new(p).exists());
        found.then(|| (f.clone() as Arc<dyn Fetcher>, "2026.08.19".to_owned()))
    });
    (
        PreviewHandle::start(Arc::new(NativeSpawner), cfg, finder, || {}).unwrap(),
        d,
    )
}

const SCRIPT: &[u8] = b"#!/bin/sh\necho 2026.08.19\n";

#[test]
fn waiting_entries_start_once_the_apps_own_yt_dlp_is_installed() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let source = Arc::new(FakeSource::new("2026.08.19", SCRIPT, true));
    let (h, d) = start_with_tools("tools-install", &fake, &source);
    want(&h, &[clip(1)]);
    until(&h, |ev| ev.contains(&PreviewEvent::NeedsYtDlp));
    assert!(
        source.asked.lock().unwrap().is_empty(),
        "nothing downloaded unasked"
    );
    h.send(PreviewCommand::DownloadYtDlp);
    let ev = until(&h, |ev| done_count(ev) == 1);
    assert!(ev.contains(&PreviewEvent::YtDlpProgress(100)));
    assert!(ev.contains(&PreviewEvent::YtDlp {
        version: "2026.08.19".into(),
        managed: true
    }));
    assert!(tools::program(&d.join("tools")).exists());
}

#[test]
fn a_mismatched_download_is_reported_and_entries_keep_waiting() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let source = Arc::new(FakeSource::new("2026.08.19", SCRIPT, false));
    let (h, d) = start_with_tools("tools-mismatch", &fake, &source);
    want(&h, &[clip(1)]);
    until(&h, |ev| ev.contains(&PreviewEvent::NeedsYtDlp));
    h.send(PreviewCommand::DownloadYtDlp);
    until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::YtDlpFailed(why) if why.contains("checksum")))
    });
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(done_count(&h.poll()), 0);
    assert!(fake.started().is_empty());
    assert!(!tools::program(&d.join("tools")).exists());
}

#[test]
fn the_users_own_yt_dlp_wins_and_is_never_updated() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let source = Arc::new(FakeSource::new("2099.01.01", SCRIPT, true));
    let (h, _) = start("tools-user", &fake, |c| {
        c.tools = Some(Tools {
            dir: c.dir.join("tools"),
            source: source.clone(),
        })
    });
    want(&h, &[clip(1), clip(2)]);
    let ev = until(&h, |ev| done_count(ev) == 2);
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::YtDlp { managed: false, .. }))
    );
    assert!(
        source.asked.lock().unwrap().is_empty(),
        "never checked for updates"
    );
}

#[test]
fn the_apps_copy_is_updated_while_previews_are_fetched() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let source = Arc::new(FakeSource::new("2026.08.19", SCRIPT, true));
    let (h, d) = start_with_tools("tools-update", &fake, &source);
    // An older copy is installed, never checked.
    std::fs::create_dir_all(d.join("tools")).unwrap();
    std::fs::write(tools::program(&d.join("tools")), b"old").unwrap();
    *source.tag.lock().unwrap() = "2099.01.01".into();
    want(&h, &[clip(1)]);
    until(&h, |ev| done_count(ev) == 1);
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::fs::read(tools::program(&d.join("tools"))).unwrap() == b"old" {
        assert!(Instant::now() < deadline, "the newer release is installed");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read(tools::program(&d.join("tools"))).unwrap(),
        SCRIPT
    );
    assert!(d.join("tools/checked").exists());
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

// ---- searching for tracks without a clip ---------------------------------------------------

use dig::preview::search::{SearchRequest, SearchResult, Searches};

fn track(key: &str, title: &str, secs: f64) -> SearchRequest {
    SearchRequest {
        key: key.into(),
        artist: "The 89th Passenger".into(),
        record_artist: "The 89th Passenger".into(),
        title: title.into(),
        duration: Some(secs),
    }
}

fn upload(id: &str, title: &str, secs: f64) -> SearchResult {
    SearchResult {
        id: id.into(),
        duration: Some(secs),
        channel: "Analogical Force".into(),
        title: format!("The 89th Passenger - {title}"),
    }
}

fn answers(ev: &[PreviewEvent]) -> usize {
    ev.iter()
        .filter(|e| {
            matches!(
                e,
                PreviewEvent::Found { .. } | PreviewEvent::NotFound { .. }
            )
        })
        .count()
}

#[test]
fn tracks_are_searched_one_at_a_time_and_remembered() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.answer(
        "The 89th Passenger Paper Wings",
        vec![upload("paperwings1", "Paper Wings", 292.0)],
    );
    // "Hidden Soul": only a live set, far too long.
    fake.answer(
        "The 89th Passenger Hidden Soul",
        vec![upload("hiddenlive1", "Hidden Soul (live)", 900.0)],
    );
    // Results are remembered in the test's own folder.
    let (h, d) = start("search", &fake, |c| c.searches = Some(c.dir.clone()));
    h.send(PreviewCommand::Search(vec![
        track("release/1/A1", "Paper Wings", 291.0),
        track("release/1/A2", "Hidden Soul", 299.0),
    ]));
    let ev = until(&h, |ev| answers(ev) == 2);
    assert!(ev.contains(&PreviewEvent::Found {
        key: "release/1/A1".into(),
        clip: "paperwings1".into(),
        title: "The 89th Passenger - Paper Wings".into(),
        duration: Some(292.0),
    }));
    assert!(ev.contains(&PreviewEvent::NotFound {
        key: "release/1/A2".into(),
        duration: Some(299.0),
    }));
    assert_eq!(fake.searched().len(), 2, "one search each");
    // Each search says when it starts and ends, one at a time: start, end, start, end.
    let marks: Vec<Option<String>> = ev
        .iter()
        .filter_map(|e| match e {
            PreviewEvent::Searching(k) => Some(k.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        marks,
        [
            Some("release/1/A1".to_owned()),
            None,
            Some("release/1/A2".to_owned()),
            None
        ]
    );
    assert_eq!(
        Searches::load(&d).tracks.len(),
        2,
        "both remembered, by track"
    );

    // Asked again (another crate, a restart): answered without searching.
    h.send(PreviewCommand::Search(vec![track(
        "release/1/A1",
        "Paper Wings",
        291.0,
    )]));
    until(&h, |ev| answers(ev) == 1);
    assert_eq!(fake.searched().len(), 2);

    // The same track on another release (another position, so another entry key): no search.
    h.send(PreviewCommand::Search(vec![track(
        "track/the 89th passenger/paper wings",
        "Paper Wings",
        290.0,
    )]));
    let ev = until(&h, |ev| answers(ev) == 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Found { key, clip, .. }
        if key == "track/the 89th passenger/paper wings" && clip == "paperwings1"))
    );
    assert_eq!(
        fake.searched().len(),
        2,
        "answered from the first release's search"
    );
}

#[test]
fn a_failed_search_is_not_remembered() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.search_fails.store(true, Ordering::SeqCst);
    let (h, d) = start("search-offline", &fake, |c| {
        c.searches = Some(c.dir.clone())
    });
    let cache = d.to_path_buf();
    h.send(PreviewCommand::Search(vec![track(
        "release/1/A1",
        "Paper Wings",
        291.0,
    )]));
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut ev = Vec::new();
    while Instant::now() < deadline {
        ev.extend(h.poll());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(answers(&ev), 0, "no answer: the entry stays to search");
    assert_eq!(fake.searched().len(), 1, "not tried again at once");
    assert!(Searches::load(&cache).results.is_empty());
}

/// The real yt-dlp against YouTube: run by hand (`cargo test -p dig --test previews -- --ignored
/// --nocapture`), it needs yt-dlp and the network.
#[test]
#[ignore]
fn the_real_search_finds_hidden_soul_ep() {
    let (ytdlp, version) = dig::preview::fetcher::find(None).expect("yt-dlp on this machine");
    println!("yt-dlp {version}");
    let tracks = [
        ("A1", "Paper Wings", 291.0),
        ("A2", "Hidden Soul", 299.0),
        ("B1", "What It Takes To Fly", 276.0),
        ("B2", "Eren", 237.0),
        ("B3", "Analog Serenade", 241.0),
    ];
    let mut found = 0;
    for (pos, title, secs) in tracks {
        let req = SearchRequest {
            key: format!("release/38583846/{pos}"),
            ..track("", title, secs)
        };
        let results = ytdlp
            .search(&dig::preview::search::query(&req.artist, &req.title))
            .expect("a search");
        let best = dig::preview::search::best(&req, &results);
        println!("{pos} {title}: {} results, best {best:?}", results.len());
        found += best.is_some() as usize;
    }
    assert!(found > 0, "at least one track of the EP found");
}

// ---- Download all tracks: the background list ------------------------------------------------

fn background(h: &PreviewHandle, clips: &[String]) {
    h.send(PreviewCommand::Background(clips.to_vec()));
}

#[test]
fn background_clips_take_the_slots_the_horizon_leaves() {
    let fake = Arc::new({
        let mut f = FakeFetcher::new(fixture());
        f.delay = Duration::from_millis(60);
        f
    });
    let (h, d) = start("background", &fake, |_| {});
    let horizon: Vec<String> = (0..2).map(clip).collect();
    let rest: Vec<String> = (10..14).map(clip).collect();
    want(&h, &horizon);
    background(&h, &rest);
    until(&h, |ev| done_count(ev) == 6);
    let started = fake.started();
    let mut first: Vec<String> = started[..2].to_vec();
    first.sort();
    assert_eq!(first, horizon, "the horizon first: {started:?}");
    for c in horizon.iter().chain(&rest) {
        assert!(d.join(format!("{c}.m4a")).exists());
    }
    // A new horizon doesn't cancel the background list.
    background(&h, &(20..24).map(clip).collect::<Vec<_>>());
    until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Progress(c, _) if *c == clip(20)))
    });
    want(&h, &[clip(1)]);
    until(&h, |ev| done_count(ev) == 4);
}

#[test]
fn a_full_cache_pauses_the_background_list_instead_of_evicting() {
    let size = std::fs::metadata(fixture()).unwrap().len();
    let fake = Arc::new(FakeFetcher::new(fixture()));
    // Room for two and a half previews.
    let (h, d) = start("bg-full", &fake, |cfg| cfg.limit = size * 5 / 2);
    let clips: Vec<String> = (0..5).map(clip).collect();
    background(&h, &clips);
    let ev = until(&h, |ev| ev.contains(&PreviewEvent::CacheFull));
    assert!(
        !ev.iter().any(|e| matches!(e, PreviewEvent::Evicted(_))),
        "nothing deleted: {ev:?}"
    );
    std::thread::sleep(Duration::from_millis(200));
    // Two download at once: the cache may end one preview past its limit, never more.
    let downloaded = fake.started().len();
    assert!((3..=4).contains(&downloaded), "{downloaded}");
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(fake.started().len(), downloaded, "paused");
    // A larger cache resumes it.
    h.send(PreviewCommand::SetLimit(size * 100));
    let all_there = || clips.iter().all(|c| d.join(format!("{c}.m4a")).exists());
    until(&h, |_| all_there());
}

#[test]
fn an_empty_background_list_stops_it() {
    let fake = Arc::new({
        let mut f = FakeFetcher::new(fixture());
        f.delay = Duration::from_millis(80);
        f
    });
    let (h, d) = start("bg-stop", &fake, |_| {});
    let clips: Vec<String> = (0..6).map(clip).collect();
    background(&h, &clips);
    until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::Progress(..)))
    });
    background(&h, &[]);
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(fake.started().len(), 2, "only the two that had started");
    assert!(!d.join(format!("{}.m4a", clip(5))).exists());
}

// ---- YouTube limiting requests ------------------------------------------------------------------

fn limited_events(ev: &[PreviewEvent]) -> usize {
    ev.iter()
        .filter(|e| matches!(e, PreviewEvent::Limited { .. }))
        .count()
}

#[test]
fn a_limited_youtube_pauses_everything_and_fails_nothing() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.limited.store(true, Ordering::SeqCst);
    let (h, d) = start("limited", &fake, |cfg| {
        cfg.limited_wait = Duration::from_millis(400)
    });
    let wanted: Vec<String> = (0..3).map(clip).collect();
    want(&h, &wanted);
    let ev = until(&h, |ev| limited_events(ev) == 1);
    assert!(
        !ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..))),
        "not a failure: {ev:?}"
    );
    // Within the wait, nothing more starts.
    let tried = fake.started().len();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(fake.started().len(), tried, "waiting");
    // After it, YouTube answers again: everything downloads, and the wait is over.
    fake.limited.store(false, Ordering::SeqCst);
    let ev = until(&h, |ev| done_count(ev) == 3);
    assert!(
        ev.contains(&PreviewEvent::Unlimited {
            source: Source::YouTube
        }),
        "{ev:?}"
    );
    for c in &wanted {
        assert!(d.join(format!("{c}.m4a")).exists());
    }
}

#[test]
fn an_unreachable_youtube_fails_nothing_and_juno_goes_on() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.unreachable.store(true, Ordering::SeqCst);
    fake.answer(
        "The 89th Passenger Paper Wings",
        vec![upload("paperwings1", "Paper Wings", 292.0)],
    );
    let (h, d) = start_with_samples("unreachable", &fake, |cfg| {
        cfg.unreachable_wait = Duration::from_millis(30)
    });
    want(&h, &[clip(1), clip(2), JN.to_owned()]);
    h.send(PreviewCommand::Search(vec![track(
        "track/89th/paper wings",
        "Paper Wings",
        292.0,
    )]));
    // Several tries, each one waited for: still nothing failed or remembered.
    let ev = until(&h, |ev| limited_events(ev) >= 3 && done_count(ev) == 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Done(c, _) if c == JN)),
        "Juno went on"
    );
    assert!(
        !ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..))),
        "{ev:?}"
    );
    assert_eq!(answers(&ev), 0, "neither found nor not found");
    assert!(ev.iter().any(|e| matches!(
        e,
        PreviewEvent::Limited { source: Source::YouTube, reason: Some(r), .. }
            if r.contains("certificate verify failed")
    )));
    // The network is back: everything carries on.
    fake.unreachable.store(false, Ordering::SeqCst);
    let ev = until(&h, |ev| done_count(ev) == 2 && answers(ev) == 1);
    assert!(ev.contains(&PreviewEvent::Unlimited {
        source: Source::YouTube
    }));
    assert!(d.join(format!("{}.m4a", clip(2))).exists());
}

#[test]
fn try_now_ends_the_wait() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.limited.store(true, Ordering::SeqCst);
    // A wait far longer than the test.
    let (h, _) = start("try-now", &fake, |cfg| {
        cfg.limited_wait = Duration::from_secs(600)
    });
    want(&h, &[clip(1)]);
    until(&h, |ev| limited_events(ev) == 1);
    fake.limited.store(false, Ordering::SeqCst);
    h.send(PreviewCommand::TryNow);
    let ev = until(&h, |ev| done_count(ev) == 1);
    assert!(ev.contains(&PreviewEvent::Unlimited {
        source: Source::YouTube
    }));
}

#[test]
fn a_limited_search_is_not_remembered_as_not_found() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.answer(
        "The 89th Passenger Paper Wings",
        vec![upload("paperwings1", "Paper Wings", 292.0)],
    );
    fake.limited.store(true, Ordering::SeqCst);
    let (h, _) = start("limited-search", &fake, |cfg| {
        cfg.limited_wait = Duration::from_secs(600)
    });
    let req = track("track/89th/paper wings", "Paper Wings", 292.0);
    h.send(PreviewCommand::Search(vec![req]));
    let ev = until(&h, |ev| limited_events(ev) == 1);
    assert_eq!(answers(&ev), 0, "neither found nor not found: {ev:?}");
    fake.limited.store(false, Ordering::SeqCst);
    h.send(PreviewCommand::TryNow);
    let ev = until(&h, |ev| answers(ev) == 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Found { clip, .. } if clip == "paperwings1")),
        "searched again after the wait: {ev:?}"
    );
}

#[test]
fn retry_tries_given_up_clips_and_not_found_searches_again() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.fail(&clip(7), 2);
    let (h, d) = start("retry", &fake, |_| {});
    want(&h, &[clip(7)]);
    until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Failed(c, _) if *c == clip(7)))
    });
    // Given up: wanting it again does nothing, until Retry.
    want(&h, &[clip(7)]);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(fake.started().len(), 2);
    h.send(PreviewCommand::Retry {
        clips: vec![clip(7)],
        searches: Vec::new(),
    });
    want(&h, &[]);
    want(&h, &[clip(7)]);
    until(&h, |ev| done_count(ev) == 1);
    assert!(d.join(format!("{}.m4a", clip(7))).exists());

    // Not found, remembered; found once YouTube has it and the search is retried.
    let req = track("track/89th/hidden soul", "Hidden Soul", 300.0);
    h.send(PreviewCommand::Search(vec![req.clone()]));
    until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::NotFound { .. }))
    });
    fake.answer(
        "The 89th Passenger Hidden Soul",
        vec![upload("hiddensoul1", "Hidden Soul", 300.0)],
    );
    h.send(PreviewCommand::Search(Vec::new()));
    h.send(PreviewCommand::Retry {
        clips: Vec::new(),
        searches: vec![req.clone()],
    });
    h.send(PreviewCommand::Search(vec![req]));
    until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Found { clip, .. } if clip == "hiddensoul1"))
    });
}

// ---- Bandcamp -------------------------------------------------------------------------------

const OOZE: &str = "https://analogicalforce.bandcamp.com/track/the-ooze";

fn bc_track(id: &str, title: &str) -> BandcampTrack {
    BandcampTrack {
        track_id: id.into(),
        url: format!(
            "https://analogicalforce.bandcamp.com/track/{}",
            title.to_lowercase()
        ),
        artist: "Patricia".into(),
        title: title.into(),
        duration: Some(400.0),
        streamable: true,
    }
}

fn bc_album(slug: &str, tracks: Vec<BandcampTrack>) -> BandcampAlbum {
    BandcampAlbum {
        url: format!("https://analogicalforce.bandcamp.com/album/{slug}"),
        title: slug.into(),
        catno: String::new(),
        artist: "Patricia".into(),
        cover: String::new(),
        year: None,
        tracks,
    }
}

#[test]
fn a_bandcamp_track_downloads_as_mp3_by_its_page() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let (h, d) = start("bandcamp", &fake, |_| {});
    let key = "bc.3020153053".to_owned();
    h.send(PreviewCommand::Locate(vec![(key.clone(), OOZE.into())]));
    want(&h, std::slice::from_ref(&key));
    until(&h, |ev| done_count(ev) == 1);
    assert!(d.join("bc.3020153053.mp3").exists());
    // A Bandcamp key nobody located is given up on, never guessed.
    want(&h, &["bc.42".to_owned()]);
    until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Failed(c, _) if c == "bc.42"))
    });
}

#[test]
fn a_label_page_is_read_album_by_album() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let label = bandcamp::parse("https://analogicalforce.bandcamp.com/").unwrap();
    let a = "https://analogicalforce.bandcamp.com/album/a";
    let b = "https://analogicalforce.bandcamp.com/album/b";
    fake.page(&label.url(), Listing::Albums(vec![a.into(), b.into()]));
    fake.page(a, Listing::Album(bc_album("a", vec![bc_track("1", "One")])));
    // `b` isn't there: it fails, and the rest goes on.
    let (h, _) = start("bandcamp-read", &fake, |cfg| {
        cfg.read_gap = Duration::from_millis(10)
    });
    h.send(PreviewCommand::ReadBandcamp {
        job: 7,
        page: label.clone(),
        skip: Vec::new(),
        only: Vec::new(),
    });
    let ev = until(&h, |ev| ev.contains(&PreviewEvent::BandcampDone { job: 7 }));
    assert!(ev.contains(&PreviewEvent::BandcampListed { job: 7, albums: 2 }));
    assert!(ev.iter().any(
        |e| matches!(e, PreviewEvent::BandcampAlbum { job: 7, album } if album.tracks.len() == 1)
    ));
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::BandcampFailed { job: 7, page, .. } if page == b))
    );
    assert_eq!(
        fake.pages_read().len(),
        3,
        "the label, then each album once"
    );
    // A refresh leaves out what was read before.
    h.send(PreviewCommand::ReadBandcamp {
        job: 8,
        page: label,
        skip: vec![a.into()],
        only: Vec::new(),
    });
    let ev = until(&h, |ev| ev.contains(&PreviewEvent::BandcampDone { job: 8 }));
    assert!(ev.contains(&PreviewEvent::BandcampListed { job: 8, albums: 1 }));
    assert_eq!(fake.pages_read().len(), 5, "the label and `b` only");
    // Narrowed to a catalogue number no address holds: nothing is read.
    h.send(PreviewCommand::ReadBandcamp {
        job: 9,
        page: bandcamp::parse("https://analogicalforce.bandcamp.com/").unwrap(),
        skip: Vec::new(),
        only: vec![bandcamp::squash("TOBAS 005")],
    });
    let ev = until(&h, |ev| ev.contains(&PreviewEvent::BandcampDone { job: 9 }));
    assert!(
        ev.contains(&PreviewEvent::BandcampListed { job: 9, albums: 0 }),
        "{ev:?}"
    );
}

#[test]
fn a_fan_is_read_then_the_items_pages() {
    use dig::bandcamp::fan::{self, FanItem, FanList, FanLists, ItemKind};
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let a = "https://analogicalforce.bandcamp.com/album/a";
    let b = "https://analogicalforce.bandcamp.com/album/b";
    fake.page(a, Listing::Album(bc_album("a", vec![bc_track("1", "One")])));
    fake.page(b, Listing::Album(bc_album("b", vec![bc_track("2", "Two")])));
    let item = |key: &str, page: &str| FanItem {
        key: key.into(),
        page: page.into(),
        artist: "Patricia".into(),
        title: key.into(),
        catno: String::new(),
        kind: ItemKind::Digital,
    };
    fake.fan(
        "toast3r",
        FanLists {
            fan_id: 1,
            collection: vec![item("a1", a), item("a2", b)],
            wishlist: vec![item("a3", a)],
        },
    );
    let (h, _) = start("bandcamp-fan", &fake, |cfg| {
        cfg.read_gap = Duration::from_millis(10)
    });
    h.send(PreviewCommand::ReadFan {
        job: 3,
        page: fan::parse("https://bandcamp.com/toast3r/wishlist").unwrap(),
    });
    let ev = until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::FanRead { job: 3, .. }))
    });
    assert!(ev.contains(&PreviewEvent::FanProgress {
        job: 3,
        list: FanList::Collection,
        done: 2,
        total: 2
    }));
    let Some(PreviewEvent::FanRead {
        result: Ok(lists), ..
    }) = ev
        .iter()
        .find(|e| matches!(e, PreviewEvent::FanRead { .. }))
    else {
        panic!("{ev:?}");
    };
    assert_eq!((lists.collection.len(), lists.wishlist.len()), (2, 1));
    // Its items' pages, as one job.
    let pages: Vec<_> = [a, b].iter().map(|u| bandcamp::parse(u).unwrap()).collect();
    h.send(PreviewCommand::ReadBandcampPages { job: 4, pages });
    let ev = until(&h, |ev| ev.contains(&PreviewEvent::BandcampDone { job: 4 }));
    assert!(ev.contains(&PreviewEvent::BandcampListed { job: 4, albums: 2 }));
    let albums = ev
        .iter()
        .filter(|e| matches!(e, PreviewEvent::BandcampAlbum { job: 4, .. }))
        .count();
    assert_eq!(albums, 2);
    // An unknown fan fails; a stopped job drops the pages it hadn't read.
    h.send(PreviewCommand::ReadFan {
        job: 5,
        page: fan::parse("https://bandcamp.com/nobody").unwrap(),
    });
    let ev = until(&h, |ev| {
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::FanRead { job: 5, .. }))
    });
    assert!(ev.iter().any(|e| matches!(
        e,
        PreviewEvent::FanRead {
            job: 5,
            result: Err(_)
        }
    )));
    let pages: Vec<_> = (0..20).map(|_| bandcamp::parse(a).unwrap()).collect();
    h.send(PreviewCommand::ReadBandcampPages { job: 6, pages });
    h.send(PreviewCommand::StopBandcamp(6));
    let ev = until(&h, |ev| ev.contains(&PreviewEvent::BandcampDone { job: 6 }));
    let read = ev
        .iter()
        .filter(|e| matches!(e, PreviewEvent::BandcampAlbum { job: 6, .. }))
        .count();
    assert!(read <= 1, "{read} read after the stop");
}

#[test]
fn a_limited_bandcamp_waits_while_youtube_goes_on() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.bandcamp_limited.store(true, Ordering::SeqCst);
    let (h, d) = start("bandcamp-limited", &fake, |cfg| {
        cfg.limited_wait = Duration::from_secs(600)
    });
    let bc = "bc.3020153053".to_owned();
    h.send(PreviewCommand::Locate(vec![(bc.clone(), OOZE.into())]));
    want(&h, &[bc.clone(), clip(1)]);
    let ev = until(&h, |ev| limited_events(ev) == 1 && done_count(ev) == 1);
    assert!(ev.iter().any(|e| matches!(
        e,
        PreviewEvent::Limited {
            source: Source::Bandcamp,
            ..
        }
    )));
    assert!(
        d.join(format!("{}.m4a", clip(1))).exists(),
        "YouTube went on"
    );
    assert!(!ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..))));
    // A page read waits too, then goes through after Try now.
    let page = bandcamp::parse(OOZE).unwrap();
    fake.page(
        OOZE,
        Listing::Album(bc_album("x", vec![bc_track("3020153053", "Ooze")])),
    );
    h.send(PreviewCommand::ReadBandcamp {
        job: 1,
        page,
        skip: Vec::new(),
        only: Vec::new(),
    });
    fake.bandcamp_limited.store(false, Ordering::SeqCst);
    h.send(PreviewCommand::TryNow);
    let ev = until(&h, |ev| {
        done_count(ev) == 1 && ev.contains(&PreviewEvent::BandcampDone { job: 1 })
    });
    assert!(ev.contains(&PreviewEvent::Unlimited {
        source: Source::Bandcamp
    }));
    assert!(d.join("bc.3020153053.mp3").exists());
}

const JN: &str = "jn.1178909-01-01-01";

/// Like [`start`], with the fake downloading Juno samples too.
fn start_with_samples(
    name: &str,
    fake: &Arc<FakeFetcher>,
    tweak: impl FnOnce(&mut Config),
) -> (PreviewHandle, TestDir) {
    let f = fake.clone();
    start(name, fake, move |cfg| {
        cfg.samples = f as Arc<dyn SampleFetcher>;
        tweak(cfg);
    })
}

#[test]
fn a_juno_sample_downloads_without_yt_dlp() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.available.store(false, Ordering::SeqCst);
    let (h, d) = start_with_samples("juno", &fake, |_| {});
    want(&h, &[JN.to_owned(), clip(1)]);
    let ev = until(&h, |ev| {
        done_count(ev) == 1 && ev.contains(&PreviewEvent::NeedsYtDlp)
    });
    assert!(
        ev.iter()
            .any(|e| matches!(e, PreviewEvent::Done(c, _) if c == JN))
    );
    assert!(d.join(format!("{JN}.mp3")).exists());
    assert!(
        !d.join(format!("{}.m4a", clip(1))).exists(),
        "the clip waits for yt-dlp"
    );
}

const BP: &str = "bp.b776a31b-cb84-477f-a7bb-0476c5a1ee8e";

#[test]
fn a_refusing_beatport_waits_while_juno_and_youtube_go_on() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.beatport_limited.store(true, Ordering::SeqCst);
    let (h, d) = start_with_samples("beatport-limited", &fake, |cfg| {
        cfg.limited_wait = Duration::from_secs(600)
    });
    want(&h, &[BP.to_owned(), JN.to_owned(), clip(1)]);
    let ev = until(&h, |ev| limited_events(ev) == 1 && done_count(ev) == 2);
    assert!(ev.iter().any(|e| matches!(
        e,
        PreviewEvent::Limited {
            source: Source::Beatport,
            ..
        }
    )));
    assert!(d.join(format!("{JN}.mp3")).exists(), "Juno went on");
    assert!(!ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..))));
    fake.beatport_limited.store(false, Ordering::SeqCst);
    h.send(PreviewCommand::TryNow);
    let ev = until(&h, |ev| done_count(ev) == 1);
    assert!(ev.contains(&PreviewEvent::Unlimited {
        source: Source::Beatport
    }));
    assert!(d.join(format!("{BP}.mp3")).exists());
}

#[test]
fn a_refusing_juno_waits_while_youtube_goes_on() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.juno_limited.store(true, Ordering::SeqCst);
    let (h, d) = start_with_samples("juno-limited", &fake, |cfg| {
        cfg.limited_wait = Duration::from_secs(600)
    });
    want(&h, &[JN.to_owned(), clip(1)]);
    let ev = until(&h, |ev| limited_events(ev) == 1 && done_count(ev) == 1);
    assert!(ev.iter().any(|e| matches!(
        e,
        PreviewEvent::Limited {
            source: Source::Juno,
            ..
        }
    )));
    assert!(
        d.join(format!("{}.m4a", clip(1))).exists(),
        "YouTube went on"
    );
    assert!(!ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..))));
    fake.juno_limited.store(false, Ordering::SeqCst);
    h.send(PreviewCommand::TryNow);
    let ev = until(&h, |ev| done_count(ev) == 1);
    assert!(ev.contains(&PreviewEvent::Unlimited {
        source: Source::Juno
    }));
}

#[test]
fn a_missing_juno_sample_fails_after_two_tries() {
    let fake = Arc::new(FakeFetcher::new(fixture()));
    fake.fail(JN, u32::MAX);
    let (h, _) = start_with_samples("juno-gone", &fake, |_| {});
    want(&h, &[JN.to_owned()]);
    let ev = until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::Failed(..)))
    });
    assert!(ev.contains(&PreviewEvent::Failed(JN.into(), "clip failed".into())));
    assert_eq!(fake.started(), [JN, JN]);
}

// ---- mixes ----------------------------------------------------------------------------------

#[test]
fn a_mix_is_read_off_the_worker_and_nothing_is_downloaded() {
    use dig::tracklist::{MixError, MixSource};
    let fake = Arc::new(FakeFetcher::new(fixture()));
    let mix = MixSource::YouTube("dQw4w9WgXcQ".into());
    fake.mix(
        &mix.url(),
        r#"{"title": "Deep House Mix", "description": "00:00 A - B\n05:00 C - D\n10:00 ID - ID"}"#,
    );
    let (h, d) = start("mix-read", &fake, |_| {});
    h.send(PreviewCommand::ReadMix(mix.clone()));
    let ev = until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::MixRead { .. }))
    });
    let Some(PreviewEvent::MixRead { source, result }) = ev
        .into_iter()
        .find(|e| matches!(e, PreviewEvent::MixRead { .. }))
    else {
        unreachable!()
    };
    assert_eq!(source, mix);
    let list = result.unwrap();
    assert_eq!(
        (list.title.as_str(), list.tracks.len()),
        ("Deep House Mix", 3)
    );
    assert!(list.tracks[2].is_id);
    assert_eq!(
        fake.pages_read(),
        [mix.url()],
        "one read of the mix's own address"
    );
    assert!(fake.started().is_empty(), "nothing downloaded");
    assert_eq!(
        std::fs::read_dir(&*d)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "m4a"))
            .count(),
        0
    );
    // A video that can't be read says why.
    let gone = MixSource::YouTube("aaaaaaaaaaa".into());
    h.send(PreviewCommand::ReadMix(gone));
    let ev = until(&h, |ev| {
        ev.iter().any(|e| matches!(e, PreviewEvent::MixRead { .. }))
    });
    assert!(ev.iter().any(|e| matches!(e,
        PreviewEvent::MixRead { result: Err(MixError::Failed(why)), .. } if why.contains("unavailable"))));
}

/// The real yt-dlp reading a YouTube and a SoundCloud mix: run by hand (`cargo test -p dig
/// --test previews -- --ignored --nocapture`), it needs yt-dlp and the network.
#[test]
#[ignore]
fn real_mixes_are_read_into_tracklists() {
    use dig::tracklist::{from_ytdlp, parse_url};
    let (ytdlp, version) = dig::preview::fetcher::find(None).expect("yt-dlp on this machine");
    println!("yt-dlp {version}");
    for url in [
        "https://www.youtube.com/watch?v=1nQzXzBvcWk",
        "https://soundcloud.com/benedictquinn/discodeeplofielectrohousemix?si=x",
    ] {
        let source = parse_url(url).unwrap();
        let json = ytdlp.read_mix(&source).expect("read");
        let list = from_ytdlp(source, &json).expect("a tracklist");
        println!(
            "{}: {} tracks, first {:?}",
            list.title,
            list.tracks.len(),
            list.tracks[0]
        );
        assert!(list.tracks.len() >= 10, "{url}");
    }
}
