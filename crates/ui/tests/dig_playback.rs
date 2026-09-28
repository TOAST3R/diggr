//! Spec (discogs-digging, "Digging never disturbs playback"): an 800-release label expands, and
//! previews are downloaded and prepared, while the engine plays, with zero underruns; a prepared
//! preview then starts within the fast-start budget (30 ms).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio::{Engine, EngineConfig, PlayState, RepeatMode};
use dig::clock::FakeClock;
use dig::discogs::cache::DiskCache;
use dig::discogs::client::Client;
use dig::discogs::transport::FakeTransport;
use dig::discogs::url;
use dig::intake::{Command, Event, Intake, IntakeHandle, Outcome};
use dig::jobs::Filters;
use dig::prepare::PrepareHandle;
use dig::preview::fetcher::{FakeFetcher, Fetcher};
use dig::preview::scheduler::{Config, Finder, PreviewCommand, PreviewEvent, PreviewHandle};
use platform::native::{NativeFileSource, NativeSpawner};
use platform::testing::{ManualSink, TestDir};
use platform::{CallbackInfo, FileSource, TrackRef};

const LABEL: u64 = 9000;
const FIRST: u64 = 50_000;
const RELEASES: u64 = 800;
const PER_PAGE: u64 = 100;
/// Previews downloaded and prepared (the horizon: the playing one and the next 3).
const PREVIEWS: usize = 4;
const START_BUDGET_MS: f64 = 30.0;

fn fixture(name: &str) -> String {
    format!(
        "{}/../audio/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn clip(id: u64) -> String {
    format!("clip{id:07}")
}

/// A label of 800 vinyl releases, one track and one clip each.
fn big_label(t: &FakeTransport) {
    t.route(
        format!("/labels/{LABEL}"),
        200,
        format!(r#"{{"id": {LABEL}, "name": "Deep Crates"}}"#),
    );
    let pages = RELEASES / PER_PAGE;
    for page in 1..=pages {
        let items: Vec<String> = (0..PER_PAGE)
            .map(|i| {
                let id = FIRST + (page - 1) * PER_PAGE + i;
                format!(
                    r#"{{"id": {id}, "status": "Accepted", "format": "12\"", "catno": "DC-{id}", "artist": "Crew", "title": "Record {id}", "year": 2001, "resource_url": "https://api.discogs.com/releases/{id}"}}"#
                )
            })
            .collect();
        t.route(
            format!("/labels/{LABEL}/releases?page={page}&per_page={PER_PAGE}"),
            200,
            format!(
                r#"{{"pagination": {{"page": {page}, "pages": {pages}, "per_page": {PER_PAGE}, "items": {RELEASES}}}, "releases": [{}]}}"#,
                items.join(",")
            ),
        );
    }
    for id in FIRST..FIRST + RELEASES {
        t.route(
            format!("/releases/{id}?curr_abbr=USD"),
            200,
            format!(
                r#"{{"id": {id}, "title": "Record {id}", "year": 2001,
                "artists": [{{"name": "Crew", "anv": "", "join": "", "id": 1}}],
                "labels": [{{"name": "Deep Crates", "catno": "DC-{id}", "id": {LABEL}}}],
                "formats": [{{"name": "Vinyl", "qty": "1", "descriptions": ["12\""]}}],
                "master_id": 0,
                "tracklist": [{{"position": "A", "type_": "track", "title": "Cut {id}", "duration": "2:00"}}],
                "videos": [{{"uri": "https://www.youtube.com/watch?v={}", "title": "Crew - Cut {id}", "duration": 120, "embed": true}}],
                "num_for_sale": 3, "lowest_price": 8.0}}"#,
                clip(id)
            ),
        );
    }
}

/// One device buffer at the real-time pace, with the sink's clock following the wall clock.
struct Device {
    sink: ManualSink,
    start: Instant,
}

impl Device {
    fn pump(&self, frames: usize) {
        let now_ns = self.start.elapsed().as_nanos() as u64;
        self.sink.set_now_ns(now_ns);
        self.sink.pull(
            frames,
            CallbackInfo {
                host_ns: now_ns,
                output_latency_ns: 0,
            },
        );
        std::thread::sleep(Duration::from_micros(frames as u64 * 1_000_000 / 48_000));
    }
}

#[test]
fn an_800_release_label_is_dug_while_playback_runs_without_underruns() {
    let dir = TestDir::new("ui-dig-playback");
    let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);

    let sink = ManualSink::new(48_000, 2);
    let device = Device {
        sink: sink.clone(),
        start: Instant::now(),
    };
    let mut engine = Engine::new(
        Arc::new(sink),
        &NativeSpawner,
        files.clone(),
        EngineConfig::default(),
    )
    .unwrap();
    engine.set_repeat(RepeatMode::One);
    engine.set_queue(vec![TrackRef::new(fixture("tone.flac"))]);
    engine.play_index(0);

    let transport = Arc::new(FakeTransport::new());
    big_label(&transport);
    // Rate limiting is real, but on a clock that moves when the worker waits.
    let client = Client::new(
        transport.clone(),
        Arc::new(FakeClock::default()),
        None,
        DiskCache::new(Some(&dir.join("cache"))),
    );
    let intake = IntakeHandle::start(&NativeSpawner, Intake::new(client, None), || {}).unwrap();
    intake.send(Command::Send {
        page: url::parse(&format!("https://www.discogs.com/label/{LABEL}")).unwrap(),
        target: 1,
        filters: Filters::default(),
    });
    let fetcher = Arc::new(FakeFetcher::new(fixture("tone.m4a")));
    let f = fetcher.clone();
    let finder: Finder = Arc::new(move |_| {
        let v = f.version()?;
        Some((f.clone() as Arc<dyn Fetcher>, v))
    });
    let previews = PreviewHandle::start(
        Arc::new(NativeSpawner),
        Config::new(dir.join("cache/previews"), 1_000_000_000, None),
        finder,
        || {},
    )
    .unwrap();
    let prepare = PrepareHandle::start(&NativeSpawner, files, dir.join("cache"), || {}).unwrap();
    prepare.set_gate(true);

    let mut clips: Vec<String> = Vec::new();
    let mut records = 0u64;
    let mut finished = false;
    let mut downloaded: Vec<TrackRef> = Vec::new();
    let mut prepared: HashSet<TrackRef> = HashSet::new();
    let deadline = Instant::now() + Duration::from_secs(120);
    while !(finished && prepared.len() == PREVIEWS) {
        assert!(
            Instant::now() < deadline,
            "stalled: {records} records, {} prepared",
            prepared.len()
        );
        device.pump(512);
        for e in intake.poll() {
            match e {
                Event::Record(_, _, Outcome::Clips(c)) => {
                    records += 1;
                    let before = clips.len();
                    clips.extend(c.into_iter().map(|c| c.clip));
                    if before < PREVIEWS && clips.len() >= PREVIEWS {
                        let wanted = clips[..PREVIEWS].to_vec();
                        previews.send(PreviewCommand::Want {
                            wanted: wanted.clone(),
                            protected: wanted,
                        });
                    }
                }
                Event::Record(..) => records += 1,
                Event::Finished(_) => finished = true,
                Event::Failed(_, e) => panic!("the send failed: {e:?}"),
                _ => {}
            }
        }
        let mut more = false;
        for e in previews.poll() {
            if let PreviewEvent::Done(_, path) = e {
                downloaded.push(TrackRef::new(path.to_string_lossy()));
                more = true;
            }
        }
        if more {
            prepare.prepare(downloaded.clone());
        }
        prepared.extend(prepare.poll());
    }
    assert_eq!(records, RELEASES, "every release expanded");
    assert_eq!(clips.len(), RELEASES as usize);
    assert_eq!(engine.state(), PlayState::Playing);
    assert_eq!(engine.stats().underruns, 0, "digging disturbed playback");

    // A prepared preview starts as fast as a local file: within the fast-start budget. Small
    // device buffers here, so the measurement is the engine's and not the fake device's 11 ms
    // quantum. Unoptimized (`audio` is a workspace crate, built at opt-level 0 in dev), a start
    // takes about the budget itself, so there the preview is held to the local file's time.
    let mut start = |track: TrackRef| {
        engine.set_queue(vec![track]);
        engine.play_index(0);
        let t = Instant::now();
        while !(engine.position().state == PlayState::Playing && engine.position().frame > 0) {
            assert!(t.elapsed() < Duration::from_secs(5), "didn't start");
            device.pump(128);
        }
        engine.stats().last_start_latency_ms
    };
    let local = start(TrackRef::new(fixture("tone.m4a")));
    let preview = start(downloaded[0].clone());
    eprintln!("start: preview {preview:.1} ms, local file {local:.1} ms");
    if cfg!(debug_assertions) {
        assert!(
            preview < local + 5.0,
            "a prepared preview took {preview:.1} ms to start, a local file {local:.1} ms"
        );
    } else {
        assert!(
            preview < START_BUDGET_MS,
            "a prepared preview took {preview:.1} ms to start"
        );
    }
    assert_eq!(engine.stats().underruns, 0);
}

/// Design D8's measurement: prepare one long preview (`DIG_PREPARE_FILE`, e.g. a 6-minute
/// M4A) while another track plays. Run optimized:
/// `DIG_PREPARE_FILE=… cargo test --release -p ui --test dig_playback -- --ignored --nocapture`
#[test]
#[ignore]
fn preparing_a_long_preview_while_a_track_plays() {
    let Ok(file) = std::env::var("DIG_PREPARE_FILE") else {
        eprintln!("DIG_PREPARE_FILE isn't set");
        return;
    };
    let dir = TestDir::new("ui-dig-prepare");
    let preview = dir.join("preview.m4a");
    std::fs::copy(&file, &preview).unwrap();
    let files: Arc<dyn FileSource> = Arc::new(NativeFileSource);
    let sink = ManualSink::new(48_000, 2);
    let device = Device {
        sink: sink.clone(),
        start: Instant::now(),
    };
    let mut engine = Engine::new(
        Arc::new(sink),
        &NativeSpawner,
        files.clone(),
        EngineConfig::default(),
    )
    .unwrap();
    engine.set_repeat(RepeatMode::One);
    engine.set_queue(vec![TrackRef::new(fixture("tone.flac"))]);
    engine.play_index(0);
    let prepare = PrepareHandle::start(&NativeSpawner, files, dir.join("cache"), || {}).unwrap();
    prepare.set_gate(true);
    let t = Instant::now();
    prepare.prepare(vec![TrackRef::new(preview.to_string_lossy())]);
    while prepare.poll().is_empty() {
        assert!(t.elapsed() < Duration::from_secs(300), "stalled");
        device.pump(512);
    }
    let secs = t.elapsed().as_secs_f64();
    eprintln!(
        "prepared in {secs:.1} s; underruns {}",
        engine.stats().underruns
    );
    assert_eq!(engine.stats().underruns, 0);
}
