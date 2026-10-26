//! The browser bridge over real loopback HTTP on an ephemeral port: hello, pairing (expiry,
//! lockout, single use), keys and Forget browsers, every refusal (host, origin, preflight,
//! size, unknown fields, addresses that aren't Discogs pages), the snapshot answers, a send
//! answered at once while Discogs is slow, Juno sends (records, not addresses), and store cart
//! actions (held waits, results and their checks).

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
const ARTIST: &str = "https://www.discogs.com/artist/4242-Nightcraft";

fn send_body(url: &str, mode: &str) -> String {
    format!(r#"{{"url":"{url}","mode":"{mode}","skip_passed":false}}"#)
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

    // An older extension still sends vinyl only: accepted, and ignored.
    let a = r.send(
        &key,
        &format!(r#"{{"url":"{ARTIST}","mode":"crate","crate":" Friday ","vinyl_only":true,"skip_passed":true}}"#),
    );
    assert_eq!(a.status, 202);
    assert_eq!(a.json["page"], "Artist: Nightcraft");
    assert_eq!(a.json["crate"], "Friday");
    match &r.bridge.poll()[..] {
        [
            BridgeCommand::Send {
                page,
                mode,
                filters,
                ..
            },
        ] => {
            assert_eq!(page.url(), "https://www.discogs.com/artist/4242");
            assert_eq!(*mode, Mode::Crate("Friday".into()));
            assert_eq!(*filters, Filters { skip_passed: true });
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
                enqueue: false,
                playing: true,
            },
            CrateState {
                name: "Friday".into(),
                shown: true,
                enqueue: true,
                playing: false,
            },
        ],
        playing: Some(Playing {
            artist: "Nightcraft".into(),
            title: "Glasshouse".into(),
            crate_name: "Playlist".into(),
            release: Some(1001),
            position: "A1".into(),
        }),
        sends: vec![SendState {
            page: "Label: Lowtide Tapes".into(),
            done: 120,
            total: 312,
        }],
        sellers: Vec::new(),
        labels: Vec::new(),
        label_crates: Vec::new(),
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
        serde_json::json!({"artist": "Nightcraft", "title": "Glasshouse", "crate": "Playlist", "release": 1001, "position": "A1"})
    );
    assert_eq!(
        s["sends"],
        serde_json::json!([{"page": "Label: Lowtide Tapes", "done": 120, "total": 312}])
    );
    let a = r.send(&key, &send_body(ARTIST, "enqueue"));
    assert_eq!(a.json["crate"], "Friday", "enqueue goes to the shown crate");
}

const RELEASE: &str = "https://www.discogs.com/release/5077187-Massive-Attack-Mezzanine";

fn position_body(url: &str, mode: &str, position: &str) -> String {
    format!(r#"{{"url":"{url}","mode":"{mode}","skip_passed":false,"position":"{position}"}}"#)
}

#[test]
fn a_play_send_of_a_release_can_name_one_track() {
    let r = rig("position");
    let key = r.pair();
    r.bridge.poll();
    let a = r.send(&key, &position_body(RELEASE, "play", " B2 "));
    assert_eq!(a.status, 202, "{}", a.json);
    match r.bridge.poll().as_slice() {
        [BridgeCommand::Send { mode, position, .. }] => {
            assert_eq!(*mode, Mode::Play);
            assert_eq!(position.as_deref(), Some("B2"), "trimmed");
        }
        other => panic!("{other:?}"),
    }
    // Without one, the first track plays, as before.
    r.send(&key, &send_body(RELEASE, "play"));
    assert!(matches!(
        r.bridge.poll().as_slice(),
        [BridgeCommand::Send { position: None, .. }]
    ));
    for (url, mode, position) in [
        (LABEL, "play", "A1"),
        (RELEASE, "enqueue", "A1"),
        (RELEASE, "play", ""),
        (RELEASE, "play", "A1234567890"),
        (RELEASE, "play", "A\\u00071"),
        ("https://www.discogs.com/master/98765", "play", "A1"),
    ] {
        let a = r.send(&key, &position_body(url, mode, position));
        assert_eq!(a.status, 422, "{url} {mode} {position:?}");
    }
    assert!(r.bridge.poll().is_empty(), "nothing refused is sent on");
}

#[test]
fn control_stops_playback_and_nothing_else() {
    let r = rig("control");
    let key = r.pair();
    r.bridge.poll();
    let stop = r#"{"action":"stop"}"#;
    assert_eq!(r.call("POST", "/v1/control", None, stop).status, 401);
    let t = Instant::now();
    assert_eq!(r.call("POST", "/v1/control", Some(&key), stop).status, 202);
    assert!(t.elapsed() < Duration::from_millis(100));
    assert_eq!(r.bridge.poll(), vec![BridgeCommand::Stop]);
    for body in [
        r#"{"action":"next"}"#,
        r#"{"action":"stop","volume":1}"#,
        "",
    ] {
        assert_eq!(
            r.call("POST", "/v1/control", Some(&key), body).status,
            422,
            "{body}"
        );
    }
    assert!(r.bridge.poll().is_empty());
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

#[test]
fn a_label_page_follows_or_refreshes_whatever_the_mode() {
    let r = rig("label");
    let key = r.pair();
    r.bridge.poll();
    for mode in ["enqueue", "play"] {
        let a = r.send(&key, &send_body(LABEL, mode));
        assert_eq!(a.status, 202);
        assert_eq!(a.json["message"], "Added label Lowtide Tapes");
        assert_eq!(
            (&a.json["label"], &a.json["added"]),
            (&12345.into(), &true.into())
        );
        assert_eq!(a.json["crate"], "Label: Lowtide Tapes", "its own crate");
        match &r.bridge.poll()[..] {
            [BridgeCommand::Send { page, filters, .. }] => {
                assert_eq!(page.kind, dig::discogs::url::PageKind::Label(12345));
                assert_eq!(*filters, Filters { skip_passed: true }, "passes stay out");
            }
            other => panic!("{other:?}"),
        }
    }
    r.shared.snapshot.store(Arc::new(Snapshot {
        labels: vec![12345],
        ..Snapshot::default()
    }));
    let a = r.send(&key, &send_body(LABEL, "enqueue"));
    assert_eq!(a.json["message"], "Refreshed label Lowtide Tapes");
    assert_eq!(a.json["added"], false);
}

#[test]
fn only_the_app_bundle_has_a_url_to_come_forward() {
    use dig::bridge::front_url;
    use std::path::Path;
    let bundled = front_url(Path::new("/Applications/Diggr.app/Contents/MacOS/diggr"));
    assert_eq!(bundled, cfg!(target_os = "macos").then_some("diggr://show"));
    assert_eq!(
        front_url(Path::new("/src/diggr/target/release/diggr")),
        None
    );
}

#[test]
fn a_seller_page_adds_or_refreshes_whatever_the_mode() {
    let r = rig("seller");
    let key = r.pair();
    r.bridge.poll();
    let seller = "https://www.discogs.com/seller/decks.de/profile";
    let a = r.send(&key, &send_body(seller, "enqueue"));
    assert_eq!(a.status, 202);
    assert_eq!(a.json["message"], "Added seller decks.de");
    assert_eq!(a.json["added"], true);
    assert!(
        a.json.get("front").is_none(),
        "only the app bundle answers with a front URL"
    );
    match &r.bridge.poll()[..] {
        [BridgeCommand::Send { page, .. }] => {
            assert_eq!(
                page.kind,
                dig::discogs::url::PageKind::Seller("decks.de".into())
            );
        }
        other => panic!("{other:?}"),
    }
    r.shared.snapshot.store(Arc::new(Snapshot {
        sellers: vec!["decks.de".into()],
        ..Snapshot::default()
    }));
    let a = r.send(
        &key,
        &send_body("https://www.discogs.com/user/DECKS.DE", "play"),
    );
    assert_eq!(a.json["message"], "Refreshed seller DECKS.DE");
    assert_eq!(a.json["added"], false);
    let status = r.call("GET", "/v1/status", Some(&key), "");
    assert!(
        status.json.get("sellers").is_none(),
        "never part of an answer"
    );
}

#[test]
fn a_bandcamp_album_goes_where_the_mode_says() {
    let r = rig("bandcamp-album");
    let key = r.pair();
    r.bridge.poll();
    let album = "https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep?from=x";
    let a = r.send(&key, &send_body(album, "enqueue"));
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(a.json["page"], "af070 the ooze ep");
    match &r.bridge.poll()[..] {
        [BridgeCommand::SendBandcamp { page, mode, .. }] => {
            assert_eq!(
                page.url(),
                "https://analogicalforce.bandcamp.com/album/af070-the-ooze-ep"
            );
            assert_eq!(*mode, dig::bridge::Mode::Enqueue);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_bandcamp_label_is_added_merged_or_refreshed() {
    let r = rig("bandcamp-label");
    let key = r.pair();
    r.bridge.poll();
    let label = "https://analogicalforce.bandcamp.com/music";
    let titled = format!(
        r#"{{"url":"{label}","mode":"enqueue","skip_passed":false,"title":"Music | Analogical Force"}}"#
    );
    let a = r.send(&key, &titled);
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(a.json["message"], "Added label Analogical Force");
    match &r.bridge.poll()[..] {
        [BridgeCommand::SendBandcamp { title, filters, .. }] => {
            assert_eq!(title.as_deref(), Some("Music | Analogical Force"));
            assert_eq!(*filters, Filters { skip_passed: true });
        }
        other => panic!("{other:?}"),
    }
    // The Discogs label of that name is followed: merged.
    r.shared.snapshot.store(Arc::new(Snapshot {
        label_crates: vec![("Label: Analogical Force".into(), None)],
        ..Snapshot::default()
    }));
    let a = r.send(&key, &send_body(label, "play"));
    assert_eq!(a.json["message"], "Merged into Label: Analogical Force");
    // Following that Bandcamp already: refreshed.
    r.shared.snapshot.store(Arc::new(Snapshot {
        label_crates: vec![(
            "Label: Analogical Force".into(),
            Some("analogicalforce".into()),
        )],
        ..Snapshot::default()
    }));
    let a = r.send(&key, &send_body(label, "enqueue"));
    assert_eq!(a.json["message"], "Refreshed label Analogical Force");
}

#[test]
fn bandcamp_look_alikes_and_long_titles_are_refused() {
    let r = rig("bandcamp-refused");
    let key = r.pair();
    r.bridge.poll();
    for url in [
        "https://bandcamp.com.evil.net/album/x",
        "https://a.bandcamp.com/merch",
        "https://a.bandcamp.com/album/x;rm",
    ] {
        let a = r.send(&key, &send_body(url, "enqueue"));
        assert_eq!(a.status, 422, "{url}");
    }
    let long = "x".repeat(201);
    let body = format!(
        r#"{{"url":"https://a.bandcamp.com/","mode":"enqueue","skip_passed":false,"title":"{long}"}}"#
    );
    assert_eq!(r.send(&key, &body).status, 422);
    // A title belongs to Bandcamp pages only.
    let body =
        format!(r#"{{"url":"{LABEL}","mode":"enqueue","skip_passed":false,"title":"Lowtide"}}"#);
    assert_eq!(r.send(&key, &body).status, 422);
    assert!(r.bridge.poll().is_empty(), "nothing was sent on");
}

#[test]
fn a_bandcamp_fan_page_imports_the_fan() {
    let r = rig("bandcamp-fan");
    let key = r.pair();
    r.bridge.poll();
    let a = r.send(
        &key,
        &send_body("https://bandcamp.com/toast3r/wishlist?x=1", "play"),
    );
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(
        a.json["message"],
        "Importing toast3r's Bandcamp collection & wishlist"
    );
    match &r.bridge.poll()[..] {
        [BridgeCommand::ImportFan(page)] => {
            assert_eq!(page.url(), "https://bandcamp.com/toast3r/wishlist")
        }
        other => panic!("{other:?}"),
    }
    for url in [
        "https://bandcamp.com/discover",
        "https://bandcamp.com/toast3r;rm",
    ] {
        assert_eq!(
            r.send(&key, &send_body(url, "enqueue")).status,
            422,
            "{url}"
        );
    }
    assert!(r.bridge.poll().is_empty());
}

fn juno_record(product: &str) -> Value {
    serde_json::json!({
        "product": product,
        "slug": "0-mika-vainio-rontgen-vinyl",
        "artist": "0 aka Mika Vainio",
        "title": "Rontgen (reissue)",
        "label": "Sahko Finland",
        "catno": "SAHKO 00012",
        "stock": "in",
        "price": "€14.78",
        "tracks": [
            { "side": "1", "n": 1, "title": "Rontgen", "secs": 301, "sample": "01-01" },
            { "side": "2", "n": 1, "title": "Track 2", "secs": 450, "sample": "02-01" }
        ]
    })
}

fn juno_body(records: Vec<Value>, mode: &str) -> String {
    serde_json::json!({
        "juno": { "page": "https://www.juno.co.uk/labels/Sahko+Finland/", "records": records },
        "mode": mode,
        "skip_passed": true,
    })
    .to_string()
}

#[test]
fn a_juno_product_page_is_sent_as_records() {
    let r = rig("juno-product");
    let key = r.pair();
    r.bridge.poll();
    let started = Instant::now();
    let a = r.send(&key, &juno_body(vec![juno_record("1178909-01")], "enqueue"));
    assert!(started.elapsed() < Duration::from_millis(100));
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(a.json["page"], "0 aka Mika Vainio - Rontgen (reissue)");
    assert_eq!(
        (a.json["records"].as_u64(), a.json["refused"].as_u64()),
        (Some(1), Some(0))
    );
    match &r.bridge.poll()[..] {
        [
            BridgeCommand::SendStore {
                records,
                mode,
                filters,
            },
        ] => {
            assert_eq!(records[0].tracks.len(), 2);
            assert_eq!(*mode, Mode::Enqueue);
            assert!(filters.skip_passed);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn bad_juno_records_are_refused_alone() {
    let r = rig("juno-refused");
    let key = r.pair();
    r.bridge.poll();
    let mut records = vec![juno_record("1178909-01"); 19];
    records.push(juno_record("1178909-01/../x"));
    let a = r.send(&key, &juno_body(records, "enqueue"));
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(
        (a.json["records"].as_u64(), a.json["refused"].as_u64()),
        (Some(19), Some(1))
    );
    assert_eq!(a.json["message"], "Sent 19 records, 1 refused");
}

fn beatport_track(id: &str) -> Value {
    serde_json::json!({
        "id": id, "slug": "input", "artists": ["Martin Cantrill"], "title": "Input",
        "mix": "Original Mix", "sample": "b776a31b-cb84-477f-a7bb-0476c5a1ee8e",
        "bpm": 126, "key": "8A", "secs": 305, "release": "4630549",
        "release_name": "Dreams Album", "release_slug": "dreams-album", "price": "€1.39",
    })
}

fn beatport_body(tracks: Vec<Value>) -> String {
    serde_json::json!({
        "beatport": { "page": "https://www.beatport.com/chart/x/1", "tracks": tracks },
        "mode": "enqueue",
        "skip_passed": true,
    })
    .to_string()
}

#[test]
fn a_beatport_chart_is_sent_as_tracks() {
    let r = rig("beatport-chart");
    let key = r.pair();
    r.bridge.poll();
    let mut tracks: Vec<Value> = (1..=100).map(|i| beatport_track(&i.to_string())).collect();
    tracks[7]["sample"] = Value::from("../../etc");
    let started = Instant::now();
    let a = r.send(&key, &beatport_body(tracks));
    assert!(started.elapsed() < Duration::from_millis(100));
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(a.json["message"], "Sent 99 tracks, 1 refused");
    match &r.bridge.poll()[..] {
        [BridgeCommand::SendStore { records, .. }] => {
            assert_eq!(records.iter().map(|r| r.tracks.len()).sum::<usize>(), 99);
            assert_eq!(records[0].store, ::dig::stores::Store::Beatport);
        }
        other => panic!("{other:?}"),
    }
}

fn bleep_record(id: &str) -> Value {
    serde_json::json!({
        "id": id, "slug": "russell-haswell-deep-time", "artist": "Russell Haswell",
        "title": "Deep Time", "label": "Diagonal Records", "catno": "DIAG045",
        "image": format!("{id}.jpg"), "formats": [{ "name": "LP" }, { "name": "Download" }],
    })
}

#[test]
fn a_bleep_list_is_sent_as_records() {
    let r = rig("bleep-list");
    let key = r.pair();
    r.bridge.poll();
    let mut records: Vec<Value> = (1..=24).map(|i| bleep_record(&i.to_string())).collect();
    records[3]["slug"] = Value::from("../x");
    let body = serde_json::json!({
        "bleep": { "page": "https://bleep.com/label/12257-diagonal-records", "records": records },
        "mode": "enqueue",
        "skip_passed": true,
    })
    .to_string();
    let started = Instant::now();
    let a = r.send(&key, &body);
    assert!(started.elapsed() < Duration::from_millis(100));
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(a.json["message"], "Sent 23 records, 1 refused");
    match &r.bridge.poll()[..] {
        [BridgeCommand::SendStore { records, .. }] => {
            assert_eq!(records.len(), 23);
            assert_eq!(records[0].store, ::dig::stores::Store::Bleep);
        }
        other => panic!("{other:?}"),
    }
    // An unknown field is refused whole.
    let odd = serde_json::json!({
        "bleep": { "page": "https://bleep.com/", "records": [bleep_record("1")], "extra": 1 },
        "mode": "enqueue",
        "skip_passed": true,
    })
    .to_string();
    assert_eq!(r.send(&key, &odd).status, 422);
    assert!(r.bridge.poll().is_empty());
}

#[test]
fn a_store_send_names_one_store_and_fits_in_a_mebibyte() {
    let r = rig("store-shape");
    let key = r.pair();
    r.bridge.poll();
    let both = serde_json::json!({
        "beatport": { "page": "https://www.beatport.com/", "tracks": [beatport_track("1")] },
        "traxsource": { "page": "https://www.traxsource.com/", "tracks": [] },
        "mode": "enqueue",
        "skip_passed": false,
    })
    .to_string();
    assert_eq!(r.send(&key, &both).status, 422);
    let mut big = beatport_track("1");
    big["title"] = Value::from("x".repeat(1 << 20));
    assert_eq!(r.send(&key, &beatport_body(vec![big])).status, 413);
    assert!(r.bridge.poll().is_empty(), "nothing reached the player");
}

#[test]
fn juno_sends_are_limited_in_size_and_shape() {
    let r = rig("juno-size");
    let key = r.pair();
    r.bridge.poll();
    // 600 KB of records is fine; over 1 MiB is not.
    let big: Vec<Value> = (0..500)
        .map(|i| juno_record(&format!("{}-01", 1_000_000 + i)))
        .collect();
    let body = juno_body(big, "enqueue");
    assert!(body.len() > MAX_BODY);
    assert_eq!(r.send(&key, &body).status, 202);
    let mut huge = juno_record("1178909-01");
    huge["pad"] = Value::String("x".repeat(1_600_000));
    assert_eq!(r.send(&key, &juno_body(vec![huge], "enqueue")).status, 413);
    // An unknown field beside `juno`, or a page that isn't Juno's.
    let extra =
        juno_body(vec![juno_record("1178909-01")], "enqueue").replacen('{', r#"{"x":1,"#, 1);
    assert_eq!(r.send(&key, &extra).status, 422);
    let elsewhere = juno_body(vec![juno_record("1178909-01")], "enqueue")
        .replace("https://www.juno.co.uk/", "https://evil.net/");
    assert_eq!(r.send(&key, &elsewhere).status, 422);
    // Not paired: refused before anything is read.
    let a = r.call(
        "POST",
        "/v1/send",
        None,
        &juno_body(vec![juno_record("1178909-01")], "play"),
    );
    assert_eq!(a.status, 401);
    // A large body that isn't Juno's is still refused at 16 KB.
    let url = format!("{LABEL}?x={}", "a".repeat(40_000));
    assert_eq!(r.send(&key, &send_body(&url, "enqueue")).status, 413);
    let cmds = r.bridge.poll();
    assert_eq!(
        cmds.len(),
        1,
        "only the 500 records went through: {}",
        cmds.len()
    );
}

// ---- store cart actions --------------------------------------------------------------------

const ADD_RESULT: &str =
    r#"{"id":1,"outcome":"added","basket":[{"item":"277377","price":"2.49 €"}]}"#;

fn cart_results(r: &Rig) -> Vec<dig::carts::CartResult> {
    r.bridge
        .poll()
        .into_iter()
        .filter_map(|c| match c {
            BridgeCommand::CartResult(r) => Some(r),
            _ => None,
        })
        .collect()
}

fn queue_add(r: &Rig, item: &str) -> u64 {
    use dig::carts::CartKind;
    use dig::stores::Store;
    let store = if item.contains('-') {
        Store::Juno
    } else {
        Store::Bleep
    };
    r.shared
        .cart()
        .push(store, CartKind::Add, item, Instant::now())
        .unwrap()
}

/// A cart wait from another thread, as the extension makes it: its status, answer and when
/// the answer came.
fn wait_in_thread(r: &Rig, key: &str) -> std::sync::mpsc::Receiver<(u16, Value, Instant)> {
    let (tx, rx) = channel();
    let (host, port, key) = (r.host(), r.bridge.port(), key.to_owned());
    std::thread::spawn(move || {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let req = format!(
            "GET /v1/cart/next HTTP/1.1\r\nConnection: close\r\nHost: {host}\r\nX-Bridge-Key: {key}\r\nContent-Length: 0\r\n\r\n"
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let at = Instant::now();
        let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
        let status = head.split(' ').nth(1).unwrap().parse().unwrap();
        let _ = tx.send((
            status,
            serde_json::from_str(body).unwrap_or(Value::Null),
            at,
        ));
    });
    rx
}

#[test]
fn a_waiting_extension_gets_an_action_within_100_ms() {
    let r = rig("cart-wait");
    let key = r.pair();
    let rx = wait_in_thread(&r, &key);
    // The wait is parked before anything is queued.
    std::thread::sleep(Duration::from_millis(300));
    assert!(rx.try_recv().is_err(), "held open while nothing is pending");
    let queued = Instant::now();
    queue_add(&r, "1170462-01");
    let (status, json, at) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(status, 200);
    assert_eq!(
        json["action"],
        serde_json::json!({"id": 1, "store": "juno", "kind": "add", "item": "1170462-01"})
    );
    assert!(
        at - queued < Duration::from_millis(100),
        "{:?}",
        at - queued
    );
}

#[test]
fn a_pending_action_is_given_at_once_and_a_wait_ends_empty() {
    let r = rig("cart-empty");
    let key = r.pair();
    queue_add(&r, "277377");
    let a = r.call("GET", "/v1/cart/next", Some(&key), "");
    assert_eq!(a.json["action"]["item"], "277377");
    r.shared
        .cart_hold_ms
        .store(300, std::sync::atomic::Ordering::Relaxed);
    let t = Instant::now();
    let a = r.call("GET", "/v1/cart/next", Some(&key), "");
    assert_eq!((a.status, a.json["action"].clone()), (200, Value::Null));
    assert!(t.elapsed() >= Duration::from_millis(300));
    // Requests are still answered while a wait is held.
    let rx = wait_in_thread(&r, &key);
    std::thread::sleep(Duration::from_millis(50));
    let t = Instant::now();
    assert_eq!(r.call("GET", "/v1/crates", Some(&key), "").status, 200);
    assert!(t.elapsed() < Duration::from_millis(100));
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn a_result_reaches_the_ui_once() {
    let r = rig("cart-result");
    let key = r.pair();
    queue_add(&r, "277377");
    r.call("GET", "/v1/cart/next", Some(&key), "");
    let a = r.call("POST", "/v1/cart/result", Some(&key), ADD_RESULT);
    assert_eq!(a.status, 200, "{}", a.json);
    let got = cart_results(&r);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].outcome, dig::carts::Outcome::Added);
    assert_eq!(got[0].basket.as_ref().unwrap()[0].price, "2.49 €");
    // The same answer again names no pending action.
    let a = r.call("POST", "/v1/cart/result", Some(&key), ADD_RESULT);
    assert_eq!(a.status, 422);
    assert!(cart_results(&r).is_empty());
}

#[test]
fn forged_unpaired_and_bad_results_are_refused() {
    let r = rig("cart-refused");
    let key = r.pair();
    queue_add(&r, "277377");
    // Unpaired: no action is given out.
    let a = r.call("GET", "/v1/cart/next", Some("not-the-key"), "");
    assert_eq!(a.status, 401);
    assert!(r.shared.cart().has_waiting());
    r.call("GET", "/v1/cart/next", Some(&key), "");
    for body in [
        r#"{"id":7,"outcome":"added"}"#,
        r#"{"id":1,"outcome":"exploded"}"#,
        r#"{"id":1,"outcome":"added","basket":[{"item":"1170462-01","price":""}]}"#,
        r#"{"id":1,"outcome":"added","basket":[{"item":"277377","price":"\u0007"}]}"#,
        r#"{"id":1,"outcome":"added","url":"https://bleep.com/x"}"#,
        r#"{"id":1,"outcome":"no_extension"}"#,
    ] {
        let a = r.call("POST", "/v1/cart/result", Some(&key), body);
        assert_eq!(a.status, 422, "{body}");
    }
    let a = r.call("POST", "/v1/cart/result", None, ADD_RESULT);
    assert_eq!(a.status, 401);
    assert!(cart_results(&r).is_empty(), "no badge changes");
    // The real answer still counts.
    let a = r.call("POST", "/v1/cart/result", Some(&key), ADD_RESULT);
    assert_eq!(a.status, 200);
}

#[test]
fn an_action_no_extension_picks_up_expires() {
    let r = rig("cart-expire");
    queue_add(&r, "1170462-01");
    let t = Instant::now();
    let got = loop {
        let got = cart_results(&r);
        if !got.is_empty() || t.elapsed() > Duration::from_secs(5) {
            break got;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(got[0].outcome, dig::carts::Outcome::NoExtension);
    assert!(t.elapsed() >= dig::carts::PICK_UP);
}

// ---- tracklist sends (1001tracklists, Mixcloud) --------------------------------------------

fn tracklist_body(tracks: Vec<Value>, mode: &str) -> String {
    serde_json::json!({
        "tracklist": {
            "url": "https://www.1001tracklists.com/tracklist/2p7bd3vt/duarte-sterium.html",
            "title": "Duarte b2b Sterium @ Boiler Room",
            "tracks": tracks,
        },
        "mode": mode,
        "skip_passed": true,
    })
    .to_string()
}

#[test]
fn a_tracklist_send_is_answered_at_once_and_refuses_bad_tracks_alone() {
    let r = rig("tracklist-send");
    let key = r.pair();
    r.bridge.poll();
    let mut tracks: Vec<Value> = (0..17)
        .map(|i| serde_json::json!({ "time": i * 200, "artist": format!("A{i}"), "title": "T" }))
        .collect();
    tracks.push(serde_json::json!({ "time": 4000, "id": true }));
    tracks.push(serde_json::json!({ "time": 99999, "artist": "Late", "title": "T" }));
    let started = Instant::now();
    let a = r.send(&key, &tracklist_body(tracks, "play"));
    assert!(started.elapsed() < Duration::from_millis(100));
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(a.json["page"], "Mix: Duarte b2b Sterium @ Boiler Room");
    assert_eq!(
        (a.json["records"].as_u64(), a.json["refused"].as_u64()),
        (Some(18), Some(1))
    );
    match &r.bridge.poll()[..] {
        [BridgeCommand::SendTracklist { list, mode }] => {
            assert_eq!(list.tracks.len(), 18);
            assert!(list.tracks[17].is_id);
            assert_eq!(*mode, Mode::Play);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn tracklist_sends_are_limited_in_size_and_shape() {
    let r = rig("tracklist-shape");
    let key = r.pair();
    r.bridge.poll();
    // Over 300 tracks, or over 256 KB, or from another site: refused whole.
    let many: Vec<Value> = (0..301)
        .map(|i| serde_json::json!({ "artist": format!("A{i}"), "title": "T" }))
        .collect();
    assert_eq!(r.send(&key, &tracklist_body(many, "play")).status, 422);
    let pad = "x".repeat(190);
    let big: Vec<Value> = (0..300)
        .map(|i| serde_json::json!({ "artist": format!("{pad}{i}"), "title": pad, "label": pad, "version": pad, "x": pad }))
        .collect();
    let body = tracklist_body(big, "play");
    assert!(body.len() > 256 * 1024, "{}", body.len());
    assert_eq!(r.send(&key, &body).status, 413);
    let other = tracklist_body(
        vec![serde_json::json!({ "artist": "A", "title": "T" })],
        "play",
    )
    .replace("www.1001tracklists.com", "evil.net");
    assert_eq!(r.send(&key, &other).status, 422);
    assert!(r.bridge.poll().is_empty(), "nothing reached the player");
}

#[test]
fn a_youtube_or_soundcloud_mix_page_is_sent_by_its_address() {
    use dig::tracklist::MixSource;
    let r = rig("mix-page");
    let key = r.pair();
    r.bridge.poll();
    let a = r.send(
        &key,
        &send_body(
            "https://soundcloud.com/tracklistings3-0/tracklistings-mixtape-781?in=toast3r/sets/x",
            "play",
        ),
    );
    assert_eq!(a.status, 202, "{}", a.json);
    assert_eq!(a.json["message"], "Reading the mix's tracklist");
    match &r.bridge.poll()[..] {
        [BridgeCommand::SendMix { source, mode }] => {
            assert_eq!(
                *source,
                MixSource::SoundCloud {
                    user: "tracklistings3-0".into(),
                    slug: "tracklistings-mixtape-781".into()
                }
            );
            assert_eq!(*mode, Mode::Play);
        }
        other => panic!("{other:?}"),
    }
    // A playlist, or a look-alike host, isn't a mix.
    for url in [
        "https://soundcloud.com/toast3r/sets/2023-2025-vinyl-mixes",
        "https://soundcloud.com.evil.net/a/b",
    ] {
        assert_eq!(r.send(&key, &send_body(url, "play")).status, 422, "{url}");
    }
    assert!(r.bridge.poll().is_empty());
}
