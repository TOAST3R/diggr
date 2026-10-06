//! The browser bridge: a tiny HTTP server on 127.0.0.1 through which a paired browser
//! extension sends Discogs pages to the player.
//!
//! It is the app's only network listener, so it accepts as little as it can: loopback only,
//! requests addressed to 127.0.0.1 or localhost (against DNS rebinding), no web-page origins,
//! no preflight and no cross-origin headers (so no web page can use it), a key from pairing on
//! every request that does anything, bodies of at most 16 KB with no unknown fields, and only
//! supported Discogs addresses.
//!
//! Every request is answered on the bridge thread without waiting for the UI or Discogs: a
//! send is checked, answered `202` at once, and handed to the UI as a [`BridgeCommand`]; the
//! crates and status come from a [`Snapshot`] the UI keeps up to date.

pub mod pairing;

use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use platform::{Priority, Spawner};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::collection::{Collection, Owned};
use crate::discogs::cache::{DiskCache, Kind};
use crate::discogs::url::{self, Page, PageKind};
use crate::jobs::Filters;
pub use pairing::{DEFAULT_PORT, PairError, Pairing};

/// The bridge's protocol version, reported by `/v1/hello`.
pub const API: u32 = 1;
/// Largest request body accepted.
pub const MAX_BODY: usize = 16 * 1024;
/// Crate names: 1 to 40 characters (the crates' own limit).
pub const MAX_CRATE_NAME: usize = 40;
pub const KEY_HEADER: &str = "X-Bridge-Key";

/// Where a sent page's tracks go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Into a new crate named after the page, played when ready.
    Play,
    /// To the end of the shown crate.
    Enqueue,
    /// Into the named crate, created if needed.
    Crate(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeCommand {
    Send {
        page: Page,
        mode: Mode,
        filters: Filters,
    },
    /// A browser was paired.
    Paired,
    /// The browser asked about a marketplace item whose release isn't known yet: look it up
    /// (once; it is cached for good).
    ResolveShopItem(u64),
}

/// What `/v1/crates` and `/v1/status` answer, kept up to date by the UI.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Snapshot {
    pub crates: Vec<CrateState>,
    pub playing: Option<Playing>,
    pub sends: Vec<SendState>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CrateState {
    pub name: String,
    pub shown: bool,
    pub playing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Playing {
    pub artist: String,
    pub title: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendState {
    pub page: String,
    pub done: usize,
    pub total: usize,
}

/// What the bridge thread and the UI share.
pub struct Shared {
    pub pairing: Mutex<Pairing>,
    pub snapshot: ArcSwap<Snapshot>,
    /// The user's collection, for `/v1/owned` (`None`: no token, or none fetched yet).
    pub collection: ArcSwap<Option<Arc<Collection>>>,
    /// A Discogs token is set (without one, `/v1/owned` says so, so the browser can tell the
    /// user what's missing).
    pub has_token: std::sync::atomic::AtomicBool,
    /// Discogs data already fetched (a release's master, a marketplace item's release), read
    /// for `/v1/owned` without any request.
    cache: DiskCache,
}

impl Shared {
    pub fn new(pairing: Pairing) -> Arc<Self> {
        Self::with_cache(pairing, None)
    }

    /// `cache_root` is the app's cache folder (where the Discogs responses are).
    pub fn with_cache(pairing: Pairing, cache_root: Option<&std::path::Path>) -> Arc<Self> {
        Arc::new(Self {
            pairing: Mutex::new(pairing),
            snapshot: ArcSwap::from_pointee(Snapshot::default()),
            collection: ArcSwap::from_pointee(None),
            has_token: std::sync::atomic::AtomicBool::new(false),
            cache: DiskCache::new(cache_root),
        })
    }

    pub fn pairing(&self) -> std::sync::MutexGuard<'_, Pairing> {
        self.pairing.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The running server; dropping it stops the thread.
pub struct BridgeHandle {
    server: Arc<tiny_http::Server>,
    port: u16,
    commands: Receiver<BridgeCommand>,
}

impl BridgeHandle {
    /// Binds 127.0.0.1:`port` (0 picks a free one) and serves on a low-priority thread; `wake`
    /// is called when a command is ready. Fails with a message when the port is taken.
    pub fn start(
        spawner: &dyn Spawner,
        shared: Arc<Shared>,
        port: u16,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, String> {
        let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| {
            if port != 0 && std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
                format!("Port {port} is taken by another program")
            } else {
                format!("Could not listen on port {port}: {e}")
            }
        })?;
        let server = Arc::new(server);
        let port = server.server_addr().to_ip().map_or(port, |a| a.port());
        let (tx, rx) = channel();
        let s = server.clone();
        spawner
            .spawn(
                "dig-bridge",
                Priority::Low,
                Box::new(move || serve(&s, &shared, port, &tx, &wake)),
            )
            .map_err(|e| format!("Could not start the bridge: {e:?}"))?;
        Ok(Self {
            server,
            port,
            commands: rx,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn poll(&self) -> Vec<BridgeCommand> {
        self.commands.try_iter().collect()
    }
}

impl Drop for BridgeHandle {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

fn serve(
    server: &tiny_http::Server,
    shared: &Shared,
    port: u16,
    tx: &Sender<BridgeCommand>,
    wake: &dyn Fn(),
) {
    for mut req in server.incoming_requests() {
        let (status, body, cmd) = answer(&mut req, shared, port);
        // Queued before the answer goes out, so the UI never learns of a send after the browser.
        if let Some(c) = cmd {
            if tx.send(c).is_err() {
                return;
            }
            wake();
        }
        let json = tiny_http::Header::from_bytes("Content-Type", "application/json")
            .expect("valid header");
        let _ = req.respond(
            tiny_http::Response::from_string(body.to_string())
                .with_status_code(status)
                .with_header(json),
        );
    }
}

fn error(status: u16, what: &str) -> (u16, Value, Option<BridgeCommand>) {
    (status, json!({ "error": what }), None)
}

fn header<'a>(req: &'a tiny_http::Request, name: &'static str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairBody {
    code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ModeName {
    Play,
    Enqueue,
    Crate,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendBody {
    url: String,
    mode: ModeName,
    #[serde(rename = "crate", default)]
    crate_name: Option<String>,
    skip_passed: bool,
    /// Sent by older extensions; every send now keeps every format.
    #[serde(default, rename = "vinyl_only")]
    _vinyl_only: Option<serde::de::IgnoredAny>,
}

/// Checks a request in the order of the design (host, origin, route, size, key, body) and
/// answers it.
fn answer(
    req: &mut tiny_http::Request,
    shared: &Shared,
    port: u16,
) -> (u16, Value, Option<BridgeCommand>) {
    use tiny_http::Method::{Get, Options, Post};

    // 1. Only requests addressed to this computer by address: a web page's own host name
    // pointed at 127.0.0.1 (DNS rebinding) carries its own name here.
    let host_ok = header(req, "Host").is_some_and(|h| {
        let h = h.to_ascii_lowercase();
        h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}")
    });
    if !host_ok {
        return error(403, "wrong host");
    }
    // 2. No web page: an extension's origin or none at all, and never a preflight.
    let origin_ok = header(req, "Origin").is_none_or(|o| o.starts_with("chrome-extension://"));
    if !origin_ok || *req.method() == Options {
        return error(403, "forbidden");
    }
    // 3. Route and method, then the size.
    let path = req.url().split('?').next().unwrap_or("").to_owned();
    let route = match (req.method(), path.as_str()) {
        (Get, "/v1/hello") => "hello",
        (Post, "/v1/pair") => "pair",
        (Get, "/v1/crates") => "crates",
        (Get, "/v1/status") => "status",
        (Post, "/v1/send") => "send",
        (Post, "/v1/owned") => "owned",
        _ => return error(404, "not found"),
    };
    if req.body_length().is_some_and(|n| n > MAX_BODY) {
        return error(413, "too large");
    }
    let mut body = Vec::new();
    if req
        .as_reader()
        .take(MAX_BODY as u64 + 1)
        .read_to_end(&mut body)
        .is_err()
    {
        return error(422, "unreadable body");
    }
    if body.len() > MAX_BODY {
        return error(413, "too large");
    }
    // 4. The key, except to say hello and to pair.
    if !matches!(route, "hello" | "pair") {
        let key = header(req, KEY_HEADER).unwrap_or("");
        if !shared.pairing().check_key(key) {
            return error(401, "not paired");
        }
    }
    // 5. The body's shape, then what it says.
    match route {
        "hello" => (200, json!({ "app": crate::APP_NAME, "api": API }), None),
        "pair" => {
            let Ok(b) = serde_json::from_slice::<PairBody>(&body) else {
                return error(422, "invalid body");
            };
            match shared.pairing().pair(&b.code) {
                Ok(key) => (200, json!({ "key": key }), Some(BridgeCommand::Paired)),
                Err(PairError::Wrong) => error(401, "wrong code"),
                Err(PairError::NoCode) => error(410, "no pairing code is shown"),
                Err(PairError::Locked) => error(429, "too many wrong codes: wait a minute"),
                Err(PairError::Failed(e)) => error(500, &e),
            }
        }
        "crates" => (
            200,
            json!({ "crates": shared.snapshot.load().crates }),
            None,
        ),
        "status" => {
            let s = shared.snapshot.load();
            (
                200,
                json!({ "app": crate::APP_NAME, "playing": s.playing, "sends": s.sends }),
                None,
            )
        }
        "owned" => owned(&body, shared),
        _ => send(&body, shared),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedBody {
    url: String,
}

/// Is the page's record in the user's collection? Answered from the shared collection and
/// the disk cache, never with a request to Discogs; a marketplace item whose release isn't
/// known yet is "checking" (the app looks it up once) until it is.
fn owned(body: &[u8], shared: &Shared) -> (u16, Value, Option<BridgeCommand>) {
    let Ok(b) = serde_json::from_slice::<OwnedBody>(body) else {
        return error(422, "invalid body");
    };
    let Ok(page) = url::parse(&b.url) else {
        return error(422, "unsupported");
    };
    let answer = |v: Value| (200, v, None);
    if !shared.has_token.load(std::sync::atomic::Ordering::Relaxed) {
        return answer(json!({ "owned": "no-token" }));
    }
    let collection = shared.collection.load();
    let Some(c) = collection.as_ref().as_ref() else {
        return answer(json!({ "owned": "unknown" }));
    };
    let (release, master) = match page.kind {
        PageKind::Release(id) => (Some(id), master_of(&shared.cache, id)),
        PageKind::Master(id) => (None, Some(id)),
        PageKind::ShopItem(id) => {
            let release = shared
                .cache
                .get(Kind::ShopItem, &id.to_string())
                .and_then(|c| serde_json::from_str::<Value>(&c.body).ok())
                .and_then(|v| v["release"]["id"].as_u64());
            match release {
                Some(r) => (Some(r), master_of(&shared.cache, r)),
                None => {
                    return (
                        200,
                        json!({ "owned": "checking" }),
                        Some(BridgeCommand::ResolveShopItem(id)),
                    );
                }
            }
        }
        _ => return answer(json!({ "owned": "unknown" })),
    };
    answer(match c.owned(release, master) {
        Some(Owned::ThisPressing) => json!({ "owned": "this" }),
        Some(Owned::Another { catno, year }) => {
            json!({ "owned": "another", "catno": catno, "year": year })
        }
        None => json!({ "owned": "no" }),
    })
}

/// A release's master, when its data was fetched before (while digging).
fn master_of(cache: &DiskCache, release: u64) -> Option<u64> {
    let c = cache.get(Kind::Release, &release.to_string())?;
    let v: Value = serde_json::from_str(&c.body).ok()?;
    v["master_id"].as_u64().filter(|&m| m > 0)
}

fn send(body: &[u8], shared: &Shared) -> (u16, Value, Option<BridgeCommand>) {
    let Ok(b) = serde_json::from_slice::<SendBody>(body) else {
        return error(422, "invalid body");
    };
    let Ok(page) = url::parse(&b.url) else {
        return error(422, "unsupported");
    };
    let mode = match (b.mode, b.crate_name) {
        (ModeName::Play, None) => Mode::Play,
        (ModeName::Enqueue, None) => Mode::Enqueue,
        (ModeName::Crate, Some(name)) => {
            let name = name.trim();
            let len = name.chars().count();
            if len == 0 || len > MAX_CRATE_NAME {
                return error(422, "invalid crate name");
            }
            Mode::Crate(name.to_owned())
        }
        _ => return error(422, "invalid body"),
    };
    let target = match &mode {
        Mode::Enqueue => shared
            .snapshot
            .load()
            .crates
            .iter()
            .find(|c| c.shown)
            .map(|c| c.name.clone())
            .unwrap_or_default(),
        Mode::Play => page
            .provisional_name()
            .chars()
            .take(MAX_CRATE_NAME)
            .collect(),
        Mode::Crate(name) => name.clone(),
    };
    let answer = json!({ "page": page.provisional_name(), "crate": target });
    let cmd = BridgeCommand::Send {
        page,
        mode,
        filters: Filters {
            skip_passed: b.skip_passed,
        },
    };
    (202, answer, Some(cmd))
}
