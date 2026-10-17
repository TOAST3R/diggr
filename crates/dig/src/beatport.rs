//! Beatport pages, sent by the browser extension as data, never fetched by the app.
//!
//! beatport.com answers anything but a browser with a challenge, so the extension reads the
//! tracks of the page the user has open (from its own page data) and sends them; nothing
//! here trusts that data (see [`crate::stores::check_listed`]). Every address is rebuilt from
//! checked ids. A sample is the one thing the app downloads from Beatport: a 2-minute MP3 on
//! `geo-samples.beatport.com`, which answers a plain request (see `preview::sample`).

use serde_json::Value;

use crate::stores::{self, ListError, Listed, ListedTrack, Store};

const KEY_PREFIX: &str = "bp.";
const PAGE_HOST: &str = "https://www.beatport.com";

/// Checks a send's `beatport` object.
pub fn check(value: &Value) -> Result<Listed, ListError> {
    stores::check_listed(
        Store::Beatport,
        value,
        |p| stores::page_on(p, "beatport.com"),
        |t| match t.release.is_empty() {
            false => (
                format!("r{}", t.release),
                release_url(&t.release_slug, &t.release).unwrap_or_default(),
                t.release_slug.clone(),
                image_url(&t.image).unwrap_or_default(),
            ),
            true => (
                format!("t{}", t.id),
                track_url(&t.slug, &t.id).unwrap_or_default(),
                t.slug.clone(),
                image_url(&t.image).unwrap_or_default(),
            ),
        },
        |t: &ListedTrack| {
            (
                Some(key(&t.sample)),
                track_url(&t.slug, &t.id).unwrap_or_default(),
            )
        },
    )
}

/// A sample's preview key: `bp.‹uuid›`.
pub fn key(sample: &str) -> String {
    format!("{KEY_PREFIX}{sample}")
}

/// The sample a key names, when it is a valid Beatport key.
pub fn parse_key(key: &str) -> Option<&str> {
    key.strip_prefix(KEY_PREFIX)
        .filter(|u| stores::valid_uuid(u))
}

/// Whether a key is a Beatport sample's (valid or not): `bp.` never starts a YouTube id.
pub fn is_key(key: &str) -> bool {
    key.starts_with(KEY_PREFIX)
}

/// The sample's address, rebuilt from a valid key.
pub fn sample_url(key: &str) -> Option<String> {
    parse_key(key).map(|u| format!("https://geo-samples.beatport.com/track/{u}.LOFI.mp3"))
}

/// Whether `host` may serve a sample (the request and any redirect).
pub fn sample_host(host: &str) -> bool {
    host == "beatport.com" || host.ends_with(".beatport.com")
}

/// A track's page, rebuilt from checked parts.
pub fn track_url(slug: &str, id: &str) -> Option<String> {
    (stores::valid_slug(slug) && stores::valid_id(id))
        .then(|| format!("{PAGE_HOST}/track/{slug}/{id}"))
}

fn release_url(slug: &str, id: &str) -> Option<String> {
    (stores::valid_slug(slug) && stores::valid_id(id))
        .then(|| format!("{PAGE_HOST}/release/{slug}/{id}"))
}

fn image_url(image: &str) -> Option<String> {
    stores::valid_uuid(image)
        .then(|| format!("https://geo-media.beatport.com/image_size/250x250/{image}.jpg"))
}

/// Whether `text` looks like a Beatport address (pasted ones are refused with a hint).
pub fn looks_like_beatport(text: &str) -> bool {
    stores::looks_like(text, "beatport.com")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SAMPLE: &str = "b776a31b-cb84-477f-a7bb-0476c5a1ee8e";

    fn track(id: &str) -> Value {
        json!({
            "id": id, "slug": "input", "artists": ["Martin Cantrill"], "title": "Input",
            "mix": "Original Mix", "sample": SAMPLE, "bpm": 126, "key": "8A", "secs": 305,
            "release": "4630549", "release_name": "Dreams Album", "release_slug": "dreams-album",
            "label": "Deepsessions Recordings", "catno": "DSR123", "price": "€1.39",
            "image": "9ccc4215-f816-49ce-80a5-70279df868dc",
        })
    }

    fn send(tracks: Vec<Value>) -> Value {
        json!({ "page": "https://www.beatport.com/release/dreams-album/4630549", "tracks": tracks })
    }

    #[test]
    fn a_release_becomes_one_record_with_rebuilt_addresses() {
        let mut second = track("19209676");
        second["mix"] = json!("Dennis Ferrer Remix");
        let out = check(&send(vec![track("19209675"), second])).unwrap();
        assert_eq!(out.refused, 0);
        let [r] = out.records.as_slice() else {
            panic!("{out:?}")
        };
        assert_eq!(r.id, "r4630549");
        assert_eq!(
            r.page,
            "https://www.beatport.com/release/dreams-album/4630549"
        );
        assert_eq!(r.title, "Dreams Album");
        assert_eq!(
            r.cover,
            "https://geo-media.beatport.com/image_size/250x250/9ccc4215-f816-49ce-80a5-70279df868dc.jpg"
        );
        assert_eq!(r.tracks[0].title, "Input", "Original Mix left out");
        assert_eq!(r.tracks[1].title, "Input (Dennis Ferrer Remix)");
        assert_eq!(
            r.tracks[0].clip.as_deref(),
            Some("bp.b776a31b-cb84-477f-a7bb-0476c5a1ee8e")
        );
        assert_eq!(
            r.tracks[0].page,
            "https://www.beatport.com/track/input/19209675"
        );
        assert_eq!(
            (r.tracks[0].bpm, r.tracks[0].key.as_deref()),
            (Some(126), Some("8A"))
        );
        assert_eq!(
            sample_url(r.tracks[0].clip.as_deref().unwrap()).unwrap(),
            "https://geo-samples.beatport.com/track/b776a31b-cb84-477f-a7bb-0476c5a1ee8e.LOFI.mp3"
        );
    }

    #[test]
    fn a_bad_track_is_refused_alone() {
        let mut bad = Vec::new();
        for (field, value) in [
            ("sample", json!("../../etc")),
            ("sample", json!("B776A31B-CB84-477F-A7BB-0476C5A1EE8E")),
            ("id", json!("12345678901")),
            ("slug", json!("Input")),
            ("bpm", json!(300)),
            ("key", json!("13A")),
            ("key", json!("A min")),
            ("secs", json!(7_201)),
            ("artists", json!([])),
            ("title", json!("a\u{7}b")),
            ("image", json!("cover.jpg")),
            ("catno", json!("x".repeat(41))),
            ("extra", json!(1)),
        ] {
            let mut t = track("1");
            t[field] = value;
            bad.push(t);
        }
        let n = bad.len();
        let mut all = vec![track("2")];
        all.extend(bad);
        let out = check(&send(all)).unwrap();
        assert_eq!((out.records.len(), out.refused), (1, n));
    }

    #[test]
    fn whole_sends_refused() {
        let many: Vec<Value> = (0..501).map(|i| track(&i.to_string())).collect();
        assert_eq!(check(&send(many)), Err(ListError::TooMany));
        let mut t = track("1");
        t["id"] = json!("x");
        assert_eq!(
            check(&send(vec![t])),
            Err(ListError::NothingUsable { refused: 1 })
        );
        let elsewhere =
            json!({ "page": "https://www.beatport.com.evil.net/x", "tracks": [track("1")] });
        assert_eq!(check(&elsewhere), Err(ListError::Unsupported));
        assert_eq!(
            check(&json!({ "page": "https://www.beatport.com/", "tracks": [], "x": 1 })),
            Err(ListError::Unsupported)
        );
    }

    #[test]
    fn keys_hosts_and_pastes() {
        assert!(parse_key("bp.b776a31b-cb84-477f-a7bb-0476c5a1ee8e").is_some());
        assert!(parse_key("bp.b776a31b").is_none());
        assert!(sample_host("geo-samples.beatport.com") && !sample_host("beatport.com.evil.net"));
        assert!(looks_like_beatport("https://www.beatport.com/release/x/1"));
        assert!(!looks_like_beatport("https://www.discogs.com/release/1"));
    }
}
