//! HTTP to Discogs behind a trait, so every test runs offline against recorded JSON.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

pub const API: &str = "https://api.discogs.com";
pub const USER_AGENT: &str = concat!(
    "Diggr/",
    env!("CARGO_PKG_VERSION"),
    " +https://github.com/TOAST3R/diggr"
);
pub const ACCEPT: &str = "application/vnd.discogs.v2.discogs+json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Put,
    Delete,
    Post,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    /// Path and query, e.g. `/labels/1/releases?page=2&per_page=100`.
    pub path: String,
    pub token: Option<String>,
    /// A JSON body (the cart's POST); `None` sends an empty one.
    pub body: Option<String>,
}

impl Request {
    pub fn get(path: impl Into<String>, token: Option<&str>) -> Self {
        Self {
            method: Method::Get,
            path: path.into(),
            token: token.map(str::to_owned),
            body: None,
        }
    }

    /// Every header Discogs is sent.
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        let mut h = vec![
            ("User-Agent", USER_AGENT.to_owned()),
            ("Accept", ACCEPT.to_owned()),
        ];
        if let Some(t) = &self.token {
            h.push(("Authorization", format!("Discogs token={t}")));
        }
        if self.body.is_some() {
            h.push(("Content-Type", "application/json".to_owned()));
        }
        h
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// `X-Discogs-Ratelimit-Remaining`, when sent.
    pub remaining: Option<u32>,
    pub body: String,
}

/// The request never got an answer (no network, DNS, timeout).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetError(pub String);

pub trait Transport: Send + Sync {
    fn call(&self, req: &Request) -> Result<Response, NetError>;
}

/// The real thing: `ureq` with rustls.
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl Default for UreqTransport {
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

impl Transport for UreqTransport {
    fn call(&self, req: &Request) -> Result<Response, NetError> {
        let url = format!("{API}{}", req.path);
        let headers = req.headers();
        let result = match req.method {
            Method::Get => {
                let mut b = self.agent.get(&url);
                for (k, v) in &headers {
                    b = b.header(*k, v);
                }
                b.call()
            }
            Method::Delete => {
                let mut b = self.agent.delete(&url);
                for (k, v) in &headers {
                    b = b.header(*k, v);
                }
                b.call()
            }
            Method::Put => {
                let mut b = self.agent.put(&url);
                for (k, v) in &headers {
                    b = b.header(*k, v);
                }
                b.send_empty()
            }
            Method::Post => {
                let mut b = self.agent.post(&url);
                for (k, v) in &headers {
                    b = b.header(*k, v);
                }
                match &req.body {
                    Some(json) => b.send(json.as_str()),
                    None => b.send_empty(),
                }
            }
        };
        let resp = result.map_err(|e| NetError(e.to_string()))?;
        let status = resp.status().as_u16();
        let remaining = resp
            .headers()
            .get("X-Discogs-Ratelimit-Remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok());
        let body = resp
            .into_body()
            .read_to_string()
            .map_err(|e| NetError(e.to_string()))?;
        Ok(Response {
            status,
            remaining,
            body,
        })
    }
}

/// What a fake call does instead of answering normally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    Status(u16),
    Network,
    /// Answers normally after a real sleep.
    Delay(Duration),
}

/// A request and the headers it carried.
pub type Logged = (Request, Vec<(&'static str, String)>);

/// Serves JSON by path: routes added in code, then `<fixtures>/<path>.json` files, where the
/// path's `/`, `?`, `&` and `=` become `_` (`/releases/1?curr_abbr=EUR` is
/// `releases_1_curr_abbr_EUR.json`). Anything else is a 404. A PUT, POST or DELETE answers
/// its route for that method when one is set ([`FakeTransport::route_for`]); otherwise a PUT
/// or DELETE succeeds with an empty body, and a POST (a collection add) answers a new
/// `instance_id` each time, as Discogs does. Records every request it sees.
#[derive(Default)]
pub struct FakeTransport {
    routes: Mutex<HashMap<String, (u16, String)>>,
    fixtures: Option<PathBuf>,
    faults: Mutex<VecDeque<Fault>>,
    offline: std::sync::atomic::AtomicBool,
    remaining: Mutex<Option<u32>>,
    log: Mutex<Vec<Logged>>,
    instances: std::sync::atomic::AtomicU64,
}

impl FakeTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fixtures(dir: impl Into<PathBuf>) -> Self {
        Self {
            fixtures: Some(dir.into()),
            ..Self::default()
        }
    }

    pub fn route(&self, path: impl Into<String>, status: u16, body: impl Into<String>) {
        self.routes
            .lock()
            .unwrap()
            .insert(path.into(), (status, body.into()));
    }

    /// The answer to a PUT, POST or DELETE on `path` (a GET uses [`FakeTransport::route`]).
    pub fn route_for(
        &self,
        method: Method,
        path: impl Into<String>,
        status: u16,
        body: impl Into<String>,
    ) {
        let key = format!("{method:?} {}", path.into());
        self.routes
            .lock()
            .unwrap()
            .insert(key, (status, body.into()));
    }

    /// The next calls fail this way, one fault per call.
    pub fn fault(&self, f: Fault) {
        self.faults.lock().unwrap().push_back(f);
    }

    /// Every call fails with a network error until this is set back.
    pub fn set_offline(&self, offline: bool) {
        self.offline
            .store(offline, std::sync::atomic::Ordering::SeqCst);
    }

    /// The rate-limit header value sent with every answer.
    pub fn set_remaining(&self, n: Option<u32>) {
        *self.remaining.lock().unwrap() = n;
    }

    /// Requests seen, with their headers, oldest first.
    pub fn log(&self) -> Vec<Logged> {
        self.log.lock().unwrap().clone()
    }

    pub fn paths(&self) -> Vec<String> {
        self.log().into_iter().map(|(r, _)| r.path).collect()
    }

    pub fn count(&self) -> usize {
        self.log.lock().unwrap().len()
    }

    fn fixture_name(path: &str) -> String {
        path.trim_start_matches('/')
            .replace(['/', '?', '&', '='], "_")
            + ".json"
    }
}

impl Transport for FakeTransport {
    fn call(&self, req: &Request) -> Result<Response, NetError> {
        self.log.lock().unwrap().push((req.clone(), req.headers()));
        if self.offline.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(NetError("offline".into()));
        }
        let fault = self.faults.lock().unwrap().pop_front();
        match fault {
            Some(Fault::Network) => return Err(NetError("connection reset".into())),
            Some(Fault::Status(s)) => {
                return Ok(Response {
                    status: s,
                    remaining: *self.remaining.lock().unwrap(),
                    body: String::new(),
                });
            }
            Some(Fault::Delay(d)) => std::thread::sleep(d),
            None => {}
        }
        let remaining = *self.remaining.lock().unwrap();
        if req.method != Method::Get {
            let key = format!("{:?} {}", req.method, req.path);
            if let Some((status, body)) = self.routes.lock().unwrap().get(&key).cloned() {
                return Ok(Response {
                    status,
                    remaining,
                    body,
                });
            }
        }
        match req.method {
            Method::Get => {}
            Method::Post => {
                let n = self
                    .instances
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return Ok(Response {
                    status: 201,
                    remaining,
                    body: format!(r#"{{"instance_id": {}}}"#, 900_001 + n),
                });
            }
            Method::Put | Method::Delete => {
                let status = if req.method == Method::Put { 201 } else { 204 };
                return Ok(Response {
                    status,
                    remaining,
                    body: String::new(),
                });
            }
        }
        let routed = self.routes.lock().unwrap().get(&req.path).cloned();
        let (status, body) = routed
            .or_else(|| {
                let f = self.fixtures.as_ref()?.join(Self::fixture_name(&req.path));
                Some((200, std::fs::read_to_string(f).ok()?))
            })
            .unwrap_or((404, r#"{"message": "Resource not found."}"#.into()));
        Ok(Response {
            status,
            remaining,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_carry_user_agent_accept_and_token() {
        let t = FakeTransport::new();
        t.route("/releases/1", 200, "{}");
        t.call(&Request::get("/releases/1", Some("s3cret")))
            .unwrap();
        t.call(&Request::get("/releases/1", None)).unwrap();
        let log = t.log();
        let header = |i: usize, name: &str| {
            log[i]
                .1
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.clone())
        };
        let ua = header(0, "User-Agent").unwrap();
        assert!(
            ua.starts_with("Diggr/0.1.0 +https://github.com/TOAST3R/diggr"),
            "{ua}"
        );
        assert_eq!(header(0, "Accept").unwrap(), ACCEPT);
        assert_eq!(
            header(0, "Authorization").as_deref(),
            Some("Discogs token=s3cret")
        );
        assert_eq!(header(1, "Authorization"), None, "no token, no header");
        assert!(header(1, "User-Agent").is_some());
    }

    #[test]
    fn fakes_serve_routes_fixtures_faults_and_404s() {
        let dir = crate::test_dir("transport");
        std::fs::write(dir.join("releases_7_curr_abbr_EUR.json"), r#"{"id":7}"#).unwrap();
        let t = FakeTransport::with_fixtures(dir.path());
        t.set_remaining(Some(59));
        let get = |p: &str| t.call(&Request::get(p, None));
        let r = get("/releases/7?curr_abbr=EUR").unwrap();
        assert_eq!(
            (r.status, r.body.as_str(), r.remaining),
            (200, r#"{"id":7}"#, Some(59))
        );
        assert_eq!(get("/releases/8").unwrap().status, 404);
        t.fault(Fault::Status(429));
        t.fault(Fault::Network);
        assert_eq!(get("/releases/7?curr_abbr=EUR").unwrap().status, 429);
        assert!(get("/releases/7?curr_abbr=EUR").is_err());
        assert_eq!(get("/releases/7?curr_abbr=EUR").unwrap().status, 200);
        t.set_offline(true);
        assert!(get("/releases/7?curr_abbr=EUR").is_err());
        assert_eq!(t.count(), 6);
    }

    #[test]
    fn a_fake_post_adds_a_new_instance_each_time() {
        let t = FakeTransport::new();
        let post = |p: &str| {
            t.call(&Request {
                method: Method::Post,
                path: p.into(),
                token: Some("tok".into()),
                body: None,
            })
            .unwrap()
        };
        let a = post("/users/digger/collection/folders/1/releases/1001");
        let b = post("/users/digger/collection/folders/1/releases/1001");
        assert_eq!(a.status, 201);
        assert_eq!(a.body, r#"{"instance_id": 900001}"#);
        assert_eq!(
            b.body, r#"{"instance_id": 900002}"#,
            "every add is another copy"
        );
    }

    #[test]
    fn a_json_body_is_sent_and_routed_by_method() {
        let t = FakeTransport::new();
        t.route_for(Method::Post, "/cart/items", 201, r#"{"errors":[]}"#);
        let req = Request {
            method: Method::Post,
            path: "/cart/items".into(),
            token: Some("tok".into()),
            body: Some(r#"{"item_ids":[7]}"#.into()),
        };
        let r = t.call(&req).unwrap();
        assert_eq!((r.status, r.body.as_str()), (201, r#"{"errors":[]}"#));
        let (sent, headers) = &t.log()[0];
        assert_eq!(sent.body.as_deref(), Some(r#"{"item_ids":[7]}"#));
        assert!(headers.contains(&("Content-Type", "application/json".to_owned())));
        assert!(
            !Request::get("/cart", None)
                .headers()
                .iter()
                .any(|(k, _)| *k == "Content-Type"),
            "no body, no content type"
        );
    }
}
