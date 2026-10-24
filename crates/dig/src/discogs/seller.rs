//! A seller's stock: their inventory read as copies for sale, the criteria that narrow it,
//! and the first Top Sellers list taken from the user's purchases.
//!
//! Discogs applies the search text (`q`) and the order (`sort=listed`), and serves at most 100
//! pages of another user's inventory (10,000 copies). Its search matches the whole release
//! (genres, styles, format, year), so every criterion is a search word and a count is one
//! request: nothing waits for pages of listings to be read. Parsing is lenient: a field Discogs leaves out is "unknown", never an
//! error, because these answers are only partly documented.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::{ApiError, Client, path_segment};
use super::model::{Format, Listed, RecordKey, Role, clean_artist, formats_from_listing, thumb};

/// A crate is dug only when at most this many copies match.
pub const LIMIT: usize = 1000;
/// Inventory pages Discogs serves for someone else's stock.
pub const MAX_PAGES: u32 = 100;
pub const PER_PAGE: u32 = 100;

/// Media condition, worst to best, as Discogs grades it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Condition {
    Poor,
    Fair,
    Good,
    GoodPlus,
    VeryGood,
    VeryGoodPlus,
    NearMint,
    Mint,
}

impl Condition {
    pub const ALL: [Condition; 8] = [
        Condition::Poor,
        Condition::Fair,
        Condition::Good,
        Condition::GoodPlus,
        Condition::VeryGood,
        Condition::VeryGoodPlus,
        Condition::NearMint,
        Condition::Mint,
    ];

    /// "VG+", as shown on rows.
    pub fn abbr(self) -> &'static str {
        match self {
            Condition::Poor => "P",
            Condition::Fair => "F",
            Condition::Good => "G",
            Condition::GoodPlus => "G+",
            Condition::VeryGood => "VG",
            Condition::VeryGoodPlus => "VG+",
            Condition::NearMint => "NM",
            Condition::Mint => "M",
        }
    }

    /// From Discogs' text, "Very Good Plus (VG+)" or "Near Mint (NM or M-)"; `None` for a
    /// sleeve's "Generic", "Not Graded" or "No Cover", and for anything unknown.
    pub fn parse(text: &str) -> Option<Self> {
        let name = text.split('(').next().unwrap_or("").trim();
        Some(match name.to_ascii_lowercase().as_str() {
            "mint" => Condition::Mint,
            "near mint" => Condition::NearMint,
            "very good plus" => Condition::VeryGoodPlus,
            "very good" => Condition::VeryGood,
            "good plus" => Condition::GoodPlus,
            "good" => Condition::Good,
            "fair" => Condition::Fair,
            "poor" => Condition::Poor,
            _ => return None,
        })
    }
}

/// A grade as shown: the abbreviation when known, else Discogs' own words ("Generic").
pub fn grade_text(text: &str) -> String {
    match Condition::parse(text) {
        Some(c) => c.abbr().to_owned(),
        None => text.trim().to_owned(),
    }
}

/// What narrows a seller's stock, all applied by Discogs: a genre or a style, a format, a year
/// and search text (see [`Criteria::discogs_query`]), and "newest N".
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Criteria {
    /// A Discogs genre ("Electronic"), searched for as a phrase.
    pub genre: Option<String>,
    /// A Discogs style ("Deep House"), searched for as a phrase.
    pub style: Option<String>,
    /// The user's own search text.
    pub query: String,
    /// Vinyl, CD or Cassette (the formats Discogs' search tells apart), as a search word.
    pub format: Option<Format>,
    /// A release year, as a search word.
    pub year: Option<u16>,
    /// Only the N most recently listed copies.
    pub newest: Option<u32>,
}

/// The formats Narrow down offers: Discogs' search finds them by name.
pub const SEARCHED_FORMATS: [Format; 3] = [Format::Vinyl, Format::Cd, Format::Cassette];

impl Criteria {
    /// The search Discogs runs: the genre and style as quoted phrases, then the format, the
    /// year and the search text (`"Deep House" vinyl 2025 ostgut`). Discogs matches them
    /// anywhere in the release (its styles, format and year included) and wants every part.
    pub fn discogs_query(&self) -> String {
        let quoted = |s: &Option<String>| {
            s.as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| format!("\"{s}\""))
        };
        let format = self
            .format
            .filter(|f| SEARCHED_FORMATS.contains(f))
            .map(|f| f.name().to_lowercase());
        [
            quoted(&self.genre),
            quoted(&self.style),
            format,
            self.year.map(|y| y.to_string()),
        ]
        .into_iter()
        .flatten()
        .chain(Some(self.query.trim().to_owned()).filter(|q| !q.is_empty()))
        .collect::<Vec<_>>()
        .join(" ")
    }

    /// Pages to read for these criteria, given how many the seller has.
    pub fn pages(&self, total_pages: u32) -> u32 {
        let wanted = self.newest.map_or(MAX_PAGES, |n| n.div_ceil(PER_PAGE));
        total_pages.min(MAX_PAGES).min(wanted)
    }
}

/// One copy for sale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Copy {
    pub listing: u64,
    pub release: u64,
    pub price: f64,
    /// ISO code, e.g. "EUR".
    pub currency: String,
    /// Discogs' grade text ("Very Good Plus (VG+)").
    pub media: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sleeve: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ships_from: String,
    /// When it was listed, as Discogs gives it ("2026-02-06T08:12:00-08:00").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub posted: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comments: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formats: Vec<Format>,
    /// No longer listed at the last refresh, or refused by the cart as not for sale.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sold: bool,
    /// The price before the last refresh changed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was: Option<f64>,
}

impl Copy {
    /// The listing on discogs.com.
    pub fn url(&self) -> String {
        format!("https://www.discogs.com/sell/item/{}", self.listing)
    }

    /// "VG+ / VG", or just the media grade without a sleeve grade.
    pub fn grades(&self) -> String {
        let media = grade_text(&self.media);
        match self.sleeve.trim() {
            "" => media,
            s => format!("{media} / {}", grade_text(s)),
        }
    }
}

/// An inventory listing as a copy, and as the record it sells.
pub fn copy_from_listing(item: &Value) -> Option<(Copy, Listed)> {
    let listing = item["id"].as_u64()?;
    let r = &item["release"];
    let release = r["id"].as_u64()?;
    let price = item["price"]["value"].as_f64()?;
    let text = |v: &Value| v.as_str().unwrap_or("").trim().to_owned();
    let formats = formats_from_listing(r["format"].as_str().unwrap_or(""));
    let media = text(&item["condition"]);
    let copy = Copy {
        listing,
        release,
        price,
        currency: text(&item["price"]["currency"]),
        condition: Condition::parse(&media),
        media,
        sleeve: text(&item["sleeve_condition"]),
        ships_from: text(&item["ships_from"]),
        posted: text(&item["posted"]),
        comments: text(&item["comments"]),
        formats: formats.clone(),
        sold: false,
        was: None,
    };
    let listed = Listed {
        key: RecordKey::Release(release),
        artist: clean_artist(r["artist"].as_str().unwrap_or("")),
        title: text(&r["title"]),
        label: text(&r["label"]),
        catno: text(&r["catalog_number"]),
        year: r["year"]
            .as_u64()
            .and_then(|y| u16::try_from(y).ok())
            .filter(|&y| y > 0),
        formats,
        old_vinyl: None,
        role: Role::Main,
        cover: thumb(r["thumbnail"].as_str().unwrap_or("")),
    };
    Some((copy, listed))
}

/// One page of a seller's inventory, newest first, with Discogs' search text applied.
pub fn inventory_path(seller: &str, query: &str, page: u32, per_page: u32) -> String {
    let mut p = format!(
        "/users/{}/inventory?status=For%20Sale&sort=listed&sort_order=desc&page={page}&per_page={per_page}",
        path_segment(seller)
    );
    let q = query.trim();
    if !q.is_empty() {
        p.push_str("&q=");
        p.push_str(&path_segment(q));
    }
    p
}

/// A page of the inventory: the copies that pass the criteria and their records, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryPage {
    pub copies: Vec<Copy>,
    pub listed: Vec<Listed>,
    /// Pages to read for these criteria.
    pub pages: u32,
    /// Copies for sale matching the search text, as Discogs counts them.
    pub total: usize,
}

/// Parses one inventory answer, keeping the copies for sale.
pub fn inventory_page(v: &Value, criteria: &Criteria) -> InventoryPage {
    let mut copies = Vec::new();
    let mut listed = Vec::new();
    for item in v["listings"].as_array().into_iter().flatten() {
        if item["status"].as_str().is_some_and(|s| s != "For Sale") {
            continue;
        }
        if let Some((c, l)) = copy_from_listing(item) {
            copies.push(c);
            listed.push(l);
        }
    }
    let p = &v["pagination"];
    let total = p["items"].as_u64().unwrap_or(copies.len() as u64) as usize;
    InventoryPage {
        copies,
        listed,
        pages: criteria.pages(p["pages"].as_u64().unwrap_or(1).max(1) as u32),
        total,
    }
}

/// How many copies a seller has for sale (matching the search text): one request.
pub fn count(client: &mut Client, seller: &str, query: &str) -> Result<usize, ApiError> {
    let v = client.get_json(&inventory_path(seller, query, 1, 1))?;
    Ok(v["pagination"]["items"].as_u64().unwrap_or(0) as usize)
}

/// Copies a dig would bring in, before any listing is read: the count for the search text,
/// capped by "newest N" and by what Discogs serves.
pub fn reachable(total: usize, criteria: &Criteria) -> usize {
    let cap = (MAX_PAGES * PER_PAGE) as usize;
    let newest = criteria.newest.map_or(cap, |n| n as usize);
    total.min(cap).min(newest)
}

/// Orders the first Top Sellers list is taken from, newest first.
pub const PURCHASES: usize = 100;
/// Sellers in the first list.
pub const FIRST_LIST: usize = 10;

/// A seller the user bought from, as the purchases rank them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked {
    pub username: String,
    pub id: u64,
    pub orders: usize,
    /// The latest order's date, as Discogs gives it (ISO 8601, so it sorts as text).
    pub last: String,
}

/// Sellers of the `PURCHASES` most recent orders, by number of orders, then latest order.
pub fn rank_purchases(v: &Value) -> Vec<Ranked> {
    let mut orders: Vec<&Value> = v["items"].as_array().into_iter().flatten().collect();
    orders.sort_by(|a, b| b["created"].as_str().cmp(&a["created"].as_str()));
    let mut ranked: Vec<Ranked> = Vec::new();
    for o in orders.into_iter().take(PURCHASES) {
        let Some(name) = o["seller"]["username"].as_str().filter(|n| !n.is_empty()) else {
            continue;
        };
        let created = o["created"].as_str().unwrap_or("").to_owned();
        match ranked.iter_mut().find(|r| r.username == name) {
            Some(r) => {
                r.orders += 1;
                if created > r.last {
                    r.last = created;
                }
            }
            None => ranked.push(Ranked {
                username: name.to_owned(),
                id: o["seller"]["id"].as_u64().unwrap_or(0),
                orders: 1,
                last: created,
            }),
        }
    }
    ranked.sort_by(|a, b| b.orders.cmp(&a.orders).then_with(|| b.last.cmp(&a.last)));
    ranked
}

/// The first Top Sellers list: the best-ranked sellers of the user's recent purchases that
/// still have copies for sale, with how many. About 1 + 10 requests; a seller gone from
/// Discogs is skipped. `/purchases` isn't documented: its answer is read leniently.
pub fn first_list(client: &mut Client) -> Result<Vec<(Ranked, usize)>, ApiError> {
    let v = client.get_json(&format!(
        "/purchases?sort=created&sort_order=desc&page=1&per_page={PURCHASES}"
    ))?;
    let mut out = Vec::new();
    for r in rank_purchases(&v) {
        if out.len() == FIRST_LIST {
            break;
        }
        match count(client, &r.username, "") {
            Ok(0) | Err(ApiError::NotFound | ApiError::Private) => {}
            Ok(n) => out.push((r, n)),
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// One listing as Discogs sends it (fields as seen on decks.de, October 2026).
    pub fn listing_json(id: u64, release: u64, price: f64, format: &str, media: &str) -> Value {
        serde_json::json!({
            "id": id,
            "status": "For Sale",
            "price": {"value": price, "currency": "EUR"},
            "condition": media,
            "sleeve_condition": "Very Good (VG)",
            "ships_from": "Germany",
            "posted": "2026-02-06T08:12:00-08:00",
            "comments": "Plays fine",
            "uri": format!("https://www.discogs.com/sell/item/{id}"),
            "audio": false,
            "release": {
                "id": release,
                "artist": "Nightcraft (2)",
                "title": "Glasshouse EP",
                "format": format,
                "label": "Lowtide Tapes",
                "catalog_number": "LT-012",
                "year": 1994,
                "thumbnail": "https://i.discogs.com/x/thumb.jpg",
                "description": "Nightcraft - Glasshouse EP (12\")",
                "stats": {"community": {"in_wantlist": 5}}
            }
        })
    }

    fn page(items: Vec<Value>, total: u64, pages: u64) -> Value {
        serde_json::json!({
            "pagination": {"page": 1, "pages": pages, "per_page": 100, "items": total},
            "listings": items,
        })
    }

    #[test]
    fn a_listing_becomes_a_copy_and_its_record() {
        let v = listing_json(824756436, 123456, 15.05, "12\", EP", "Mint (M)");
        let (c, l) = copy_from_listing(&v).unwrap();
        assert_eq!((c.listing, c.release, c.price), (824756436, 123456, 15.05));
        assert_eq!(c.currency, "EUR");
        assert_eq!(c.condition, Some(Condition::Mint));
        assert_eq!(c.grades(), "M / VG");
        assert_eq!(c.ships_from, "Germany");
        assert_eq!(c.formats, [Format::Vinyl]);
        assert_eq!(c.url(), "https://www.discogs.com/sell/item/824756436");
        assert_eq!(l.key, RecordKey::Release(123456));
        assert_eq!(
            (l.artist.as_str(), l.title.as_str()),
            ("Nightcraft", "Glasshouse EP")
        );
        assert_eq!((l.catno.as_str(), l.year), ("LT-012", Some(1994)));
    }

    #[test]
    fn missing_fields_are_unknown_not_errors() {
        let v = serde_json::json!({"id": 1, "price": {"value": 2.5}, "release": {"id": 9}});
        let (c, l) = copy_from_listing(&v).unwrap();
        assert_eq!(
            (c.currency.as_str(), c.condition, c.sleeve.as_str()),
            ("", None, "")
        );
        assert_eq!(c.grades(), "");
        assert!(l.title.is_empty() && l.formats.is_empty());
        assert!(
            copy_from_listing(&serde_json::json!({"id": 1})).is_none(),
            "no release"
        );
    }

    #[test]
    fn grades_read_as_discogs_writes_them() {
        assert_eq!(
            Condition::parse("Very Good Plus (VG+)"),
            Some(Condition::VeryGoodPlus)
        );
        assert_eq!(
            Condition::parse("Near Mint (NM or M-)"),
            Some(Condition::NearMint)
        );
        assert_eq!(Condition::parse("Generic"), None);
        assert_eq!(grade_text("Generic"), "Generic");
        assert!(Condition::Mint > Condition::VeryGoodPlus);
    }

    #[test]
    fn format_and_year_are_search_words() {
        let c = Criteria {
            style: Some("Deep House".into()),
            format: Some(Format::Vinyl),
            year: Some(2025),
            query: "ostgut".into(),
            ..Criteria::default()
        };
        assert_eq!(c.discogs_query(), "\"Deep House\" vinyl 2025 ostgut");
        let cd = Criteria {
            format: Some(Format::Cd),
            ..Criteria::default()
        };
        assert_eq!(cd.discogs_query(), "cd");
        let file = Criteria {
            format: Some(Format::File),
            ..Criteria::default()
        };
        assert_eq!(
            file.discogs_query(),
            "",
            "not a format Discogs' search tells apart"
        );
    }

    #[test]
    fn genre_and_style_are_searched_as_phrases_before_the_text() {
        let c = |genre: Option<&str>, style: Option<&str>, query: &str| Criteria {
            genre: genre.map(Into::into),
            style: style.map(Into::into),
            query: query.into(),
            ..Criteria::default()
        };
        assert_eq!(
            c(None, Some("Deep House"), "").discogs_query(),
            "\"Deep House\""
        );
        assert_eq!(
            c(Some("Electronic"), Some("Deep House"), " vinyl ").discogs_query(),
            "\"Electronic\" \"Deep House\" vinyl"
        );
        assert_eq!(c(None, None, " techno ").discogs_query(), "techno");
        assert_eq!(c(None, Some(" "), "").discogs_query(), "");
        assert_eq!(Criteria::default().discogs_query(), "");
        let folk = c(Some("Folk, World, & Country"), None, "").discogs_query();
        assert!(
            inventory_path("hhv", &folk, 1, 100)
                .ends_with("&q=%22Folk%2C%20World%2C%20%26%20Country%22"),
            "{}",
            inventory_path("hhv", &folk, 1, 100)
        );
    }

    #[test]
    fn criteria_saved_with_old_filters_still_load() {
        let saved = r#"(query: "techno", newest: Some(500), ships_from: ["Germany"], formats: [Vinyl], min_cents: None, max_cents: Some(1500), min_condition: Some(VeryGood))"#;
        let c: Criteria = ron::from_str(saved).unwrap();
        assert_eq!(c.query, "techno");
        assert_eq!(c.newest, Some(500));
        assert_eq!((c.style, c.format, c.year), (None, None, None));
    }

    #[test]
    fn pages_stop_at_newest_and_at_discogs_limit() {
        let all = Criteria::default();
        assert_eq!(
            all.pages(408),
            100,
            "Discogs serves 100 pages of someone else's stock"
        );
        assert_eq!(all.pages(5), 5);
        let newest = Criteria {
            newest: Some(250),
            ..Criteria::default()
        };
        assert_eq!(newest.pages(408), 3);
        assert_eq!(reachable(40_728, &all), 10_000);
        assert_eq!(reachable(40_728, &newest), 250);
        assert_eq!(reachable(475, &all), 475);
    }

    #[test]
    fn a_page_keeps_the_copies_for_sale() {
        let mut sold = listing_json(3, 30, 4.0, "LP", "Mint (M)");
        sold["status"] = "Sold".into();
        let v = page(
            vec![
                listing_json(1, 10, 9.0, "12\"", "Mint (M)"),
                listing_json(2, 20, 9.0, "CD", "Mint (M)"),
                sold,
                serde_json::json!({"broken": true}),
            ],
            1851,
            19,
        );
        let p = inventory_page(&v, &Criteria::default());
        assert_eq!(
            p.copies.iter().map(|c| c.listing).collect::<Vec<_>>(),
            [1, 2],
            "not the sold one, nor the broken one"
        );
        assert_eq!(p.listed[0].key, RecordKey::Release(10));
        assert_eq!((p.pages, p.total), (19, 1851));
    }

    #[test]
    fn paths_carry_the_search_and_the_order() {
        assert_eq!(
            inventory_path("decks.de", "", 2, 100),
            "/users/decks.de/inventory?status=For%20Sale&sort=listed&sort_order=desc&page=2&per_page=100"
        );
        assert!(
            inventory_path("www.hhv.de", " deep house ", 1, 1).ends_with("&q=deep%20house"),
            "search text is escaped"
        );
    }

    fn order(seller: &str, created: &str) -> Value {
        serde_json::json!({
            "id": format!("{created}-{seller}"),
            "created": created,
            "seller": {"id": seller.len(), "username": seller},
            "items": [{"release_id": 1}],
            "total": {"value": 10.0, "curr_abbr": "EUR"}
        })
    }

    #[test]
    fn purchases_rank_by_orders_then_the_latest() {
        let mut items = Vec::new();
        for d in 1..=11 {
            items.push(order("www.hhv.de", &format!("2024-09-{d:02}")));
        }
        for d in 1..=3 {
            items.push(order("logon", &format!("2026-03-{d:02}")));
            items.push(order("ADEPTA_STORE", &format!("2026-04-{d:02}")));
        }
        items.push(order("musicman1", "2014-04-05"));
        let ranked = rank_purchases(&serde_json::json!({ "items": items }));
        let names: Vec<_> = ranked
            .iter()
            .map(|r| (r.username.as_str(), r.orders))
            .collect();
        assert_eq!(
            names,
            [
                ("www.hhv.de", 11),
                ("ADEPTA_STORE", 3),
                ("logon", 3),
                ("musicman1", 1)
            ]
        );
        assert_eq!(ranked[1].last, "2026-04-03");
    }

    #[test]
    fn only_the_last_hundred_orders_count() {
        let mut items: Vec<Value> = (0..100)
            .map(|n| order("recent", &format!("2025-01-01T{:02}:{:02}", n / 60, n % 60)))
            .collect();
        for n in 0..150 {
            items.push(order(
                "old",
                &format!("2010-01-01T{:02}:{:02}", n / 60, n % 60),
            ));
        }
        let ranked = rank_purchases(&serde_json::json!({ "items": items }));
        assert_eq!(ranked.len(), 1);
        assert_eq!(
            (ranked[0].username.as_str(), ranked[0].orders),
            ("recent", 100)
        );
        assert!(
            rank_purchases(&serde_json::json!({})).is_empty(),
            "no purchases"
        );
    }

    #[test]
    fn the_first_list_skips_closed_and_gone_sellers() {
        use crate::clock::FakeClock;
        use crate::discogs::cache::DiskCache;
        use crate::discogs::transport::FakeTransport;
        use std::sync::Arc;
        let t = Arc::new(FakeTransport::new());
        let mut items = Vec::new();
        for (i, s) in ["shop0", "closed", "gone", "shop1"].iter().enumerate() {
            for _ in 0..(4 - i) {
                items.push(order(s, &format!("2026-01-0{}", i + 1)));
            }
        }
        for n in 2..15 {
            items.push(order(&format!("shop{n}"), "2020-01-01"));
        }
        t.route(
            "/purchases?sort=created&sort_order=desc&page=1&per_page=100",
            200,
            serde_json::json!({ "items": items }).to_string(),
        );
        let count_page = |n: u64| {
            serde_json::json!({"pagination": {"items": n, "pages": n}, "listings": []}).to_string()
        };
        for n in 0..15 {
            t.route(
                inventory_path(&format!("shop{n}"), "", 1, 1),
                200,
                count_page(5),
            );
        }
        t.route(inventory_path("closed", "", 1, 1), 200, count_page(0));
        let mut c = Client::new(
            t.clone(),
            Arc::new(FakeClock::default()),
            Some("tok".into()),
            DiskCache::default(),
        );
        let list = first_list(&mut c).unwrap();
        let names: Vec<_> = list.iter().map(|(r, _)| r.username.as_str()).collect();
        assert_eq!(names.len(), FIRST_LIST);
        assert_eq!(&names[..3], ["shop0", "shop1", "shop2"]);
        assert!(!names.contains(&"closed") && !names.contains(&"gone"));
        assert_eq!(list[0].1, 5);
    }

    #[test]
    fn a_count_is_one_request() {
        use crate::clock::FakeClock;
        use crate::discogs::cache::DiskCache;
        use crate::discogs::transport::FakeTransport;
        use std::sync::Arc;
        let t = Arc::new(FakeTransport::new());
        t.route(
            inventory_path("decks.de", "techno", 1, 1),
            200,
            page(vec![], 8389, 8389).to_string(),
        );
        let mut c = Client::new(
            t.clone(),
            Arc::new(FakeClock::default()),
            Some("tok".into()),
            DiskCache::default(),
        );
        assert_eq!(count(&mut c, "decks.de", "techno"), Ok(8389));
        assert_eq!(t.count(), 1);
        assert_eq!(count(&mut c, "nobody", ""), Err(ApiError::NotFound));
    }
}
