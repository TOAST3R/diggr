//! Connect with Discogs: OAuth 1.0a with the PLAINTEXT signature, which Discogs accepts over
//! HTTPS (every request is HTTPS), so no request needs a base string or a crypto dependency.
//!
//! The flow (`discogs-oauth`): the app asks for a request token with a callback on diggr.es,
//! the user authorizes in the browser, the diggr.es page forwards the answer to
//! `http://127.0.0.1:‹port›/discogs`, and the app exchanges the verifier for an access token.
//! The request token's secret never leaves the intake worker, so the verifier that travels
//! through the browser and the site is useless on its own.
//!
//! The consumer key and secret are built into release builds (`option_env!`, from the
//! maintainer's Keychain through the release scripts). A build without them has no Connect with
//! Discogs; the personal token still works.

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Where Discogs sends the browser after Authorize; `?port=` tells the page where the app is.
pub const CALLBACK: &str = "https://diggr.es/discogs/callback";
pub const AUTHORIZE: &str = "https://www.discogs.com/oauth/authorize";
pub const REQUEST_TOKEN: &str = "/oauth/request_token";
pub const ACCESS_TOKEN: &str = "/oauth/access_token";
/// The path the callback page forwards to, on the bridge or the one-time listener.
pub const CALLBACK_PATH: &str = "/discogs";
/// How long the app waits for the browser to come back.
pub const WAIT: Duration = Duration::from_secs(5 * 60);

/// The app's key and secret, as registered with Discogs.
#[derive(Clone, PartialEq, Eq)]
pub struct Consumer {
    pub key: String,
    pub secret: String,
}

impl fmt::Debug for Consumer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Consumer({}, ••••)", self.key)
    }
}

impl Consumer {
    /// The key and secret this build was made with, if any.
    pub fn built_in() -> Option<Self> {
        Self::from_parts(
            option_env!("DISCOGS_CONSUMER_KEY"),
            option_env!("DISCOGS_CONSUMER_SECRET"),
        )
    }

    fn from_parts(key: Option<&str>, secret: Option<&str>) -> Option<Self> {
        let (key, secret) = (key?.trim(), secret?.trim());
        (!key.is_empty() && !secret.is_empty()).then(|| Self {
            key: key.to_owned(),
            secret: secret.to_owned(),
        })
    }
}

/// What one OAuth request is signed with: the consumer, the token (none when asking for a
/// request token) and its secret, and the step's own parameters (`oauth_callback`,
/// `oauth_verifier`). A connected account has no parameters.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    pub consumer: Consumer,
    pub token: Option<String>,
    pub secret: String,
    pub params: Vec<(&'static str, String)>,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Credentials(••••)")
    }
}

impl Credentials {
    /// A connected account: its access token and secret.
    pub fn account(consumer: Consumer, token: String, secret: String) -> Self {
        Self {
            consumer,
            token: Some(token),
            secret,
            params: Vec::new(),
        }
    }
}

/// A request token waiting for the user's Authorize.
#[derive(Clone, PartialEq, Eq)]
pub struct RequestToken {
    pub token: String,
    pub secret: String,
}

impl fmt::Debug for RequestToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RequestToken({}, ••••)", self.token)
    }
}

/// RFC 5849 percent-encoding: everything but `A-Z a-z 0-9 - . _ ~`.
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The `Authorization` header's value for one request.
pub fn header(c: &Credentials, nonce: &str, timestamp: u64) -> String {
    let signature = format!("{}&{}", encode(&c.consumer.secret), encode(&c.secret));
    let mut fields: Vec<(&str, String)> = vec![
        ("oauth_consumer_key", c.consumer.key.clone()),
        ("oauth_nonce", nonce.to_owned()),
        ("oauth_signature", signature),
        ("oauth_signature_method", "PLAINTEXT".to_owned()),
        ("oauth_timestamp", timestamp.to_string()),
    ];
    if let Some(t) = &c.token {
        fields.push(("oauth_token", t.clone()));
    }
    fields.push(("oauth_version", "1.0".to_owned()));
    fields.extend(c.params.iter().map(|(k, v)| (*k, v.clone())));
    let parts: Vec<String> = fields
        .into_iter()
        .map(|(k, v)| {
            // The signature is made of encoded parts, and sent as it is: its `&` is the
            // separator (what Discogs accepted in the manual check, `findings.md`).
            let v = if k == "oauth_signature" {
                v
            } else {
                encode(&v)
            };
            format!("{k}=\"{v}\"")
        })
        .collect();
    format!("OAuth {}", parts.join(", "))
}

/// 16 random bytes, hex; a counter and the clock if the system has no randomness to give.
pub fn nonce() -> String {
    let mut b = [0u8; 16];
    if getrandom::fill(&mut b).is_ok() {
        return b.iter().map(|x| format!("{x:02x}")).collect();
    }
    static N: AtomicU64 = AtomicU64::new(0);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{t:x}{:x}", N.fetch_add(1, Ordering::Relaxed))
}

pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// How far this computer's clock may be from Discogs's before a refused signed request is
/// blamed on the clock: every OAuth request carries its time, and Discogs refuses one too far
/// from its own (seen: a Mac 46 minutes slow).
pub const CLOCK_SLACK: Duration = Duration::from_secs(5 * 60);

/// Seconds this computer's clock is behind Discogs's (`server`, its `Date` header), negative
/// when ahead; `None` within [`CLOCK_SLACK`] or without a date.
pub fn clock_off(server: Option<SystemTime>, now: SystemTime) -> Option<i64> {
    let server = server?;
    let off = match server.duration_since(now) {
        Ok(behind) => behind.as_secs() as i64,
        Err(ahead) => -(ahead.duration().as_secs() as i64),
    };
    (off.unsigned_abs() > CLOCK_SLACK.as_secs()).then_some(off)
}

/// What to tell the user when the clock is `off` seconds behind Discogs (negative: ahead),
/// with where their system sets the time.
pub fn clock_message(off: i64) -> String {
    let n = off.unsigned_abs();
    let (count, unit) = if n < 2 * 3600 {
        ((n + 30) / 60, "minute")
    } else if n < 2 * 86400 {
        ((n + 1800) / 3600, "hour")
    } else {
        ((n + 43200) / 86400, "day")
    };
    let s = if count == 1 { "" } else { "s" };
    let way = if off > 0 { "slow" } else { "fast" };
    let fix = if cfg!(target_os = "macos") {
        "turn on Set time and date automatically in System Settings ▸ General ▸ Date & Time"
    } else if cfg!(windows) {
        "turn on Set time automatically in Settings ▸ Time & language ▸ Date & time"
    } else {
        "set the date and time automatically in your system settings"
    };
    format!(
        "This computer's clock is {count} {unit}{s} {way}, so Discogs refused it; {fix}, then try again"
    )
}

/// The callback Discogs is given: diggr.es, with the port the app listens on.
pub fn callback(port: u16) -> String {
    format!("{CALLBACK}?port={port}")
}

/// The page the user authorizes on.
pub fn authorize_url(request_token: &str) -> String {
    format!("{AUTHORIZE}?oauth_token={}", encode(request_token))
}

/// `a=1&b=%20x` → `{a: "1", b: " x"}` (an exchange's answer, or a callback's query).
pub fn parse_form(s: &str) -> HashMap<String, String> {
    s.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (decode(k), decode(v))
        })
        .collect()
}

fn decode(s: &str) -> String {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let escaped = (b[i] == b'%' && i + 2 < b.len())
            .then(|| Some(hex(b[i + 1])? * 16 + hex(b[i + 2])?))
            .flatten();
        match (escaped, b[i]) {
            (Some(x), _) => {
                out.push(x);
                i += 3;
                continue;
            }
            (None, b'+') => out.push(b' '),
            (None, c) => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What came back on the callback address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackAnswer {
    Verifier(String),
    Denied,
}

/// What the callback address answers the browser with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackPage {
    Connected,
    Denied,
    NotFound,
}

impl CallbackPage {
    pub fn status(self) -> u16 {
        match self {
            CallbackPage::NotFound => 404,
            _ => 200,
        }
    }

    pub fn html(self) -> &'static str {
        match self {
            CallbackPage::Connected => PAGE_CONNECTED,
            CallbackPage::Denied => PAGE_DENIED,
            CallbackPage::NotFound => PAGE_NOT_FOUND,
        }
    }
}

const PAGE_CONNECTED: &str = "<!doctype html><meta charset=utf-8><title>Diggr is connected</title>\
<body style=\"background:#0b0a09;color:#f3ede2;font:16px system-ui,sans-serif;display:grid;\
place-items:center;height:90vh\"><div style=\"text-align:center\"><h1>Diggr is connected</h1>\
<p>You can close this tab and go back to Diggr.</p></div></body>";
const PAGE_DENIED: &str = "<!doctype html><meta charset=utf-8><title>Diggr wasn't allowed</title>\
<body style=\"background:#0b0a09;color:#f3ede2;font:16px system-ui,sans-serif;display:grid;\
place-items:center;height:90vh\"><div style=\"text-align:center\"><h1>Diggr wasn't allowed</h1>\
<p>Nothing was connected. You can close this tab.</p></div></body>";
const PAGE_NOT_FOUND: &str = "<!doctype html><meta charset=utf-8><title>Not found</title>\
<body>Not found</body>";

/// The one answer the app waits for, shared by the bridge's `/discogs` route and the one-time
/// listener: set while a connection waits, answered once, by the request token it was given.
#[derive(Default)]
pub struct CallbackSlot {
    state: Mutex<SlotState>,
}

#[derive(Default)]
struct SlotState {
    /// The request token and when the wait ends.
    waiting: Option<(String, Instant)>,
    answer: Option<CallbackAnswer>,
}

impl CallbackSlot {
    fn lock(&self) -> std::sync::MutexGuard<'_, SlotState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Waits for the answer to `request_token`, until `deadline`.
    pub fn wait_for(&self, request_token: &str, deadline: Instant) {
        let mut s = self.lock();
        s.waiting = Some((request_token.to_owned(), deadline));
        s.answer = None;
    }

    /// Stops waiting (Cancel, or the app moved on); a later answer gets a 404.
    pub fn cancel(&self) {
        let mut s = self.lock();
        s.waiting = None;
        s.answer = None;
    }

    pub fn waiting(&self, now: Instant) -> bool {
        self.lock().waiting.as_ref().is_some_and(|(_, d)| now < *d)
    }

    /// The wait ran out without an answer (it is then over).
    pub fn expired(&self, now: Instant) -> bool {
        let mut s = self.lock();
        let over = s.waiting.as_ref().is_some_and(|(_, d)| now >= *d);
        if over {
            s.waiting = None;
        }
        over
    }

    /// The answer, once.
    pub fn take(&self) -> Option<CallbackAnswer> {
        self.lock().answer.take()
    }

    /// A request to the callback path, with its query (after `?`): answered only while a
    /// connection waits and only for its request token, once; anything else is not found.
    pub fn handle(&self, query: &str, now: Instant) -> CallbackPage {
        let q = parse_form(query);
        let mut s = self.lock();
        let Some((token, deadline)) = s.waiting.clone() else {
            return CallbackPage::NotFound;
        };
        if now >= deadline {
            s.waiting = None;
            return CallbackPage::NotFound;
        }
        if q.get("denied").is_some_and(|d| *d == token) {
            s.waiting = None;
            s.answer = Some(CallbackAnswer::Denied);
            return CallbackPage::Denied;
        }
        let verifier = q.get("oauth_verifier").filter(|v| valid(v));
        match (q.get("oauth_token"), verifier) {
            (Some(t), Some(v)) if *t == token => {
                s.waiting = None;
                s.answer = Some(CallbackAnswer::Verifier(v.clone()));
                CallbackPage::Connected
            }
            _ => CallbackPage::NotFound,
        }
    }
}

/// The shape the callback page also checks: 1 to 200 characters of `[A-Za-z0-9._~-]`.
fn valid(v: &str) -> bool {
    (1..=200).contains(&v.len())
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._~-".contains(&b))
}

/// The callback address is addressed to this computer by address (against DNS rebinding).
pub fn host_ok(host: Option<&str>, port: u16) -> bool {
    host.is_some_and(|h| {
        let h = h.to_ascii_lowercase();
        h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consumer() -> Consumer {
        Consumer {
            key: "ck".into(),
            secret: "cs".into(),
        }
    }

    #[test]
    fn a_build_without_keys_has_no_consumer() {
        assert_eq!(Consumer::from_parts(None, Some("s")), None);
        assert_eq!(Consumer::from_parts(Some(" "), Some("s")), None);
        assert_eq!(
            Consumer::from_parts(Some("k"), Some("s")),
            Some(Consumer {
                key: "k".into(),
                secret: "s".into()
            })
        );
    }

    #[test]
    fn the_header_is_signed_with_plaintext() {
        let c = Credentials::account(consumer(), "at".into(), "as".into());
        assert_eq!(
            header(&c, "n0nce", 1_700_000_000),
            "OAuth oauth_consumer_key=\"ck\", oauth_nonce=\"n0nce\", oauth_signature=\"cs&as\", \
             oauth_signature_method=\"PLAINTEXT\", oauth_timestamp=\"1700000000\", \
             oauth_token=\"at\", oauth_version=\"1.0\""
        );
    }

    #[test]
    fn a_request_token_step_has_no_token_and_carries_its_callback() {
        let c = Credentials {
            consumer: consumer(),
            token: None,
            secret: String::new(),
            params: vec![("oauth_callback", callback(47811))],
        };
        let h = header(&c, "n", 1);
        assert!(h.contains("oauth_signature=\"cs&\""), "{h}");
        assert!(!h.contains("oauth_token"), "{h}");
        assert!(
            h.contains(
                "oauth_callback=\"https%3A%2F%2Fdiggr.es%2Fdiscogs%2Fcallback%3Fport%3D47811\""
            ),
            "{h}"
        );
    }

    #[test]
    fn awkward_characters_are_encoded() {
        assert_eq!(encode("a b&c=d/é~._-"), "a%20b%26c%3Dd%2F%C3%A9~._-");
        let c = Credentials::account(
            Consumer {
                key: "k\"y".into(),
                secret: "s&t".into(),
            },
            "t".into(),
            "x y".into(),
        );
        let h = header(&c, "n", 1);
        assert!(h.contains("oauth_consumer_key=\"k%22y\""), "{h}");
        assert!(h.contains("oauth_signature=\"s%26t&x%20y\""), "{h}");
    }

    #[test]
    fn every_nonce_is_fresh() {
        let (a, b) = (nonce(), nonce());
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn secrets_stay_out_of_debug_output() {
        let c = Credentials::account(consumer(), "at".into(), "as3cret".into());
        let r = RequestToken {
            token: "rt".into(),
            secret: "rs3cret".into(),
        };
        let shown = format!("{c:?} {r:?} {:?}", consumer());
        for s in ["as3cret", "rs3cret", "\"cs\"", "cs)"] {
            assert!(!shown.contains(s), "{shown}");
        }
    }

    #[test]
    fn forms_are_parsed_and_decoded() {
        let f =
            parse_form("oauth_token=abc&oauth_token_secret=x%2By+z&oauth_callback_confirmed=true");
        assert_eq!(f["oauth_token"], "abc");
        assert_eq!(f["oauth_token_secret"], "x+y z");
        assert_eq!(f["oauth_callback_confirmed"], "true");
        assert_eq!(parse_form("a=%"), HashMap::from([("a".into(), "%".into())]));
    }

    #[test]
    fn a_clock_off_by_more_than_the_slack_is_reported() {
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let at = |s: i64| {
            Some(if s >= 0 {
                now + Duration::from_secs(s as u64)
            } else {
                now - Duration::from_secs(s.unsigned_abs())
            })
        };
        assert_eq!(clock_off(None, now), None, "no Date header");
        assert_eq!(clock_off(at(0), now), None);
        assert_eq!(clock_off(at(300), now), None, "within 5 minutes");
        assert_eq!(clock_off(at(-300), now), None);
        assert_eq!(
            clock_off(at(2748), now),
            Some(2748),
            "the Mac 46 minutes slow"
        );
        assert_eq!(clock_off(at(-3 * 3600), now), Some(-3 * 3600));
    }

    #[test]
    fn the_clock_message_says_how_far_and_which_way() {
        let m = clock_message(2748);
        assert!(
            m.starts_with("This computer's clock is 46 minutes slow, so Discogs refused it; "),
            "{m}"
        );
        assert!(m.ends_with(", then try again"), "{m}");
        assert!(m.contains("automatically"), "{m}");
        assert!(clock_message(-60).contains("is 1 minute fast"));
        assert!(clock_message(3 * 3600).contains("is 3 hours slow"));
        assert!(clock_message(-400 * 86400).contains("is 400 days fast"));
    }

    #[test]
    fn urls_for_discogs() {
        assert_eq!(
            authorize_url("abc"),
            "https://www.discogs.com/oauth/authorize?oauth_token=abc"
        );
        assert_eq!(
            callback(47800),
            "https://diggr.es/discogs/callback?port=47800"
        );
    }

    #[test]
    fn the_slot_answers_only_its_request_token_once() {
        let slot = CallbackSlot::default();
        let now = Instant::now();
        let q = "oauth_token=rt&oauth_verifier=V3rif1er";
        assert_eq!(slot.handle(q, now), CallbackPage::NotFound, "nothing waits");
        slot.wait_for("rt", now + WAIT);
        assert_eq!(
            slot.handle("oauth_token=other&oauth_verifier=V", now),
            CallbackPage::NotFound
        );
        assert_eq!(
            slot.handle("oauth_token=rt&oauth_verifier=bad%20v", now),
            CallbackPage::NotFound,
            "a verifier of the wrong shape"
        );
        assert!(slot.waiting(now), "still waiting after wrong requests");
        assert_eq!(slot.handle(q, now), CallbackPage::Connected);
        assert_eq!(
            slot.take(),
            Some(CallbackAnswer::Verifier("V3rif1er".into()))
        );
        assert_eq!(slot.take(), None);
        assert_eq!(slot.handle(q, now), CallbackPage::NotFound, "only once");
    }

    #[test]
    fn a_deny_ends_the_wait() {
        let slot = CallbackSlot::default();
        let now = Instant::now();
        slot.wait_for("rt", now + WAIT);
        assert_eq!(slot.handle("denied=other", now), CallbackPage::NotFound);
        assert_eq!(slot.handle("denied=rt", now), CallbackPage::Denied);
        assert_eq!(slot.take(), Some(CallbackAnswer::Denied));
        assert!(!slot.waiting(now));
    }

    #[test]
    fn the_wait_runs_out_and_cancel_stops_it() {
        let slot = CallbackSlot::default();
        let now = Instant::now();
        slot.wait_for("rt", now + Duration::from_secs(1));
        let later = now + Duration::from_secs(2);
        assert!(!slot.expired(now));
        assert_eq!(
            slot.handle("oauth_token=rt&oauth_verifier=V", later),
            CallbackPage::NotFound
        );
        slot.wait_for("rt", now + Duration::from_secs(1));
        assert!(slot.expired(later));
        assert!(!slot.expired(later), "said once");
        slot.wait_for("rt", now + WAIT);
        slot.cancel();
        assert_eq!(
            slot.handle("oauth_token=rt&oauth_verifier=V", now),
            CallbackPage::NotFound
        );
    }

    #[test]
    fn only_addresses_of_this_computer() {
        assert!(host_ok(Some("127.0.0.1:47800"), 47800));
        assert!(host_ok(Some("LOCALHOST:47800"), 47800));
        assert!(!host_ok(Some("evil.example:47800"), 47800));
        assert!(!host_ok(Some("127.0.0.1:47801"), 47800));
        assert!(!host_ok(None, 47800));
    }
}
