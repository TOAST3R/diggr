//! Listings and record details for every kind of page, against recorded Discogs JSON.

use std::sync::Arc;

use dig::clock::FakeClock;
use dig::discogs::cache::DiskCache;
use dig::discogs::client::{ApiError, Client};
use dig::discogs::expand::{listing, page_name, record};
use dig::discogs::model::{RecordKey, Role};
use dig::discogs::transport::FakeTransport;
use dig::discogs::url::parse;

const NOW: u64 = 1_790_000_000;

fn fixtures() -> String {
    format!("{}/tests/fixtures/discogs", env!("CARGO_MANIFEST_DIR"))
}

fn client(token: bool, cache: Option<&std::path::Path>) -> (Client, Arc<FakeTransport>) {
    let t = Arc::new(FakeTransport::with_fixtures(fixtures()));
    let mut c = Client::new(
        t.clone(),
        Arc::new(FakeClock::default()),
        token.then(|| "tok".to_owned()),
        DiskCache::new(cache),
    );
    if token {
        c.check_identity().unwrap();
    }
    (c, t)
}

fn keys(items: &[dig::discogs::model::Listed]) -> Vec<RecordKey> {
    items.iter().map(|l| l.key).collect()
}

#[test]
fn a_label_lists_all_its_pages_with_formats() {
    let (mut c, t) = client(true, None);
    let page = parse("https://www.discogs.com/label/12345-Lowtide-Tapes").unwrap();
    assert_eq!(
        page_name(&mut c, &page, NOW).unwrap(),
        "Label: Lowtide Tapes"
    );
    let first = listing(&mut c, &page, 1, NOW).unwrap();
    assert_eq!((first.pages, first.total), (3, 6));
    let mut all = first.items;
    for n in 2..=3 {
        all.extend(listing(&mut c, &page, n, NOW).unwrap().items);
    }
    use RecordKey::Release as R;
    assert_eq!(
        keys(&all),
        [R(1001), R(1002), R(1003), R(1004), R(1005), R(1006)]
    );
    let vinyl: Vec<Option<bool>> = all.iter().map(|l| l.vinyl).collect();
    assert_eq!(
        vinyl,
        [
            Some(true),
            Some(false),
            Some(true),
            Some(true),
            None,
            Some(true)
        ],
        "from the listing's format, unknown when it has none"
    );
    assert_eq!((all[0].catno.as_str(), all[0].year), ("LT-012", Some(1994)));
    assert!(
        t.paths()
            .iter()
            .any(|p| p == "/labels/12345/releases?page=3&per_page=100")
    );
    // Unknown from the listing: the record's details decide.
    assert!(!record(&mut c, R(1005), false, NOW).unwrap().vinyl);
}

#[test]
fn an_artist_lists_main_and_remix_credits_oldest_first() {
    let (mut c, t) = client(true, None);
    let page = parse("https://www.discogs.com/artist/4242-Nightcraft").unwrap();
    assert_eq!(page_name(&mut c, &page, NOW).unwrap(), "Artist: Nightcraft");
    let p = listing(&mut c, &page, 1, NOW).unwrap();
    assert_eq!(
        keys(&p.items),
        [
            RecordKey::Master(98765),
            RecordKey::Release(2001),
            RecordKey::Release(1004)
        ],
        "appearances are left out"
    );
    let roles: Vec<Role> = p.items.iter().map(|l| l.role).collect();
    assert_eq!(roles, [Role::Main, Role::Remix, Role::Main]);
    assert!(
        t.paths().contains(
            &"/artists/4242/releases?sort=year&sort_order=asc&page=1&per_page=100".into()
        )
    );
}

#[test]
fn a_master_takes_its_main_release_details() {
    let (mut c, _) = client(true, None);
    let page = parse("https://www.discogs.com/master/98765-Nightcraft-Glasshouse-EP").unwrap();
    assert_eq!(listing(&mut c, &page, 1, NOW).unwrap().items.len(), 1);
    assert_eq!(
        page_name(&mut c, &page, NOW).unwrap(),
        "Master: Nightcraft – Glasshouse EP"
    );
    let r = record(&mut c, RecordKey::Master(98765), false, NOW).unwrap();
    assert_eq!(r.clips.len(), 2, "the master's own clips");
    assert_eq!(
        (r.release, r.catno.as_str(), r.year),
        (Some(1001), "LT-012", Some(1994))
    );
    let fs = r.for_sale.unwrap();
    assert_eq!(
        (fs.count, fs.lowest, fs.currency.as_str()),
        (6, Some(9.0), "EUR")
    );
    assert!(r.vinyl);
}

#[test]
fn a_release_page_and_its_name() {
    let (mut c, _) = client(false, None);
    let page = parse("https://discogs.com/release/1001").unwrap();
    assert_eq!(
        keys(&listing(&mut c, &page, 1, NOW).unwrap().items),
        [RecordKey::Release(1001)]
    );
    assert_eq!(
        page_name(&mut c, &page, NOW).unwrap(),
        "Release: Nightcraft – Glasshouse EP"
    );
    let r = record(&mut c, RecordKey::Release(1001), false, NOW).unwrap();
    assert_eq!(r.for_sale.unwrap().currency, "USD", "without a token");
}

#[test]
fn a_shop_item_is_dug_as_the_release_it_sells() {
    let dir = platform::testing::TestDir::new("dig-expand-shop-item");
    let (mut c, t) = client(false, Some(dir.path()));
    let page = parse("https://www.discogs.com/shop/item/3923678974").unwrap();
    assert_eq!(
        keys(&listing(&mut c, &page, 1, NOW).unwrap().items),
        [RecordKey::Release(1001)]
    );
    assert_eq!(
        page_name(&mut c, &page, NOW).unwrap(),
        "Release: Nightcraft – Glasshouse EP",
        "named after the release"
    );
    let lookups = |t: &FakeTransport| {
        t.paths()
            .iter()
            .filter(|p| p.starts_with("/marketplace/listings/"))
            .count()
    };
    assert_eq!(lookups(&t), 1, "the item is looked up once");
    // Sent again, even after a restart: the release is known without a request.
    let (mut again, t2) = client(false, Some(dir.path()));
    listing(&mut again, &page, 1, NOW).unwrap();
    assert_eq!(lookups(&t2), 0);
}

#[test]
fn a_shop_item_discogs_does_not_know_is_not_found() {
    let (mut c, _) = client(false, None);
    let page = parse("https://www.discogs.com/sell/item/1").unwrap();
    assert!(matches!(
        listing(&mut c, &page, 1, NOW),
        Err(ApiError::NotFound)
    ));
}

/// digger's collection: releases 1001 and 1003 (served only where a test routes it, so
/// other tests' users own nothing).
const COLLECTION_PAGE: &str =
    "/users/digger/collection/folders/0/releases?sort=added&sort_order=desc&page=1&per_page=100";
const COLLECTION: &str = r#"{"pagination": {"page": 1, "pages": 1, "per_page": 100, "items": 2}, "releases": [{"id": 1001, "instance_id": 11, "basic_information": {"id": 1001, "master_id": 0, "title": "Glasshouse EP", "year": 1994, "formats": [{"name": "Vinyl", "qty": "1"}], "labels": [{"name": "Lowtide Tapes", "catno": "LT-012", "id": 12345}], "artists": [{"name": "Nightcraft", "anv": "", "join": "", "id": 4242}]}}, {"id": 1003, "instance_id": 12, "basic_information": {"id": 1003, "master_id": 0, "title": "Undertow", "year": 1995, "formats": [{"name": "Vinyl", "qty": "1"}], "labels": [{"name": "Lowtide Tapes", "catno": "LT-013", "id": 12345}], "artists": [{"name": "Nightcraft", "anv": "", "join": "", "id": 4242}]}}]}"#;

#[test]
fn a_collection_lists_its_releases() {
    let (mut c, t) = client(true, None);
    t.route(COLLECTION_PAGE, 200, COLLECTION);
    let page = parse("https://www.discogs.com/user/digger/collection").unwrap();
    assert_eq!(
        keys(&listing(&mut c, &page, 1, NOW).unwrap().items),
        [RecordKey::Release(1001), RecordKey::Release(1003)]
    );
    assert_eq!(page_name(&mut c, &page, NOW).unwrap(), "Collection: digger");
}

#[test]
fn a_wantlist_and_a_list() {
    let (mut c, _) = client(true, None);
    let wl = parse("https://www.discogs.com/wantlist?user=digger").unwrap();
    let p = listing(&mut c, &wl, 1, NOW).unwrap();
    use RecordKey::*;
    assert_eq!(
        keys(&p.items),
        [Release(1004), Release(1002), Release(1006)]
    );
    let vinyl: Vec<Option<bool>> = p.items.iter().map(|l| l.vinyl).collect();
    assert_eq!(vinyl, [Some(true), Some(false), Some(true)]);
    assert_eq!(page_name(&mut c, &wl, NOW).unwrap(), "Wantlist: digger");

    let list = parse("https://www.discogs.com/lists/Deep-Tapes/555").unwrap();
    let p = listing(&mut c, &list, 1, NOW).unwrap();
    assert_eq!(
        keys(&p.items),
        [Release(1006), Master(98765)],
        "releases and masters only"
    );
    assert_eq!(p.items[0].title, "Last Light");
    assert_eq!(page_name(&mut c, &list, NOW).unwrap(), "List: Deep Tapes");
}

#[test]
fn missing_and_private_pages() {
    let (mut c, _) = client(true, None);
    let page = parse("https://www.discogs.com/label/999-Nobody").unwrap();
    assert_eq!(page_name(&mut c, &page, NOW), Err(ApiError::NotFound));
    let (mut anon, t) = client(false, None);
    t.route("/users/secret/wants?page=1&per_page=100", 401, "{}");
    let wl = parse("https://www.discogs.com/user/secret/wantlist").unwrap();
    assert_eq!(listing(&mut anon, &wl, 1, NOW), Err(ApiError::TokenNeeded));
    t.route("/users/secret/wants?page=1&per_page=100", 403, "{}");
    assert_eq!(listing(&mut anon, &wl, 1, NOW), Err(ApiError::Private));
}

#[test]
fn the_disk_cache_saves_requests() {
    let dir = platform::testing::TestDir::new("dig-expand-cache");
    let page = parse("https://www.discogs.com/label/12345").unwrap();
    {
        let (mut c, _) = client(true, Some(&dir));
        listing(&mut c, &page, 1, NOW).unwrap();
        record(&mut c, RecordKey::Release(1001), false, NOW).unwrap();
    }
    let (mut c, t) = client(true, Some(&dir));
    let before = t.count();
    listing(&mut c, &page, 1, NOW + 3600).unwrap();
    record(&mut c, RecordKey::Release(1001), false, NOW + 86_400 * 30).unwrap();
    assert_eq!(
        t.count(),
        before,
        "a day-old listing and a month-old release: no request"
    );
    listing(&mut c, &page, 1, NOW + 86_400).unwrap();
    assert_eq!(
        t.count(),
        before + 1,
        "a listing is fetched again after 24 h"
    );
    let r = record(&mut c, RecordKey::Release(1001), true, NOW + 86_400 * 30).unwrap();
    assert_eq!(t.count(), before + 2, "fresh for-sale numbers ask again");
    assert_eq!(r.for_sale.unwrap().fetched_at, NOW + 86_400 * 30);
}
