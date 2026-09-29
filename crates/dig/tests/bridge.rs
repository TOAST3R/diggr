//! The browser bridge over real loopback HTTP on an ephemeral port: hello, pairing (expiry,
//! lockout, single use), keys and Forget browsers, every refusal (host, origin, preflight,
//! size, unknown fields, addresses that aren't Discogs pages), the snapshot answers, and a send
//! answered at once while Discogs is slow.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use dig::bridge::pairing::{CODE_VALID, LOCKOUT, MAX_WRONG};
use dig::bridge::{
    BridgeCommand, BridgeHandle, CrateState, MAX_BODY, Mode, Pairing, Playing, SendState, Shared,
    Snapshot,
};
use dig::clock::{FakeClock, RealClock};
use dig::discogs::cache::DiskCache;
use dig::discogs::client::Client;
use dig::discogs::transport::{FakeTransport, Fault};
use dig::intake::{Command, Event, Intake, IntakeHandle};
use dig::jobs::Filters;
use platform::native::NativeSpawner;
use serde_json::Value;

struct Rig {
    bridge: BridgeHandle,
    shared: Arc<Shared>,
    clock: Arc<FakeClock>,
    dir: platform::testing::TestDir,
}

fn rig(name: &str) -> Rig {
    let dir = platform::testing::TestDir::new(&format!("dig-bridge-{name}"));
    let clock = Arc::new(FakeClock::default());
    let shared = Shared::with_cache(
        Pairing::load(Some(dir.path().to_path_buf()), clock.clone()),
        Some(dir.path()),
    );
    let bridge = BridgeHandle::start(&NativeSpawner, shared.clone(), 0, || {}).unwrap();
    Rig {
        bridge,
        shared,
        clock,
        dir,
    }
}

struct Answer {
    status: u16,
    headers: String,
    json: Value,
}

impl Rig {
    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.bridge.port())
    }

    /// A raw request, so every header (Host included) is exactly what the test says.
    fn raw(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Answer {
        let mut s = TcpStream::connect(("127.0.0.1", self.bridge.port())).unwrap();
        let mut req = format!("{method} {path} HTTP/1.1\r\nConnection: close\r\n");
        for (k, v) in headers {
            req += &format!("{k}: {v}\r\n");
        }
        req += &format!("Content-Length: {}\r\n\r\n{body}", body.len());
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
        let status = head.split(' ').nth(1).unwrap().parse().unwrap();
        Answer {
            status,
            headers: head.to_ascii_lowercase(),
            json: serde_json::from_str(body).unwrap_or(Value::Null),
        }
    }

    /// As the extension calls: to 127.0.0.1, from its own origin, with `key` when given.
    fn call(&self, method: &str, path: &str, key: Option<&str>, body: &str) -> Answer {
        let host = self.host();
        let mut h = vec![
            ("Host", host.as_str()),
            ("Origin", "chrome-extension://abcdefghijklmnop"),
        ];
        if let Some(k) = key {
            h.push(("X-Bridge-Key", k));
        }
        self.raw(method, path, &h, body)
    }

    fn code(&self) -> String {
        self.shared.pairing().ensure_code().unwrap().0
    }

    fn pair(&self) -> String {
        let code = self.code();
        let a = self.call("POST", "/v1/pair", None, &format!(r#"{{"code":"{code}"}}"#));
        assert_eq!(a.status, 200, "{}", a.json);
        a.json["key"].as_str().unwrap().to_owned()
    }

    fn send(&self, key: &str, body: &str) -> Answer {
        self.call("POST", "/v1/send", Some(key), body)
    }
}

fn wrong(code: &str) -> &'static str {
    if code == "000000" { "000001" } else { "000000" }
}

const LABEL: &str = "https://www.discogs.com/label/12345-Lowtide-Tapes";

fn send_body(url: &str, mode: &str) -> String {
    format!(r#"{{"url":"{url}","mode":"{mode}","vinyl_only":true,"skip_passed":false}}"#)
}

#[test]
fn hello_needs_no_key() {
    let r = rig("hello");
    let a = r.call("GET", "/v1/hello", None, "");
    assert_eq!(a.status, 200);
    assert_eq!(a.json["app"], dig::APP_NAME);
    assert_eq!(a.json["api"], 1);
    assert!(a.headers.contains("content-type: application/json"));
}

#[test]
fn a_shown_code_buys_one_key() {
    let r = rig("pair");
    let pair = |code: &str| r.call("POST", "/v1/pair", None, &format!(r#"{{"code":"{code}"}}"#));
    assert_eq!(pair("123456").status, 410, "no code is shown");
    let code = r.code();
    assert_eq!(pair(wrong(&code)).status, 401);
    let a = pair(&code);
    assert_eq!(a.status, 200);
    let key = a.json["key"].as_str().unwrap();
    assert_eq!(key.len(), 64);
    assert!(key.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(pair(&code).status, 410, "the code works once");
    assert_eq!(r.bridge.poll(), vec![BridgeCommand::Paired]);
    assert_eq!(r.shared.pairing().paired(), 1);
    let crates = r.call("GET", "/v1/crates", Some(key), "");
    assert_eq!(crates.status, 200);
}

#[test]
fn codes_expire_after_two_minutes() {
    let r = rig("expiry");
    let code = r.code();
    r.clock.advance(CODE_VALID);
    let a = r.call("POST", "/v1/pair", None, &format!(r#"{{"code":"{code}"}}"#));
    assert_eq!(a.status, 410);
}

#[test]
fn five_wrong_codes_lock_pairing_for_a_minute() {
    let r = rig("lockout");
    let code = r.code();
    let pair = |c: &str| r.call("POST", "/v1/pair", None, &format!(r#"{{"code":"{c}"}}"#));
    for _ in 0..MAX_WRONG {
        assert_eq!(pair(wrong(&code)).status, 401);
    }
    assert_eq!(pair(&code).status, 429, "even the right code");
    r.clock.advance(LOCKOUT);
    assert_eq!(pair(&code).status, 200);
}

#[test]
fn keys_are_checked_and_forget_browsers_revokes_them() {
    let r = rig("keys");
    let key = r.pair();
    for path in ["/v1/crates", "/v1/status"] {
        assert_eq!(r.call("GET", path, None, "").status, 401, "{path}: no key");
        let other = "0".repeat(64);
        assert_eq!(r.call("GET", path, Some(&other), "").status, 401);
        assert_eq!(r.call("GET", path, Some(&key), "").status, 200);
    }
    assert_eq!(r.send("", &send_body(LABEL, "enqueue")).status, 401);
    r.shared.pairing().forget_all().unwrap();
    assert_eq!(r.call("GET", "/v1/crates", Some(&key), "").status, 401);
    assert!(r.bridge.poll().iter().all(|c| *c == BridgeCommand::Paired));
}

#[test]
fn another_host_name_is_refused() {
    let r = rig("host");
    let port = r.bridge.port();
    let ok_local = format!("localhost:{port}");
    assert_eq!(
        r.raw("GET", "/v1/hello", &[("Host", &ok_local)], "").status,
        200
    );
    for host in [
        format!("evil.example:{port}"),
        format!("127.0.0.1.evil.example:{port}"),
        "127.0.0.1".into(),
        format!("127.0.0.1:{}", port.wrapping_add(1)),
    ] {
        let a = r.raw("GET", "/v1/hello", &[("Host", &host)], "");
        assert_eq!(a.status, 403, "{host}");
    }
    assert_eq!(r.raw("GET", "/v1/hello", &[], "").status, 403, "no Host");
}

#[test]
fn web_page_origins_are_refused() {
    let r = rig("origin");
    let key = r.pair();
    let host = r.host();
    assert_eq!(
        r.raw("GET", "/v1/hello", &[("Host", &host)], "").status,
        200,
        "no origin (a context-menu fetch) is fine"
    );
    for origin in ["https://evil.example", "http://127.0.0.1:8000", "null"] {
        let a = r.raw(
            "POST",
            "/v1/send",
            &[("Host", &host), ("Origin", origin), ("X-Bridge-Key", &key)],
            &send_body(LABEL, "enqueue"),
        );
        assert_eq!(a.status, 403, "{origin}");
    }
    assert!(r.bridge.poll().iter().all(|c| *c == BridgeCommand::Paired));
}

#[test]
fn preflight_is_refused_and_no_cross_origin_header_is_ever_sent() {
    let r = rig("preflight");
    let key = r.pair();
    let host = r.host();
    let a = r.raw(
        "OPTIONS",
        "/v1/send",
        &[
            ("Host", &host),
            ("Origin", "https://evil.example"),
            ("Access-Control-Request-Method", "POST"),
        ],
        "",
    );
    assert_eq!(a.status, 403);
    let answers = [
        a,
        r.call("OPTIONS", "/v1/send", None, ""),
        r.call("GET", "/v1/hello", None, ""),
        r.call("GET", "/v1/crates", Some(&key), ""),
        r.send(&key, &send_body(LABEL, "enqueue")),
        r.call("GET", "/nope", None, ""),
    ];
    assert_eq!(answers[1].status, 403);
    for a in answers {
        assert!(!a.headers.contains("access-control-"), "{}", a.headers);
    }
}

#[test]
fn unknown_routes_are_404() {
    let r = rig("routes");
    assert_eq!(r.call("GET", "/v1/nope", None, "").status, 404);
    assert_eq!(r.call("POST", "/v1/hello", None, "").status, 404);
    assert_eq!(r.call("GET", "/v1/send", None, "").status, 404);
}

#[test]
fn bodies_over_16_kb_are_refused() {
    let r = rig("size");
    let key = r.pair();
    let url = format!("{LABEL}?x={}", "a".repeat(MAX_BODY));
    let a = r.send(&key, &send_body(&url, "enqueue"));
    assert_eq!(a.status, 413);
    assert!(r.bridge.poll().iter().all(|c| *c == BridgeCommand::Paired));
}

#[test]
fn only_supported_discogs_pages_in_the_exact_shape_are_accepted() {
    let r = rig("shape");
    let key = r.pair();
    r.bridge.poll();
    let refused = [
        send_body("https://example.com/track.mp3", "enqueue"),
        send_body("/Users/me/Music/track.mp3", "enqueue"),
        send_body("https://www.discogs.com/forum/thread/123456", "enqueue"),
        send_body(LABEL, "delete"),
        // Unknown field.
        format!(
            r#"{{"url":"{LABEL}","mode":"enqueue","vinyl_only":true,"skip_passed":false,"path":"/etc"}}"#
        ),
        // Missing switches.
        format!(r#"{{"url":"{LABEL}","mode":"enqueue"}}"#),
        // A crate without the crate mode, and the crate mode without a crate.
        format!(
            r#"{{"url":"{LABEL}","mode":"enqueue","crate":"x","vinyl_only":true,"skip_passed":true}}"#
        ),
        send_body(LABEL, "crate"),
        // Crate names: 1 to 40 characters.
        format!(
            r#"{{"url":"{LABEL}","mode":"crate","crate":"  ","vinyl_only":true,"skip_passed":true}}"#
        ),
        format!(
            r#"{{"url":"{LABEL}","mode":"crate","crate":"{}","vinyl_only":true,"skip_passed":true}}"#,
            "x".repeat(41)
        ),
        "not json".into(),
    ];
    for body in &refused {
        let a = r.send(&key, body);
        assert_eq!(a.status, 422, "{body}");
    }
    assert_eq!(
        r.send(&key, &send_body("https://example.com/track.mp3", "enqueue"))
            .json["error"],
        "unsupported"
    );
    assert_eq!(r.bridge.poll(), vec![], "nothing was sent");

    let a = r.send(
        &key,
        &format!(r#"{{"url":"{LABEL}","mode":"crate","crate":" Friday ","vinyl_only":false,"skip_passed":true}}"#),
    );
    assert_eq!(a.status, 202);
    assert_eq!(a.json["page"], "Label: Lowtide Tapes");
    assert_eq!(a.json["crate"], "Friday");
    match &r.bridge.poll()[..] {
        [
            BridgeCommand::Send {
                page,
                mode,
                filters,
            },
        ] => {
            assert_eq!(page.url(), "https://www.discogs.com/label/12345");
            assert_eq!(*mode, Mode::Crate("Friday".into()));
            assert_eq!(
                *filters,
                Filters {
                    vinyl_only: false,
                    skip_passed: true
                }
            );
        }
        other => panic!("{other:?}"),
    }
    let a = r.send(&key, &send_body(LABEL, "play"));
    assert_eq!(
        (a.status, &a.json["crate"]),
        (202, &"Label: Lowtide Tapes".into())
    );
}

#[test]
fn crates_and_status_come_from_the_snapshot() {
    let r = rig("snapshot");
    let key = r.pair();
    r.shared.snapshot.store(Arc::new(Snapshot {
        crates: vec![
            CrateState {
                name: "Playlist".into(),
                shown: false,
                playing: true,
            },
            CrateState {
                name: "Friday".into(),
                shown: true,
                playing: false,
            },
        ],
        playing: Some(Playing {
            artist: "Nightcraft".into(),
            title: "Glasshouse".into(),
            crate_name: "Playlist".into(),
        }),
        sends: vec![SendState {
            page: "Label: Lowtide Tapes".into(),
            done: 120,
            total: 312,
        }],
    }));
    let c = r.call("GET", "/v1/crates", Some(&key), "").json;
    assert_eq!(
        c,
        serde_json::json!({"crates": [
            {"name": "Playlist", "shown": false, "playing": true},
            {"name": "Friday", "shown": true, "playing": false},
        ]})
    );
    let s = r.call("GET", "/v1/status", Some(&key), "").json;
    assert_eq!(s["app"], dig::APP_NAME);
    assert_eq!(
        s["playing"],
        serde_json::json!({"artist": "Nightcraft", "title": "Glasshouse", "crate": "Playlist"})
    );
    assert_eq!(
        s["sends"],
        serde_json::json!([{"page": "Label: Lowtide Tapes", "done": 120, "total": 312}])
    );
    let a = r.send(&key, &send_body(LABEL, "enqueue"));
    assert_eq!(a.json["crate"], "Friday", "enqueue goes to the shown crate");
}

#[test]
fn a_taken_port_is_reported() {
    let taken = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = taken.local_addr().unwrap().port();
    let shared = Shared::new(Pairing::load(None, Arc::new(FakeClock::default())));
    let err = BridgeHandle::start(&NativeSpawner, shared, port, || {})
        .err()
        .unwrap();
    assert!(err.contains("taken"), "{err}");
}

#[test]
fn a_send_is_answered_at_once_while_discogs_is_slow() {
    let r = rig("fast");
    let key = r.pair();
    r.bridge.poll();
    // Every Discogs answer takes 300 ms.
    let transport = Arc::new(FakeTransport::with_fixtures(format!(
        "{}/tests/fixtures/discogs",
        env!("CARGO_MANIFEST_DIR")
    )));
    for _ in 0..20 {
        transport.fault(Fault::Delay(Duration::from_millis(300)));
    }
    let client = Client::new(
        transport,
        Arc::new(RealClock::default()),
        None,
        DiskCache::default(),
    );
    let (wake_tx, wake_rx) = channel();
    let intake = IntakeHandle::start(&NativeSpawner, Intake::new(client, None), move || {
        let _ = wake_tx.send(());
    })
    .unwrap();

    let t = Instant::now();
    let a = r.send(&key, &send_body(LABEL, "enqueue"));
    let took = t.elapsed();
    assert_eq!(a.status, 202);
    assert!(took < Duration::from_millis(100), "answered in {took:?}");

    // The UI's part: the command becomes a send, and the crate fills in afterwards.
    let Some(BridgeCommand::Send { page, filters, .. }) = r.bridge.poll().pop() else {
        panic!("no send");
    };
    intake.send(Command::Send {
        page,
        target: 7,
        filters,
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut listed = 0;
    while listed == 0 && Instant::now() < deadline {
        let _ = wake_rx.recv_timeout(Duration::from_millis(200));
        listed += intake
            .poll()
            .iter()
            .filter_map(|e| match e {
                Event::Listed(j, items) if j.target == 7 => Some(items.len()),
                _ => None,
            })
            .sum::<usize>();
    }
    assert!(listed > 0, "the label's records arrive after the answer");
}

// ---- the owned check -------------------------------------------------------------------

/// The user owns release 101 (AF014, 2018), a pressing of master 900.
fn with_collection(r: &Rig) {
    let path = r.dir.path().join("owned.ron");
    std::fs::write(
        path.with_file_name(dig::collection::FILE),
        r#"(username: "digger", fetched_at: 0, count: 1, instances: [1],
            releases: {101: (master: Some(900), catno: "AF014", year: Some(2018))})"#,
    )
    .unwrap();
    let c = dig::collection::Collection::load(r.dir.path()).unwrap();
    r.shared.collection.store(Arc::new(Some(Arc::new(c))));
}

fn owned(r: &Rig, key: &str, url: &str) -> Value {
    let a = r.call(
        "POST",
        "/v1/owned",
        Some(key),
        &format!(r#"{{"url":"{url}"}}"#),
    );
    assert_eq!(a.status, 200, "{}", a.json);
    a.json
}

#[test]
fn the_owned_check_answers_from_the_collection_without_asking_discogs() {
    let r = rig("owned");
    let key = r.pair();
    let url = "https://www.discogs.com/release/101";
    assert_eq!(owned(&r, &key, url)["owned"], "no-token");
    r.shared
        .has_token
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owned(&r, &key, url)["owned"],
        "unknown",
        "no collection yet"
    );
    with_collection(&r);
    let t = Instant::now();
    assert_eq!(owned(&r, &key, url)["owned"], "this");
    assert!(t.elapsed() < Duration::from_millis(50));
    assert_eq!(
        owned(&r, &key, "https://www.discogs.com/master/900")["owned"],
        "another"
    );
    // Another pressing, known to be of master 900 from data fetched while digging.
    DiskCache::new(Some(r.dir.path())).put(
        dig::discogs::cache::Kind::Release,
        "555",
        r#"{"id": 555, "master_id": 900}"#,
        0,
    );
    let a = owned(&r, &key, "https://www.discogs.com/release/555");
    assert_eq!(
        (a["owned"].as_str(), a["catno"].as_str(), a["year"].as_u64()),
        (Some("another"), Some("AF014"), Some(2018))
    );
    assert_eq!(
        owned(&r, &key, "https://www.discogs.com/release/777")["owned"],
        "no"
    );
    assert_eq!(
        owned(&r, &key, "https://www.discogs.com/label/12345")["owned"],
        "unknown"
    );
    // Unpaired: refused.
    let a = r.call("POST", "/v1/owned", None, &format!(r#"{{"url":"{url}"}}"#));
    assert_eq!(a.status, 401);
    assert!(r.bridge.poll().iter().all(|c| *c == BridgeCommand::Paired));
}

#[test]
fn a_new_shop_item_is_checking_until_its_release_is_known() {
    let r = rig("owned-item");
    let key = r.pair();
    with_collection(&r);
    r.shared
        .has_token
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let item = "https://www.discogs.com/shop/item/3923678974";
    assert_eq!(owned(&r, &key, item)["owned"], "checking");
    assert!(
        r.bridge
            .poll()
            .contains(&BridgeCommand::ResolveShopItem(3923678974)),
        "the app is asked to look it up"
    );
    // Once looked up (the intake caches the item), the answer is immediate.
    DiskCache::new(Some(r.dir.path())).put(
        dig::discogs::cache::Kind::ShopItem,
        "3923678974",
        r#"{"id": 3923678974, "release": {"id": 101}}"#,
        0,
    );
    assert_eq!(owned(&r, &key, item)["owned"], "this");
}
