//! Digging in the player, headless: pasted pages become crates of previews, Y / N / I decide,
//! OPT ▸ Discogs… checks a token, and nothing reaches Discogs before the window is up.

use super::digging::BridgeAction;
use super::*;
use ::dig::browser::{FakeBrowser, sell_url};
use ::dig::clock::RealClock;
use ::dig::discogs::transport::{FakeTransport, Method};
use ::dig::discogs::url;
use ::dig::jobs::{Filters, Job, Jobs};
use ::dig::memory::DigMemory;
use ::dig::preview::fetcher::{FakeFetcher, Fetcher};
use ::dig::preview::scheduler::Finder;

const RELEASE: &str = "https://www.discogs.com/release/1001-Glasshouse-EP";
const CLIPS: [&str; 3] = ["GLASShouse1", "LUMENremix1", "LASTlight01"];

#[derive(Clone)]
struct Fakes {
    transport: Arc<FakeTransport>,
    fetcher: Arc<FakeFetcher>,
    browser: Arc<FakeBrowser>,
}

impl Fakes {
    fn new() -> Self {
        Self {
            transport: Arc::new(FakeTransport::with_fixtures(format!(
                "{}/../dig/tests/fixtures/discogs",
                env!("CARGO_MANIFEST_DIR")
            ))),
            fetcher: Arc::new(FakeFetcher::new(fixture("tone.m4a"))),
            browser: Arc::new(FakeBrowser::default()),
        }
    }

    fn setup(&self, dir: &Path) -> DigSetup {
        let f = self.fetcher.clone();
        let finder: Finder = Arc::new(move |_| {
            let v = f.version()?;
            Some((f.clone() as Arc<dyn Fetcher>, v))
        });
        DigSetup {
            transport: self.transport.clone(),
            clock: Arc::new(RealClock::default()),
            finder,
            browser: self.browser.clone(),
            cache_root: Some(dir.join("cache")),
            bridge: BridgeSetup::Off,
        }
    }

    fn changes(&self) -> Vec<(Method, String)> {
        self.transport
            .log()
            .into_iter()
            .filter(|(r, _)| r.method != Method::Get)
            .map(|(r, _)| (r.method, r.path))
            .collect()
    }
}

fn rig(name: &str, fakes: &Fakes, prepare: impl FnOnce(&Store)) -> Rig {
    let f = fakes.clone();
    Rig::with_dig(name, Vec::new(), prepare, move |dir| Some(f.setup(dir)))
}

fn with_token(store: &Store) {
    ::dig::config::save_token(store.dir(), Some("tok")).unwrap();
}

fn key(rig: &mut Rig, key: Key) {
    rig.frame(vec![Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
}

fn clips(rig: &Rig, c: CrateId) -> Vec<String> {
    rig.app
        .crates
        .get(c)
        .map(|p| {
            p.entries()
                .iter()
                .filter_map(|e| e.origin.as_ref()?.clip.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn all_playable(rig: &Rig, c: CrateId, n: usize) -> bool {
    rig.app
        .crates
        .get(c)
        .is_some_and(|p| p.len() == n && p.entries().iter().all(|e| e.status.is_playable()))
}

fn message(rig: &Rig) -> String {
    rig.app
        .message
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default()
}

fn memory(rig: &Rig) -> &DigMemory {
    &rig.app.dig.as_ref().unwrap().memory
}

/// Play mode for the release, until its first preview plays.
fn play_release(rig: &mut Rig) -> CrateId {
    rig.app
        .dig_send(url::parse(RELEASE).unwrap(), SendMode::Play, None);
    let c = rig.app.crates.shown_id();
    assert_ne!(c, PLAYLIST, "a new crate is shown");
    rig.until(
        |r| r.app.position.state == PlayState::Playing && r.app.crates.playing_id() == c,
        "the first preview plays",
    );
    c
}

fn playing_clip(rig: &Rig) -> Option<String> {
    let p = rig.app.crates.playing();
    p.get(p.current()?)?.origin.as_ref()?.clip.clone()
}

#[test]
fn a_pasted_release_fills_the_shown_crate_with_playable_previews() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-paste", &fakes, |_| {});
    rig.frame(vec![Event::Paste(RELEASE.into())]);
    rig.until(|r| all_playable(r, PLAYLIST, 3), "three previews ready");
    assert_eq!(clips(&rig, PLAYLIST), CLIPS);
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    let o = p.entries()[0].origin.clone().unwrap();
    assert_eq!((o.release, o.position.as_str()), (Some(1001), "A1"));
    assert!(
        o.for_sale
            .is_some_and(|f| f.count == 6 && f.lowest_cents == Some(900))
    );
    for c in CLIPS {
        assert!(rig.dir.join(format!("cache/previews/{c}.m4a")).exists());
    }

    // Other text is ignored; a Discogs page that can't be dug says which can.
    let before = fakes.transport.count();
    rig.frame(vec![Event::Paste("just some words".into())]);
    assert_eq!(fakes.transport.count(), before);
    rig.frame(vec![Event::Paste(
        "https://www.discogs.com/sell/list".into(),
    )]);
    assert_eq!(message(&rig), url::SUPPORTED);
    assert_eq!(rig.app.crates.get(PLAYLIST).unwrap().len(), 3);
}

#[test]
fn play_mode_names_a_new_crate_after_the_page_and_plays_its_first_preview() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-play", &fakes, |_| {});
    let c = play_release(&mut rig);
    assert_eq!(playing_clip(&rig).as_deref(), Some(CLIPS[0]));
    rig.until(
        |r| r.app.crates.name(c).contains("Glasshouse EP"),
        "the crate takes the page's name",
    );
    // The title line carries the side, year and for-sale summary.
    let (artist, title, duration) = rig.app.now_playing_names().unwrap();
    let mut line = format::title_line(1, &artist, &title, duration);
    let p = rig.app.crates.playing();
    line += &format::origin_details(
        p.get(p.current().unwrap())
            .unwrap()
            .origin
            .as_ref()
            .unwrap(),
    );
    assert!(line.contains(" · A1"), "{line}");
    assert!(line.contains("6 for sale from"), "{line}");
}

#[test]
fn a_send_for_a_missing_page_takes_its_new_crate_away_again() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-missing", &fakes, |_| {});
    let page = url::parse("https://www.discogs.com/label/999").unwrap();
    let name = page.provisional_name();
    rig.app.dig_send(page, SendMode::Play, None);
    assert!(rig.app.crates.find(&name).is_some());
    rig.until(
        |r| r.app.crates.find(&name).is_none(),
        "the empty crate goes",
    );
    assert_eq!(rig.app.crates.shown_id(), PLAYLIST);
    assert!(message(&rig).contains(&name), "{}", message(&rig));
}

#[test]
fn y_keeps_the_playing_preview_and_y_again_undoes_it() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-keep", &fakes, with_token);
    let c = play_release(&mut rig);
    key(&mut rig, Key::Y);
    let keepers = rig
        .app
        .crates
        .find(digging::KEEPERS)
        .expect("Keepers created");
    rig.app.crates.load(keepers);
    assert_eq!(clips(&rig, keepers), [CLIPS[0]]);
    assert!(memory(&rig).is_kept(CLIPS[0]));
    let out = rig.frame(Vec::new());
    assert!(
        texts(&out).iter().any(|t| t.text.starts_with("1. ✓ ")),
        "{:?}",
        text_list(&out)
    );
    let put = (Method::Put, "/users/digger/wants/1001".to_owned());
    rig.until(
        |r| {
            memory(r)
                .kept
                .get(CLIPS[0])
                .is_some_and(|k| k.added_to_wantlist)
        },
        "on the wantlist",
    );
    assert_eq!(fakes.changes(), std::slice::from_ref(&put));

    key(&mut rig, Key::Y);
    assert!(!memory(&rig).is_kept(CLIPS[0]));
    assert!(clips(&rig, keepers).is_empty());
    assert_eq!(clips(&rig, c).len(), 3, "the dug crate keeps it");
    rig.until(|_| fakes.changes().len() == 2, "taken off the wantlist");
    assert_eq!(fakes.changes()[1], (Method::Delete, put.1));
    // Saved for the next session.
    let saved = DigMemory::load(&rig.dir.join("config"));
    assert!(saved.kept.is_empty());
}

#[test]
fn keeping_without_a_token_keeps_locally_and_says_why_not_the_wantlist() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-keep-anon", &fakes, |_| {});
    play_release(&mut rig);
    key(&mut rig, Key::Y);
    assert!(memory(&rig).is_kept(CLIPS[0]));
    assert!(message(&rig).contains("token"), "{}", message(&rig));
    assert!(fakes.changes().is_empty());
}

#[test]
fn n_passes_the_playing_preview_and_later_sends_leave_it_out() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-pass", &fakes, |_| {});
    let c = play_release(&mut rig);
    key(&mut rig, Key::N);
    assert!(memory(&rig).is_passed(CLIPS[0]));
    rig.until(
        |r| playing_clip(r).as_deref() == Some(CLIPS[1]),
        "the next preview plays",
    );
    // Dimmed in its row.
    rig.until(|r| all_playable(r, c, 3), "all previews ready");
    let out = rig.frame(Vec::new());
    let col = |prefix: &str| {
        texts(&out)
            .into_iter()
            .find(|t| t.text.starts_with(prefix))
            .and_then(|t| t.color)
            .unwrap()
    };
    let (passed, other) = (col("1. "), col("3. "));
    assert!(passed.r() + passed.g() + passed.b() < other.r() + other.g() + other.b());

    // A later send of the same page leaves it out; undoing the pass brings it back next time.
    rig.app.dig_send(
        url::parse(RELEASE).unwrap(),
        SendMode::Crate("Again".into()),
        None,
    );
    let again = rig.app.crates.find("Again").unwrap();
    rig.until(|r| all_playable(r, again, 2), "two previews");
    assert_eq!(clips(&rig, again), &CLIPS[1..]);
    let first = rig.ids(c)[0];
    rig.app.dig_act(c, DigAction::UndoPass(first));
    assert!(!memory(&rig).is_passed(CLIPS[0]));
}

#[test]
fn a_kept_preview_cannot_be_passed() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-pass-kept", &fakes, |_| {});
    play_release(&mut rig);
    key(&mut rig, Key::Y);
    key(&mut rig, Key::N);
    assert!(!memory(&rig).is_passed(CLIPS[0]));
    assert!(message(&rig).contains("is kept"), "{}", message(&rig));
    assert_eq!(playing_clip(&rig).as_deref(), Some(CLIPS[0]));
}

#[test]
fn i_opens_the_for_sale_page_and_verdict_keys_wait_while_stopped() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-sale", &fakes, |_| {});
    key(&mut rig, Key::I);
    key(&mut rig, Key::Y);
    assert!(
        fakes.browser.opened.lock().unwrap().is_empty(),
        "nothing plays"
    );
    assert!(memory(&rig).kept.is_empty());
    play_release(&mut rig);
    key(&mut rig, Key::I);
    assert_eq!(*fakes.browser.opened.lock().unwrap(), [sell_url(1001)]);
    // The fullscreen host keeps them too.
    key(&mut rig, Key::F);
    assert!(rig.app.fullscreen.is_some());
    key(&mut rig, Key::I);
    assert_eq!(fakes.browser.opened.lock().unwrap().len(), 2);
    key(&mut rig, Key::Y);
    assert!(memory(&rig).is_kept(CLIPS[0]));
}

#[test]
fn i_on_a_local_file_says_it_is_not_from_discogs() {
    let fakes = Fakes::new();
    let f = fakes.clone();
    let mut rig = Rig::with_dig(
        "dig-sale-local",
        vec![fixture("tone.wav")],
        |_| {},
        move |dir| Some(f.setup(dir)),
    );
    rig.until(
        |r| r.app.position.state == PlayState::Playing,
        "the file plays",
    );
    key(&mut rig, Key::I);
    assert!(
        message(&rig).contains("isn't from Discogs"),
        "{}",
        message(&rig)
    );
    assert!(fakes.browser.opened.lock().unwrap().is_empty());
}

#[test]
fn the_entry_menu_keeps_passes_and_opens_the_for_sale_page() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-menu", &fakes, |_| {});
    rig.frame(vec![Event::Paste(RELEASE.into())]);
    rig.until(|r| all_playable(r, PLAYLIST, 3), "three previews ready");
    let row = rig.row(1);
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Pass (N)");
    assert!(memory(&rig).is_passed(CLIPS[1]));
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Undo pass");
    assert!(!memory(&rig).is_passed(CLIPS[1]));
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Keep (Y)");
    assert!(memory(&rig).is_kept(CLIPS[1]));
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Open for-sale page (I)");
    assert_eq!(*fakes.browser.opened.lock().unwrap(), [sell_url(1001)]);
}

#[test]
fn the_discogs_dialog_checks_a_token_before_saving_it() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-dialog", &fakes, |_| {});
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::OpenDialog), &ctx);
    rig.until(
        |r| {
            shows(
                &r.frame(Vec::new()),
                "No token: pages still work, at Discogs' lower rate limit",
            )
        },
        "the dialog shows",
    );
    let out = rig.frame(Vec::new());
    let field = texts(&out)
        .into_iter()
        .find(|t| t.text == "Personal access token")
        .unwrap()
        .rect
        .center();
    rig.click(field);
    rig.frame(vec![Event::Text("secrettoken1234".into())]);
    rig.click_text("Check and save");
    rig.until(
        |r| shows(&r.frame(Vec::new()), "Connected as digger (••••1234)"),
        "the account shows, with only the token's end",
    );
    let config = rig.dir.join("config");
    assert_eq!(
        ::dig::config::load_token(&config).as_deref(),
        Some("secrettoken1234")
    );
    let out = rig.frame(Vec::new());
    assert!(!text_list(&out).iter().any(|t| t.contains("secrettoken")));
}

#[test]
fn nothing_reaches_discogs_before_the_window_is_interactive() {
    let fakes = Fakes::new();
    let f = fakes.clone();
    // A token and a send that was interrupted last session.
    let prepare = |store: &Store| {
        with_token(store);
        let mut jobs = Jobs::default();
        let id = jobs.alloc_id();
        jobs.jobs.push(Job::new(
            id,
            url::parse(RELEASE).unwrap(),
            PLAYLIST,
            Filters::default(),
        ));
        jobs.save(store.dir()).unwrap();
    };
    let mut rig = Rig::build("dig-launch", Vec::new(), prepare, move |dir| {
        Some(f.setup(dir))
    });
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        fakes.transport.count(),
        0,
        "no request before the first frame"
    );
    rig.until(|r| r.app.engine().is_some(), "the engine starts");
    rig.until(|r| all_playable(r, PLAYLIST, 3), "the send resumes");
}

#[test]
fn without_yt_dlp_entries_say_so_once_and_download_when_it_appears() {
    let fakes = Fakes::new();
    fakes
        .fetcher
        .available
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let mut rig = rig("dig-no-ytdlp", &fakes, |_| {});
    rig.frame(vec![Event::Paste(RELEASE.into())]);
    let needs = |r: &mut Rig| {
        r.app.crates.get(PLAYLIST).is_some_and(|p| {
            p.len() == 3
                && p.entries()
                    .iter()
                    .all(|e| e.status == EntryStatus::Waiting("needs yt-dlp".into()))
        })
    };
    rig.until(needs, "entries wait for yt-dlp");
    assert!(
        message(&rig).contains("brew install yt-dlp"),
        "{}",
        message(&rig)
    );
    fakes
        .fetcher
        .available
        .store(true, std::sync::atomic::Ordering::SeqCst);
    // The scheduler looks again every 30 s; the dialog asks at once.
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::OpenDialog), &ctx);
    rig.until(|r| all_playable(r, PLAYLIST, 3), "downloaded once found");
}

#[test]
fn a_keep_while_discogs_is_offline_waits_as_wantlist_pending() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-keep-offline", &fakes, with_token);
    play_release(&mut rig);
    fakes.transport.set_offline(true);
    key(&mut rig, Key::Y);
    rig.until(
        |r| memory(r).is_want_pending(1001),
        "the wantlist change is kept for later",
    );
    assert!(memory(&rig).is_kept(CLIPS[0]), "kept locally all the same");
    let out = rig.frame(Vec::new());
    assert!(
        texts(&out)
            .iter()
            .any(|t| t.text.starts_with("1. ✓ ") && t.text.ends_with("(wantlist pending)")),
        "{:?}",
        text_list(&out)
    );
    let saved = DigMemory::load(&rig.dir.join("config"));
    assert!(
        saved.is_want_pending(1001),
        "and saved for the next session"
    );
}

// ---- the browser bridge ------------------------------------------------------------------------

const LABEL: &str = "https://www.discogs.com/label/12345-Lowtide-Tapes";

fn bridge_rig(name: &str, fakes: &Fakes, bridge: BridgeSetup) -> Rig {
    let f = fakes.clone();
    Rig::with_dig(
        name,
        Vec::new(),
        |_| {},
        move |dir| {
            Some(DigSetup {
                bridge,
                ..f.setup(dir)
            })
        },
    )
}

fn bridge_port(rig: &Rig) -> u16 {
    rig.app
        .dig
        .as_ref()
        .and_then(|d| d.bridge_port())
        .expect("the bridge runs")
}

fn shared(rig: &Rig) -> Arc<::dig::bridge::Shared> {
    rig.app
        .dig
        .as_ref()
        .and_then(|d| d.bridge_shared.clone())
        .unwrap()
}

/// As the extension calls: status and JSON body.
fn http(port: u16, method: &str, path: &str, key: Option<&str>, body: &str) -> (u16, String) {
    use std::io::{Read, Write};
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
         Origin: chrome-extension://abcdefghijklmnop\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(k) = key {
        req += &format!("X-Bridge-Key: {k}\r\n");
    }
    req += "\r\n";
    req += body;
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, body.to_owned())
}

fn pair(rig: &Rig) -> String {
    let port = bridge_port(rig);
    let code = shared(rig).pairing().ensure_code().unwrap().0;
    let (status, body) = http(
        port,
        "POST",
        "/v1/pair",
        None,
        &format!(r#"{{"code":"{code}"}}"#),
    );
    assert_eq!(status, 200, "{body}");
    body.split('"').nth(3).unwrap().to_owned()
}

fn open_browser_dialog(rig: &mut Rig) {
    rig.until(
        |r| {
            shows(
                &r.frame(Vec::new()),
                "Enter this code in the extension's options.",
            )
        },
        "OPT ▸ Browser… shows",
    );
}

#[test]
fn the_bridge_starts_only_after_the_first_frame() {
    let fakes = Fakes::new();
    let f = fakes.clone();
    let mut rig = Rig::build(
        "dig-bridge-launch",
        Vec::new(),
        |_| {},
        move |dir| {
            Some(DigSetup {
                bridge: BridgeSetup::Port(0),
                ..f.setup(dir)
            })
        },
    );
    assert!(rig.app.dig.as_ref().unwrap().bridge_port().is_none());
    rig.until(|r| r.app.engine().is_some(), "the engine starts");
    rig.frame(Vec::new());
    bridge_port(&rig);
}

#[test]
fn a_browser_send_is_answered_at_once_and_the_crate_fills_in_afterwards() {
    let fakes = Fakes::new();
    let mut rig = bridge_rig("dig-bridge-send", &fakes, BridgeSetup::Port(0));
    let key = pair(&rig);
    rig.frame(Vec::new());
    // Discogs is slow: each of the next answers takes 250 ms.
    for _ in 0..4 {
        fakes
            .transport
            .fault(::dig::discogs::transport::Fault::Delay(
                Duration::from_millis(250),
            ));
    }
    let body =
        format!(r#"{{"url":"{LABEL}","mode":"enqueue","vinyl_only":false,"skip_passed":true}}"#);
    let t = Instant::now();
    let (status, answer) = http(bridge_port(&rig), "POST", "/v1/send", Some(&key), &body);
    let took = t.elapsed();
    assert_eq!(status, 202, "{answer}");
    assert!(took < Duration::from_millis(100), "answered in {took:?}");
    assert!(
        answer.contains(r#""page":"Label: Lowtide Tapes""#),
        "{answer}"
    );
    assert!(answer.contains(r#""crate":"Playlist""#), "{answer}");

    // Exactly as a paste: the label's records arrive in the shown crate.
    rig.until(
        |r| r.app.crates.get(PLAYLIST).is_some_and(|p| !p.is_empty()),
        "the crate fills in",
    );
    let (status, crates) = http(bridge_port(&rig), "GET", "/v1/crates", Some(&key), "");
    assert_eq!(status, 200);
    assert!(
        crates.contains(r#"{"name":"Playlist","playing":false,"shown":true}"#),
        "{crates}"
    );
}

#[test]
fn opt_browser_shows_a_code_that_pairs_once_and_forget_browsers_revokes() {
    let fakes = Fakes::new();
    let mut rig = bridge_rig("dig-bridge-dialog", &fakes, BridgeSetup::Port(0));
    let port = bridge_port(&rig);
    let shared = shared(&rig);
    assert!(
        shared.pairing().code().is_none(),
        "no code before the dialog opens"
    );
    rig.app.dig_act(PLAYLIST, DigAction::OpenBrowserDialog);
    open_browser_dialog(&mut rig);
    let out = rig.frame(Vec::new());
    let (code, _) = shared
        .pairing()
        .code()
        .map(|(c, l)| (c.to_owned(), l))
        .unwrap();
    let texts = text_list(&out);
    let shown = format!("{} {}", &code[..3], &code[3..]);
    assert!(texts.contains(&shown), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Valid for 2:00") || t.starts_with("Valid for 1:5"))
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains(&format!("127.0.0.1:{port}")))
    );

    let pair_body = format!(r#"{{"code":"{code}"}}"#);
    let (status, body) = http(port, "POST", "/v1/pair", None, &pair_body);
    assert_eq!(status, 200);
    let key = body.split('"').nth(3).unwrap().to_owned();
    let out = rig.frame(Vec::new());
    assert_eq!(message(&rig), "A browser was paired");
    let texts = text_list(&out);
    assert!(texts.iter().any(|t| t == "1 paired browser"), "{texts:?}");
    // The used code no longer works; the dialog shows a fresh one.
    assert_eq!(http(port, "POST", "/v1/pair", None, &pair_body).0, 401);
    assert_ne!(shared.pairing().code().unwrap().0, code);
    assert_eq!(http(port, "GET", "/v1/status", Some(&key), "").0, 200);

    rig.click_text("Forget browsers");
    assert_eq!(
        message(&rig),
        "Browsers forgotten: each one needs pairing again"
    );
    assert_eq!(http(port, "GET", "/v1/status", Some(&key), "").0, 401);
    rig.frame(Vec::new());

    // Closed, no code works.
    let code = shared.pairing().code().unwrap().0.to_owned();
    rig.app.dig_bridge_act(BridgeAction::Close);
    let (status, _) = http(
        port,
        "POST",
        "/v1/pair",
        None,
        &format!(r#"{{"code":"{code}"}}"#),
    );
    assert_eq!(status, 410);
}

#[test]
fn a_taken_port_leaves_the_player_working_and_says_so() {
    let taken = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = taken.local_addr().unwrap().port();
    let fakes = Fakes::new();
    let mut rig = bridge_rig("dig-bridge-taken", &fakes, BridgeSetup::Port(port));
    assert!(rig.app.dig.as_ref().unwrap().bridge_port().is_none());
    rig.app.dig_act(PLAYLIST, DigAction::OpenBrowserDialog);
    open_browser_dialog(&mut rig);
    let taken_text = format!("Port {port} is taken by another program");
    assert!(shows(&rig.frame(Vec::new()), &taken_text));
    // Pasting still works.
    rig.frame(vec![Event::Paste(RELEASE.into())]);
    rig.until(|r| all_playable(r, PLAYLIST, 3), "three previews ready");

    // Another port, saved for next time.
    let free = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let other = free.local_addr().unwrap().port();
    drop(free);
    rig.app.dig_bridge_act(BridgeAction::SetPort(other));
    assert_eq!(bridge_port(&rig), other);
    let saved = ::dig::bridge::pairing::load(&rig.dir.join("config"));
    assert_eq!(saved.port, other);
}
