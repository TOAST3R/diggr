//! Discogs web addresses: which page a pasted address names.
//!
//! Accepted: a release, master, artist, label, wantlist or list, on `discogs.com` with or
//! without `www.`, over http or https (or no scheme), with an optional two-letter language
//! segment (`/de/`), with or without the name part after the id, and with the legacy
//! slug-first form of releases and masters. A marketplace item (`/shop/item/…`, or the older
//! `/sell/item/…`) is accepted too, and dug as the release it sells. The query (except the
//! wantlist's `user`) and the fragment are ignored.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageKind {
    Release(u64),
    Master(u64),
    Artist(u64),
    Label(u64),
    Wantlist(String),
    List(u64),
    /// A record for sale in the marketplace: dug as its release.
    ShopItem(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub kind: PageKind,
    /// The name part of the address (`Lowtide-Tapes`), for a name before the API gives one.
    pub slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Not a Discogs address at all (a paste of other text is ignored).
    NotDiscogs,
    /// A Discogs address, but not a page we can dig.
    Unsupported,
}

pub const SUPPORTED: &str = "Supported Discogs pages: a release, master release, artist, \
     label, wantlist, list or marketplace item";

impl Page {
    pub fn new(kind: PageKind) -> Self {
        Self { kind, slug: None }
    }

    /// The page's address on discogs.com.
    pub fn url(&self) -> String {
        let base = "https://www.discogs.com";
        match &self.kind {
            PageKind::Release(id) => format!("{base}/release/{id}"),
            PageKind::Master(id) => format!("{base}/master/{id}"),
            PageKind::Artist(id) => format!("{base}/artist/{id}"),
            PageKind::Label(id) => format!("{base}/label/{id}"),
            PageKind::Wantlist(user) => format!("{base}/wantlist?user={user}"),
            PageKind::List(id) => format!("{base}/lists/{id}"),
            PageKind::ShopItem(id) => format!("{base}/shop/item/{id}"),
        }
    }

    /// "Label", "Artist"… as used in names and messages.
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            PageKind::Release(_) => "Release",
            PageKind::Master(_) => "Master",
            PageKind::Artist(_) => "Artist",
            PageKind::Label(_) => "Label",
            PageKind::Wantlist(_) => "Wantlist",
            PageKind::List(_) => "List",
            PageKind::ShopItem(_) => "Shop item",
        }
    }

    /// A name until the API gives the real one: from the slug, else the id.
    pub fn provisional_name(&self) -> String {
        let what = match (&self.kind, &self.slug) {
            (PageKind::Wantlist(user), _) => user.clone(),
            (_, Some(slug)) => slug.replace('-', " "),
            (
                PageKind::Release(id)
                | PageKind::Master(id)
                | PageKind::Artist(id)
                | PageKind::Label(id)
                | PageKind::List(id)
                | PageKind::ShopItem(id),
                None,
            ) => id.to_string(),
        };
        format!("{}: {what}", self.kind_name())
    }
}

pub fn parse(text: &str) -> Result<Page, Refused> {
    let text = text.trim();
    if text.contains(char::is_whitespace) {
        return Err(Refused::NotDiscogs);
    }
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
        .unwrap_or(text);
    let (host, rest) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.to_ascii_lowercase();
    if host != "discogs.com" && host != "www.discogs.com" {
        return Err(Refused::NotDiscogs);
    }
    let rest = rest.split('#').next().unwrap_or("");
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let mut segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segs
        .first()
        .is_some_and(|s| s.len() == 2 && s.chars().all(|c| c.is_ascii_lowercase()))
    {
        segs.remove(0);
    }
    page(&segs, query).ok_or(Refused::Unsupported)
}

fn page(segs: &[&str], query: &str) -> Option<Page> {
    let with_slug = |kind: fn(u64) -> PageKind, seg: &str| {
        let (id, slug) = id_slug(seg)?;
        Some(Page {
            kind: kind(id),
            slug,
        })
    };
    match segs {
        ["release", s] => with_slug(PageKind::Release, s),
        ["master", s] => with_slug(PageKind::Master, s),
        ["artist", s] => with_slug(PageKind::Artist, s),
        ["label", s] => with_slug(PageKind::Label, s),
        // Legacy: /Nightcraft-Glasshouse/release/123456
        [slug, "release", id] => Some(Page {
            kind: PageKind::Release(number(id)?),
            slug: Some((*slug).to_owned()),
        }),
        [slug, "master", id] => Some(Page {
            kind: PageKind::Master(number(id)?),
            slug: Some((*slug).to_owned()),
        }),
        ["wantlist"] => {
            let user = query
                .split('&')
                .find_map(|kv| kv.strip_prefix("user="))
                .map(percent_decode)
                .filter(|u| valid_user(u))?;
            Some(Page::new(PageKind::Wantlist(user)))
        }
        ["user", user, "wantlist"] => {
            let user = percent_decode(user);
            valid_user(&user).then(|| Page::new(PageKind::Wantlist(user)))
        }
        ["lists", s] => with_slug(PageKind::List, s),
        ["shop" | "sell", "item", s] => with_slug(PageKind::ShopItem, s),
        ["lists", slug, id] => Some(Page {
            kind: PageKind::List(number(id)?),
            slug: Some((*slug).to_owned()),
        }),
        _ => None,
    }
}

/// `12345` or `12345-Lowtide-Tapes`.
fn id_slug(seg: &str) -> Option<(u64, Option<String>)> {
    let (id, slug) = match seg.split_once('-') {
        Some((id, slug)) => (id, (!slug.is_empty()).then(|| slug.to_owned())),
        None => (seg, None),
    };
    Some((number(id)?, slug))
}

fn number(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok().filter(|&n| n > 0)
}

fn valid_user(u: &str) -> bool {
    !u.is_empty() && u.len() <= 64 && !u.contains(['/', '?', '#'])
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            let v = (h * 16 + l) as u8;
            out.push(v);
            i += 3;
        } else {
            out.push(if b[i] == b'+' { b' ' } else { b[i] });
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use PageKind::*;

    #[test]
    fn every_supported_form() {
        let table: &[(&str, PageKind, Option<&str>)] = &[
            (
                "https://www.discogs.com/de/label/12345-Lowtide-Tapes",
                Label(12345),
                Some("Lowtide-Tapes"),
            ),
            ("https://discogs.com/release/123456", Release(123456), None),
            (
                "https://www.discogs.com/shop/item/3923678974",
                ShopItem(3923678974),
                None,
            ),
            (
                "https://discogs.com/de/sell/item/3923678974?ev=bp_det#x",
                ShopItem(3923678974),
                None,
            ),
            (
                "http://www.discogs.com/release/123456-Nightcraft-Glasshouse",
                Release(123456),
                Some("Nightcraft-Glasshouse"),
            ),
            (
                "www.discogs.com/release/123456?ev=rr#images",
                Release(123456),
                None,
            ),
            (
                "https://www.discogs.com/Nightcraft-Glasshouse/release/123456",
                Release(123456),
                Some("Nightcraft-Glasshouse"),
            ),
            (
                "https://www.discogs.com/master/98765-Nightcraft-Glasshouse",
                Master(98765),
                Some("Nightcraft-Glasshouse"),
            ),
            (
                "https://www.discogs.com/Nightcraft-Glasshouse/master/98765",
                Master(98765),
                Some("Nightcraft-Glasshouse"),
            ),
            (
                "https://www.discogs.com/fr/master/98765",
                Master(98765),
                None,
            ),
            (
                "https://www.discogs.com/artist/4242-Nightcraft",
                Artist(4242),
                Some("Nightcraft"),
            ),
            ("https://www.discogs.com/ja/artist/4242", Artist(4242), None),
            (
                "https://www.discogs.com/label/12345-Lowtide-Tapes?page=3&type=Releases",
                Label(12345),
                Some("Lowtide-Tapes"),
            ),
            (
                "https://www.discogs.com/wantlist?user=digger&page=2",
                Wantlist("digger".into()),
                None,
            ),
            (
                "https://www.discogs.com/user/digger/wantlist",
                Wantlist("digger".into()),
                None,
            ),
            (
                "https://www.discogs.com/es/user/dj%20x/wantlist",
                Wantlist("dj x".into()),
                None,
            ),
            (
                "https://www.discogs.com/lists/Deep-Tapes/555",
                List(555),
                Some("Deep-Tapes"),
            ),
            ("https://www.discogs.com/lists/555", List(555), None),
            ("  https://WWW.DISCOGS.COM/release/1  ", Release(1), None),
        ];
        for (text, kind, slug) in table {
            let p = parse(text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
            assert_eq!(&p.kind, kind, "{text}");
            assert_eq!(p.slug.as_deref(), *slug, "{text}");
        }
    }

    #[test]
    fn everything_else_is_refused() {
        use Refused::*;
        let table: &[(&str, Refused)] = &[
            ("https://www.discogs.com/forum/thread/123456", Unsupported),
            ("https://www.discogs.com/", Unsupported),
            ("https://www.discogs.com/sell/list", Unsupported),
            ("https://www.discogs.com/release/", Unsupported),
            ("https://www.discogs.com/release/abc", Unsupported),
            ("https://www.discogs.com/release/0", Unsupported),
            ("https://www.discogs.com/label/12x-Name", Unsupported),
            ("https://www.discogs.com/wantlist", Unsupported),
            ("https://www.discogs.com/search/?q=lowtide", Unsupported),
            ("https://evil.com/discogs.com/release/1", NotDiscogs),
            ("https://discogs.com.evil.com/release/1", NotDiscogs),
            ("https://www.youtube.com/watch?v=abcdefghijk", NotDiscogs),
            ("Nightcraft - Glasshouse", NotDiscogs),
            ("", NotDiscogs),
        ];
        for (text, want) in table {
            assert_eq!(parse(text).as_ref().err(), Some(want), "{text}");
        }
    }

    #[test]
    fn names_until_the_api_answers() {
        let p = parse("https://www.discogs.com/de/label/12345-Lowtide-Tapes").unwrap();
        assert_eq!(p.provisional_name(), "Label: Lowtide Tapes");
        assert_eq!(p.url(), "https://www.discogs.com/label/12345");
        let p = parse("https://discogs.com/release/123456").unwrap();
        assert_eq!(p.provisional_name(), "Release: 123456");
        let p = parse("https://www.discogs.com/user/digger/wantlist").unwrap();
        assert_eq!(p.provisional_name(), "Wantlist: digger");
        let p = parse("https://www.discogs.com/sell/item/3923678974").unwrap();
        assert_eq!(p.url(), "https://www.discogs.com/shop/item/3923678974");
    }
}
