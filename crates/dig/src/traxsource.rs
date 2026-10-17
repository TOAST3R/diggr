//! Traxsource pages, sent by the browser extension as data, never fetched by the app.
//!
//! traxsource.com answers anything but a browser with a challenge, so the extension reads the
//! tracks of the page the user has open and sends them (checked in
//! [`crate::stores::check_listed`]). Traxsource's samples can't be rebuilt from what the page
//! shows, so a Traxsource track has no clip: its preview is searched for, as for a Discogs
//! track without one.

use serde_json::Value;

use crate::stores::{self, ListError, Listed, Store};

const PAGE_HOST: &str = "https://www.traxsource.com";

/// Checks a send's `traxsource` object.
pub fn check(value: &Value) -> Result<Listed, ListError> {
    stores::check_listed(
        Store::Traxsource,
        value,
        |p| stores::page_on(p, "traxsource.com"),
        |t| match t.release.is_empty() {
            false => (
                format!("r{}", t.release),
                title_url(&t.release, &t.release_slug).unwrap_or_default(),
                t.release_slug.clone(),
                String::new(),
            ),
            true => (
                format!("t{}", t.id),
                track_url(&t.id, &t.slug).unwrap_or_default(),
                t.slug.clone(),
                String::new(),
            ),
        },
        |t| (None, track_url(&t.id, &t.slug).unwrap_or_default()),
    )
}

/// A track's page, rebuilt from checked parts.
pub fn track_url(id: &str, slug: &str) -> Option<String> {
    (stores::valid_slug(slug) && stores::valid_id(id))
        .then(|| format!("{PAGE_HOST}/track/{id}/{slug}"))
}

fn title_url(id: &str, slug: &str) -> Option<String> {
    (stores::valid_slug(slug) && stores::valid_id(id))
        .then(|| format!("{PAGE_HOST}/title/{id}/{slug}"))
}

/// Whether `text` looks like a Traxsource address (pasted ones are refused with a hint).
pub fn looks_like_traxsource(text: &str) -> bool {
    stores::looks_like(text, "traxsource.com")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn track(id: &str) -> Value {
        json!({
            "id": id, "slug": "rain", "artists": ["Kerri Chandler"], "title": "Rain",
            "mix": "Extended Mix", "bpm": 124, "key": "A min", "secs": 432,
            "release": "2345678", "release_name": "Rain EP", "release_slug": "rain-ep",
            "label": "Madhouse", "price": "$2.49",
        })
    }

    #[test]
    fn tracks_have_no_clip_and_fold_their_key() {
        let send = json!({ "page": "https://www.traxsource.com/genre/4/house", "tracks": [track("12815016")] });
        let out = check(&send).unwrap();
        let t = &out.records[0].tracks[0];
        assert_eq!(t.title, "Rain (Extended Mix)");
        assert_eq!(t.clip, None, "searched for, never fetched");
        assert_eq!(t.key.as_deref(), Some("8A"));
        assert_eq!(t.page, "https://www.traxsource.com/track/12815016/rain");
        assert_eq!(
            out.records[0].page,
            "https://www.traxsource.com/title/2345678/rain-ep"
        );
    }

    #[test]
    fn a_sample_or_an_odd_key_is_handled() {
        let mut with_sample = track("1");
        with_sample["sample"] = json!("b776a31b-cb84-477f-a7bb-0476c5a1ee8e");
        let mut odd_key = track("2");
        odd_key["key"] = json!("Dorian");
        let send =
            json!({ "page": "https://www.traxsource.com/", "tracks": [with_sample, odd_key] });
        let out = check(&send).unwrap();
        assert_eq!(out.refused, 1, "Traxsource sends carry no sample");
        assert_eq!(
            out.records[0].tracks[0].key, None,
            "a key that doesn't fold is left out"
        );
        assert!(looks_like_traxsource("www.traxsource.com/track/1/x"));
    }
}
