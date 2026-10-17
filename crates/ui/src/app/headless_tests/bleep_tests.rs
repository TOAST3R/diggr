//! Bleep, headless: a release page's tracks and a list's records sent by the extension, the
//! tracks searched for, a list record standing as one entry until a Discogs link brings its
//! release's tracks, the price note and Open on Bleep; against a fake Discogs and fake
//! previews.

use super::*;
use crate::playlist::{EntryStatus, WaitKind};
use ::dig::bridge::{BridgeCommand, Mode};
use ::dig::browser::FakeBrowser;
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::search::{Wanted, path};
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
    /// Discogs has exactly one "Nightcraft - Glasshouse EP" (release 1001, in the fixtures).
    fn new() -> Self {
        let transport = Arc::new(FakeTransport::with_fixtures(format!(
            "{}/../dig/tests/fixtures/discogs",
            env!("CARGO_MANIFEST_DIR")
        )));
        transport.route(
            path(&Wanted::new("Nightcraft", "Glasshouse EP", "LT-012", true)),
            200,
            r#"{"results":[{"id":1001,"title":"Nightcraft - Glasshouse EP","format":["Vinyl"],"catno":"LT-012"}]}"#,
        );
        transport.route("/cart", 200, "[]");
        transport.route(
            "/purchases?sort=created&sort_order=desc&page=1&per_page=100",
            200,
            r#"{"items": []}"#,
        );
        Self {
            fetcher: Arc::new(FakeFetcher::new(fixture("tone.m4a"))),
            browser: Arc::new(FakeBrowser::default()),
            transport,
        }
    }
}

fn rig(name: &str, fakes: &Fakes, token: bool) -> Rig {
    let f = fakes.clone();
    Rig::with_dig(
        name,
        Vec::new(),
        move |store| {
            if token {
                ::dig::config::save_token(store.dir(), Some("tok")).unwrap();
            }
        },
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

/// Release 100168 as its page shows it.
fn zen_drums() -> Value {
    json!({
        "id": "100168", "slug": "bibio-zen-drumsdada-drums", "artist": "Bibio",
        "title": "Zen Drums/Dada Drums", "label": "Warp Records", "catno": "WAP419",
        "image": "100168.jpg", "formats": [{ "name": "Download", "price": "2.49 €" }],
        "tracks": [
            { "id": "635821", "n": 1, "title": "Zen Drums", "secs": 397 },
            { "id": "635822", "n": 2, "title": "Dada Drums", "secs": 301 },
        ],
    })
}

/// A list's tile: no tracks.
fn glasshouse_tile() -> Value {
    json!({
        "id": "509246", "slug": "nightcraft-glasshouse-ep", "artist": "Nightcraft",
        "title": "Glasshouse EP", "label": "Lowtide Tapes", "catno": "LT-012",
        "image": "509246-1.jpg", "formats": [{ "name": "LP" }, { "name": "Download" }],
    })
}

fn records(records: Vec<Value>) -> Vec<StoreRecord> {
    let send = json!({ "page": "https://bleep.com/label/6593-warp-records", "records": records });
    ::dig::bleep::check(&send).unwrap().records
}

fn send(rig: &mut Rig, records: Vec<StoreRecord>) {
    rig.app.dig_bridge_command(BridgeCommand::SendStore {
        records,
        mode: Mode::Enqueue,
        filters: Filters { skip_passed: true },
    });
}

fn says(rig: &Rig, i: usize, label: &str) -> String {
    let e = &rig.app.crates.get(PLAYLIST).unwrap().entries()[i];
    crate::format::entry_details(e, rig.app.dig_marks(e), 0)
        .into_iter()
        .find(|(l, _)| *l == label)
        .map(|(_, v)| v)
        .unwrap_or_default()
}

#[test]
fn a_bleep_release_comes_in_searched_with_its_note_and_page() {
    let fakes = Fakes::new();
    fakes.fetcher.answer(
        "Bibio Zen Drums",
        vec![SearchResult {
            id: "ZENzenZENze".into(),
            duration: Some(397.0),
            channel: "Bibio".into(),
            title: "Bibio - Zen Drums".into(),
        }],
    );
    let mut rig = rig("bleep-release", &fakes, false);
    send(&mut rig, records(vec![zen_drums()]));
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    let rows: Vec<(&str, &str)> = p
        .entries()
        .iter()
        .map(|e| {
            (
                e.title.as_str(),
                e.origin.as_ref().unwrap().position.as_str(),
            )
        })
        .collect();
    assert_eq!(rows, [("Zen Drums", "1"), ("Dada Drums", "2")]);
    assert_eq!(
        p.entries()[0].status,
        EntryStatus::Waiting(WaitKind::Search)
    );
    assert_eq!(p.entries()[0].origin.as_ref().unwrap().catno, "WAP419");
    assert!(
        says(&rig, 0, "Bleep").starts_with("Download 2.49 € (seen "),
        "{}",
        says(&rig, 0, "Bleep")
    );
    rig.until(
        |r| {
            r.app.crates.get(PLAYLIST).unwrap().entries()[0]
                .status
                .is_playable()
        },
        "found and downloaded",
    );
    assert_eq!(says(&rig, 0, "Source"), "YouTube (found for Bleep)");
    // Open on Bleep, and pasted Bleep addresses are refused.
    let id = rig.app.crates.get(PLAYLIST).unwrap().entries()[1].id;
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::Dig(DigAction::OpenStore(id, Store::Bleep)), &ctx);
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://bleep.com/release/100168-bibio-zen-drumsdada-drums"]
    );
    for text in [
        "https://bleep.com/release/100168-bibio-zen-drumsdada-drums",
        "https://bleep.co.uk/label/4035-rubadub",
    ] {
        rig.app.dig_paste(text);
        let m = rig.app.message.as_ref().map(|(m, _)| m.clone()).unwrap();
        assert_eq!(m, "Bleep pages are sent from the browser extension");
    }
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().len(), 2);
}

#[test]
fn a_list_record_stands_as_one_entry_until_discogs_brings_its_tracks() {
    let fakes = Fakes::new();
    let mut rig = rig("bleep-list", &fakes, true);
    send(&mut rig, records(vec![glasshouse_tile()]));
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    // At first: one entry for the whole record, searched by its title.
    let e = p.entries().first().expect("the record's entry");
    assert_eq!(
        (e.artist.as_str(), e.title.as_str()),
        ("Nightcraft", "Glasshouse EP")
    );
    assert!(e.origin.as_ref().unwrap().position.is_empty());
    // Then exactly one release fits: its tracks come in and the standing entry goes.
    rig.until(
        |r| {
            let titles: Vec<String> = r
                .app
                .crates
                .get(PLAYLIST)
                .unwrap()
                .entries()
                .iter()
                .map(|e| e.title.clone())
                .collect();
            titles.contains(&"Last Light".to_owned())
                && !titles.contains(&"Glasshouse EP".to_owned())
        },
        "the release's tracks replace the record's entry",
    );
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    assert!(
        p.entries()
            .iter()
            .all(|e| e.origin.as_ref().unwrap().release == Some(1001))
    );
}

#[test]
fn without_a_token_a_list_record_stays_one_entry() {
    let fakes = Fakes::new();
    let mut rig = rig("bleep-list-no-token", &fakes, false);
    send(&mut rig, records(vec![glasshouse_tile()]));
    for _ in 0..20 {
        rig.pump();
    }
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p.entries()[0].title, "Glasshouse EP");
    assert!(
        !fakes
            .transport
            .paths()
            .iter()
            .any(|p| p.starts_with("/database/search")),
        "nothing asked of Discogs"
    );
}

#[test]
fn five_hundred_bleep_records_go_in_quickly() {
    let fakes = Fakes::new();
    let mut rig = rig("bleep-big", &fakes, false);
    let tiles: Vec<Value> = (0..500)
        .map(|i| {
            let mut t = glasshouse_tile();
            t["id"] = json!((600_000 + i).to_string());
            t["title"] = json!(format!("Record {i}"));
            t
        })
        .collect();
    let records = records(tiles);
    let started = Instant::now();
    send(&mut rig, records);
    assert!(
        started.elapsed() < Duration::from_millis(50),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().len(), 500);
}
