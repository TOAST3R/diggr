//! Where a record can be bought: the stores Diggr knows, and each one's search page for a
//! record it hasn't seen there. A search link needs no request from the app and no matching:
//! at worst the store shows no results.

use crate::bandcamp::catno_and_album;
use crate::discogs::client::path_segment;

/// The stores a record can be bought from, in the order the Buy chooser lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shop {
    Discogs,
    Bandcamp,
    Juno,
    Bleep,
    Beatport,
    Traxsource,
}

impl Shop {
    pub const ALL: [Shop; 6] = [
        Shop::Discogs,
        Shop::Bandcamp,
        Shop::Juno,
        Shop::Bleep,
        Shop::Beatport,
        Shop::Traxsource,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Shop::Discogs => "Discogs",
            Shop::Bandcamp => "Bandcamp",
            Shop::Juno => "Juno",
            Shop::Bleep => "Bleep",
            Shop::Beatport => "Beatport",
            Shop::Traxsource => "Traxsource",
        }
    }

    /// The store's search page for `query`.
    pub fn search_url(self, query: &str) -> String {
        let q = path_segment(query);
        match self {
            Shop::Discogs => format!("https://www.discogs.com/search/?q={q}&type=release"),
            Shop::Bandcamp => format!("https://bandcamp.com/search?q={q}&item_type=a"),
            Shop::Juno => format!("https://www.juno.co.uk/search/?q%5Ball%5D%5B%5D={q}"),
            Shop::Bleep => format!("https://bleep.com/search/query?q={q}"),
            Shop::Beatport => format!("https://www.beatport.com/search?q={q}"),
            Shop::Traxsource => format!("https://www.traxsource.com/search?term={q}"),
        }
    }
}

impl From<crate::stores::Store> for Shop {
    fn from(s: crate::stores::Store) -> Shop {
        use crate::stores::Store;
        match s {
            Store::Juno => Shop::Juno,
            Store::Bleep => Shop::Bleep,
            Store::Beatport => Shop::Beatport,
            Store::Traxsource => Shop::Traxsource,
        }
    }
}

/// What to search a record by: its artist (the track's when the record's is "Various" or
/// unknown) and its title, without a bracketed catalogue number.
pub fn query(record_artist: &str, track_artist: &str, title: &str) -> String {
    let artist = match record_artist.trim() {
        a if a.is_empty() || a.eq_ignore_ascii_case("various") => track_artist.trim(),
        a => a,
    };
    let (_, title) = catno_and_album(title);
    format!("{artist} {title}").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_store_searches_its_own_page() {
        let urls: Vec<String> = Shop::ALL
            .iter()
            .map(|s| s.search_url("Kerri Chandler Rain"))
            .collect();
        assert_eq!(
            urls,
            [
                "https://www.discogs.com/search/?q=Kerri%20Chandler%20Rain&type=release",
                "https://bandcamp.com/search?q=Kerri%20Chandler%20Rain&item_type=a",
                "https://www.juno.co.uk/search/?q%5Ball%5D%5B%5D=Kerri%20Chandler%20Rain",
                "https://bleep.com/search/query?q=Kerri%20Chandler%20Rain",
                "https://www.beatport.com/search?q=Kerri%20Chandler%20Rain",
                "https://www.traxsource.com/search?term=Kerri%20Chandler%20Rain",
            ]
        );
    }

    #[test]
    fn the_query_skips_various_and_the_catalogue_number() {
        assert_eq!(
            query("Various", "Moodymann", "Black Mahogani [KDJ 33]"),
            "Moodymann Black Mahogani"
        );
        assert_eq!(
            query("", "Moodymann", "Forevernevermore"),
            "Moodymann Forevernevermore"
        );
        assert_eq!(
            query("Kerri Chandler", "Someone", "Rain"),
            "Kerri Chandler Rain"
        );
        // Characters that would break the address are encoded.
        assert!(Shop::Bleep.search_url("A&B").ends_with("q=A%26B"));
    }
}
