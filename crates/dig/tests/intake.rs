//! The intake worker, step by step against recorded Discogs JSON: listings before details,
//! focus first, resuming after a restart, offline and back, and wantlist changes.

use std::sync::Arc;

use dig::clock::FakeClock;
use dig::discogs::cache::DiskCache;
use dig::discogs::client::{ApiError, Client};
use dig::discogs::model::RecordKey::{self, Master, Release};
use dig::discogs::transport::{FakeTransport, Fault, Method};
use dig::discogs::url::parse;
use dig::intake::{Command, Event, Intake, Outcome};
use dig::jobs::Filters;

const CRATE: u64 = 7;

fn fixtures() -> String {
    format!("{}/tests/fixtures/discogs", env!("CARGO_MANIFEST_DIR"))
}

fn temp(name: &str) -> platform::testing::TestDir {
    platform::testing::TestDir::new(&format!("dig-intake-{name}"))
}

fn intake(t: &Arc<FakeTransport>, token: bool, config: Option<std::path::PathBuf>) -> Intake {
    let client = Client::new(
        t.clone(),
        Arc::new(FakeClock::default()),
        token.then(|| "tok".to_owned()),
        DiskCache::default(),
    );
    Intake::new(client, config).with_now(|| 1_790_000_000)
}

fn send(i: &mut Intake, url: &str) {
    i.handle(Command::Send {
        page: parse(url).unwrap(),
        target: CRATE,
        filters: Filters::default(),
    });
}

/// Steps until idle (or `max` steps), returning every event.
fn run(i: &mut Intake, max: usize) -> Vec<Event> {
    let mut all = Vec::new();
    for _ in 0..max {
        let worked = i.step();
        all.extend(i.take_events());
        if !worked {
            break;
        }
    }
    all
}

fn records(events: &[Event]) -> Vec<(RecordKey, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Record(_, info, o) => Some((info.key, o.clone())),
            _ => None,
        })
        .collect()
}

fn listed(events: &[Event]) -> Vec<RecordKey> {
    events
        .iter()
        .flat_map(|e| match e {
            Event::Listed(_, items) => items.iter().map(|l| l.key).collect(),
            _ => Vec::new(),
        })
        .collect()
}

fn clips(o: &Outcome) -> Vec<String> {
    match o {
        Outcome::Clips(e) => e.iter().map(|e| e.clip.clone()).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn a_label_lists_every_record_before_any_details_then_expands_them() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, true, None);
    send(&mut i, "https://www.discogs.com/label/12345-Lowtide-Tapes");
    let ev = run(&mut i, 100);
    assert!(matches!(&ev[0], Event::Started(j) if j.name == "Label: Lowtide Tapes"));
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::Named(j) if j.name == "Label: Lowtide Tapes"))
    );
    // The CD-only release is known from the listing and never listed.
    assert_eq!(
        listed(&ev),
        [
            Release(1001),
            Release(1003),
            Release(1004),
            Release(1005),
            Release(1006)
        ]
    );
    let first_record = ev
        .iter()
        .position(|e| matches!(e, Event::Record(..)))
        .unwrap();
    let last_listed = ev
        .iter()
        .rposition(|e| matches!(e, Event::Listed(..)))
        .unwrap();
    assert!(last_listed < first_record, "all listings come first");

    let r = records(&ev);
    let keys: Vec<RecordKey> = r.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys,
        [
            Release(1001),
            Release(1003),
            Release(1004),
            Release(1005),
            Release(1006)
        ]
    );
    assert_eq!(
        clips(&r[0].1),
        ["GLASShouse1", "LUMENremix1", "LASTlight01"]
    );
    assert_eq!(r[1].1, Outcome::Unavailable("no clip".into()));
    assert_eq!(
        r[3].1,
        Outcome::Excluded,
        "a CD, known only from its details"
    );
    assert!(matches!(ev.last(), Some(Event::Finished(_))));
    let progress = ev.iter().rev().find_map(|e| match e {
        Event::Progress(_, d, t) => Some((*d, *t)),
        _ => None,
    });
    assert_eq!(progress, Some((6, 6)));
    assert!(i.jobs().is_empty());
    // For-sale numbers in the account's currency.
    let info = ev.iter().find_map(|e| match e {
        Event::Record(_, info, _) if info.key == Release(1001) => Some(info.clone()),
        _ => None,
    });
    let info = info.unwrap();
    let fs = info.for_sale.unwrap();
    assert_eq!(
        (fs.count, fs.lowest, fs.currency.as_str()),
        (6, Some(9.0), "EUR")
    );
    // The album's name and its primary image's thumbnail, from the release; the listing's
    // own thumbnail comes earlier, with the listed record.
    assert_eq!(info.title, "Glasshouse EP");
    assert_eq!(
        info.cover,
        "https://i.discogs.com/fake/R-1001-front-150.jpeg"
    );
    let listed_cover = ev.iter().find_map(|e| match e {
        Event::Listed(_, items) => items.iter().find(|l| l.key == Release(1001)).cloned(),
        _ => None,
    });
    assert_eq!(
        listed_cover.unwrap().cover,
        "https://i.discogs.com/fake/R-1001-thumb.jpeg"
    );
}

#[test]
fn a_shop_item_expands_exactly_like_its_release() {
    let run_one = |url: &str| {
        let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
        let mut i = intake(&t, true, None);
        send(&mut i, url);
        let ev = run(&mut i, 100);
        let named: Vec<String> = ev
            .iter()
            .filter_map(|e| match e {
                Event::Named(j) => Some(j.name.clone()),
                _ => None,
            })
            .collect();
        (named, listed(&ev), records(&ev).len())
    };
    let item = run_one("https://www.discogs.com/shop/item/3923678974");
    let release = run_one("https://www.discogs.com/release/1001");
    assert_eq!(item, release);
    assert_eq!(item.0, ["Release: Nightcraft – Glasshouse EP"]);
}

#[test]
fn the_collection_sync_waits_for_the_digs_to_finish() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    t.route(
        "/users/digger/collection/folders/0/releases?sort=added&sort_order=desc&page=1&per_page=100",
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "items": 1},
            "releases": [{"id": 1003, "instance_id": 7,
                          "basic_information": {"id": 1003, "master_id": 0, "year": 2016,
                                                "labels": [{"catno": "LT-003"}]}}]}"#,
    );
    let mut i = intake(&t, true, None);
    send(&mut i, "https://www.discogs.com/label/12345");
    i.handle(Command::SyncCollection(None));
    i.handle(Command::ResolveShopItem(3923678974));
    let ev = run(&mut i, 200);
    let paths = t.paths();
    let last_dig = paths
        .iter()
        .rposition(|p| p.starts_with("/labels/") || p.starts_with("/releases/"))
        .unwrap();
    let item = paths
        .iter()
        .position(|p| p.starts_with("/marketplace/"))
        .unwrap();
    let sync = paths
        .iter()
        .position(|p| p.contains("/collection/"))
        .unwrap();
    assert!(
        last_dig < item && item < sync,
        "digs, then the item, then the collection: {paths:?}"
    );
    assert!(ev.contains(&Event::ShopItem(3923678974, Some(1001))));
    let c = ev
        .iter()
        .find_map(|e| match e {
            Event::Collection(Ok(c)) => Some(c.clone()),
            _ => None,
        })
        .expect("synced");
    assert_eq!(c.len(), 1);
    assert_eq!(c.username, "digger");
}

#[test]
fn without_a_token_the_collection_is_never_asked_for() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, false, None);
    i.handle(Command::SyncCollection(None));
    let ev = run(&mut i, 10);
    assert!(ev.contains(&Event::Collection(Err(ApiError::TokenNeeded))));
    assert!(!t.paths().iter().any(|p| p.contains("/collection/")));
}

#[test]
fn details_follow_the_focus() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, true, None);
    send(&mut i, "https://www.discogs.com/label/12345");
    // Name and three listing pages.
    let mut ev = run(&mut i, 4);
    assert_eq!(listed(&ev).len(), 5);
    assert!(records(&ev).is_empty());
    // The user is on the fourth listed record: from there on, wrapping around.
    i.handle(Command::Focus {
        target: CRATE,
        order: vec![
            Release(1005),
            Release(1006),
            Release(1001),
            Release(1003),
            Release(1004),
        ],
    });
    ev.extend(run(&mut i, 2));
    // Then moves back to the first.
    i.handle(Command::Focus {
        target: CRATE,
        order: vec![Release(1001), Release(1003), Release(1004)],
    });
    ev.extend(run(&mut i, 100));
    let keys: Vec<RecordKey> = records(&ev).iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys,
        [
            Release(1005),
            Release(1006),
            Release(1001),
            Release(1003),
            Release(1004)
        ]
    );
}

#[test]
fn a_half_done_job_resumes_after_a_restart_without_duplicates() {
    let config = temp("resume");
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut first = intake(&t, true, Some(config.to_path_buf()));
    send(&mut first, "https://www.discogs.com/label/12345");
    // Name, 3 listing pages, 2 records; then quit.
    let before = run(&mut first, 6);
    assert_eq!(records(&before).len(), 2);
    first.flush();
    drop(first);
    let requests = t.count();

    let mut second = intake(&t, true, Some(config.to_path_buf()));
    assert_eq!(second.jobs().len(), 1);
    assert!(
        run(&mut second, 10).is_empty(),
        "nothing runs until its crate is shown"
    );
    assert_eq!(t.count(), requests, "and nothing is requested");
    second.handle(Command::CrateActive(CRATE));
    let after = run(&mut second, 100);
    assert!(listed(&after).is_empty(), "the listing isn't fetched again");
    let mut keys: Vec<RecordKey> = records(&before)
        .iter()
        .chain(records(&after).iter())
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(keys.len(), 5);
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 5, "each record once");
    assert!(matches!(after.last(), Some(Event::Finished(_))));
    assert!(
        dig::jobs::Jobs::load(&config).jobs.is_empty(),
        "the finished job is forgotten"
    );
}

#[test]
fn offline_pauses_and_resumes_where_it_stopped() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, true, None);
    send(&mut i, "https://www.discogs.com/label/12345");
    let mut ev = run(&mut i, 5); // name, 3 pages, the first record
    assert_eq!(records(&ev).len(), 1);
    t.set_offline(true);
    ev.extend(run(&mut i, 3));
    assert!(i.is_offline());
    assert_eq!(
        ev.iter()
            .filter(|e| matches!(e, Event::Offline(true)))
            .count(),
        1,
        "said once"
    );
    assert_eq!(records(&ev).len(), 1, "nothing is lost or skipped");
    t.set_offline(false);
    ev.extend(run(&mut i, 100));
    assert!(ev.iter().any(|e| matches!(e, Event::Offline(false))));
    let keys: Vec<RecordKey> = records(&ev).iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys,
        [
            Release(1001),
            Release(1003),
            Release(1004),
            Release(1005),
            Release(1006)
        ]
    );
}

#[test]
fn a_missing_page_fails_the_send() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, false, None);
    send(&mut i, "https://www.discogs.com/label/999");
    let ev = run(&mut i, 10);
    assert!(matches!(
        ev.last(),
        Some(Event::Failed(_, ApiError::NotFound))
    ));
    assert!(listed(&ev).is_empty());
    assert!(i.jobs().is_empty());
}

#[test]
fn artists_masters_and_remix_credits() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, true, None);
    send(&mut i, "https://www.discogs.com/artist/4242-Nightcraft");
    let r = records(&run(&mut i, 100));
    let keys: Vec<RecordKey> = r.iter().map(|(k, _)| *k).collect();
    assert_eq!(keys, [Master(98765), Release(2001), Release(1004)]);
    assert_eq!(clips(&r[0].1), ["GLASShouse1", "TIDALpull01"]);
    assert_eq!(
        clips(&r[1].1),
        ["CURRENTSnc1"],
        "only the clip naming the artist"
    );
}

#[test]
fn a_rejected_token_carries_on_without_one() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    t.route("/oauth/identity", 401, "{}");
    let mut i = intake(&t, true, None);
    send(&mut i, "https://discogs.com/release/1001");
    let ev = run(&mut i, 10);
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::Identity(Err(ApiError::TokenRejected))))
    );
    assert_eq!(records(&ev).len(), 1);
    let last = t.log().last().unwrap().clone();
    assert!(last.0.token.is_none(), "later requests go without it");
}

fn wantlist_result(ev: &[Event]) -> Result<bool, ApiError> {
    ev.iter()
        .find_map(|e| match e {
            Event::Wantlist { result, .. } => Some(result.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn keep_adds_to_the_wantlist_unless_it_is_there_already() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, true, None);
    i.handle(Command::Keep(1004));
    assert_eq!(
        wantlist_result(&i.take_events()),
        Ok(false),
        "already on it"
    );
    i.handle(Command::Keep(1001));
    assert_eq!(wantlist_result(&i.take_events()), Ok(true));
    i.handle(Command::Unkeep(1001));
    assert_eq!(wantlist_result(&i.take_events()), Ok(true));
    let changes: Vec<(Method, String)> = t
        .log()
        .into_iter()
        .filter(|(r, _)| r.method != Method::Get)
        .map(|(r, _)| (r.method, r.path))
        .collect();
    assert_eq!(
        changes,
        [
            (Method::Put, "/users/digger/wants/1001".to_owned()),
            (Method::Delete, "/users/digger/wants/1001".to_owned())
        ]
    );
    let reads = t.paths().iter().filter(|p| p.contains("/wants?")).count();
    assert_eq!(reads, 1, "the wantlist is read once per session");

    t.fault(Fault::Network);
    i.handle(Command::Keep(1003));
    assert_eq!(wantlist_result(&i.take_events()), Err(ApiError::Offline));

    let mut anon = intake(&t, false, None);
    anon.handle(Command::Keep(1001));
    assert_eq!(
        wantlist_result(&anon.take_events()),
        Err(ApiError::TokenNeeded)
    );
}

#[test]
fn a_token_is_checked_before_it_is_used() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, false, None);
    i.handle(Command::CheckToken("good".into()));
    let ev = i.take_events();
    assert!(
        matches!(&ev[0], Event::TokenChecked(tok, Ok(id)) if tok == "good" && id.username == "digger")
    );
    t.route("/oauth/identity", 401, "{}");
    i.handle(Command::CheckToken("bad".into()));
    let ev = i.take_events();
    assert!(matches!(
        &ev[0],
        Event::TokenChecked(_, Err(ApiError::TokenRejected))
    ));
}

#[test]
fn a_stale_release_is_refreshed() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, true, None);
    i.handle(Command::Refresh(1006));
    let ev = i.take_events();
    assert!(
        ev.iter().any(
            |e| matches!(e, Event::ForSale(1006, fs) if fs.count == 1 && fs.lowest == Some(4.5))
        )
    );
}

/// Against the live API, by hand: `cargo test -p dig --test intake -- --ignored`, with
/// `WINAMP_DISCOGS_TOKEN` set to use a token.
#[test]
#[ignore]
fn a_real_release_expands() {
    let client = Client::new(
        Arc::new(dig::discogs::transport::UreqTransport::default()),
        Arc::new(dig::clock::RealClock::default()),
        std::env::var("WINAMP_DISCOGS_TOKEN").ok(),
        DiskCache::default(),
    );
    let mut i = Intake::new(client, None);
    i.handle(Command::Send {
        page: parse("https://www.discogs.com/release/1-The-Persuader-Stockholm").unwrap(),
        target: CRATE,
        filters: Filters {
            vinyl_only: false,
            skip_passed: false,
        },
    });
    let ev = run(&mut i, 10);
    let r = records(&ev);
    assert_eq!(r.len(), 1, "{ev:?}");
    assert_eq!(r[0].0, Release(1));
}

#[test]
fn a_stale_cover_is_looked_up_again_once() {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut i = intake(&t, true, None);
    i.handle(Command::RefreshCover(Release(1001)));
    let ev = i.take_events();
    let cover = ev.iter().find_map(|e| match e {
        Event::Cover(k, url) => Some((*k, url.clone())),
        _ => None,
    });
    assert_eq!(
        cover,
        Some((
            Release(1001),
            "https://i.discogs.com/fake/R-1001-front-150.jpeg".to_owned()
        ))
    );
    let releases = t
        .paths()
        .iter()
        .filter(|p| p.starts_with("/releases/1001"))
        .count();
    assert_eq!(releases, 1, "one request for the release's data");
    // A record Discogs doesn't know: no address.
    i.handle(Command::RefreshCover(Release(424242)));
    assert!(
        i.take_events()
            .iter()
            .any(|e| matches!(e, Event::Cover(Release(424242), url) if url.is_empty()))
    );
}
