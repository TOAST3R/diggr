//! From a page to records: its name, its listing (100 records per request), and each
//! record's details, all through the disk cache.

use serde_json::Value;

use super::cache::Kind;
use super::client::{ApiError, Client, path_segment};
use super::model::{
    Listed, Record, RecordKey, listed_from_list_item, listed_from_listing, listed_from_want,
};
use super::url::{Page, PageKind};

pub const PER_PAGE: u32 = 100;

/// One page of a listing.
#[derive(Debug, Clone, PartialEq)]
pub struct ListingPage {
    pub items: Vec<Listed>,
    /// Listing pages in all.
    pub pages: u32,
    /// Records in the whole listing, as Discogs counts them (before filtering).
    pub total: usize,
}

/// The page's name, as messages and Play crate names use it: "Label: Lowtide Tapes".
pub fn page_name(client: &mut Client, page: &Page, now: u64) -> Result<String, ApiError> {
    if let PageKind::ShopItem(id) = page.kind {
        let release = Page::new(PageKind::Release(shop_item_release(client, id, now)?));
        return page_name(client, &release, now);
    }
    let name = match &page.kind {
        PageKind::Label(id) => {
            cached_json(client, Kind::Label, &format!("/labels/{id}"), *id, now)?["name"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        }
        PageKind::Artist(id) => {
            let v = cached_json(client, Kind::Artist, &format!("/artists/{id}"), *id, now)?;
            super::model::clean_artist(v["name"].as_str().unwrap_or(""))
        }
        PageKind::Release(id) => {
            let r = record(client, RecordKey::Release(*id), false, now)?;
            format!("{} – {}", r.artist, r.title)
        }
        PageKind::Master(id) => {
            let r = record(client, RecordKey::Master(*id), false, now)?;
            format!("{} – {}", r.artist, r.title)
        }
        PageKind::Wantlist(user) => user.clone(),
        PageKind::List(id) => list_json(client, *id, now)?["name"]
            .as_str()
            .unwrap_or("")
            .to_owned(),
        PageKind::ShopItem(_) => unreachable!("resolved above"),
    };
    let name = name.trim();
    Ok(if name.is_empty() {
        page.provisional_name()
    } else {
        format!("{}: {name}", page.kind_name())
    })
}

/// Page `n` (from 1) of the page's listing. A release or master lists itself.
pub fn listing(
    client: &mut Client,
    page: &Page,
    n: u32,
    now: u64,
) -> Result<ListingPage, ApiError> {
    let one = |key| ListingPage {
        items: vec![Listed::new(key)],
        pages: 1,
        total: 1,
    };
    match &page.kind {
        PageKind::Release(id) => Ok(one(RecordKey::Release(*id))),
        PageKind::Master(id) => Ok(one(RecordKey::Master(*id))),
        PageKind::ShopItem(id) => Ok(one(RecordKey::Release(shop_item_release(
            client, *id, now,
        )?))),
        PageKind::Label(id) => {
            let path = format!("/labels/{id}/releases?page={n}&per_page={PER_PAGE}");
            let v = listing_json(client, &path, now)?;
            Ok(paged(&v, "releases", listed_from_listing))
        }
        PageKind::Artist(id) => {
            let path = format!(
                "/artists/{id}/releases?sort=year&sort_order=asc&page={n}&per_page={PER_PAGE}"
            );
            let v = listing_json(client, &path, now)?;
            // Main and remix credits only (not appearances, production…).
            Ok(paged(&v, "releases", |i| {
                matches!(i["role"].as_str(), Some("Main" | "Remix"))
                    .then(|| listed_from_listing(i))
                    .flatten()
            }))
        }
        PageKind::Wantlist(user) => {
            // Never cached: it changes with every keep.
            let path = format!(
                "/users/{}/wants?page={n}&per_page={PER_PAGE}",
                path_segment(user)
            );
            let v = client.get_json(&path)?;
            Ok(paged(&v, "wants", listed_from_want))
        }
        PageKind::List(id) => {
            let v = list_json(client, *id, now)?;
            let items: Vec<Listed> = v["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(listed_from_list_item)
                .collect();
            Ok(ListingPage {
                total: items.len(),
                items,
                pages: 1,
            })
        }
    }
}

fn paged(v: &Value, field: &str, item: fn(&Value) -> Option<Listed>) -> ListingPage {
    let items: Vec<Listed> = v[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(item)
        .collect();
    ListingPage {
        pages: v["pagination"]["pages"].as_u64().unwrap_or(1).max(1) as u32,
        total: v["pagination"]["items"]
            .as_u64()
            .map_or(items.len(), |n| n as usize),
        items,
    }
}

/// A record's details. A release is read from the cache unless `fresh` asks for new
/// marketplace numbers (or the cached prices are in another currency); a master also reads its
/// main release, for the label, catalog number and marketplace numbers.
pub fn record(
    client: &mut Client,
    key: RecordKey,
    fresh: bool,
    now: u64,
) -> Result<Record, ApiError> {
    match key {
        RecordKey::Release(id) => {
            let currency = client.currency().to_owned();
            let key = id.to_string();
            if !fresh
                && let Some(c) = client.cache.get(Kind::Release, &key)
                && c.currency.as_deref() == Some(currency.as_str())
                && let Ok(v) = serde_json::from_str::<Value>(&c.body)
                && let Some(r) = Record::from_release(&v, &currency, c.fetched_at)
            {
                return Ok(r);
            }
            let path = format!("/releases/{id}?curr_abbr={currency}");
            let r = client.call(super::transport::Method::Get, &path)?;
            let v: Value = serde_json::from_str(&r.body)
                .map_err(|e| ApiError::Other(format!("release {id}: {e}")))?;
            let rec = Record::from_release(&v, &currency, now)
                .ok_or_else(|| ApiError::Other(format!("release {id}: unexpected answer")))?;
            client
                .cache
                .put_priced(Kind::Release, &key, &r.body, now, Some(&currency));
            Ok(rec)
        }
        RecordKey::Master(id) => {
            let v = cached_json(client, Kind::Master, &format!("/masters/{id}"), id, now)?;
            let main = match v["main_release"].as_u64() {
                Some(m) => Some(record(client, RecordKey::Release(m), fresh, now)?),
                None => None,
            };
            Record::from_master(&v, main.as_ref())
                .ok_or_else(|| ApiError::Other(format!("master {id}: unexpected answer")))
        }
    }
}

/// The release a marketplace item sells: one request, then from the cache for good (an item
/// never changes release). An item Discogs doesn't know (or no longer lists) is not found.
pub fn shop_item_release(client: &mut Client, id: u64, now: u64) -> Result<u64, ApiError> {
    let v = cached_json(
        client,
        Kind::ShopItem,
        &format!("/marketplace/listings/{id}"),
        id,
        now,
    )?;
    v["release"]["id"].as_u64().ok_or(ApiError::NotFound)
}

/// Record data (labels, artists, masters): from the cache whenever it's there.
fn cached_json(
    client: &mut Client,
    kind: Kind,
    path: &str,
    id: u64,
    now: u64,
) -> Result<Value, ApiError> {
    let key = id.to_string();
    if let Some(c) = client.cache.get(kind, &key)
        && let Ok(v) = serde_json::from_str(&c.body)
    {
        return Ok(v);
    }
    let r = client.call(super::transport::Method::Get, path)?;
    let v = serde_json::from_str(&r.body).map_err(|e| ApiError::Other(format!("{path}: {e}")))?;
    client.cache.put(kind, &key, &r.body, now);
    Ok(v)
}

/// A listing page: from the cache for 24 h.
fn listing_json(client: &mut Client, path: &str, now: u64) -> Result<Value, ApiError> {
    if let Some(body) = client.cache.listing(path, now)
        && let Ok(v) = serde_json::from_str(&body)
    {
        return Ok(v);
    }
    let r = client.call(super::transport::Method::Get, path)?;
    let v = serde_json::from_str(&r.body).map_err(|e| ApiError::Other(format!("{path}: {e}")))?;
    client.cache.put(Kind::Listing, path, &r.body, now);
    Ok(v)
}

fn list_json(client: &mut Client, id: u64, now: u64) -> Result<Value, ApiError> {
    listing_json(client, &format!("/lists/{id}"), now)
}
