//! Finding the one Discogs release a Bandcamp item is, or none: a wrong pick would write to
//! the user's real collection or wantlist, so only an exact match counts.
//!
//! One database search per item (artist, title, and catalogue number when there is one), then
//! the results are kept only when their format fits (File for digital, anything else for a
//! record), their artist and title are the item's, and their catalogue number is the item's
//! when both have one. Exactly one left is the release; more is ambiguous; none is no match.

use serde_json::Value;

use super::client::{ApiError, Client, path_segment};
use super::matching::fold;
use crate::bandcamp::fan::DiscogsMatch;

/// What is looked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
    pub artist: String,
    pub title: String,
    pub catno: String,
    /// A record (vinyl, cassette, CD…), else the File release.
    pub physical: bool,
    /// `title` names a track on the release, not the release (a mix's tracks): searched as a
    /// track, and only the artist must match.
    pub track: bool,
}

impl Wanted {
    /// A label's account sells "Artist - Album" under its own name: the artist is in the title.
    pub fn new(account: &str, title: &str, catno: &str, physical: bool) -> Self {
        let (artist, title) = match title.split_once(" - ") {
            Some((a, t)) if !a.trim().is_empty() && !t.trim().is_empty() => (a.trim(), t.trim()),
            _ => (account.trim(), title.trim()),
        };
        Self {
            artist: artist.to_owned(),
            title: title.to_owned(),
            catno: catno.trim().to_owned(),
            physical,
            track: false,
        }
    }

    /// A track by `artist` called `title`, on a record.
    pub fn track(artist: &str, title: &str) -> Self {
        Self {
            artist: artist.trim().to_owned(),
            title: title.trim().to_owned(),
            catno: String::new(),
            physical: true,
            track: true,
        }
    }
}

/// Letters and digits only, folded.
fn key(s: &str) -> String {
    fold(s)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// A Discogs artist name without its variant asterisk or disambiguation number ("Patricia (2)").
fn artist_key(s: &str) -> String {
    let s = s.replace('*', "");
    let s = match s.rfind(" (") {
        Some(i)
            if s.ends_with(')') && s[i + 2..s.len() - 1].bytes().all(|b| b.is_ascii_digit()) =>
        {
            &s[..i]
        }
        _ => s.as_str(),
    };
    key(s)
}

/// The search's results narrowed to the exact release.
pub fn pick(results: &Value, w: &Wanted) -> DiscogsMatch {
    let (artist, title, catno) = (artist_key(&w.artist), key(&w.title), key(&w.catno));
    let mut ids: Vec<u64> = Vec::new();
    for r in results["results"].as_array().into_iter().flatten() {
        let Some(id) = r["id"].as_u64() else { continue };
        let file = r["format"][0].as_str() == Some("File");
        if r["format"][0].as_str().is_none() || file == w.physical {
            continue;
        }
        let (a, t) = r["title"]
            .as_str()
            .unwrap_or("")
            .split_once(" - ")
            .unwrap_or(("", ""));
        if artist_key(a) != artist || (!w.track && key(t) != title) {
            continue;
        }
        let theirs = key(r["catno"].as_str().unwrap_or(""));
        if !catno.is_empty() && !theirs.is_empty() && theirs != catno && theirs != "none" {
            continue;
        }
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    match ids.as_slice() {
        [] => DiscogsMatch::NoMatch,
        [id] => DiscogsMatch::Found(*id),
        more => DiscogsMatch::Ambiguous(more.len()),
    }
}

/// The search request's path.
pub fn path(w: &Wanted) -> String {
    let mut p = format!(
        "/database/search?type=release&artist={}&{}={}&per_page=50",
        path_segment(&w.artist),
        if w.track { "track" } else { "release_title" },
        path_segment(&w.title)
    );
    if !w.catno.is_empty() {
        p.push_str(&format!("&catno={}", path_segment(&w.catno)));
    }
    p
}

/// Looks `w` up on Discogs (one request; a token is needed).
pub fn find_release(client: &mut Client, w: &Wanted) -> Result<DiscogsMatch, ApiError> {
    if client.token().is_none() {
        return Err(ApiError::TokenNeeded);
    }
    Ok(pick(&client.get_json(&path(w))?, w))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn result(id: u64, title: &str, format: &[&str], catno: &str) -> Value {
        json!({"id": id, "title": title, "format": format, "catno": catno})
    }

    fn wanted(physical: bool, catno: &str) -> Wanted {
        Wanted::new("Patricia", "The Ooze EP", catno, physical)
    }

    #[test]
    fn a_track_is_searched_by_its_name_and_matched_by_artist() {
        let w = Wanted::track("Kerri Chandler", "Rain");
        assert_eq!(
            path(&w),
            "/database/search?type=release&artist=Kerri%20Chandler&track=Rain&per_page=50"
        );
        let one = json!({"results": [
            result(1, "Kerri Chandler - Rain EP", &["Vinyl"], "MAD-01"),
            result(2, "Various - House Classics", &["Vinyl"], "HC-1"),
            result(3, "Kerri Chandler - Rain", &["File"], "none"),
        ]});
        assert_eq!(
            pick(&one, &w),
            DiscogsMatch::Found(1),
            "the record, not the compilation"
        );
        let two = json!({"results": [
            result(1, "Kerri Chandler - Rain EP", &["Vinyl"], "MAD-01"),
            result(4, "Kerri Chandler - Rain EP", &["Vinyl"], "MAD-01R"),
        ]});
        assert_eq!(pick(&two, &w), DiscogsMatch::Ambiguous(2));
    }

    #[test]
    fn digital_and_physical_are_told_apart() {
        let r = json!({"results": [
            result(2001, "Patricia - The Ooze EP", &["File", "MP3", "EP"], "AF070"),
            result(1001, "Patricia (2) - The Ooze EP", &["Vinyl", "12\"", "EP"], "AF070"),
        ]});
        assert_eq!(pick(&r, &wanted(false, "AF070")), DiscogsMatch::Found(2001));
        assert_eq!(pick(&r, &wanted(true, "AF070")), DiscogsMatch::Found(1001));
        assert_eq!(pick(&r, &wanted(true, "")), DiscogsMatch::Found(1001));
    }

    #[test]
    fn a_repress_is_ambiguous() {
        let r = json!({"results": [
            result(1001, "Patricia - The Ooze EP", &["Vinyl"], "AF070"),
            result(1002, "Patricia - The Ooze EP", &["Vinyl"], "AF070"),
            result(1002, "Patricia - The Ooze EP", &["Vinyl"], "AF070"),
        ]});
        assert_eq!(pick(&r, &wanted(true, "AF070")), DiscogsMatch::Ambiguous(2));
    }

    #[test]
    fn other_records_do_not_count() {
        let r = json!({"results": [
            result(1, "Patricia - The Ooze EP (Remixes)", &["Vinyl"], "AF070"),
            result(2, "Patrice - The Ooze EP", &["Vinyl"], "AF070"),
            result(3, "Patricia - The Ooze EP", &["Vinyl"], "AF071"),
            result(4, "Patricia - The Ooze EP", &[], "AF070"),
        ]});
        assert_eq!(pick(&r, &wanted(true, "AF070")), DiscogsMatch::NoMatch);
        assert_eq!(pick(&json!({}), &wanted(true, "")), DiscogsMatch::NoMatch);
    }

    #[test]
    fn a_label_account_names_the_artist_in_the_title() {
        let w = Wanted::new(
            "Diffuse Reality Records",
            "Gioele Menoni - Mental Roots",
            "",
            true,
        );
        assert_eq!(
            (w.artist.as_str(), w.title.as_str()),
            ("Gioele Menoni", "Mental Roots")
        );
        let r = json!({"results": [result(9, "Gioele Menoni - Mental Roots", &["Vinyl"], "DR01")]});
        assert_eq!(pick(&r, &w), DiscogsMatch::Found(9));
    }

    #[test]
    fn the_search_is_one_encoded_request() {
        let w = Wanted::new("Serge Geyzel", "I don't want to fly.", "TP 01", true);
        assert_eq!(
            path(&w),
            "/database/search?type=release&artist=Serge%20Geyzel&release_title=I%20don%27t%20want%20to%20fly.&per_page=50&catno=TP%2001"
        );
    }
}
