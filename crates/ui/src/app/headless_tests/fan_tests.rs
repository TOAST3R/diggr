//! A Bandcamp fan's collection and wishlist, headless: imported into the collection and
//! wantlist crates without doubles, looked up on Discogs (exact matches only), refreshed with
//! two sources, stopped, and reported; against fake Bandcamp pages and a fake Discogs.

use super::*;
use crate::app::digging::DigAction;
use crate::playlist::WaitKind;
use ::dig::bandcamp::fan::{DiscogsMatch, FanItem, FanList, FanLists, ItemKind};
use ::dig::bandcamp::{self, BandcampAlbum, BandcampTrack, Listing};
use ::dig::browser::FakeBrowser;
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::search::{Wanted, path};
use ::dig::discogs::transport::{FakeTransport, Method};
use ::dig::preview::fetcher::{FakeFetcher, Fetcher};
use ::dig::preview::scheduler::Finder;

const FAN: &str = "https://bandcamp.com/toast3r";
const GLASSHOUSE: &str = "https://lowtidetapes.bandcamp.com/album/glasshouse-ep";
const OOZE: &str = "https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep";
const SWAMP: &str = "https://analogicalforce.bandcamp.com/album/swamp-things";
const MARSH: &str = "https://analogicalforce.bandcamp.com/album/marsh";
const COLLECTION_PAGE: &str =
    "/users/digger/collection/folders/0/releases?sort=added&sort_order=desc&page=1&per_page=100";

#[derive(Clone)]
struct Fakes {
    transport: Arc<FakeTransport>,
    fetcher: Arc<FakeFetcher>,
}

fn track(page: &str, id: &str, artist: &str, title: &str) -> BandcampTrack {
    let root = page.split("/album/").next().unwrap();
    BandcampTrack {
        track_id: id.into(),
        url: format!("{root}/track/{}", title.to_lowercase().replace(' ', "-")),
        artist: artist.into(),
        title: title.into(),
        duration: Some(300.0),
        streamable: true,
    }
}

fn album(url: &str, artist: &str, title: &str, catno: &str, tracks: Vec<BandcampTrack>) -> Listing {
    Listing::Album(BandcampAlbum {
        url: url.into(),
        title: title.into(),
        catno: catno.into(),
        artist: artist.into(),
        cover: String::new(),
        year: Some(2024),
        tracks,
    })
}

fn item(key: &str, page: &str, artist: &str, title: &str, catno: &str, kind: ItemKind) -> FanItem {
    FanItem {
        key: key.into(),
        page: page.into(),
        artist: artist.into(),
        title: title.into(),
        catno: catno.into(),
        kind,
    }
}

impl Fakes {
    /// The fan "toast3r": a physical Glasshouse EP (Discogs release 1001) and a digital Ooze
    /// EP (not on Discogs) bought; Swamp Things (two pressings on Discogs) and Marsh wished.
    fn new() -> Self {
        let fetcher = Arc::new(FakeFetcher::new(fixture("tone.m4a")));
        fetcher.page(
            GLASSHOUSE,
            album(
                GLASSHOUSE,
                "Nightcraft",
                "Glasshouse EP",
                "LT-012",
                vec![
                    track(GLASSHOUSE, "101", "Nightcraft", "Glasshouse"),
                    track(GLASSHOUSE, "102", "Nightcraft", "Last Light"),
                ],
            ),
        );
        let json = std::fs::read_to_string(format!(
            "{}/../dig/tests/fixtures/bandcamp/album.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let ooze = bandcamp::parse(OOZE).unwrap();
        fetcher.page(OOZE, bandcamp::parse_listing(&ooze, &json).unwrap());
        fetcher.page(
            SWAMP,
            album(
                SWAMP,
                "Patricia",
                "Swamp Things",
                "",
                vec![track(SWAMP, "201", "Patricia", "Bog")],
            ),
        );
        fetcher.page(
            MARSH,
            album(
                MARSH,
                "Patricia",
                "Marsh",
                "",
                vec![track(MARSH, "301", "Patricia", "Reeds")],
            ),
        );
        let f = Self {
            transport: Arc::new(FakeTransport::with_fixtures(format!(
                "{}/../dig/tests/fixtures/discogs",
                env!("CARGO_MANIFEST_DIR")
            ))),
            fetcher,
        };
        f.lists(&["c1", "c2"], &["w1", "w2"]);
        f.transport.route("/cart", 200, "[]");
        f.transport.route(
            "/purchases?sort=created&sort_order=desc&page=1&per_page=100",
            200,
            r#"{"items": []}"#,
        );
        f.transport.route(
            COLLECTION_PAGE,
            200,
            r#"{"pagination": {"page": 1, "pages": 1, "items": 0}, "releases": []}"#,
        );
        f.transport.route(
            "/users/digger/collection/releases/1001",
            200,
            r#"{"releases": []}"#,
        );
        f.transport.route(
            path(&Wanted::new("Nightcraft", "Glasshouse EP", "LT-012", true)),
            200,
            r#"{"results":[{"id":1001,"title":"Nightcraft - Glasshouse EP","format":["Vinyl"],"catno":"LT-012"},
                           {"id":2001,"title":"Nightcraft - Glasshouse EP","format":["File"],"catno":"LT-012"}]}"#,
        );
        f.transport.route(
            path(&Wanted::new("Patricia", "The Ooze EP", "AF070", false)),
            200,
            r#"{"results":[]}"#,
        );
        f.transport.route(
            path(&Wanted::new("Patricia", "Swamp Things", "", true)),
            200,
            r#"{"results":[{"id":3001,"title":"Patricia - Swamp Things","format":["Vinyl"],"catno":"AF012"},
                           {"id":3002,"title":"Patricia - Swamp Things","format":["Vinyl"],"catno":"AF012R"}]}"#,
        );
        f.transport.route(
            path(&Wanted::new("Patricia", "Marsh", "", true)),
            200,
            r#"{"results":[]}"#,
        );
        f
    }

    /// The fan's lists hold these items (by key).
    fn lists(&self, collection: &[&str], wishlist: &[&str]) {
        let all = [
            item(
                "c1",
                GLASSHOUSE,
                "Nightcraft",
                "Glasshouse EP",
                "LT-012",
                ItemKind::Physical,
            ),
            item(
                "c2",
                OOZE,
                "Patricia",
                "The Ooze EP",
                "AF070",
                ItemKind::Digital,
            ),
            item(
                "w1",
                SWAMP,
                "Patricia",
                "Swamp Things",
                "",
                ItemKind::Digital,
            ),
            item("w2", MARSH, "Patricia", "Marsh", "", ItemKind::Digital),
            item(
                "w3",
                GLASSHOUSE,
                "Nightcraft",
                "Glasshouse EP",
                "LT-012",
                ItemKind::Digital,
            ),
        ];
        let pick = |keys: &[&str]| -> Vec<FanItem> {
            keys.iter()
                .filter_map(|k| all.iter().find(|i| i.key == *k).cloned())
                .collect()
        };
        self.fetcher.fan(
            "toast3r",
            FanLists {
                fan_id: 425839,
                collection: pick(collection),
                wishlist: pick(wishlist),
            },
        );
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
            browser: Arc::new(FakeBrowser::default()),
            cache_root: Some(dir.join("cache")),
            bridge: BridgeSetup::Off,
            tools: None,
        }
    }

    fn searches(&self) -> usize {
        self.transport
            .paths()
            .iter()
            .filter(|p| p.starts_with("/database/search"))
            .count()
    }

    fn changes(&self) -> Vec<(Method, String)> {
        self.transport
            .log()
            .into_iter()
            .filter(|(r, _)| r.method != Method::Get)
            .map(|(r, _)| (r.method, r.path))
            .collect()
    }
}

fn rig(name: &str, fakes: &Fakes, tracks: Vec<PathBuf>, prepare: impl FnOnce(&Store)) -> Rig {
    let f = fakes.clone();
    Rig::with_dig(name, tracks, prepare, move |dir| Some(f.setup(dir)))
}

fn with_token(store: &Store) {
    ::dig::config::save_token(store.dir(), Some("tok")).unwrap();
}

fn running(rig: &Rig) -> bool {
    rig.app
        .dig
        .as_ref()
        .is_some_and(|d| d.bandcamp.fan.run.is_some())
}

fn import(rig: &mut Rig, url: &str) {
    rig.frame(vec![Event::Paste(url.into())]);
    rig.until(|r| !running(r), "the import ends");
    for _ in 0..20 {
        rig.pump();
    }
}

fn report(rig: &Rig) -> ::dig::bandcamp::fan::FanReport {
    let d = rig.app.dig.as_ref().unwrap();
    d.bandcamp
        .memory
        .fan
        .as_ref()
        .unwrap()
        .report
        .clone()
        .unwrap()
}

fn crate_named(rig: &Rig, name: &str) -> CrateId {
    rig.app
        .crates
        .find(name)
        .unwrap_or_else(|| panic!("no crate {name}"))
}

fn titles(rig: &Rig, c: CrateId) -> Vec<String> {
    let mut t: Vec<String> = rig
        .app
        .crates
        .get(c)
        .unwrap()
        .entries()
        .iter()
        .map(|e| e.title.clone())
        .collect();
    t.sort();
    t
}

#[test]
fn without_a_token_both_crates_are_made_and_nothing_goes_to_discogs() {
    let fakes = Fakes::new();
    let mut rig = rig("fan-anon", &fakes, Vec::new(), |_| {});
    import(&mut rig, &format!("{FAN}/wishlist?from=menubar"));
    let coll = crate_named(&rig, "Collection: toast3r");
    let wl = crate_named(&rig, "Wantlist: toast3r");
    let info = |c| rig.app.crates.info(c).unwrap().clone();
    assert!(info(coll).collection && info(wl).wantlist, "under DISCOGS");
    assert_eq!(
        titles(&rig, coll),
        ["Bonus", "Glasshouse", "Last Light", "Swamp", "The Ooze"]
    );
    assert_eq!(titles(&rig, wl), ["Bog", "Reeds"]);
    assert_eq!(fakes.searches(), 0);
    assert!(fakes.changes().is_empty(), "{:?}", fakes.changes());
    let r = report(&rig);
    assert_eq!(r.lists.len(), 2);
    assert_eq!((r.lists[0].added, r.lists[0].unsearched), (2, 2));
    assert!(rig.app.dig.as_ref().unwrap().bandcamp.fan.report_open);
    // The crates offer Refresh (Bandcamp only) and the report.
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::ShowFanReport), &ctx);
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "Bandcamp import: toast3r"),
        "{:?}",
        text_list(&out)
    );
    // A Bandcamp page that's Bandcamp's own is refused.
    rig.frame(vec![Event::Paste("https://bandcamp.com/discover".into())]);
    assert_eq!(
        rig.app
            .message
            .as_ref()
            .map(|(m, _)| m.clone())
            .unwrap_or_default(),
        bandcamp::SUPPORTED
    );
}

#[test]
fn exact_matches_go_to_discogs_and_take_the_bandcamp_entries_over() {
    let fakes = Fakes::new();
    let mut rig = rig("fan-token", &fakes, Vec::new(), with_token);
    import(&mut rig, FAN);
    let coll = crate_named(&rig, "Collection: toast3r");
    let add = (
        Method::Post,
        "/users/digger/collection/folders/1/releases/1001".to_owned(),
    );
    rig.until(
        |_| fakes.changes().contains(&add),
        "1001 added to the collection",
    );
    // The vinyl's record takes the Bandcamp tracks over, and brings the one Bandcamp lacked.
    // Its tracks come in as "Release 1001" first, then by name.
    let all = [
        "Bonus",
        "Glasshouse",
        "Glasshouse (Lumen Remix)",
        "Last Light",
        "Swamp",
        "The Ooze",
    ];
    rig.until(
        |r| titles(r, coll) == all,
        "the record arrives in the crate",
    );
    let p = rig.app.crates.get(coll).unwrap();
    let glass = p
        .entries()
        .iter()
        .find(|e| e.title == "Glasshouse")
        .unwrap();
    let o = glass.origin.as_ref().unwrap();
    assert_eq!(o.release, Some(1001));
    assert_eq!(
        o.bandcamp,
        "https://lowtidetapes.bandcamp.com/track/glasshouse"
    );
    // Nothing for the rest: not on Discogs, or two pressings.
    assert_eq!(
        fakes
            .changes()
            .iter()
            .filter(|(m, _)| *m != Method::Get)
            .count(),
        1,
        "{:?}",
        fakes.changes()
    );
    let r = report(&rig);
    let c = r
        .lists
        .iter()
        .find(|l| l.list == Some(FanList::Collection))
        .unwrap();
    assert_eq!((c.discogs_added, c.unsearched), (1, 0));
    let why: Vec<(&str, &str)> = r
        .not_added
        .iter()
        .map(|i| (i.item.title.as_str(), i.why.as_str()))
        .collect();
    assert!(why.contains(&("The Ooze EP", "not on Discogs")), "{why:?}");
    assert!(why.contains(&("Swamp Things", "2 candidates")), "{why:?}");
    assert!(
        r.text().contains("Not added to Discogs (3):"),
        "{}",
        r.text()
    );
    // A refresh with nothing new asks Discogs nothing more.
    let searched = fakes.searches();
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::RefreshWantlist), &ctx);
    rig.until(|r| !running(r), "the refresh ends");
    assert_eq!(fakes.searches(), searched);
    let d = rig.app.dig.as_ref().unwrap();
    let mem = d.bandcamp.memory.fan.as_ref().unwrap();
    assert_eq!(mem.wishlist[0].discogs, DiscogsMatch::Ambiguous(2));
}

#[test]
fn a_refresh_reads_only_its_own_new_items_and_lets_go_of_the_gone() {
    let fakes = Fakes::new();
    let mut rig = rig("fan-refresh", &fakes, Vec::new(), |_| {});
    import(&mut rig, FAN);
    let wl = crate_named(&rig, "Wantlist: toast3r");
    let coll = crate_named(&rig, "Collection: toast3r");
    let read = fakes.fetcher.pages_read().len();
    // Marsh is off the Bandcamp wishlist; nothing else changed.
    fakes.lists(&["c1", "c2"], &["w1"]);
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::RefreshWantlist), &ctx);
    rig.until(|r| !running(r), "the refresh ends");
    assert_eq!(titles(&rig, wl), ["Bog"]);
    assert_eq!(titles(&rig, coll).len(), 5, "the collection isn't touched");
    assert_eq!(
        fakes.fetcher.pages_read().len(),
        read + 1,
        "only the fan's lists, no album again"
    );
    let r = report(&rig);
    assert_eq!(r.lists.len(), 1);
    assert_eq!(r.lists[0].gone, 1);
    // The report is kept across a restart.
    let saved = ::dig::bandcamp::BandcampMemory::load(&rig.dir.join("config"));
    assert_eq!(saved.fan.unwrap().report.unwrap().lists[0].gone, 1);
}

#[test]
fn stop_keeps_what_was_read_and_a_refresh_reads_the_rest() {
    let fakes = Fakes::new();
    let mut rig = rig("fan-stop", &fakes, Vec::new(), |_| {});
    rig.frame(vec![Event::Paste(FAN.into())]);
    rig.until(
        |r| {
            r.app
                .crates
                .find("Collection: toast3r")
                .and_then(|c| r.app.crates.get(c))
                .is_some_and(|p| !p.is_empty())
        },
        "the first album arrives",
    );
    rig.app.fan_stop();
    assert!(!running(&rig));
    assert!(report(&rig).stopped);
    for _ in 0..40 {
        rig.pump();
    }
    let d = rig.app.dig.as_ref().unwrap();
    let mem = d.bandcamp.memory.fan.as_ref().unwrap();
    let unread = mem
        .collection
        .iter()
        .chain(&mem.wishlist)
        .filter(|e| !e.read)
        .count();
    assert!(unread > 0, "stopped before the end");
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::Dig(DigAction::RefreshCollection), &ctx);
    rig.until(|r| !running(r), "the refresh ends");
    let d = rig.app.dig.as_ref().unwrap();
    let mem = d.bandcamp.memory.fan.as_ref().unwrap();
    assert!(mem.collection.iter().all(|e| e.read), "the rest was read");
}

#[test]
fn an_import_while_playing_keeps_playback_going() {
    let fakes = Fakes::new();
    let mut rig = rig("fan-play", &fakes, vec![fixture("tone.flac")], |_| {});
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    import(&mut rig, FAN);
    assert_eq!(rig.app.position.state, PlayState::Playing);
    assert_eq!(rig.engine().stats().underruns, 0);
}

#[test]
fn a_token_saved_later_fills_the_bandcamp_crate_instead_of_making_another() {
    let fakes = Fakes::new();
    let mut rig = rig("fan-token-later", &fakes, Vec::new(), |_| {});
    import(&mut rig, FAN);
    let coll = crate_named(&rig, "Collection: toast3r");
    rig.app.dig_token_checked(
        "tok".into(),
        Ok(::dig::discogs::client::Identity {
            username: "digger".into(),
            currency: "EUR".into(),
        }),
    );
    rig.until(
        |_| {
            fakes
                .transport
                .paths()
                .iter()
                .any(|p| p.starts_with("/users/digger/collection/folders/0/releases"))
        },
        "the Discogs collection is read",
    );
    assert!(rig.app.crates.find("Collection: digger").is_none());
    let marked: Vec<CrateId> = rig
        .app
        .crates
        .list()
        .iter()
        .filter(|c| c.collection)
        .map(|c| c.id)
        .collect();
    assert_eq!(marked, [coll]);
}

#[test]
fn a_record_off_the_discogs_wantlist_stays_while_the_bandcamp_wishlist_holds_it() {
    let fakes = Fakes::new();
    fakes.lists(&[], &["w3"]);
    let mut rig = rig("fan-two-lists", &fakes, Vec::new(), with_token);
    import(&mut rig, FAN);
    let put = (Method::Put, "/users/digger/wants/1001".to_owned());
    rig.until(
        |_| fakes.changes().contains(&put),
        "1001 on the Discogs wantlist",
    );
    let wl = rig.app.dig.as_ref().unwrap().settings.wantlist.unwrap();
    // Discogs records now carry the wished tracks (whichever record took them over).
    let held = |r: &Rig| -> Vec<String> {
        let mut t: Vec<String> = r
            .app
            .crates
            .get(wl)
            .unwrap()
            .entries()
            .iter()
            .filter(|e| {
                e.origin
                    .as_ref()
                    .is_some_and(|o| o.release.is_some() && !o.bandcamp.is_empty())
            })
            .map(|e| e.title.clone())
            .collect();
        t.sort();
        t
    };
    rig.until(
        |r| held(r).len() == 2,
        "Discogs records take the Bandcamp entries over",
    );
    let ids: Vec<EntryId> = rig
        .app
        .crates
        .get(wl)
        .unwrap()
        .entries()
        .iter()
        .map(|e| e.id)
        .collect();
    rig.app.crates.show(wl);
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::Unwant(ids)), &ctx);
    assert!(!rig.app.dig.as_ref().unwrap().memory.is_wanted(1001));
    assert_eq!(
        titles(&rig, wl),
        ["Glasshouse", "Last Light"],
        "only what the Bandcamp wishlist holds stays"
    );
}

#[test]
fn a_record_already_in_the_crate_gets_the_bandcamp_track_without_switching() {
    let fakes = Fakes::new();
    fakes.lists(&[], &["w3"]);
    let mut rig = rig("fan-link", &fakes, Vec::new(), |_| {});
    let wl = rig.app.crates.create("Wantlist: toast3r").unwrap();
    rig.app.crates.set_wantlist(wl, true);
    rig.app.dig.as_mut().unwrap().settings.wantlist = Some(wl);
    let p = rig.app.crates.get_mut(wl).unwrap();
    p.add_waiting(
        "Nightcraft",
        "Glasshouse",
        Some(::dig::preview::fetcher::clip_url("GLASS000001")),
        Some(Origin {
            page: "https://www.discogs.com/release/1001".into(),
            release: Some(1001),
            catno: "LT-012".into(),
            clip: Some("GLASS000001".into()),
            ..Origin::default()
        }),
        WaitKind::Queued,
    );
    import(&mut rig, FAN);
    let e = rig
        .app
        .crates
        .get(wl)
        .unwrap()
        .entries()
        .iter()
        .find(|e| e.title == "Glasshouse")
        .cloned()
        .unwrap();
    let o = e.origin.unwrap();
    assert_eq!(
        o.bandcamp,
        "https://lowtidetapes.bandcamp.com/track/glasshouse"
    );
    assert_eq!(
        o.clip.as_deref(),
        Some("GLASS000001"),
        "it still plays from YouTube"
    );
    assert_eq!(
        titles(&rig, wl),
        ["Glasshouse", "Last Light"],
        "not added a second time"
    );
}
