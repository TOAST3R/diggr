//! Beatport and Traxsource, headless: tracks sent from their pages by the extension, merged
//! into crates with the store's BPM and key, Beatport samples played, Traxsource tracks
//! searched for, the price note and Open on ‹store›; against a fake Discogs and fake
//! previews.

use super::*;
use crate::playlist::{ClipSource, EntryStatus, UnavailableKind, WaitKind};
use ::dig::bridge::{BridgeCommand, Mode};
use ::dig::browser::FakeBrowser;
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::transport::FakeTransport;
use ::dig::jobs::Filters;
use ::dig::preview::fetcher::{FakeFetcher, Fetcher};
use ::dig::preview::scheduler::Finder;
use ::dig::preview::search::SearchResult;
use ::dig::stores::{Store, StoreRecord};
use serde_json::{Value, json};

#[derive(Clone)]
struct Fakes {
    fetcher: Arc<FakeFetcher>,
    browser: Arc<FakeBrowser>,
    transport: Arc<FakeTransport>,
}

impl Fakes {
    fn new() -> Self {
        let transport = Arc::new(FakeTransport::with_fixtures(format!(
            "{}/../dig/tests/fixtures/discogs",
            env!("CARGO_MANIFEST_DIR")
        )));
        transport.route("/cart", 200, "[]");
        Self {
            fetcher: Arc::new(FakeFetcher::new(fixture("tone.m4a"))),
            browser: Arc::new(FakeBrowser::default()),
            transport,
        }
    }
}

fn rig(name: &str, fakes: &Fakes) -> Rig {
    let f = fakes.clone();
    Rig::with_dig(
        name,
        Vec::new(),
        |_| {},
        move |dir| {
            let fetcher = f.fetcher.clone();
            let finder: Finder = Arc::new(move |_| {
                let v = fetcher.version()?;
                Some((fetcher.clone() as Arc<dyn Fetcher>, v))
            });
            Some(DigSetup {
                transport: f.transport.clone(),
                images: FakeImages::new(Ok(test_jpeg(300, 300))),
                clock: Arc::new(RealClock::default()),
                finder,
                samples: f.fetcher.clone(),
                browser: f.browser.clone(),
                cache_root: Some(dir.join("cache")),
                bridge: BridgeSetup::Off,
                tools: None,
            })
        },
    )
}

const UUIDS: [&str; 3] = [
    "b776a31b-cb84-477f-a7bb-0476c5a1ee8e",
    "eca17004-de79-427b-ad80-08d7ab295e7c",
    "0f1e2d3c-4b5a-6978-8a9b-0c1d2e3f4a5b",
];

fn beatport(id: u32, title: &str, bpm: u16, key: &str, sample: &str) -> Value {
    json!({
        "id": id.to_string(), "slug": "input", "artists": ["Martin Cantrill"], "title": title,
        "mix": "Original Mix", "sample": sample, "bpm": bpm, "key": key, "secs": 305,
        "release": "4630549", "release_name": "Dreams Album", "release_slug": "dreams-album",
        "label": "Deepsessions Recordings", "catno": "DSR123", "price": "€1.39",
    })
}

fn beatport_records(tracks: Vec<Value>) -> Vec<StoreRecord> {
    let send = json!({ "page": "https://www.beatport.com/chart/x/1", "tracks": tracks });
    ::dig::beatport::check(&send).unwrap().records
}

fn traxsource_records(tracks: Vec<Value>) -> Vec<StoreRecord> {
    let send = json!({ "page": "https://www.traxsource.com/genre/4/house", "tracks": tracks });
    ::dig::traxsource::check(&send).unwrap().records
}

fn send(rig: &mut Rig, records: Vec<StoreRecord>) {
    rig.app.dig_bridge_command(BridgeCommand::SendStore {
        records,
        mode: Mode::Enqueue,
        filters: Filters { skip_passed: true },
    });
}

fn chart() -> Vec<StoreRecord> {
    beatport_records(vec![
        beatport(1, "Input", 126, "8A", UUIDS[0]),
        beatport(2, "Output", 124, "5A", UUIDS[1]),
        beatport(3, "Loop", 132, "9B", UUIDS[2]),
    ])
}

fn tip(rig: &Rig, i: usize) -> Vec<(&'static str, String)> {
    let e = &rig.app.crates.get(PLAYLIST).unwrap().entries()[i];
    crate::format::entry_details(e, rig.app.dig_marks(e), 0)
}

fn says(tip: &[(&'static str, String)], label: &str) -> String {
    tip.iter()
        .find(|(l, _)| *l == label)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

#[test]
fn a_beatport_chart_comes_in_order_with_bpm_key_and_samples() {
    let fakes = Fakes::new();
    let mut rig = rig("beatport-chart", &fakes);
    send(&mut rig, chart());
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    let titles: Vec<&str> = p.entries().iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Input", "Output", "Loop"], "in chart order");
    // The store's BPM is there before any preview: the filter works at once.
    assert_eq!(p.entries()[1].bpm, Some(124));
    rig.app
        .crates
        .get_mut(PLAYLIST)
        .unwrap()
        .set_bpm_filter(Some((124, 128)));
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().shown_rows().len(), 2);
    rig.app
        .crates
        .get_mut(PLAYLIST)
        .unwrap()
        .set_bpm_filter(None);
    let t = tip(&rig, 0);
    assert_eq!(says(&t, "Tempo"), "126 BPM (from Beatport)");
    assert_eq!(says(&t, "Key"), "8A (A minor)");
    assert_eq!(says(&t, "Source"), "Beatport sample (2 min)");
    assert!(says(&t, "Beatport").starts_with("€1.39 (seen "), "{t:?}");
    assert_eq!(ClipSource::Beatport.badge(), "BP");
    rig.until(
        |r| {
            r.app.crates.get(PLAYLIST).unwrap().entries()[0]
                .status
                .is_playable()
        },
        "the sample plays",
    );
    assert!(
        rig.dir
            .join(format!("cache/previews/bp.{}.mp3", UUIDS[0]))
            .exists()
    );
    // The analyser's tempo takes over.
    let key = rig.app.crates.get(PLAYLIST).unwrap().entries()[0].duplicate_key();
    rig.app.crates.get_mut(PLAYLIST).unwrap().set_bpm(&key, 126);
    assert_eq!(says(&tip(&rig, 0), "Tempo"), "126 BPM");
}

#[test]
fn open_on_beatport_and_pasted_store_addresses() {
    let fakes = Fakes::new();
    let mut rig = rig("beatport-open", &fakes);
    send(&mut rig, chart());
    let id = rig.app.crates.get(PLAYLIST).unwrap().entries()[0].id;
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::Dig(DigAction::OpenStore(id, Store::Beatport)), &ctx);
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://www.beatport.com/track/input/1"]
    );
    for (text, store) in [
        ("https://www.beatport.com/release/x/1", "Beatport"),
        ("https://www.traxsource.com/title/1/x", "Traxsource"),
    ] {
        rig.app.dig_paste(text);
        let m = rig.app.message.as_ref().map(|(m, _)| m.clone()).unwrap();
        assert_eq!(
            m,
            format!("{store} pages are sent from the browser extension")
        );
    }
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().len(), 3);
}

#[test]
fn a_beatport_sample_fixes_a_track_not_found() {
    let fakes = Fakes::new();
    let mut rig = rig("beatport-fix", &fakes);
    let p = rig.app.crates.get_mut(PLAYLIST).unwrap();
    let id = p.add_waiting(
        "Martin Cantrill",
        "Input",
        None,
        Some(Origin {
            catno: "DSR123".into(),
            search_key: "release/1/A1".into(),
            ..Origin::default()
        }),
        WaitKind::Search,
    );
    p.set_unavailable(id, UnavailableKind::NotFound);
    send(&mut rig, chart());
    let m = rig.app.message.as_ref().map(|(m, _)| m.clone()).unwrap();
    assert!(m.ends_with("2 added, 1 fixed, 0 skipped"), "{m}");
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    assert_eq!(p.len(), 3, "no second Input");
    let o = p.get(id).unwrap().origin.clone().unwrap();
    assert_eq!(o.source(), Some(ClipSource::Beatport));
    assert_eq!(o.key, "8A");
}

#[test]
fn a_traxsource_track_is_searched_for_and_keeps_its_store_details() {
    let fakes = Fakes::new();
    fakes.fetcher.answer(
        "Kerri Chandler Rain (Extended Mix)",
        vec![SearchResult {
            id: "RAINrainRAI".into(),
            duration: Some(432.0),
            channel: "Kerri Chandler".into(),
            title: "Kerri Chandler - Rain (Extended Mix)".into(),
        }],
    );
    let mut rig = rig("traxsource-search", &fakes);
    let tx = |id: u32, title: &str| {
        json!({
            "id": id.to_string(), "slug": "rain", "artists": ["Kerri Chandler"], "title": title,
            "mix": "Extended Mix", "bpm": 124, "key": "A min", "secs": 432,
            "label": "Madhouse", "price": "$2.49",
        })
    };
    send(
        &mut rig,
        traxsource_records(vec![tx(12815016, "Rain"), tx(12815017, "Lost Track")]),
    );
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    assert_eq!(
        p.entries()[0].status,
        EntryStatus::Waiting(WaitKind::Search)
    );
    assert_eq!(p.entries()[0].title, "Rain (Extended Mix)");
    assert_eq!(says(&tip(&rig, 0), "Key"), "8A (A minor)");
    rig.until(
        |r| {
            r.app.crates.get(PLAYLIST).unwrap().entries()[0]
                .status
                .is_playable()
        },
        "found and downloaded",
    );
    assert_eq!(
        says(&tip(&rig, 0), "Source"),
        "YouTube (found for Traxsource)"
    );
    // One not found keeps its BPM, key and page.
    let id = rig.app.crates.get(PLAYLIST).unwrap().entries()[1].id;
    rig.app
        .crates
        .get_mut(PLAYLIST)
        .unwrap()
        .set_unavailable(id, UnavailableKind::NotFound);
    let e = rig
        .app
        .crates
        .get(PLAYLIST)
        .unwrap()
        .get(id)
        .unwrap()
        .clone();
    assert_eq!(e.bpm, Some(124));
    assert_eq!(
        rig.app.store_pages(&e),
        [(
            Store::Traxsource,
            "https://www.traxsource.com/track/12815017/rain".to_owned()
        )]
    );
}

#[test]
fn five_hundred_beatport_tracks_go_in_quickly() {
    let fakes = Fakes::new();
    let mut rig = rig("beatport-big", &fakes);
    let tracks: Vec<Value> = (0..500u32)
        .map(|i| {
            let mut t = beatport(i + 1, &format!("Track {i}"), 126, "8A", UUIDS[0]);
            t["release"] = json!((10_000 + i / 10).to_string());
            t
        })
        .collect();
    let records = beatport_records(tracks);
    let started = Instant::now();
    send(&mut rig, records);
    assert!(
        started.elapsed() < Duration::from_millis(50),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().len(), 500);
}

#[test]
fn play_adds_to_the_crate_on_screen_and_plays_the_first_new_track() {
    let fakes = Fakes::new();
    let mut rig = rig("store-play", &fakes);
    send(&mut rig, chart());
    let crates = rig.app.crates.list().len();
    let play = |rig: &mut Rig, records| {
        rig.app.dig_bridge_command(BridgeCommand::SendStore {
            records,
            mode: Mode::Play,
            filters: Filters { skip_passed: true },
        })
    };
    // "Input" is there already: the first track the send adds is "Fourth".
    let fourth = "1a2b3c4d-5e6f-4a1b-8c2d-3e4f5a6b7c8d";
    play(
        &mut rig,
        beatport_records(vec![
            beatport(1, "Input", 126, "8A", UUIDS[0]),
            beatport(4, "Fourth", 128, "8A", fourth),
        ]),
    );
    let playing = |r: &Rig| {
        let p = r.app.crates.get(PLAYLIST).unwrap();
        p.current()
            .and_then(|id| p.get(id))
            .map(|e| e.title.clone())
    };
    rig.until(
        |r| r.app.position.state == PlayState::Playing && playing(r).as_deref() == Some("Fourth"),
        "the new track plays",
    );
    assert_eq!(rig.app.crates.list().len(), crates, "no crate made");
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().entries().len(), 4);
    // A send adding nothing leaves nothing waiting to play.
    play(&mut rig, chart());
    assert!(rig.app.dig.as_ref().unwrap().play_when_ready.is_none());
}
