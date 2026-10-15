//! Juno, headless: records sent from a Juno page by the extension, merged into crates (fix
//! what had no preview, add what's missing), their samples played, linked to Discogs when
//! exactly one release fits, the stock note and Open on Juno; against a fake Discogs and fake
//! previews.

use super::*;
use crate::playlist::{ClipSource, UnavailableKind, WaitKind};
use ::dig::bridge::{BridgeCommand, Mode};
use ::dig::browser::FakeBrowser;
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::search::{Wanted, path};
use ::dig::discogs::transport::FakeTransport;
use ::dig::jobs::Filters;
use ::dig::juno::{JunoRecord, JunoTrack, Stock};
use ::dig::preview::fetcher::{FakeFetcher, Fetcher};
use ::dig::preview::scheduler::Finder;

const PRODUCT: &str = "1178909-01";

#[derive(Clone)]
struct Fakes {
    transport: Arc<FakeTransport>,
    fetcher: Arc<FakeFetcher>,
    browser: Arc<FakeBrowser>,
}

impl Fakes {
    /// Discogs has exactly one "Rontgen (reissue)" (release 123456), two "Aste" and no
    /// "Kiila".
    fn new() -> Self {
        let transport = Arc::new(FakeTransport::with_fixtures(format!(
            "{}/../dig/tests/fixtures/discogs",
            env!("CARGO_MANIFEST_DIR")
        )));
        transport.route(
            path(&Wanted::new("0 aka Mika Vainio", "Rontgen (reissue)", "SAHKO 00012", true)),
            200,
            r#"{"results":[{"id":123456,"title":"0 aka Mika Vainio - Rontgen (reissue)","format":["Vinyl"],"catno":"SAHKO 00012"}]}"#,
        );
        transport.route(
            path(&Wanted::new("0 aka Mika Vainio", "Aste", "SAHKO 00013", true)),
            200,
            r#"{"results":[{"id":7001,"title":"0 aka Mika Vainio - Aste","format":["Vinyl"],"catno":"SAHKO 00013"},
                           {"id":7002,"title":"0 aka Mika Vainio - Aste","format":["Vinyl"],"catno":"SAHKO 00013"}]}"#,
        );
        transport.route(
            path(&Wanted::new(
                "0 aka Mika Vainio",
                "Kiila",
                "SAHKO 00014",
                true,
            )),
            200,
            r#"{"results":[]}"#,
        );
        transport.route("/cart", 200, "[]");
        transport.route(
            "/purchases?sort=created&sort_order=desc&page=1&per_page=100",
            200,
            r#"{"items": []}"#,
        );
        Self {
            transport,
            fetcher: Arc::new(FakeFetcher::new(fixture("tone.m4a"))),
            browser: Arc::new(FakeBrowser::default()),
        }
    }

    fn setup(&self, dir: &Path) -> DigSetup {
        let f = self.fetcher.clone();
        let finder: Finder = Arc::new(move |_| {
            let v = f.version()?;
            Some((f.clone() as Arc<dyn Fetcher>, v))
        });
        DigSetup {
            transport: self.transport.clone(),
            images: FakeImages::new(Ok(test_jpeg(300, 300))),
            clock: Arc::new(RealClock::default()),
            finder,
            samples: self.fetcher.clone(),
            browser: self.browser.clone(),
            cache_root: Some(dir.join("cache")),
            bridge: BridgeSetup::Off,
        }
    }

    fn searches(&self) -> usize {
        self.transport
            .paths()
            .iter()
            .filter(|p| p.starts_with("/database/search"))
            .count()
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
        move |dir| Some(f.setup(dir)),
    )
}

fn message(rig: &Rig) -> String {
    rig.app
        .message
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default()
}

fn track(side: &str, n: u8, title: &str, secs: u32, sample: &str) -> JunoTrack {
    JunoTrack {
        side: side.into(),
        n,
        title: title.into(),
        artist: String::new(),
        secs,
        sample: sample.into(),
    }
}

/// Rontgen as its product page shows it.
fn rontgen() -> JunoRecord {
    JunoRecord {
        product: PRODUCT.into(),
        slug: "0-mika-vainio-rontgen-vinyl".into(),
        artist: "0 aka Mika Vainio".into(),
        title: "Rontgen (reissue)".into(),
        label: "Sahko Finland".into(),
        catno: "SAHKO 00012".into(),
        format: "12\"".into(),
        stock: Stock::In,
        price: "€14.78".into(),
        image: "CS1178909-01A-BIG.jpg".into(),
        tracks: vec![
            track("1", 1, "Rontgen", 301, "01-01"),
            track("2", 1, "Track 2", 450, "02-01"),
        ],
    }
}

fn send(rig: &mut Rig, records: Vec<JunoRecord>) {
    rig.app.dig_bridge_command(BridgeCommand::SendJuno {
        records,
        mode: Mode::Enqueue,
        filters: Filters { skip_passed: true },
    });
}

fn origin(rig: &Rig, i: usize) -> Origin {
    rig.app.crates.get(PLAYLIST).unwrap().entries()[i]
        .origin
        .clone()
        .unwrap()
}

/// Lets the intake worker answer (Discogs lookups).
fn settle(rig: &mut Rig) {
    for _ in 0..40 {
        rig.pump();
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_product_page_becomes_juno_entries_that_play() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-product", &fakes, false);
    send(&mut rig, vec![rontgen()]);
    assert_eq!(
        message(&rig),
        "0 aka Mika Vainio - Rontgen (reissue): 2 added, 0 fixed, 0 skipped"
    );
    rig.until(
        |r| {
            r.app
                .crates
                .get(PLAYLIST)
                .is_some_and(|p| p.len() == 2 && p.entries().iter().all(|e| e.status.is_playable()))
        },
        "both samples ready",
    );
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    let e = &p.entries()[1];
    assert_eq!(
        (e.artist.as_str(), e.title.as_str()),
        ("0 aka Mika Vainio", "Track 2")
    );
    assert_eq!(e.duration, Some(450.0));
    let o = origin(&rig, 1);
    assert_eq!(o.source(), Some(ClipSource::Juno));
    assert_eq!(
        (o.catno.as_str(), o.position.as_str(), o.label.as_str()),
        ("SAHKO 00012", "2.1", "Sahko Finland")
    );
    assert_eq!(o.clip.as_deref(), Some("jn.1178909-01-02-01"));
    assert_eq!(
        o.cover,
        "https://imagescdn.juno.co.uk/full/CS1178909-01A-BIG.jpg"
    );
    assert!(
        rig.dir
            .join("cache/previews/jn.1178909-01-02-01.mp3")
            .exists()
    );
    let tip = crate::format::entry_details(e, rig.app.dig_marks(e), 0);
    assert!(
        tip.contains(&("Source", "Juno sample (2 min)".into())),
        "{tip:?}"
    );
    let note = tip
        .iter()
        .find(|(l, _)| *l == "Juno")
        .map(|(_, v)| v.clone());
    assert!(
        note.as_deref()
            .is_some_and(|n| n.starts_with("in stock, €14.78 (seen ")),
        "{tip:?}"
    );
    assert_eq!(ClipSource::Juno.badge(), "JN");
    assert_eq!(fakes.searches(), 0, "no token: nothing asked of Discogs");
}

#[test]
fn a_track_with_no_preview_gets_the_sample_and_nothing_is_doubled() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-fix", &fakes, false);
    let p = rig.app.crates.get_mut(PLAYLIST).unwrap();
    let id = p.add_waiting(
        "0 aka Mika Vainio",
        "Rontgen",
        Some(::dig::preview::fetcher::clip_url("RONTGENclip")),
        Some(Origin {
            release: Some(555),
            catno: "SAHKO 00012".into(),
            clip: Some("RONTGENclip".into()),
            ..Origin::default()
        }),
        WaitKind::Queued,
    );
    p.set_unavailable(id, UnavailableKind::NotFound);
    send(&mut rig, vec![rontgen()]);
    assert_eq!(
        message(&rig),
        "0 aka Mika Vainio - Rontgen (reissue): 1 added, 1 fixed, 0 skipped"
    );
    let o = origin(&rig, 0);
    assert_eq!(o.source(), Some(ClipSource::Juno), "fixed in place");
    assert_eq!(
        (o.release, o.youtube_clip.as_str()),
        (Some(555), "RONTGENclip")
    );
    assert_eq!(o.other_sources(), [ClipSource::YouTube]);
    // Sent again: nothing changes.
    send(&mut rig, vec![rontgen()]);
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().len(), 2);
    assert!(
        message(&rig).ends_with("0 added, 0 fixed, 2 skipped"),
        "{}",
        message(&rig)
    );
}

#[test]
fn a_listing_item_with_one_sample_is_one_entry_named_after_the_record() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-listing", &fakes, false);
    let mut r = rontgen();
    r.artist = "Two Shell".into();
    r.title = "Infinite Now".into();
    r.catno = String::new();
    r.tracks = vec![track("", 1, "", 0, "01-01")];
    send(&mut rig, vec![r]);
    let e = &rig.app.crates.get(PLAYLIST).unwrap().entries()[0];
    assert_eq!(
        (e.artist.as_str(), e.title.as_str(), e.duration),
        ("Two Shell", "Infinite Now", None)
    );
}

#[test]
fn a_500_record_page_is_merged_at_once() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-big", &fakes, false);
    let records: Vec<JunoRecord> = (0..500)
        .map(|i| {
            let mut r = rontgen();
            r.product = format!("{}-01", 2_000_000 + i);
            r.title = format!("Record {i}");
            r.catno = format!("CAT {i}");
            r.tracks = (1..=10)
                .map(|n| {
                    track(
                        "1",
                        n,
                        &format!("Track {i}.{n}"),
                        300,
                        &format!("01-{n:02}"),
                    )
                })
                .collect();
            r
        })
        .collect();
    let again = records.clone();
    let started = std::time::Instant::now();
    send(&mut rig, records);
    let took = started.elapsed();
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().len(), 5_000);
    // The 50 ms target is for optimised builds; debug builds of this crate are far slower.
    let limit = if cfg!(debug_assertions) { 2_000 } else { 50 };
    assert!(took.as_millis() < limit, "took {took:?}");
    // Sent again into the full crate: everything is skipped, as fast.
    let started = std::time::Instant::now();
    send(&mut rig, again);
    assert!(
        started.elapsed().as_millis() < limit,
        "took {:?}",
        started.elapsed()
    );
    assert!(message(&rig).ends_with("0 added, 0 fixed, 5000 skipped"));
}

#[test]
fn exactly_one_discogs_release_links_the_record_once() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-link", &fakes, true);
    let mut aste = rontgen();
    aste.product = "1113596-01".into();
    aste.title = "Aste".into();
    aste.catno = "SAHKO 00013".into();
    aste.tracks = vec![track("1", 1, "Aste", 300, "01-01")];
    let mut kiila = aste.clone();
    kiila.product = "1113597-01".into();
    kiila.title = "Kiila".into();
    kiila.catno = "SAHKO 00014".into();
    kiila.tracks = vec![track("1", 1, "Kiila", 300, "01-01")];
    send(&mut rig, vec![rontgen(), aste, kiila]);
    rig.until(
        |r| {
            r.app.crates.get(PLAYLIST).unwrap().entries()[0]
                .origin
                .as_ref()
                .is_some_and(|o| o.release.is_some())
        },
        "Rontgen is linked",
    );
    settle(&mut rig);
    assert_eq!(fakes.searches(), 3);
    assert_eq!(origin(&rig, 1).release, Some(123456), "every track of it");
    assert_eq!(origin(&rig, 2).release, None, "two releases fit Aste");
    assert_eq!(origin(&rig, 3).release, None, "no release fits Kiila");
    // Y works on the linked record, and says why not on the other.
    let ctx = rig.ctx.clone();
    let ids: Vec<EntryId> = rig
        .app
        .crates
        .get(PLAYLIST)
        .unwrap()
        .entries()
        .iter()
        .map(|e| e.id)
        .collect();
    rig.app
        .apply(Action::Dig(DigAction::Want(vec![ids[0]])), &ctx);
    assert!(rig.app.dig.as_ref().unwrap().memory.is_wanted(123456));
    rig.app
        .apply(Action::Dig(DigAction::Want(vec![ids[2]])), &ctx);
    let name = rig.app.crates.get(PLAYLIST).unwrap().entries()[2].display_name();
    assert_eq!(message(&rig), format!("{name} isn't linked to Discogs"));
    // Sent again: no lookup, and the link is applied at once.
    send(&mut rig, vec![rontgen()]);
    settle(&mut rig);
    assert_eq!(fakes.searches(), 3);
    let mem = &rig.app.dig.as_ref().unwrap().juno.memory;
    assert_eq!(mem.products[PRODUCT].release, Some(123456));
    assert!(mem.products["1113596-01"].searched);
    assert!(mem.products["1113597-01"].searched);
    let file = rig.dir.join("cache/juno.ron");
    rig.until(|_| file.exists(), "juno.ron is written in the background");
}

#[test]
fn the_note_shows_on_discogs_entries_of_the_release_and_opens_juno() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-note", &fakes, true);
    send(&mut rig, vec![rontgen()]);
    rig.until(
        |r| {
            r.app
                .dig
                .as_ref()
                .is_some_and(|d| d.juno.memory.products[PRODUCT].release.is_some())
        },
        "linked",
    );
    let c = rig.app.crates.create("Discogs").unwrap();
    rig.app.crates.load(c);
    let p = rig.app.crates.get_mut(c).unwrap();
    let id = p.add_waiting(
        "0 aka Mika Vainio",
        "Rontgen",
        Some(::dig::preview::fetcher::clip_url("RONTGENclip")),
        Some(Origin {
            release: Some(123456),
            clip: Some("RONTGENclip".into()),
            ..Origin::default()
        }),
        WaitKind::Queued,
    );
    let e = rig.app.crates.get(c).unwrap().get(id).unwrap().clone();
    let note = rig.app.juno_note(&e).unwrap();
    assert!(note.starts_with("in stock, €14.78 (seen "), "{note}");
    assert!(!note.contains("may have changed"));
    rig.app.show_crate(c);
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::OpenJuno(id)), &ctx);
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://www.juno.co.uk/products/0-mika-vainio-rontgen-vinyl/1178909-01/"]
    );
    // A note from 40 days ago may have changed.
    let d = rig.app.dig.as_mut().unwrap();
    d.juno.memory.products.get_mut(PRODUCT).unwrap().seen -= 40 * 24 * 3600;
    assert!(
        rig.app
            .juno_note(&e)
            .unwrap()
            .ends_with(", may have changed)")
    );
}

#[test]
fn a_discogs_record_takes_over_juno_only_entries() {
    let mut p = crate::playlist::Playlist::default();
    p.add_waiting(
        "0 aka Mika Vainio",
        "Rontgen",
        Some("https://www.juno.co.uk/products/x/1178909-01/".into()),
        Some(Origin {
            catno: "SAHKO 00012".into(),
            clip: Some("jn.1178909-01-01-01".into()),
            juno: PRODUCT.into(),
            juno_slug: "x".into(),
            juno_clip: "jn.1178909-01-01-01".into(),
            ..Origin::default()
        }),
        WaitKind::Queued,
    );
    let new = |title: &str, clip: &str| crate::playlist::NewEntry {
        artist: "0 aka Mika Vainio".into(),
        title: title.into(),
        source: Some(::dig::preview::fetcher::clip_url(clip)),
        origin: Some(Origin {
            release: Some(123456),
            catno: "SAHKO 00012".into(),
            clip: Some(clip.into()),
            ..Origin::default()
        }),
        duration: None,
        status: WaitKind::Queued,
    };
    let left = crate::app::bandcamp::absorb_into_bandcamp(
        &mut p,
        vec![new("Rontgen", "RONTGENclip"), new("Track 2", "TRACK2clip0")],
    );
    assert_eq!(left.len(), 1, "only Track 2 is added");
    let o = p.entries()[0].origin.clone().unwrap();
    assert_eq!(o.release, Some(123456));
    assert_eq!(
        o.source(),
        Some(ClipSource::Juno),
        "keeps playing from Juno"
    );
    assert_eq!(o.youtube_clip, "RONTGENclip");
    assert_eq!(o.juno, PRODUCT);
}

#[test]
fn juno_addresses_are_refused_when_pasted() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-paste", &fakes, false);
    rig.frame(vec![Event::Paste(
        "https://www.juno.co.uk/products/0-mika-vainio-rontgen-vinyl/1178909-01/".into(),
    )]);
    assert_eq!(
        message(&rig),
        "Juno pages are sent from the browser extension"
    );
    assert!(rig.app.crates.get(PLAYLIST).unwrap().is_empty());
}

#[test]
fn an_entry_switches_between_three_sources() {
    let mut o = Origin {
        clip: Some("jn.1178909-01-01-01".into()),
        youtube_clip: "RONTGENclip".into(),
        bandcamp_clip: "bc.3020153053".into(),
        juno_clip: "jn.1178909-01-01-01".into(),
        ..Origin::default()
    };
    assert_eq!(
        o.other_sources(),
        [ClipSource::YouTube, ClipSource::Bandcamp]
    );
    assert!(o.switch_to(ClipSource::YouTube));
    assert_eq!(o.clip.as_deref(), Some("RONTGENclip"));
    assert_eq!(o.other_sources(), [ClipSource::Bandcamp, ClipSource::Juno]);
    assert!(o.switch_to(ClipSource::Bandcamp));
    assert_eq!(o.youtube_clip, "RONTGENclip", "set aside again");
    assert!(o.switch_to(ClipSource::Juno));
    assert_eq!(o.source(), Some(ClipSource::Juno));
    assert!(!o.switch_to(ClipSource::Juno));
    // An entry saved before Juno loads without it.
    let old: Origin = ron::from_str(r#"(page: "x", clip: Some("dQw4w9WgXcQ"))"#).unwrap();
    assert_eq!(
        (old.juno.as_str(), old.source()),
        ("", Some(ClipSource::YouTube))
    );
}

#[test]
fn enqueue_never_fills_a_crate_the_app_fills_itself() {
    let fakes = Fakes::new();
    let mut rig = rig("juno-enqueue-special", &fakes, false);
    let wl = rig.app.crates.create("Wantlist").unwrap();
    rig.app.crates.set_wantlist(wl, true);
    let seller = rig.app.crates.create("Seller: digger").unwrap();
    rig.app.crates.set_seller(seller, "digger");
    for c in [wl, seller] {
        rig.app.crates.load(c);
        rig.app.show_crate(c);
        assert_eq!(rig.app.enqueue_target(), PLAYLIST);
        send(&mut rig, vec![rontgen()]);
        assert!(rig.app.crates.get(c).unwrap().is_empty());
    }
    assert_eq!(
        rig.app.crates.get(PLAYLIST).unwrap().len(),
        2,
        "into the Playlist"
    );
    let normal = rig.app.crates.create("Friday").unwrap();
    rig.app.show_crate(normal);
    assert_eq!(rig.app.enqueue_target(), normal);
}
