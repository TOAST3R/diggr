//! A Bandcamp fan's collection and wishlist (`bandcamp.com/‹user›`, `/wishlist`).
//!
//! yt-dlp doesn't read these pages, and Bandcamp has no public API for them, so they are read
//! the way Bandcamp's own site reads them: the fan page carries the fan id and the first items
//! of each list in its `data-blob`, and the rest comes from the endpoint the page pages with
//! (`api/fancollection/1/collection_items`, `…/wishlist_items`), which needs no login. Each
//! item names an album or track page on the artist's own subdomain; those are then read like
//! any pasted album. The address is checked strictly and rebuilt, as for other Bandcamp pages,
//! and item pages are kept only when they pass [`super::parse`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::discogs::url::Refused;

/// Which of a fan's lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FanList {
    Collection,
    Wishlist,
}

impl FanList {
    pub fn name(self) -> &'static str {
        match self {
            FanList::Collection => "collection",
            FanList::Wishlist => "wishlist",
        }
    }
}

/// A checked fan page.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FanPage {
    pub user: String,
    pub list: FanList,
}

/// Bandcamp's own pages on the bare host, never a fan.
const RESERVED: [&str; 24] = [
    "about",
    "album",
    "api",
    "artists",
    "contact",
    "cookies",
    "discover",
    "download",
    "fans",
    "feed",
    "gift_cards",
    "guide",
    "help",
    "login",
    "logout",
    "privacy",
    "search",
    "settings",
    "signup",
    "tag",
    "terms_of_use",
    "track",
    "yum",
    "pricing",
];

fn valid_user(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        && !RESERVED.contains(&s)
}

/// Checks a fan page address. Text that isn't on the bare `bandcamp.com` host is
/// [`Refused::NotDiscogs`] (another parser may take it); a `bandcamp.com` path that isn't a
/// fan's collection or wishlist is [`Refused::Unsupported`].
pub fn parse(text: &str) -> Result<FanPage, Refused> {
    let text = text.trim();
    if text.contains(char::is_whitespace) {
        return Err(Refused::NotDiscogs);
    }
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
        .ok_or(Refused::NotDiscogs)?;
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if !matches!(
        host.to_ascii_lowercase().as_str(),
        "bandcamp.com" | "www.bandcamp.com"
    ) {
        return Err(Refused::NotDiscogs);
    }
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let (user, list) = match parts.as_slice() {
        [user] => (*user, FanList::Collection),
        [user, "wishlist"] => (*user, FanList::Wishlist),
        _ => return Err(Refused::Unsupported),
    };
    if !valid_user(user) {
        return Err(Refused::Unsupported);
    }
    Ok(FanPage {
        user: user.to_owned(),
        list,
    })
}

impl FanPage {
    /// The address rebuilt from its checked parts: the only one requested.
    pub fn url(&self) -> String {
        match self.list {
            FanList::Collection => format!("https://bandcamp.com/{}", self.user),
            FanList::Wishlist => format!("https://bandcamp.com/{}/wishlist", self.user),
        }
    }
}

/// A digital purchase or wish, or a package (a record, cassette or CD from Bandcamp).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Digital,
    Physical,
}

/// One item of a list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FanItem {
    /// Bandcamp's item key, as its page keys them: `a2491645176`, `p17835704`.
    pub key: String,
    /// The album or track page, checked and rebuilt; empty when it failed the checks.
    pub page: String,
    pub artist: String,
    /// Without its catalogue number.
    pub title: String,
    pub catno: String,
    pub kind: ItemKind,
}

/// A fan's two lists, read whole.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FanLists {
    pub fan_id: u64,
    pub collection: Vec<FanItem>,
    pub wishlist: Vec<FanItem>,
}

pub const NOT_A_FAN: &str = "Not a Bandcamp fan page";
pub const STOPPED: &str = "stopped";

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().trim().to_owned()
}

/// An item from Bandcamp's JSON; `None` when it has no key.
fn item(v: &Value) -> Option<FanItem> {
    let kind = text(v, "item_type");
    let id = v["item_id"].as_u64()?;
    let key = format!("{}{id}", kind.chars().next()?);
    let page = super::parse(&text(v, "item_url"))
        .ok()
        .filter(|p| p.kind != super::BandcampKind::Label)
        .map(|p| p.url())
        .unwrap_or_default();
    let (catno, title) = super::catno_and_album(&text(v, "item_title"));
    Some(FanItem {
        key,
        page,
        artist: text(v, "band_name"),
        title,
        catno,
        kind: match kind.as_str() {
            "package" => ItemKind::Physical,
            _ => ItemKind::Digital,
        },
    })
}

/// The HTML entities a `data-blob` attribute uses.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..end];
        let c = match name {
            "quot" => Some('"'),
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "apos" => Some('\''),
            _ => name
                .strip_prefix("#x")
                .or_else(|| name.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Where a list stands after the fan page: its first items, how many it holds, and the token
/// the next items are asked from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ListStart {
    pub items: Vec<FanItem>,
    pub count: usize,
    pub token: String,
}

/// What the fan page says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageData {
    pub fan_id: u64,
    pub collection: ListStart,
    pub wishlist: ListStart,
}

/// The fan page's `data-blob`. A page without one is Bandcamp's browser check or not a fan.
pub fn parse_page(html: &str) -> Result<PageData, String> {
    let from = html.find("id=\"pagedata\"").unwrap_or(0);
    let blob = html[from..]
        .split_once("data-blob=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(b, _)| unescape(b));
    let Some(blob) = blob else {
        if html.contains("Client Challenge") {
            return Err("Bandcamp asked for a browser check; try again later".into());
        }
        return Err(NOT_A_FAN.into());
    };
    let v: Value = serde_json::from_str(&blob).map_err(|_| NOT_A_FAN.to_owned())?;
    let fan_id = v["fan_data"]["fan_id"]
        .as_u64()
        .ok_or_else(|| NOT_A_FAN.to_owned())?;
    let list = |cache: &str, data: &str| {
        let cached = &v["item_cache"][cache];
        // The page's order (newest first): `sequence` for the tab shown, `pending_sequence`
        // for the other one; the cache itself is keyed.
        let keys: Vec<&str> = ["sequence", "pending_sequence"]
            .iter()
            .flat_map(|s| v[data][*s].as_array().into_iter().flatten())
            .filter_map(Value::as_str)
            .collect();
        let order: Vec<&Value> = match keys.is_empty() {
            false => keys.iter().map(|k| &cached[*k]).collect(),
            true => cached
                .as_object()
                .into_iter()
                .flat_map(|m| m.values())
                .collect(),
        };
        let mut items: Vec<FanItem> = order.into_iter().filter_map(item).collect();
        dedup(&mut items);
        ListStart {
            items,
            count: v[data]["item_count"].as_u64().unwrap_or(0) as usize,
            token: text(&v[data], "last_token"),
        }
    };
    Ok(PageData {
        fan_id,
        collection: list("collection", "collection_data"),
        wishlist: list("wishlist", "wishlist_data"),
    })
}

fn dedup(items: &mut Vec<FanItem>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|i| seen.insert(i.key.clone()));
}

/// A page of the paging endpoint: its items, whether more follow, and the next token.
pub fn parse_items(json: &str) -> Result<(Vec<FanItem>, bool, String), String> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| format!("unreadable answer from Bandcamp: {e}"))?;
    let items = v["items"]
        .as_array()
        .ok_or("unreadable answer from Bandcamp: no items")?;
    Ok((
        items.iter().filter_map(item).collect(),
        v["more_available"].as_bool().unwrap_or(false),
        text(&v, "last_token"),
    ))
}

/// How the fan's pages are fetched: [`UreqFanHttp`], or a fake in tests.
pub trait FanHttp: Send + Sync {
    fn get(&self, url: &str) -> Result<String, String>;
    fn post_json(&self, url: &str, body: &str) -> Result<String, String>;
}

/// The real thing: `ureq` with rustls, as for Discogs. Checked on 2026-10-12: Bandcamp's
/// browser check lets it through to fan pages and the endpoint.
pub struct UreqFanHttp {
    agent: ureq::Agent,
}

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/140.0 Safari/537.36";

impl Default for UreqFanHttp {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .timeout_recv_body(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

fn answer(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<String, String> {
    let r = r.map_err(|e| format!("Bandcamp unreachable ({e})"))?;
    let status = r.status().as_u16();
    let body = r
        .into_body()
        .with_config()
        .limit(16 << 20)
        .read_to_string()
        .map_err(|e| format!("Bandcamp unreachable ({e})"))?;
    match status {
        200 => Ok(body),
        s => Err(format!("Bandcamp answered {s}")),
    }
}

impl FanHttp for UreqFanHttp {
    fn get(&self, url: &str) -> Result<String, String> {
        answer(
            self.agent
                .get(url)
                .header("User-Agent", USER_AGENT)
                .header("Accept", "text/html")
                .call(),
        )
    }

    fn post_json(&self, url: &str, body: &str) -> Result<String, String> {
        answer(
            self.agent
                .post(url)
                .header("User-Agent", USER_AGENT)
                .header("Content-Type", "application/json")
                .send(body),
        )
    }
}

/// Items asked for per request.
const PAGE: usize = 100;
/// Pages read per list at most: 100,000 items, against a page that never says it's done.
const MAX_PAGES: usize = 1000;

/// Reads both of `page`'s fan lists whole: the fan page, then the endpoint for each list's
/// rest, `gap` apart. `progress` hears (list, items read, items listed). `stop` ends it after
/// the current request with [`STOPPED`].
pub fn read_lists(
    http: &dyn FanHttp,
    page: &FanPage,
    gap: Duration,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(FanList, usize, usize),
) -> Result<FanLists, String> {
    let data = parse_page(&http.get(&page.url())?)?;
    let mut lists = FanLists {
        fan_id: data.fan_id,
        ..FanLists::default()
    };
    for (list, start) in [
        (FanList::Collection, data.collection),
        (FanList::Wishlist, data.wishlist),
    ] {
        let mut items = start.items;
        let mut token = start.token;
        let mut more = items.len() < start.count && !token.is_empty();
        progress(list, items.len(), start.count.max(items.len()));
        let mut pages = 0;
        while more && pages < MAX_PAGES {
            if stop.load(Ordering::Relaxed) {
                return Err(STOPPED.into());
            }
            std::thread::sleep(gap);
            let body = serde_json::json!({
                "fan_id": data.fan_id,
                "older_than_token": token,
                "count": PAGE,
            })
            .to_string();
            let url = format!(
                "https://bandcamp.com/api/fancollection/1/{}_items",
                list.name()
            );
            let (got, again, next) = parse_items(&http.post_json(&url, &body)?)?;
            more = again && !got.is_empty() && !next.is_empty() && next != token;
            token = next;
            items.extend(got);
            dedup(&mut items);
            pages += 1;
            progress(list, items.len(), start.count.max(items.len()));
        }
        match list {
            FanList::Collection => lists.collection = items,
            FanList::Wishlist => lists.wishlist = items,
        }
    }
    Ok(lists)
}

/// What Discogs said about an item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscogsMatch {
    /// Not looked up yet (no token, or stopped).
    #[default]
    Unsearched,
    /// Exactly this release.
    Found(u64),
    NoMatch,
    /// This many releases fit.
    Ambiguous(usize),
}

/// An item as the app keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FanEntry {
    pub item: FanItem,
    /// Its page was read into the crate (or failed for good: see `error`).
    #[serde(default)]
    pub read: bool,
    /// The track pages its page gave, for telling which crate entries it holds.
    #[serde(default)]
    pub tracks: Vec<String>,
    #[serde(default)]
    pub discogs: DiscogsMatch,
    /// Why its page couldn't be read.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
}

/// The imported fan (one at a time), in `bandcamp.ron`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FanMemory {
    pub user: String,
    pub fan_id: u64,
    /// When each list was last read (epoch seconds).
    pub read_at: u64,
    pub collection: Vec<FanEntry>,
    pub wishlist: Vec<FanEntry>,
    /// The last import's or refresh's report.
    pub report: Option<FanReport>,
}

impl FanMemory {
    pub fn list(&self, list: FanList) -> &[FanEntry] {
        match list {
            FanList::Collection => &self.collection,
            FanList::Wishlist => &self.wishlist,
        }
    }

    pub fn list_mut(&mut self, list: FanList) -> &mut Vec<FanEntry> {
        match list {
            FanList::Collection => &mut self.collection,
            FanList::Wishlist => &mut self.wishlist,
        }
    }

    /// A list read again: known items keep what was learnt about them, new ones come in
    /// unread, and items no longer listed are dropped and returned.
    pub fn update(&mut self, list: FanList, items: Vec<FanItem>) -> Vec<FanEntry> {
        let mut old: Vec<FanEntry> = std::mem::take(self.list_mut(list));
        let mut next = Vec::with_capacity(items.len());
        for item in items {
            match old.iter().position(|e| e.item.key == item.key) {
                Some(i) => {
                    let mut e = old.swap_remove(i);
                    e.item = item;
                    next.push(e);
                }
                None => next.push(FanEntry {
                    item,
                    read: false,
                    tracks: Vec::new(),
                    discogs: DiscogsMatch::Unsearched,
                    error: String::new(),
                }),
            }
        }
        *self.list_mut(list) = next;
        old
    }

    /// Every track page the list's items gave.
    pub fn tracks(&self, list: FanList) -> std::collections::HashSet<&str> {
        self.list(list)
            .iter()
            .flat_map(|e| e.tracks.iter().map(String::as_str))
            .collect()
    }
}

/// What an import or refresh did to one list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ListReport {
    pub list: Option<FanList>,
    /// The crate it went into.
    pub crate_name: String,
    /// Items read this time (new ones).
    pub read: usize,
    /// Items that brought tracks the crate lacked.
    pub added: usize,
    /// Items whose tracks the crate held already.
    pub already: usize,
    /// Items no longer on the list.
    pub gone: usize,
    /// Added to the Discogs collection (or wantlist).
    pub discogs_added: usize,
    /// Owned (or wanted) on Discogs already.
    pub discogs_already: usize,
    /// Not looked up: no token, or stopped.
    pub unsearched: usize,
}

/// An item the report lists, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportItem {
    pub list: FanList,
    pub item: FanItem,
    pub why: String,
}

/// The last import's report (kept in `bandcamp.ron`, reopened from the crates' menus).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FanReport {
    pub user: String,
    /// Epoch seconds.
    pub at: u64,
    pub stopped: bool,
    /// Why the lists couldn't be read, when they couldn't.
    pub failed: String,
    pub lists: Vec<ListReport>,
    /// Items not added to Discogs: not on it, or several candidates.
    pub not_added: Vec<ReportItem>,
    pub unreadable: Vec<ReportItem>,
}

impl ReportItem {
    fn line(&self) -> String {
        let kind = match self.item.kind {
            ItemKind::Physical => ", physical",
            ItemKind::Digital => "",
        };
        format!(
            "{} - {} ({}{kind}): {}",
            self.item.artist,
            self.item.title,
            self.list.name(),
            self.why
        )
    }
}

impl FanReport {
    /// The report as plain text, for the clipboard.
    pub fn text(&self) -> String {
        let mut out = format!("Bandcamp import: {}", self.user);
        if self.stopped {
            out += " (stopped)";
        }
        out.push('\n');
        if !self.failed.is_empty() {
            out += &format!("Couldn't read the lists: {}\n", self.failed);
        }
        for l in &self.lists {
            out += &format!(
                "{}: {} added, {} already there, {} gone\n",
                l.crate_name, l.added, l.already, l.gone
            );
            let to = match l.list {
                Some(FanList::Wishlist) => "wantlist",
                _ => "collection",
            };
            let already = match l.list {
                Some(FanList::Wishlist) => "already wanted",
                _ => "already owned",
            };
            out += &format!(
                "  Discogs {to}: {} added, {} {already}, {} not looked up\n",
                l.discogs_added, l.discogs_already, l.unsearched
            );
        }
        for (title, items) in [
            ("Not added to Discogs", &self.not_added),
            ("Couldn't read", &self.unreadable),
        ] {
            if items.is_empty() {
                continue;
            }
            out += &format!("{title} ({}):\n", items.len());
            for i in items {
                out += &format!("  {}\n", i.line());
            }
        }
        out
    }
}

/// A Discogs search for an item, for the report's link.
pub fn discogs_search_url(item: &FanItem) -> String {
    let q: String = format!("{} {}", item.artist, item.title)
        .bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            b' ' => "+".to_owned(),
            b => format!("%{b:02X}"),
        })
        .collect();
    format!("https://www.discogs.com/search/?q={q}&type=release")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const PAGE_HTML: &str = include_str!("../../tests/fixtures/bandcamp/fan-page.html");
    const ITEMS: &str = include_str!("../../tests/fixtures/bandcamp/fan-items.json");

    #[test]
    fn fan_pages_are_accepted() {
        let col = FanPage {
            user: "toast3r".into(),
            list: FanList::Collection,
        };
        for text in [
            "https://bandcamp.com/toast3r",
            "https://bandcamp.com/toast3r/",
            "https://www.bandcamp.com/toast3r?from=menubar#x",
            "  http://Bandcamp.com/toast3r ",
        ] {
            assert_eq!(parse(text), Ok(col.clone()), "{text}");
        }
        assert_eq!(col.url(), "https://bandcamp.com/toast3r");
        let wish = parse("https://bandcamp.com/toast3r/wishlist").unwrap();
        assert_eq!(wish.list, FanList::Wishlist);
        assert_eq!(wish.url(), "https://bandcamp.com/toast3r/wishlist");
        assert!(parse("https://bandcamp.com/dj_x-2").is_ok());
    }

    #[test]
    fn other_addresses_are_refused() {
        for text in [
            "https://bandcamp.com/discover",
            "https://bandcamp.com/tag/techno",
            "https://bandcamp.com/search",
            "https://bandcamp.com/toast3r;rm",
            "https://bandcamp.com/Toast3r",
            "https://bandcamp.com/toast3r/following",
            "https://bandcamp.com/",
            "https://bandcamp.com",
            "https://bandcamp.com/album/x",
        ] {
            assert_eq!(parse(text), Err(Refused::Unsupported), "{text}");
        }
        for text in [
            "https://bandcamp.com.attacker.net/toast3r",
            "https://analogicalforce.bandcamp.com/music",
            "bandcamp.com/toast3r",
            "hello",
        ] {
            assert_eq!(parse(text), Err(Refused::NotDiscogs), "{text}");
        }
    }

    #[test]
    fn the_page_gives_the_fan_and_first_items() {
        let d = parse_page(PAGE_HTML).unwrap();
        assert_eq!(d.fan_id, 425839);
        assert_eq!((d.collection.items.len(), d.collection.count), (2, 3));
        assert_eq!(d.collection.token, "1756109553:3874424269:p::");
        let first = &d.collection.items[0];
        assert_eq!(first.key, "p17835704");
        assert_eq!(first.kind, ItemKind::Physical);
        assert_eq!(first.page, "https://djjackum.bandcamp.com/album/context");
        assert_eq!(
            (first.artist.as_str(), first.title.as_str()),
            ("DJ Jackum", "Context")
        );
        assert_eq!(d.collection.items[1].kind, ItemKind::Digital);
        assert_eq!(d.collection.items[1].title, "I don't want to fly.");
        assert_eq!(d.wishlist.items.len(), 1);
        assert_eq!(d.wishlist.items[0].key, "a1697051478");
    }

    #[test]
    fn a_page_without_fan_data_is_refused() {
        assert_eq!(
            parse_page("<html><title>Client Challenge</title></html>").unwrap_err(),
            "Bandcamp asked for a browser check; try again later"
        );
        assert_eq!(parse_page("<html>nothing</html>").unwrap_err(), NOT_A_FAN);
        assert_eq!(
            parse_page(r#"<div id="pagedata" data-blob="{&quot;x&quot;:1}"></div>"#).unwrap_err(),
            NOT_A_FAN
        );
    }

    #[test]
    fn entities_are_unescaped() {
        assert_eq!(
            unescape("&quot;a&amp;b&#39;c&#x2F;&lt;&gt;&bogus; & d"),
            "\"a&b'c/<>&bogus; & d"
        );
    }

    #[test]
    fn items_pages_are_read() {
        let (items, more, token) = parse_items(ITEMS).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].artist, "Jon Murphy");
        assert!(!more);
        assert_eq!(token, "1778533720:2014758325:a::");
        assert!(parse_items("<html>").is_err());
        assert!(parse_items("{}").is_err());
    }

    #[test]
    fn items_failing_the_checks_keep_no_page() {
        let json = r#"{"items":[
            {"item_type":"album","item_id":1,"band_name":"A","item_title":"[AB012] T","item_url":"https://evil.net/album/x"},
            {"item_type":"track","item_id":2,"band_name":"B","item_title":"U","item_url":"https://b.bandcamp.com/track/u"},
            {"item_type":"album","band_name":"C","item_title":"no id"}],"more_available":false}"#;
        let (items, _, _) = parse_items(json).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            (items[0].page.as_str(), items[0].catno.as_str()),
            ("", "AB012")
        );
        assert_eq!(items[1].page, "https://b.bandcamp.com/track/u");
        assert_eq!(items[1].key, "t2");
    }

    fn fan_item(key: &str) -> FanItem {
        FanItem {
            key: key.into(),
            page: format!("https://a.bandcamp.com/album/{key}"),
            artist: "A".into(),
            title: key.into(),
            catno: String::new(),
            kind: ItemKind::Digital,
        }
    }

    #[test]
    fn a_list_read_again_keeps_what_was_learnt() {
        let mut m = FanMemory::default();
        assert!(
            m.update(FanList::Collection, vec![fan_item("a1"), fan_item("a2")])
                .is_empty()
        );
        m.collection[0].read = true;
        m.collection[0].tracks = vec!["https://a.bandcamp.com/track/x".into()];
        m.collection[0].discogs = DiscogsMatch::Found(7);
        let gone = m.update(FanList::Collection, vec![fan_item("a3"), fan_item("a1")]);
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].item.key, "a2");
        let keys: Vec<&str> = m.collection.iter().map(|e| e.item.key.as_str()).collect();
        assert_eq!(keys, ["a3", "a1"]);
        assert!(m.collection[1].read && m.collection[1].discogs == DiscogsMatch::Found(7));
        assert!(!m.collection[0].read);
        assert!(
            m.tracks(FanList::Collection)
                .contains("https://a.bandcamp.com/track/x")
        );
        assert!(m.tracks(FanList::Wishlist).is_empty());
    }

    #[test]
    fn the_fan_is_kept_in_bandcamp_ron() {
        let dir = crate::test_dir("fan-memory");
        let mut mem = super::super::BandcampMemory::default();
        let mut fan = FanMemory {
            user: "toast3r".into(),
            fan_id: 425839,
            read_at: 5,
            ..FanMemory::default()
        };
        fan.update(FanList::Wishlist, vec![fan_item("a1")]);
        fan.wishlist[0].discogs = DiscogsMatch::Ambiguous(2);
        mem.fan = Some(fan);
        mem.save(&dir).unwrap();
        assert_eq!(super::super::BandcampMemory::load(&dir), mem);
    }

    #[test]
    fn the_report_reads_as_plain_text() {
        let mut item = fan_item("a1");
        item.artist = "Bodhi".into();
        item.title = "Corrupta".into();
        item.kind = ItemKind::Physical;
        let r = FanReport {
            user: "toast3r".into(),
            stopped: true,
            lists: vec![ListReport {
                list: Some(FanList::Collection),
                crate_name: "Collection: toast3r".into(),
                read: 3,
                added: 2,
                already: 1,
                discogs_added: 1,
                unsearched: 1,
                ..ListReport::default()
            }],
            not_added: vec![ReportItem {
                list: FanList::Collection,
                item: item.clone(),
                why: "2 candidates".into(),
            }],
            ..FanReport::default()
        };
        assert_eq!(
            r.text(),
            "Bandcamp import: toast3r (stopped)\n\
             Collection: toast3r: 2 added, 1 already there, 0 gone\n\
             \x20 Discogs collection: 1 added, 0 already owned, 1 not looked up\n\
             Not added to Discogs (1):\n\
             \x20 Bodhi - Corrupta (collection, physical): 2 candidates\n"
        );
        assert_eq!(
            discogs_search_url(&item),
            "https://www.discogs.com/search/?q=Bodhi+Corrupta&type=release"
        );
    }

    /// Answers the fan page, then `pages` in turn; records each request.
    struct Fake {
        page: String,
        pages: Mutex<Vec<Result<String, String>>>,
        asked: Mutex<Vec<String>>,
    }

    impl FanHttp for Fake {
        fn get(&self, url: &str) -> Result<String, String> {
            self.asked.lock().unwrap().push(url.to_owned());
            Ok(self.page.clone())
        }
        fn post_json(&self, url: &str, body: &str) -> Result<String, String> {
            self.asked.lock().unwrap().push(format!("{url} {body}"));
            self.pages.lock().unwrap().remove(0)
        }
    }

    fn fake(pages: Vec<Result<String, String>>) -> Fake {
        Fake {
            page: PAGE_HTML.into(),
            pages: Mutex::new(pages),
            asked: Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn both_lists_are_read_whole() {
        let http = fake(vec![Ok(ITEMS.into())]);
        let page = parse("https://bandcamp.com/toast3r/wishlist").unwrap();
        let mut heard = Vec::new();
        let lists = read_lists(
            &http,
            &page,
            Duration::ZERO,
            &AtomicBool::new(false),
            &mut |l, k, n| heard.push((l, k, n)),
        )
        .unwrap();
        assert_eq!(lists.fan_id, 425839);
        assert_eq!(lists.collection.len(), 3);
        assert_eq!(lists.wishlist.len(), 1, "the page held the whole wishlist");
        let asked = http.asked.lock().unwrap();
        assert_eq!(asked[0], "https://bandcamp.com/toast3r/wishlist");
        assert_eq!(asked.len(), 2);
        assert!(asked[1].contains("collection_items"));
        assert!(asked[1].contains(r#""older_than_token":"1756109553:3874424269:p::""#));
        assert!(asked[1].contains(r#""fan_id":425839"#));
        assert_eq!(
            heard,
            [
                (FanList::Collection, 2, 3),
                (FanList::Collection, 3, 3),
                (FanList::Wishlist, 1, 1)
            ]
        );
    }

    #[test]
    fn a_failure_or_stop_ends_the_read() {
        let page = parse("https://bandcamp.com/toast3r").unwrap();
        let http = fake(vec![Err("Bandcamp answered 503".into())]);
        let r = read_lists(
            &http,
            &page,
            Duration::ZERO,
            &AtomicBool::new(false),
            &mut |_, _, _| {},
        );
        assert_eq!(r.unwrap_err(), "Bandcamp answered 503");
        let http = fake(vec![Ok(ITEMS.into())]);
        let r = read_lists(
            &http,
            &page,
            Duration::ZERO,
            &AtomicBool::new(true),
            &mut |_, _, _| {},
        );
        assert_eq!(r.unwrap_err(), STOPPED);
        assert_eq!(
            http.asked.lock().unwrap().len(),
            1,
            "nothing after the stop"
        );
    }

    /// Reads the real fan page. Run by hand: `cargo test -p dig real_fan -- --ignored`.
    #[test]
    #[ignore]
    fn real_fan() {
        let page = parse("https://bandcamp.com/toast3r").unwrap();
        let lists = read_lists(
            &UreqFanHttp::default(),
            &page,
            Duration::from_secs(1),
            &AtomicBool::new(false),
            &mut |_, _, _| {},
        )
        .unwrap();
        assert!(lists.collection.len() >= 99, "{}", lists.collection.len());
        assert!(lists.wishlist.len() >= 100, "{}", lists.wishlist.len());
    }
}
