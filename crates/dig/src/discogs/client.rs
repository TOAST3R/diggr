//! Requests to Discogs, one at a time, within the rate limit, with errors the user can act on.

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use super::cache::DiskCache;
use super::ratelimit::RateLimiter;
use super::transport::{Method, Request, Response, Transport};
use crate::clock::Clock;

/// 429s in a row before giving up on a request (the waits add up to over two minutes).
const MAX_429: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// No answer at all; work pauses and resumes once Discogs answers again.
    Offline,
    /// 401: the token is wrong or revoked.
    TokenRejected,
    /// 403: a private resource, such as another user's wantlist.
    Private,
    /// 404: no such release, label, user…
    NotFound,
    /// The action needs an account (a token).
    TokenNeeded,
    /// Anything else (a 5xx, a body that isn't what we expect).
    Other(String),
}

impl ApiError {
    /// What the main window says.
    pub fn message(&self) -> String {
        match self {
            ApiError::Offline => "Discogs offline".into(),
            ApiError::TokenRejected => "Discogs rejected the token (Options ▸ Discogs…)".into(),
            ApiError::Private => "That Discogs page is private".into(),
            ApiError::NotFound => "That Discogs page wasn't found".into(),
            ApiError::TokenNeeded => {
                "A Discogs token is needed for that (Options ▸ Discogs…)".into()
            }
            ApiError::Other(e) => format!("Discogs error: {e}"),
        }
    }
}

/// Who the token belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub username: String,
    /// The user's marketplace currency, e.g. "EUR".
    pub currency: String,
}

pub struct Client {
    transport: Arc<dyn Transport>,
    clock: Arc<dyn Clock>,
    limiter: RateLimiter,
    token: Option<String>,
    pub cache: DiskCache,
    identity: Option<Identity>,
}

impl Client {
    pub fn new(
        transport: Arc<dyn Transport>,
        clock: Arc<dyn Clock>,
        token: Option<String>,
        cache: DiskCache,
    ) -> Self {
        let token = token.filter(|t| !t.trim().is_empty());
        Self {
            limiter: RateLimiter::for_token(token.is_some()),
            transport,
            clock,
            token,
            cache,
            identity: None,
        }
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// A new token (or none): the limit changes with it, and the identity is checked again.
    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token.filter(|t| !t.trim().is_empty());
        self.limiter
            .set_limit(RateLimiter::for_token(self.token.is_some()).limit());
        self.identity = None;
    }

    pub fn identity(&self) -> Option<&Identity> {
        self.identity.as_ref()
    }

    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }

    /// Prices are asked for in the user's currency, or USD without a token.
    pub fn currency(&self) -> &str {
        self.identity
            .as_ref()
            .map_or("USD", |i| i.currency.as_str())
    }

    /// Checks the token: `/oauth/identity` for the username, then the user's profile for the
    /// currency. Without a token there is nothing to check.
    pub fn check_identity(&mut self) -> Result<Identity, ApiError> {
        if self.token.is_none() {
            return Err(ApiError::TokenNeeded);
        }
        let me = self.get_json("/oauth/identity")?;
        let username = me["username"]
            .as_str()
            .ok_or_else(|| ApiError::Other("no username in /oauth/identity".into()))?
            .to_owned();
        let profile = self.get_json(&format!("/users/{}", path_segment(&username)))?;
        let currency = profile["curr_abbr"].as_str().unwrap_or("USD").to_owned();
        let id = Identity { username, currency };
        self.identity = Some(id.clone());
        Ok(id)
    }

    /// The identity, checked once per session.
    pub fn ensure_identity(&mut self) -> Result<Identity, ApiError> {
        match &self.identity {
            Some(i) => Ok(i.clone()),
            None => self.check_identity(),
        }
    }

    pub fn get_json(&mut self, path: &str) -> Result<Value, ApiError> {
        let r = self.call(Method::Get, path)?;
        serde_json::from_str(&r.body).map_err(|e| ApiError::Other(format!("{path}: {e}")))
    }

    /// Sends one request once the rate limit allows it, waiting out 429s.
    pub fn call(&mut self, method: Method, path: &str) -> Result<Response, ApiError> {
        let req = Request {
            method,
            path: path.to_owned(),
            token: self.token.clone(),
        };
        for _ in 0..MAX_429 {
            self.clock.sleep(self.limiter.delay(self.clock.now()));
            let sent = self.clock.now();
            let result = self.transport.call(&req);
            let remaining = result.as_ref().ok().and_then(|r| r.remaining);
            self.limiter.record(sent, remaining);
            let r = result.map_err(|_| ApiError::Offline)?;
            match r.status {
                200..=299 => {
                    self.limiter.succeeded();
                    return Ok(r);
                }
                429 => {
                    self.limiter.too_many(self.clock.now());
                }
                401 if self.token.is_none() => return Err(ApiError::TokenNeeded),
                401 => return Err(ApiError::TokenRejected),
                403 => return Err(ApiError::Private),
                404 => return Err(ApiError::NotFound),
                s => return Err(ApiError::Other(format!("HTTP {s} for {path}"))),
            }
        }
        Err(ApiError::Other("Discogs kept saying it is too busy".into()))
    }

    /// Waits without sending (offline retries), on the same clock.
    pub fn sleep(&self, d: Duration) {
        self.clock.sleep(d);
    }
}

/// A username or slug as one path segment: anything but letters, digits, `-`, `_` and `.` is
/// percent-encoded.
pub fn path_segment(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::discogs::transport::{FakeTransport, Fault};

    fn client(t: &Arc<FakeTransport>, token: Option<&str>) -> (Client, Arc<FakeClock>) {
        let clock = Arc::new(FakeClock::default());
        let c = Client::new(
            t.clone(),
            clock.clone(),
            token.map(str::to_owned),
            DiskCache::default(),
        );
        (c, clock)
    }

    #[test]
    fn a_good_token_gives_the_username_and_currency() {
        let t = Arc::new(FakeTransport::new());
        t.route("/oauth/identity", 200, r#"{"id":1,"username":"digger"}"#);
        t.route(
            "/users/digger",
            200,
            r#"{"username":"digger","curr_abbr":"EUR"}"#,
        );
        let (mut c, _) = client(&t, Some("tok"));
        assert_eq!(c.currency(), "USD");
        let id = c.check_identity().unwrap();
        assert_eq!(
            (id.username.as_str(), id.currency.as_str()),
            ("digger", "EUR")
        );
        assert_eq!(c.currency(), "EUR");
        assert_eq!(t.paths(), ["/oauth/identity", "/users/digger"]);
    }

    #[test]
    fn errors_are_mapped() {
        let t = Arc::new(FakeTransport::new());
        t.route(
            "/oauth/identity",
            401,
            r#"{"message":"You must authenticate"}"#,
        );
        t.route("/users/someone/wants?page=1&per_page=100", 403, "{}");
        let (mut c, _) = client(&t, Some("bad"));
        assert_eq!(c.check_identity(), Err(ApiError::TokenRejected));
        assert_eq!(c.get_json("/labels/999999999"), Err(ApiError::NotFound));
        assert_eq!(
            c.get_json("/users/someone/wants?page=1&per_page=100"),
            Err(ApiError::Private)
        );
        t.fault(Fault::Network);
        assert_eq!(c.get_json("/labels/1"), Err(ApiError::Offline));
        t.fault(Fault::Status(503));
        assert!(matches!(c.get_json("/labels/1"), Err(ApiError::Other(_))));
        let (mut none, _) = client(&t, None);
        assert_eq!(none.check_identity(), Err(ApiError::TokenNeeded));
        for e in [
            ApiError::Offline,
            ApiError::TokenRejected,
            ApiError::NotFound,
        ] {
            assert!(!e.message().contains("bad"), "never shows the token");
        }
    }

    #[test]
    fn a_429_waits_and_retries() {
        let t = Arc::new(FakeTransport::new());
        t.route("/releases/1", 200, "{}");
        t.fault(Fault::Status(429));
        t.fault(Fault::Status(429));
        let (mut c, clock) = client(&t, Some("tok"));
        assert!(c.get_json("/releases/1").is_ok());
        assert_eq!(t.count(), 3);
        assert_eq!(clock.now(), Duration::from_secs(30), "10 s then 20 s");
    }

    #[test]
    fn usernames_are_escaped_in_paths() {
        assert_eq!(path_segment("dj x/y"), "dj%20x%2Fy");
        assert_eq!(path_segment("a.b-c_d"), "a.b-c_d");
    }
}
