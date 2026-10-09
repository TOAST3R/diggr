//! Bandcamp, headless: pasted albums, merging into a crate (skip what plays, fix what had no
//! preview, add what's missing), labels followed from the browser and merged by name, the
//! source badge, and switching an entry's source; against fake Bandcamp pages.

use super::*;
use crate::playlist::{ClipSource, EntryStatus, UnavailableKind, WaitKind};
use ::dig::bandcamp::{self, BandcampAlbum, BandcampTrack, Listing};
use ::dig::bridge::{BridgeCommand, Mode};
use ::dig::browser::FakeBrowser;
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::transport::FakeTransport;
use ::dig::jobs::Filters;
use ::dig::preview::fetcher::{FakeFetcher, Fetcher};
use ::dig::preview::scheduler::Finder;

const ROOT: &str = "https://analogicalforce.bandcamp.com";
const ALBUM: &str = "https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep";

#[derive(Clone)]
struct Fakes {
    fetcher: Arc<FakeFetcher>,
    browser: Arc<FakeBrowser>,
}

impl Fakes {
    /// Bandcamp knows the label (two albums) and the fixture album AF070.
    fn new() -> Self {
        let fetcher = Arc::new(FakeFetcher::new(fixture("tone.m4a")));
        let json = std::fs::read_to_string(format!(
            "{}/../dig/tests/fixtures/bandcamp/album.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let page = bandcamp::parse(ALBUM).unwrap();
        fetcher.page(ALBUM, bandcamp::parse_listing(&page, &json).unwrap());
        let older = format!("{ROOT}/album/af012");
        fetcher.page(
            &older,
            Listing::Album(BandcampAlbum {
                url: older.clone(),
                title: "Swamp Things".into(),
                catno: "AF012".into(),
                artist: "Patricia".into(),
                cover: String::new(),
                year: Some(2019),
                tracks: vec![BandcampTrack {
                    track_id: "1200".into(),
                    url: format!("{ROOT}/track/marsh"),
                    artist: "Patricia".into(),
                    title: "Marsh".into(),
                    duration: Some(300.0),
                    streamable: true,
                }],
            }),
        );
        fetcher.page(
            &format!("{ROOT}/music"),
            Listing::Albums(vec![ALBUM.into(), older]),
        );
        Self {
            fetcher,
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
            transport: Arc::new(FakeTransport::with_fixtures(format!(
                "{}/../dig/tests/fixtures/discogs",
                env!("CARGO_MANIFEST_DIR")
            ))),
            images: FakeImages::new(Ok(test_jpeg(300, 300))),
            clock: Arc::new(RealClock::default()),
            finder,
            browser: self.browser.clone(),
            cache_root: Some(dir.join("cache")),
            bridge: BridgeSetup::Off,
        }
    }
}

fn rig(name: &str, fakes: &Fakes) -> Rig {
    let f = fakes.clone();
    Rig::with_dig(name, Vec::new(), |_| {}, move |dir| Some(f.setup(dir)))
}

fn message(rig: &Rig) -> String {
    rig.app
        .message
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default()
}

/// A Discogs track entry in crate `c`, from release AF070, in `status`.
fn discogs_entry(rig: &mut Rig, c: CrateId, title: &str, clip: &str, status: EntryStatus) {
    let p = rig.app.crates.get_mut(c).unwrap();
    let id = p.add_waiting(
        "Patricia",
        title,
        Some(::dig::preview::fetcher::clip_url(clip)),
        Some(Origin {
            page: "https://www.discogs.com/release/7070".into(),
            release: Some(7070),
            label: "Analogical Force".into(),
            catno: "AF070".into(),
            clip: Some(clip.into()),
            ..Origin::default()
        }),
        WaitKind::Queued,
    );
    match status {
        EntryStatus::Unavailable(k) => p.set_unavailable(id, k),
        _ => {
            p.set_audio(id, TrackRef::new(fixture("tone.m4a").to_string_lossy()));
        }
    }
}

fn entries(rig: &Rig, c: CrateId) -> Vec<(String, Option<ClipSource>, EntryStatus)> {
    rig.app
        .crates
        .get(c)
        .unwrap()
        .entries()
        .iter()
        .map(|e| {
            (
                e.title.clone(),
                e.origin.as_ref().and_then(|o| o.source()),
                e.status.clone(),
            )
        })
        .collect()
}

fn bridge_send(rig: &mut Rig, url: &str, title: Option<&str>) {
    rig.app.dig_bridge_command(BridgeCommand::SendBandcamp {
        page: bandcamp::parse(url).unwrap(),
        mode: Mode::Enqueue,
        filters: Filters { skip_passed: true },
        title: title.map(str::to_owned),
    });
}

fn reading_done(rig: &mut Rig, c: CrateId) {
    rig.until(|r| !r.app.bandcamp_reading(c), "Bandcamp is read");
    for _ in 0..20 {
        rig.pump();
    }
}

#[test]
fn a_pasted_album_becomes_bandcamp_entries_that_play() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-paste", &fakes);
    rig.frame(vec![Event::Paste(format!("{ALBUM}?from=search"))]);
    rig.until(
        |r| {
            r.app.crates.get(PLAYLIST).is_some_and(|p| {
                p.len() == 3 && p.entries()[..2].iter().all(|e| e.status.is_playable())
            })
        },
        "two Bandcamp previews ready",
    );
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    let o = p.entries()[0].origin.clone().unwrap();
    assert_eq!(
        (o.catno.as_str(), o.album.as_str(), o.label.as_str()),
        ("AF070", "The Ooze EP", "analogicalforce")
    );
    assert_eq!(o.source(), Some(ClipSource::Bandcamp));
    assert_eq!(
        o.bandcamp,
        "https://analogicalforce.bandcamp.com/track/the-ooze"
    );
    assert_eq!(p.entries()[0].title, "The Ooze");
    assert!(rig.dir.join("cache/previews/bc.3020153053.mp3").exists());
    assert_eq!(
        p.entries()[2].status,
        EntryStatus::Unavailable(UnavailableKind::Other(
            crate::app::bandcamp::NOT_STREAMABLE.into()
        )),
        "the pre-order track"
    );
    // A Bandcamp page that isn't an album, track or label says which are.
    rig.frame(vec![Event::Paste(format!("{ROOT}/merch"))]);
    assert_eq!(message(&rig), bandcamp::SUPPORTED);
}

#[test]
fn an_album_skips_what_plays_fixes_what_failed_and_adds_what_is_missing() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-merge", &fakes);
    let not_found = EntryStatus::Unavailable(UnavailableKind::NotFound);
    discogs_entry(&mut rig, PLAYLIST, "The Ooze", "OOZEclip001", not_found);
    discogs_entry(
        &mut rig,
        PLAYLIST,
        "Swamp",
        "SWAMPclip01",
        EntryStatus::Ready,
    );
    rig.app.dig_paste(ALBUM);
    reading_done(&mut rig, PLAYLIST);
    assert_eq!(
        message(&rig),
        "af070 the ooze ep: 1 added, 1 fixed, 1 skipped"
    );
    let got = entries(&rig, PLAYLIST);
    assert_eq!(got.len(), 3, "{got:?}");
    assert_eq!(got[0].1, Some(ClipSource::Bandcamp), "fixed in place");
    assert_eq!(got[1].1, Some(ClipSource::YouTube), "left alone");
    assert_eq!(got[2].0, "Bonus", "added after the AF070 tracks");
    rig.until(
        |r| {
            r.app.crates.get(PLAYLIST).unwrap().entries()[0]
                .status
                .is_playable()
        },
        "the fixed track plays",
    );
    let o = rig.app.crates.get(PLAYLIST).unwrap().entries()[0]
        .origin
        .clone()
        .unwrap();
    assert_eq!(o.release, Some(7070), "its Discogs origin stays");
    assert_eq!(o.youtube_clip, "OOZEclip001");
}

#[test]
fn a_label_from_the_browser_is_followed_and_refreshed_with_new_albums_only() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-follow", &fakes);
    let shown = rig.app.crates.shown_id();
    bridge_send(
        &mut rig,
        &format!("{ROOT}/music"),
        Some("Music | Analogical Force"),
    );
    let c = rig
        .app
        .crates
        .find_bandcamp("analogicalforce")
        .expect("followed");
    assert_eq!(rig.app.crates.name(c), "Label: Analogical Force");
    assert!(rig.app.crates.is_label(c) && rig.app.crates.is_locked(c));
    assert_eq!(rig.app.crates.shown_id(), shown, "the shown crate stays");
    reading_done(&mut rig, c);
    assert_eq!(rig.app.crates.get(c).unwrap().len(), 4);
    assert_eq!(message(&rig), "Label: Analogical Force: 2 new records");
    assert_eq!(fakes.fetcher.pages_read().len(), 3);
    // Sent again: a refresh reading only what's new.
    let newer = format!("{ROOT}/album/af071");
    fakes.fetcher.page(
        &format!("{ROOT}/music"),
        Listing::Albums(vec![
            newer.clone(),
            ALBUM.into(),
            format!("{ROOT}/album/af012"),
        ]),
    );
    fakes.fetcher.page(
        &newer,
        Listing::Album(BandcampAlbum {
            url: newer.clone(),
            title: "Next".into(),
            catno: "AF071".into(),
            artist: "Patricia".into(),
            cover: String::new(),
            year: None,
            tracks: Vec::new(),
        }),
    );
    bridge_send(&mut rig, &format!("{ROOT}/"), None);
    reading_done(&mut rig, c);
    let read = fakes.fetcher.pages_read();
    assert_eq!(read.len(), 5, "the label and AF071 only: {read:?}");
    assert_eq!(read[4], newer);
    assert_eq!(message(&rig), "Label: Analogical Force: up to date");
    assert_eq!(rig.app.crates.labels().count(), 1, "no second crate");
}

#[test]
fn a_bandcamp_label_merges_into_the_discogs_label_of_the_same_name() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-merge-label", &fakes);
    let c = rig.app.crates.create("Label: Analogical Force").unwrap();
    rig.app.crates.set_label(c, 99);
    rig.app.crates.load(c);
    // Pasted without a title, the subdomain still matches the name.
    bridge_send(&mut rig, &format!("{ROOT}/music"), None);
    assert_eq!(rig.app.crates.find_bandcamp("analogicalforce"), Some(c));
    assert_eq!(
        rig.app.crates.label_of(c),
        Some(99),
        "still the Discogs label's"
    );
    assert_eq!(rig.app.crates.labels().count(), 1);
    reading_done(&mut rig, c);
    assert_eq!(rig.app.crates.get(c).unwrap().len(), 4);
}

#[test]
fn a_close_name_asks_once_and_separate_is_remembered() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-ask", &fakes);
    let c = rig.app.crates.create("Label: Analogical").unwrap();
    rig.app.crates.set_label(c, 99);
    bridge_send(
        &mut rig,
        &format!("{ROOT}/music"),
        Some("Music | Analogical Force"),
    );
    assert!(rig.app.dig_asking(), "a question is open");
    rig.frame(Vec::new());
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "Merge Analogical Force into Label: Analogical?"),
        "{:?}",
        text_list(&out)
    );
    rig.click_text("Separate");
    assert!(!rig.app.dig_asking());
    let own = rig.app.crates.find_bandcamp("analogicalforce").unwrap();
    assert_ne!(own, c);
    assert_eq!(rig.app.crates.bandcamp_of(c), None);
    reading_done(&mut rig, own);
    // Sent again: refreshed without asking.
    bridge_send(
        &mut rig,
        &format!("{ROOT}/music"),
        Some("Music | Analogical Force"),
    );
    assert!(!rig.app.dig_asking());
    assert_eq!(rig.app.crates.labels().count(), 2);
}

#[test]
fn rows_show_their_source_and_an_entry_switches_between_them() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-switch", &fakes);
    let not_found = EntryStatus::Unavailable(UnavailableKind::NotFound);
    discogs_entry(&mut rig, PLAYLIST, "The Ooze", "OOZEclip001", not_found);
    discogs_entry(
        &mut rig,
        PLAYLIST,
        "Swamp",
        "SWAMPclip01",
        EntryStatus::Ready,
    );
    rig.app.dig_paste(ALBUM);
    reading_done(&mut rig, PLAYLIST);
    let out = rig.frame(Vec::new());
    let list = text_list(&out);
    assert!(list.iter().any(|t| t == "BC"), "{list:?}");
    assert!(list.iter().any(|t| t == "YT"), "{list:?}");
    let row = rig.row(0);
    rig.click_with(row, PointerButton::Secondary);
    let menu = text_list(&rig.frame(Vec::new()));
    assert!(menu.iter().any(|t| t == "Open on Bandcamp"), "{menu:?}");
    rig.click_text("Play from YouTube");
    let o = || {
        rig.app.crates.get(PLAYLIST).unwrap().entries()[0]
            .origin
            .clone()
            .unwrap()
    };
    assert_eq!(o().clip.as_deref(), Some("OOZEclip001"));
    assert_eq!(o().source(), Some(ClipSource::YouTube));
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Play from Bandcamp");
    let o = rig.app.crates.get(PLAYLIST).unwrap().entries()[0]
        .origin
        .clone()
        .unwrap();
    assert_eq!(o.clip.as_deref(), Some("bc.3020153053"));
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Open on Bandcamp");
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://analogicalforce.bandcamp.com/track/the-ooze"]
    );
}

#[test]
fn a_bandcamp_only_entry_has_no_discogs_items() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-no-discogs", &fakes);
    rig.app.dig_paste(ALBUM);
    reading_done(&mut rig, PLAYLIST);
    let row = rig.row(0);
    rig.click_with(row, PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Add to wantlist (Y)"), "shown, disabled");
    rig.click_text("Add to wantlist (Y)");
    assert!(
        rig.app.dig.as_ref().unwrap().memory.wanted.is_empty(),
        "nothing wanted"
    );
}

#[test]
fn a_discogs_label_followed_after_its_bandcamp_joins_that_crate() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-discogs-after", &fakes);
    let c = rig.app.crates.create("Label: Lowtide Tapes").unwrap();
    rig.app.crates.set_bandcamp(c, "lowtidetapes");
    let page =
        ::dig::discogs::url::parse("https://www.discogs.com/label/12345-Lowtide-Tapes").unwrap();
    rig.app.dig_follow_label(page, 12345);
    assert_eq!(rig.app.crates.find_label(12345), Some(c), "the same crate");
    assert_eq!(rig.app.crates.bandcamp_of(c), Some("lowtidetapes"));
    assert_eq!(rig.app.crates.labels().count(), 1);
}

#[test]
fn bandcamp_albums_group_into_records_like_discogs_releases() {
    let fakes = Fakes::new();
    let mut rig = rig("bc-records", &fakes);
    bridge_send(&mut rig, &format!("{ROOT}/music"), None);
    let c = rig.app.crates.find_bandcamp("analogicalforce").unwrap();
    reading_done(&mut rig, c);
    assert!(rig.app.crates.is_grouped(c));
    let p = rig.app.crates.get(c).unwrap();
    // AF070's three tracks and AF012's one: two records.
    assert_eq!(crate::crates::count_records(p), 2);
    let keys: Vec<_> = p.entries().iter().map(|e| e.album_key()).collect();
    assert!(keys.iter().all(Option::is_some), "{keys:?}");
    assert_eq!(keys[0], keys[1]);
    // Its record row's cover comes from Bandcamp's image host, keyed by the album's art.
    let (key, url) = crate::app::covers::cover_of(&p.entries()[0]).expect("a cover");
    assert_eq!(key, ::dig::cover::CoverKey::BandcampArt(161524395));
    assert!(::dig::cover::allowed(url));
}

// ---- Discogs records meeting Bandcamp ------------------------------------------------------

const RELEASE: &str = "https://www.discogs.com/release/1001-Glasshouse-EP";
const LOWTIDE: &str = "https://lowtidetapes.bandcamp.com";

/// Lowtide Tapes on Bandcamp: "Glasshouse EP" (LT-012) with two of its three tracks, and
/// another album.
fn lowtide(fakes: &Fakes) -> String {
    let album = format!("{LOWTIDE}/album/glasshouse-ep-lt-012");
    let track = |id: &str, slug: &str, title: &str| BandcampTrack {
        track_id: id.into(),
        url: format!("{LOWTIDE}/track/{slug}"),
        artist: "Nightcraft".into(),
        title: title.into(),
        duration: Some(372.0),
        streamable: true,
    };
    fakes.fetcher.page(
        &album,
        Listing::Album(BandcampAlbum {
            url: album.clone(),
            title: "Glasshouse EP".into(),
            catno: "LT-012".into(),
            artist: "Nightcraft".into(),
            cover: String::new(),
            year: Some(1994),
            tracks: vec![
                track("5001", "glasshouse", "Glasshouse"),
                track("5003", "last-light", "Last Light"),
            ],
        }),
    );
    fakes.fetcher.page(
        &format!("{LOWTIDE}/music"),
        Listing::Albums(vec![format!("{LOWTIDE}/album/other-thing"), album.clone()]),
    );
    album
}

fn crate_with(rig: &mut Rig, name: &str) -> CrateId {
    let c = rig.app.crates.create(name).unwrap();
    rig.app.crates.load(c);
    rig.app.show_crate(c);
    c
}

#[test]
fn a_discogs_record_takes_over_the_bandcamp_tracks_instead_of_doubling_them() {
    let fakes = Fakes::new();
    let album = lowtide(&fakes);
    let mut rig = rig("bc-no-doubles", &fakes);
    let c = crate_with(&mut rig, "Dig");
    rig.app.dig_paste(&album);
    reading_done(&mut rig, c);
    assert_eq!(rig.app.crates.get(c).unwrap().len(), 2);
    rig.app.dig_paste(RELEASE);
    rig.until(
        |r| {
            r.app.crates.get(c).is_some_and(|p| {
                p.entries()
                    .iter()
                    .filter(|e| e.origin.as_ref().is_some_and(|o| o.release == Some(1001)))
                    .count()
                    == 3
            })
        },
        "the record arrives",
    );
    let p = rig.app.crates.get(c).unwrap();
    assert_eq!(p.len(), 3, "the Lumen Remix added, nothing twice");
    let glasshouse = p
        .entries()
        .iter()
        .find(|e| e.title == "Glasshouse")
        .unwrap()
        .origin
        .clone()
        .unwrap();
    assert_eq!(
        glasshouse.source(),
        Some(ClipSource::Bandcamp),
        "keeps its audio"
    );
    assert_eq!(
        glasshouse.youtube_clip, "GLASShouse1",
        "the video set aside"
    );
    assert_eq!(
        (glasshouse.catno.as_str(), glasshouse.position.as_str()),
        ("LT-012", "A1")
    );
}

#[test]
fn a_track_youtube_fails_is_found_on_the_labels_bandcamp() {
    let fakes = Fakes::new();
    lowtide(&fakes);
    fakes.fetcher.fail("LASTlight01", u32::MAX);
    let mut rig = rig("bc-fallback", &fakes);
    let c = crate_with(&mut rig, "Dig");
    rig.app.dig_paste(RELEASE);
    rig.until(
        |r| {
            r.app
                .crates
                .get(c)
                .is_some_and(|p| p.len() == 3 && p.entries().iter().all(|e| e.status.is_playable()))
        },
        "every track plays, the failed one from Bandcamp",
    );
    rig.until(|r| !r.app.bandcamp_looking(c), "the look is over");
    let p = rig.app.crates.get(c).unwrap();
    let last = p
        .entries()
        .iter()
        .find(|e| e.title == "Last Light")
        .unwrap();
    let o = last.origin.as_ref().unwrap();
    assert_eq!(o.source(), Some(ClipSource::Bandcamp));
    assert_eq!(o.release, Some(1001), "still the Discogs record's");
    assert_eq!(o.youtube_clip, "LASTlight01");
    assert_eq!(message(&rig), "Dig: 1 track found on Bandcamp");
    // The guessed Bandcamp's listing and the one album with LT-012 in its address.
    let read = fakes.fetcher.pages_read();
    assert_eq!(
        read,
        [
            format!("{LOWTIDE}/music"),
            format!("{LOWTIDE}/album/glasshouse-ep-lt-012")
        ]
    );
}

#[test]
fn a_guessed_bandcamp_that_does_not_exist_is_asked_once() {
    let fakes = Fakes::new();
    fakes.fetcher.fail("LASTlight01", u32::MAX);
    fakes.fetcher.fail("LUMENremix1", u32::MAX);
    let mut rig = rig("bc-fallback-missing", &fakes);
    let c = crate_with(&mut rig, "Dig");
    rig.app.dig_paste(RELEASE);
    rig.until(
        |r| {
            r.app.crates.get(c).is_some_and(|p| {
                p.entries()
                    .iter()
                    .filter(|e| e.status == EntryStatus::Unavailable(UnavailableKind::ClipFailed))
                    .count()
                    == 2
            })
        },
        "two clips fail",
    );
    rig.until(|r| !r.app.bandcamp_looking(c), "the look is over");
    for _ in 0..20 {
        rig.pump();
    }
    let read = fakes.fetcher.pages_read();
    assert_eq!(
        read,
        [format!("{LOWTIDE}/music")],
        "one listing for both, then never again"
    );
    assert_eq!(rig.app.crates.get(c).unwrap().len(), 3, "nothing added");
}
