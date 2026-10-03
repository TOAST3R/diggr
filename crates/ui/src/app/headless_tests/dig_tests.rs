//! Digging in the player, headless: pasted pages become crates of previews, Y / N / I decide,
//! Options ▸ Discogs… checks a token, and nothing reaches Discogs before the window is up.

use super::digging::BridgeAction;
use super::*;
use ::dig::browser::{FakeBrowser, sell_url};
use ::dig::clock::RealClock;
use ::dig::cover::{FakeImages, test_jpeg};
use ::dig::discogs::model::RecordKey;
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
    images: Arc<FakeImages>,
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
            images: FakeImages::new(Ok(test_jpeg(300, 300))),
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
            images: self.images.clone(),
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

/// The wantlist crate, once it exists.
fn wantlist(rig: &Rig) -> Option<CrateId> {
    rig.app.dig.as_ref()?.settings.wantlist
}

/// The clips of release `r` in crate `c`.
fn clips_of(rig: &Rig, c: CrateId, r: u64) -> Vec<String> {
    rig.app
        .crates
        .get(c)
        .map(|p| {
            p.entries()
                .iter()
                .filter_map(|e| e.origin.as_ref())
                .filter(|o| o.release == Some(r))
                .filter_map(|o| o.clip.clone())
                .collect()
        })
        .unwrap_or_default()
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
    let line = rig.app.now_playing_line().unwrap();
    assert!(line.contains(" · A1"), "{line}");
    assert!(line.contains("6 for sale from"), "{line}");
}

#[test]
fn a_send_that_creates_a_crate_shows_it_and_one_to_an_existing_crate_does_not() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-new-crate-shown", &fakes, |_| {});
    rig.app.settings.show_playlist = false;
    rig.app.dig_send(
        url::parse(RELEASE).unwrap(),
        SendMode::Crate("Friday".into()),
        None,
    );
    let friday = rig.app.crates.find("Friday").unwrap();
    assert_eq!(
        rig.app.crates.shown_id(),
        friday,
        "the new crate is on screen"
    );
    assert!(rig.app.settings.show_playlist, "with the playlist open");
    // Sending again to it, while another crate is shown, leaves the view alone.
    rig.app.show_crate(PLAYLIST);
    rig.app.dig_send(
        url::parse(RELEASE).unwrap(),
        SendMode::Crate("Friday".into()),
        None,
    );
    assert_eq!(rig.app.crates.shown_id(), PLAYLIST);
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
fn y_adds_the_record_to_the_wantlist_and_y_again_removes_it() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-want", &fakes, with_token);
    let c = play_release(&mut rig);
    key(&mut rig, Key::Y);
    let wl = wantlist(&rig).expect("the wantlist crate");
    rig.app.crates.load(wl);
    // The whole record, at once.
    assert_eq!(clips_of(&rig, wl, 1001), CLIPS);
    assert!(memory(&rig).is_wanted(1001));
    assert!(
        message(&rig).contains("to your wantlist"),
        "{}",
        message(&rig)
    );
    let out = rig.frame(Vec::new());
    for n in 1..=3 {
        assert!(
            texts(&out)
                .iter()
                .any(|t| t.text.starts_with(&format!("{n}. ✓ "))),
            "every entry of the record: {:?}",
            text_list(&out)
        );
    }
    let put = (Method::Put, "/users/digger/wants/1001".to_owned());
    rig.until(
        |r| !r.app.dig.as_ref().unwrap().memory.is_want_pending(1001),
        "sent",
    );
    assert!(fakes.changes().contains(&put), "{:?}", fakes.changes());

    key(&mut rig, Key::Y);
    assert!(!memory(&rig).is_wanted(1001));
    assert!(clips_of(&rig, wl, 1001).is_empty());
    assert_eq!(clips(&rig, c).len(), 3, "the dug crate keeps it");
    let delete = (Method::Delete, put.1);
    rig.until(
        |_| fakes.changes().contains(&delete),
        "taken off the wantlist",
    );
    // Saved for the next session.
    let saved = DigMemory::load(&rig.dir.join("config"));
    assert!(!saved.is_wanted(1001));
}

#[test]
fn without_a_token_the_record_is_wanted_here_and_the_connect_dialog_explains() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-want-anon", &fakes, |_| {});
    play_release(&mut rig);
    key(&mut rig, Key::Y);
    assert!(memory(&rig).is_wanted(1001));
    let wl = wantlist(&rig).unwrap();
    assert_eq!(rig.app.crates.name(wl), "Wantlist");
    assert!(
        !rig.app.crates.is_wantlist(wl),
        "one of the user's own crates"
    );
    rig.until(
        |r| shows(&r.frame(Vec::new()), "Connect to Discogs"),
        "the dialog shows",
    );
    let out = rig.frame(Vec::new());
    assert!(
        text_list(&out)
            .iter()
            .any(|t| t.contains("keep it up to date from here"))
    );
    // Shortcuts wait while it's open.
    key(&mut rig, Key::N);
    assert!(memory(&rig).passed.is_empty());
    rig.click_text("Open ↗");
    assert_eq!(
        *fakes.browser.opened.lock().unwrap(),
        ["https://www.discogs.com/settings/developers"]
    );
    rig.click_text("Don't show this again");
    rig.frame(Vec::new());
    assert!(rig.app.dig.as_ref().unwrap().connect.is_none());
    assert!(
        rig.app
            .dig
            .as_ref()
            .unwrap()
            .settings
            .connect_hint_dismissed
    );
    // From now on, one line instead (Y once takes it off, again puts it back).
    key(&mut rig, Key::Y);
    assert!(!memory(&rig).is_wanted(1001));
    key(&mut rig, Key::Y);
    assert!(memory(&rig).is_wanted(1001));
    assert!(rig.app.dig.as_ref().unwrap().connect.is_none());
    assert!(
        message(&rig).contains("Options ▸ Discogs…"),
        "{}",
        message(&rig)
    );
    assert!(fakes.changes().is_empty());
}

#[test]
fn connect_opens_the_token_field_and_add_to_collection_always_asks() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-connect", &fakes, |_| {});
    let c = play_release(&mut rig);
    let first = rig.ids(c)[0];
    rig.app.dig_act(c, DigAction::Collect(vec![first]));
    rig.until(
        |r| shows(&r.frame(Vec::new()), "Connect to Discogs"),
        "the dialog shows",
    );
    let out = rig.frame(Vec::new());
    assert!(
        !shows(&out, "Don't show this again"),
        "nothing happened here to stop asking for"
    );
    rig.click_text("Connect…");
    rig.frame(Vec::new());
    assert!(rig.app.dig.as_ref().unwrap().connect.is_none());
    assert!(
        rig.app.dig.as_ref().unwrap().dialog.is_some(),
        "Options ▸ Discogs… opens"
    );
    for _ in 0..5 {
        rig.frame(Vec::new());
    }
    assert!(rig.ctx.text_edit_focused(), "with the token field focused");
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
fn a_wanted_record_cannot_be_passed() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-pass-wanted", &fakes, |_| {});
    play_release(&mut rig);
    key(&mut rig, Key::Y);
    rig.app.dig.as_mut().unwrap().connect = None;
    key(&mut rig, Key::N);
    assert!(!memory(&rig).is_passed(CLIPS[0]));
    assert!(
        message(&rig).contains("is on your wantlist"),
        "{}",
        message(&rig)
    );
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
    assert!(memory(&rig).wanted.is_empty());
    play_release(&mut rig);
    key(&mut rig, Key::I);
    assert_eq!(*fakes.browser.opened.lock().unwrap(), [sell_url(1001)]);
    // The fullscreen host keeps them too.
    key(&mut rig, Key::F);
    assert!(rig.app.fullscreen.is_some());
    key(&mut rig, Key::I);
    assert_eq!(fakes.browser.opened.lock().unwrap().len(), 2);
    key(&mut rig, Key::Y);
    assert!(memory(&rig).is_wanted(1001));
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
fn the_entry_menu_wants_passes_and_opens_the_for_sale_page() {
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
    rig.click_text("Add to wantlist (Y)");
    assert!(memory(&rig).is_wanted(1001));
    rig.app.dig.as_mut().unwrap().connect = None;
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Open for-sale page (I)");
    assert_eq!(*fakes.browser.opened.lock().unwrap(), [sell_url(1001)]);
    rig.click_with(row, PointerButton::Secondary);
    rig.click_text("Open release on Discogs");
    let release = "https://www.discogs.com/release/1001";
    assert_eq!(
        fakes.browser.opened.lock().unwrap().last().unwrap(),
        release
    );
    rig.click_with(row, PointerButton::Secondary);
    // The copy goes out in the frame of the click's release.
    let menu = rig.frame(Vec::new());
    let at = texts(&menu)
        .into_iter()
        .find(|t| t.text == "Copy Discogs link")
        .expect("in the menu")
        .rect
        .center();
    rig.frame(vec![Event::PointerMoved(at)]);
    rig.press(at, PointerButton::Primary, true);
    let out = rig.press(at, PointerButton::Primary, false);
    assert!(
        out.platform_output
            .commands
            .contains(&egui::OutputCommand::CopyText(release.into())),
        "{:?}",
        out.platform_output.commands
    );
}

#[test]
fn the_entry_menu_has_no_discogs_links_for_a_local_file() {
    let fakes = Fakes::new();
    let f = fakes.clone();
    let mut rig = Rig::with_dig(
        "dig-menu-local",
        vec![fixture("tone.wav")],
        |_| {},
        move |dir| Some(f.setup(dir)),
    );
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Remove"), "{:?}", text_list(&out));
    assert!(!shows(&out, "Open release on Discogs"));
    assert!(!shows(&out, "Copy Discogs link"));
    assert!(!shows(&out, "Add to wantlist (Y)"));
    assert!(!shows(&out, "Add to collection"));
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
fn a_want_while_discogs_is_offline_waits_as_wantlist_pending() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-want-offline", &fakes, with_token);
    play_release(&mut rig);
    fakes.transport.set_offline(true);
    key(&mut rig, Key::Y);
    rig.until(
        |r| memory(r).is_want_pending(1001),
        "the wantlist change is kept for later",
    );
    assert!(memory(&rig).is_wanted(1001), "wanted here all the same");
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
        "Options ▸ Browser… shows",
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

// ---- the collection -------------------------------------------------------------------

const COLLECTION_PAGE: &str =
    "/users/digger/collection/folders/0/releases?sort=added&sort_order=desc&page=1&per_page=100";

/// The user owns release 1001 (the Glasshouse EP).
fn owns_the_release(fakes: &Fakes) {
    fakes.transport.route(
        COLLECTION_PAGE,
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "items": 1},
            "releases": [{"id": 1001, "instance_id": 1,
                          "basic_information": {"id": 1001, "master_id": 0, "year": 1994,
                                                "labels": [{"catno": "LT-001"}]}}]}"#,
    );
}

fn collection_requests(fakes: &Fakes) -> usize {
    fakes
        .transport
        .paths()
        .iter()
        .filter(|p| p.contains("/collection/"))
        .count()
}

#[test]
fn owned_records_are_marked_and_cannot_be_wanted() {
    let fakes = Fakes::new();
    owns_the_release(&fakes);
    let mut rig = rig("dig-owned", &fakes, with_token);
    play_release(&mut rig);
    rig.until(
        |r| r.app.dig.as_ref().is_some_and(|d| d.collection.is_some()),
        "the collection is synced",
    );
    assert_eq!(collection_requests(&fakes), 1);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "OWNED"), "{:?}", text_list(&out));
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::OpenDialog), &ctx);
    rig.until(
        |r| {
            let out = r.frame(Vec::new());
            shows(&out, "1 record, updated just now") && shows(&out, "Refresh collection")
        },
        "Options ▸ Discogs… shows the collection",
    );
    rig.click_text("Open that page");
    assert!(
        fakes
            .browser
            .opened
            .lock()
            .unwrap()
            .contains(&"https://www.discogs.com/settings/developers".to_owned()),
        "the token page opens"
    );
    rig.app.dig.as_mut().unwrap().dialog = None;

    // Owned: Y wants nothing, and says so; the menu shows it's in the collection.
    key(&mut rig, Key::Y);
    assert!(!memory(&rig).is_wanted(1001));
    assert!(
        message(&rig).contains("is already in your collection (this pressing)"),
        "{}",
        message(&rig)
    );
    assert!(fakes.changes().is_empty(), "no wantlist change");
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "In collection ✓"), "{:?}", text_list(&out));
    assert!(!shows(&out, "Add to wantlist (Y)"));
    assert!(!shows(&out, "Add to collection"));
}

#[test]
fn saving_a_token_makes_the_collection_a_crate_on_screen_once() {
    let fakes = Fakes::new();
    // digger owns releases 1001 and 1003.
    fakes.transport.route(
        COLLECTION_PAGE,
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "per_page": 100, "items": 2}, "releases": [{"id": 1001, "instance_id": 11, "basic_information": {"id": 1001, "master_id": 0, "title": "Glasshouse EP", "year": 1994, "formats": [{"name": "Vinyl", "qty": "1"}], "labels": [{"name": "Lowtide Tapes", "catno": "LT-012", "id": 12345}], "artists": [{"name": "Nightcraft", "anv": "", "join": "", "id": 4242}]}}, {"id": 1003, "instance_id": 12, "basic_information": {"id": 1003, "master_id": 0, "title": "Undertow", "year": 1995, "formats": [{"name": "Vinyl", "qty": "1"}], "labels": [{"name": "Lowtide Tapes", "catno": "LT-013", "id": 12345}], "artists": [{"name": "Nightcraft", "anv": "", "join": "", "id": 4242}]}}]}"#,
    );
    let mut rig = rig("dig-token-collection", &fakes, |_| {});
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::OpenDialog), &ctx);
    rig.until(
        |r| shows(&r.frame(Vec::new()), "Check and save"),
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
        |r| r.app.crates.find("Collection: digger").is_some(),
        "the collection crate is made",
    );
    let c = rig.app.crates.find("Collection: digger").unwrap();
    assert_eq!(rig.app.crates.shown_id(), c, "and shown");
    rig.until(
        |r| r.app.crates.get(c).is_some_and(|p| p.len() >= 2),
        "its records",
    );
    // No OWNED badge inside it, though every record is owned.
    let cache = rig.dir.join("cache");
    // The folder may not exist yet: nothing has had to write to the cache so far.
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(
        cache.join(::dig::collection::FILE),
        r#"(username: "digger", fetched_at: 0, count: 1, instances: [11],
            releases: {1001: (master: None, catno: "LT-012", year: Some(1994))})"#,
    )
    .unwrap();
    rig.app.dig.as_mut().unwrap().collection =
        ::dig::collection::Collection::load(&cache).map(Arc::new);
    assert!(!shows(&rig.frame(Vec::new()), "OWNED"));
    // Saving a token again doesn't make a second one.
    rig.app.dig_token_checked(
        "secrettoken1234".into(),
        Ok(::dig::discogs::client::Identity {
            username: "digger".into(),
            currency: "EUR".into(),
        }),
    );
    let n = rig
        .app
        .crates
        .list()
        .iter()
        .filter(|i| i.name.starts_with("Collection"))
        .count();
    assert_eq!(n, 1);
}

#[test]
fn no_collection_is_asked_for_without_a_token_or_a_crate_from_discogs() {
    // No token: a dig marks nothing and never asks.
    let fakes = Fakes::new();
    owns_the_release(&fakes);
    let mut rig = rig("dig-owned-no-token", &fakes, |_| {});
    play_release(&mut rig);
    for _ in 0..50 {
        rig.pump();
    }
    assert_eq!(collection_requests(&fakes), 0);
    assert!(
        message(&rig).contains("Add a Discogs token"),
        "says what's missing: {}",
        message(&rig)
    );
    assert!(!shows(&rig.frame(Vec::new()), "OWNED"));

    // A token, but only local files on screen: nothing to mark, nothing asked.
    let fakes = Fakes::new();
    owns_the_release(&fakes);
    let f = fakes.clone();
    let mut rig = Rig::with_dig(
        "dig-owned-local",
        vec![fixture("tone.wav")],
        with_token,
        move |dir| Some(f.setup(dir)),
    );
    for _ in 0..50 {
        rig.pump();
    }
    assert_eq!(collection_requests(&fakes), 0);
}

#[test]
fn entries_saved_without_an_album_get_it_from_the_cache_without_a_request() {
    let fakes = Fakes::new();
    // A crate saved before entries carried albums, and release 1001 in the disk cache.
    let prepare = |store: &Store| {
        let root = store.dir().parent().unwrap().join("cache");
        let body = std::fs::read_to_string(format!(
            "{}/../dig/tests/fixtures/discogs/releases_1001_curr_abbr_EUR.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        ::dig::discogs::cache::DiskCache::new(Some(&root)).put_priced(
            ::dig::discogs::cache::Kind::Release,
            "1001",
            &body,
            1,
            Some("EUR"),
        );
        let mut c = Crates::open(store);
        for (release, clip) in [(1001, CLIPS[0]), (1001, CLIPS[1]), (4040, "UNCACHED001")] {
            let origin = Origin {
                release: Some(release),
                clip: Some(clip.into()),
                ..Default::default()
            };
            c.shown_mut()
                .add_waiting("Nightcraft", clip, None, Some(origin), "queued");
        }
        c.touch(PLAYLIST);
        c.save_due(true, Duration::ZERO);
    };
    let mut rig = rig("dig-backfill", &fakes, prepare);
    rig.until(
        |r| r.app.crates.get(PLAYLIST).unwrap().entries()[0].album() == "Glasshouse EP",
        "the album is filled from the cache",
    );
    let p = rig.app.crates.get(PLAYLIST).unwrap();
    let o = p.entries()[1].origin.clone().unwrap();
    assert_eq!(
        (o.album.as_str(), o.cover.as_str()),
        (
            "Glasshouse EP",
            "https://i.discogs.com/fake/R-1001-front-150.jpeg"
        )
    );
    assert_eq!(p.entries()[2].album(), "", "not cached: left as it is");
    assert_eq!(fakes.transport.count(), 0, "no request to Discogs");
}

/// Six Discogs entries, each from its own release (100 to 105) with a cover address.
fn cover_crate(rig: &mut Rig) -> Vec<String> {
    let p = rig.app.crates.shown_mut();
    let mut urls = Vec::new();
    for i in 0..6u64 {
        let url = format!("https://i.discogs.com/cover-{i}.jpeg");
        let origin = Origin {
            release: Some(100 + i),
            album: format!("Album {i}"),
            cover: url.clone(),
            clip: Some(format!("COVERclip{i:02}")),
            ..Default::default()
        };
        p.add_waiting(
            "Nightcraft",
            format!("Track {i}"),
            None,
            Some(origin),
            "queued",
        );
        urls.push(url);
    }
    rig.frame(Vec::new());
    urls
}

fn has_cover(rig: &Rig, release: u64) -> bool {
    rig.app
        .dig
        .as_ref()
        .unwrap()
        .covers
        .has_texture(RecordKey::Release(release))
}

#[test]
fn resting_on_an_entry_shows_its_cover_fetched_once() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-cover", &fakes, |_| {});
    let urls = cover_crate(&mut rig);
    rig.frame(vec![Event::PointerMoved(rig.row(2))]);
    rig.frame(Vec::new());
    assert_eq!(fakes.images.count(), 0, "nothing before the pointer rests");
    rig.until(|r| has_cover(r, 102), "the cover arrives");
    assert_eq!(fakes.images.log(), [urls[2].clone()]);
    assert!(
        rig.dir.join("cache/covers/release-102.png").exists(),
        "kept on disk"
    );
    // Away and back: drawn from memory, no second fetch.
    rig.frame(vec![Event::PointerMoved(rig.row(4))]);
    rig.frame(vec![Event::PointerMoved(rig.row(2))]);
    std::thread::sleep(super::super::covers::REST);
    rig.frame(Vec::new());
    assert!(has_cover(&rig, 102));
    assert!(
        fakes.images.log().iter().all(|u| *u != urls[4]),
        "row 5 wasn't rested on"
    );
    assert_eq!(fakes.images.count(), 1);
}

#[test]
fn sweeping_down_the_list_fetches_only_where_the_pointer_stops() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-cover-sweep", &fakes, |_| {});
    let urls = cover_crate(&mut rig);
    for row in 0..5 {
        rig.frame(vec![Event::PointerMoved(rig.row(row))]);
    }
    rig.until(|r| has_cover(r, 104), "the last row's cover arrives");
    assert_eq!(fakes.images.log(), [urls[4].clone()]);
}

#[test]
fn a_cover_whose_address_is_gone_is_looked_up_once_more() {
    let fakes = Fakes::new();
    fakes.images.route(
        "https://i.discogs.com/expired.jpeg",
        Err(::dig::cover::ImageError::Status(404)),
    );
    let mut rig = rig("dig-cover-stale", &fakes, |_| {});
    // Release 1001's data (in the fixtures) has a different, working address.
    let p = rig.app.crates.shown_mut();
    let origin = Origin {
        release: Some(1001),
        cover: "https://i.discogs.com/expired.jpeg".into(),
        clip: Some(CLIPS[0].into()),
        ..Default::default()
    };
    p.add_waiting("Nightcraft", "Glasshouse", None, Some(origin), "queued");
    rig.frame(vec![Event::PointerMoved(rig.row(0))]);
    let fresh = "https://i.discogs.com/fake/R-1001-front-150.jpeg";
    rig.until(
        |r| {
            r.app.crates.shown().entries()[0]
                .origin
                .as_ref()
                .unwrap()
                .cover
                == fresh
        },
        "the release's data is looked up again",
    );
    rig.until(|r| has_cover(r, 1001), "the new address works");
    assert_eq!(
        fakes.images.log(),
        ["https://i.discogs.com/expired.jpeg", fresh]
    );
}

#[test]
fn previews_ahead_follow_the_bpm_filter() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-bpm-horizon", &fakes, |_| {});
    let p = rig.app.crates.shown_mut();
    for i in 0..8u64 {
        let origin = Origin {
            release: Some(500 + i),
            clip: Some(format!("BPMclip{i:04}")),
            ..Default::default()
        };
        let source = Some(::dig::preview::fetcher::clip_url(&format!("BPMclip{i:04}")));
        let id = p.add_waiting(
            "Nightcraft",
            format!("Track {i}"),
            source,
            Some(origin),
            "queued",
        );
        let bpm = if i % 2 == 0 { 134 } else { 124 };
        if let Some(e) = p.entries_mut().find(|e| e.id == id) {
            e.bpm = Some(bpm);
        }
    }
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::SetBpmFilter(Some((130, 140))), &ctx);
    rig.until(
        |r| !r.app.dig.as_ref().unwrap().horizon().is_empty(),
        "previews are asked for",
    );
    let horizon = rig.app.dig.as_ref().unwrap().horizon().to_vec();
    for clip in &horizon {
        let i: u64 = clip.trim_start_matches("BPMclip").parse().unwrap();
        assert_eq!(i % 2, 0, "{clip} is hidden by the filter");
    }
}

#[test]
fn options_on_the_main_window_open_the_discogs_dialog() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-options", &fakes, |_| {});
    rig.click_with(pos2(150.0, 28.0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Browser…"), "{:?}", text_list(&out));
    rig.click_text("Discogs…");
    assert!(rig.app.dig.as_ref().unwrap().dialog.is_some());
}

#[test]
fn the_footer_gear_opens_the_discogs_dialog() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-gear", &fakes, |_| {});
    rig.click(footer_button(&rig, "pl_opts"));
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Browser…"), "{:?}", text_list(&out));
    rig.click_text("Discogs…");
    assert!(rig.app.dig.as_ref().unwrap().dialog.is_some());
}

// ---- the wantlist and the collection, from the player ------------------------------------

const EMPTY_COLLECTION: &str =
    r#"{"pagination": {"page": 1, "pages": 1, "items": 0}, "releases": []}"#;
const RELEASE_1004: &str = "https://www.discogs.com/release/1004";

/// Enters a token in Options ▸ Discogs… and saves it, as the user does.
fn connect(rig: &mut Rig) {
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Dig(DigAction::OpenDialog), &ctx);
    rig.until(
        |r| shows(&r.frame(Vec::new()), "Check and save"),
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
        |r| {
            r.app
                .dig
                .as_ref()
                .unwrap()
                .dialog
                .as_ref()
                .is_some_and(|d| d.connected())
        },
        "connected",
    );
    rig.app.dig.as_mut().unwrap().dialog = None;
}

fn posts(fakes: &Fakes) -> usize {
    fakes
        .changes()
        .iter()
        .filter(|(m, _)| *m == Method::Post)
        .count()
}

#[test]
fn adding_to_the_collection_owns_it_at_once_and_takes_it_off_the_wantlist() {
    let fakes = Fakes::new();
    fakes
        .transport
        .route(COLLECTION_PAGE, 200, EMPTY_COLLECTION);
    let mut rig = rig("dig-collect", &fakes, with_token);
    let c = play_release(&mut rig);
    let coll = rig.app.crates.create("Collection: digger").unwrap();
    rig.app.crates.set_collection(coll);
    key(&mut rig, Key::Y);
    let wl = wantlist(&rig).unwrap();
    let put = (Method::Put, "/users/digger/wants/1001".to_owned());
    rig.until(|_| fakes.changes().contains(&put), "on the wantlist");

    rig.click_with(rig.row(0), PointerButton::Secondary);
    rig.click_text("Add to collection");
    rig.until(
        |r| !r.app.dig.as_ref().unwrap().memory.is_wanted(1001),
        "bought: no longer wanted",
    );
    // Within the frame of the answer: OWNED, out of the wantlist crate, in the collection's.
    assert!(shows(&rig.frame(Vec::new()), "OWNED"));
    assert!(clips_of(&rig, wl, 1001).is_empty());
    rig.app.crates.load(coll);
    assert_eq!(clips_of(&rig, coll, 1001), CLIPS);
    assert_eq!(clips(&rig, c).len(), 3, "the dug crate keeps them");
    let delete = (Method::Delete, put.1);
    rig.until(
        |_| fakes.changes().contains(&delete),
        "off the Discogs wantlist",
    );
    assert_eq!(posts(&fakes), 1, "one copy");
    assert!(fakes.changes().contains(&(
        Method::Post,
        "/users/digger/collection/folders/1/releases/1001".to_owned()
    )));
    // Kept on disk, so it stays owned without a sync.
    let cached = ::dig::collection::Collection::load(&rig.dir.join("cache")).unwrap();
    assert!(cached.owned(Some(1001), None).is_some());
    // Owned now: the menu says so.
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "In collection ✓"), "{:?}", text_list(&out));
    assert!(!shows(&out, "Add to collection"));
}

#[test]
fn a_selection_acts_on_its_records_once_each_and_says_what_it_skipped() {
    let fakes = Fakes::new();
    // 1004 is owned.
    fakes.transport.route(
        COLLECTION_PAGE,
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "items": 1},
            "releases": [{"id": 1004, "instance_id": 5,
                          "basic_information": {"id": 1004, "master_id": 0, "year": 1996,
                                                "labels": [{"catno": "LT-014"}]}}]}"#,
    );
    let mut rig = rig("dig-selection", &fakes, with_token);
    let c = play_release(&mut rig);
    rig.app
        .dig_send(url::parse(RELEASE_1004).unwrap(), SendMode::Enqueue, None);
    rig.until(
        |r| !clips_of(r, c, 1004).is_empty() && r.app.dig.as_ref().unwrap().collection.is_some(),
        "both records, and the collection",
    );
    let ids = rig.ids(c);
    rig.app.crates.get_mut(c).unwrap().select_only(&ids, ids[0]);
    // The menu counts records.
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(
        shows(&out, "Add 2 records to wantlist"),
        "{:?}",
        text_list(&out)
    );
    assert!(shows(&out, "Add 2 records to collection"));
    rig.click_text("Add 2 records to wantlist");
    assert!(memory(&rig).is_wanted(1001));
    assert_eq!(
        message(&rig),
        "Added 1 record to your wantlist; 1 skipped (already in your collection)"
    );

    // Three entries of one release: one copy.
    let three: Vec<EntryId> = rig
        .app
        .crates
        .get(c)
        .unwrap()
        .entries()
        .iter()
        .filter(|e| e.origin.as_ref().and_then(|o| o.release) == Some(1001))
        .map(|e| e.id)
        .collect();
    assert_eq!(three.len(), 3);
    rig.app.dig_act(c, DigAction::Collect(three));
    rig.until(
        |r| r.app.dig.as_ref().unwrap().memory.wanted.is_empty() || posts(&fakes) > 0,
        "added",
    );
    rig.until(
        |r| {
            r.app
                .dig_owned(&r.app.crates.get(c).unwrap().entries()[0])
                .is_some()
        },
        "owned",
    );
    assert_eq!(posts(&fakes), 1);
}

#[test]
fn a_wantlist_change_that_keeps_failing_says_so_and_retries_on_request() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-want-failed", &fakes, with_token);
    play_release(&mut rig);
    // The last retry of an add, answered with a server error.
    {
        let m = &mut rig.app.dig.as_mut().unwrap().memory;
        m.wanted.insert(1001);
        m.queue_want(1001, ::dig::memory::WantOp::Add);
        let p = &mut m.wantlist_pending[0];
        p.attempts = 5;
        p.next_at = u64::MAX;
    }
    rig.app.dig_intake_event(::dig::intake::Event::Wantlist {
        release: 1001,
        add: true,
        result: Err(::dig::discogs::client::ApiError::Other("HTTP 503".into())),
    });
    assert!(memory(&rig).want_failure(1001).is_some());
    assert!(
        message(&rig).starts_with("Wantlist change failed for release 1001"),
        "{}",
        message(&rig)
    );
    let out = rig.frame(Vec::new());
    assert!(
        texts(&out).iter().any(|t| t.text.starts_with("1. ⚠ ✓ ")),
        "{:?}",
        text_list(&out)
    );
    let before = fakes.changes().len();
    for _ in 0..20 {
        rig.pump();
    }
    assert_eq!(fakes.changes().len(), before, "nothing more is sent");
    rig.click_with(rig.row(0), PointerButton::Secondary);
    rig.click_text("Retry wantlist");
    rig.until(
        |r| memory(r).wantlist_pending.is_empty(),
        "sent again, at once",
    );
    assert!(
        fakes
            .changes()
            .contains(&(Method::Put, "/users/digger/wants/1001".to_owned()))
    );
}

#[test]
fn a_failed_collection_add_is_retried_only_on_request_and_never_twice() {
    let fakes = Fakes::new();
    fakes
        .transport
        .route(COLLECTION_PAGE, 200, EMPTY_COLLECTION);
    // The add went through on Discogs, but its answer was lost.
    fakes.transport.route(
        "/users/digger/collection/releases/1001",
        200,
        r#"{"releases": [{"id": 1001, "instance_id": 55, "folder_id": 1}]}"#,
    );
    let mut rig = rig("dig-collect-failed", &fakes, with_token);
    let c = play_release(&mut rig);
    rig.app.dig_intake_event(::dig::intake::Event::Collected {
        release: 1001,
        result: Err(::dig::discogs::client::ApiError::Offline),
    });
    assert!(message(&rig).contains("Could not add"), "{}", message(&rig));
    let out = rig.frame(Vec::new());
    assert!(texts(&out).iter().any(|t| t.text.starts_with("1. ⚠ ")));
    for _ in 0..20 {
        rig.pump();
    }
    assert_eq!(posts(&fakes), 0, "not retried by itself");
    rig.click_with(rig.row(0), PointerButton::Secondary);
    rig.click_text("Retry add to collection");
    rig.until(
        |r| {
            r.app
                .dig_owned(&r.app.crates.get(c).unwrap().entries()[0])
                .is_some()
        },
        "found in the collection",
    );
    assert_eq!(posts(&fakes), 0, "no second copy");
}

#[test]
fn connecting_names_the_wantlist_crate_and_pushes_what_was_wanted_here() {
    let fakes = Fakes::new();
    fakes
        .transport
        .route(COLLECTION_PAGE, 200, EMPTY_COLLECTION);
    let mut rig = rig("dig-connect-wantlist", &fakes, |_| {});
    play_release(&mut rig);
    key(&mut rig, Key::Y);
    rig.app.dig.as_mut().unwrap().connect = None;
    let wl = wantlist(&rig).unwrap();
    assert_eq!(rig.app.crates.name(wl), "Wantlist");
    connect(&mut rig);
    rig.until(
        |r| r.app.crates.is_wantlist(wl),
        "the wantlist crate is the account's",
    );
    assert_eq!(rig.app.crates.name(wl), "Wantlist: digger");
    let put = (Method::Put, "/users/digger/wants/1001".to_owned());
    rig.until(|_| fakes.changes().contains(&put), "pushed to Discogs");
    // And it follows the Discogs wantlist (1002, 1004 and 1006 are on it).
    rig.until(
        |r| !clips_of(r, wl, 1004).is_empty(),
        "the Discogs wantlist's records come in",
    );
    let m = memory(&rig);
    assert!([1001, 1002, 1004, 1006].iter().all(|&r| m.is_wanted(r)));
    assert_eq!(m.synced_user.as_deref(), Some("digger"));
    // Under DISCOGS, above the collection.
    assert!(
        rig.app
            .crates
            .list()
            .iter()
            .any(|c| c.wantlist && c.id == wl)
    );
}

#[test]
fn connecting_merges_into_a_wantlist_crate_sent_before() {
    let fakes = Fakes::new();
    fakes
        .transport
        .route(COLLECTION_PAGE, 200, EMPTY_COLLECTION);
    let mut rig = rig("dig-connect-merge", &fakes, |_| {});
    let sent = rig.app.crates.create("Wantlist: digger").unwrap();
    play_release(&mut rig);
    key(&mut rig, Key::Y);
    rig.app.dig.as_mut().unwrap().connect = None;
    connect(&mut rig);
    rig.until(
        |r| r.app.crates.find("Wantlist").is_none(),
        "the local crate goes",
    );
    assert_eq!(wantlist(&rig), Some(sent));
    assert!(rig.app.crates.is_wantlist(sent));
    assert_eq!(clips_of(&rig, sent, 1001), CLIPS);
}

#[test]
fn the_wantlist_crate_follows_records_removed_on_discogs() {
    let fakes = Fakes::new();
    fakes
        .transport
        .route(COLLECTION_PAGE, 200, EMPTY_COLLECTION);
    let mut rig = rig("dig-mirror", &fakes, |store| {
        with_token(store);
        // Wanted last session; since removed on discogs.com (the wantlist is 1002, 1004, 1006).
        let mut m = DigMemory::default();
        m.wanted.insert(1003);
        m.synced_user = Some("digger".into());
        m.save(store.dir()).unwrap();
    });
    rig.until(
        |r| !memory(r).wanted.is_empty() && !memory(r).is_wanted(1003),
        "followed",
    );
    assert!(memory(&rig).is_wanted(1004));
    assert!(
        fakes.changes().is_empty(),
        "nothing pushed: Discogs is followed"
    );
}

#[test]
fn removing_a_whole_record_from_the_wantlist_crate_asks_first() {
    let fakes = Fakes::new();
    fakes
        .transport
        .route(COLLECTION_PAGE, 200, EMPTY_COLLECTION);
    let mut rig = rig("dig-unwant-confirm", &fakes, with_token);
    play_release(&mut rig);
    key(&mut rig, Key::Y);
    let wl = wantlist(&rig).unwrap();
    rig.app.show_crate(wl);
    let of_1001 = |r: &Rig| -> Vec<EntryId> {
        r.app
            .crates
            .get(wl)
            .unwrap()
            .entries()
            .iter()
            .filter(|e| e.origin.as_ref().and_then(|o| o.release) == Some(1001))
            .map(|e| e.id)
            .collect()
    };
    let ctx = rig.ctx.clone();
    // One clip of three: local, no question.
    let ids = of_1001(&rig);
    rig.app
        .crates
        .get_mut(wl)
        .unwrap()
        .select_only(&ids[..1], ids[0]);
    rig.app.apply(Action::RemoveSelected, &ctx);
    assert!(rig.app.dig.as_ref().unwrap().confirm_unwant.is_none());
    assert_eq!(of_1001(&rig).len(), 2);
    assert!(memory(&rig).is_wanted(1001));
    // The rest: asked; Cancel changes nothing.
    let ids = of_1001(&rig);
    rig.app
        .crates
        .get_mut(wl)
        .unwrap()
        .select_only(&ids, ids[0]);
    rig.app.apply(Action::RemoveSelected, &ctx);
    rig.until(
        |r| {
            shows(
                &r.frame(Vec::new()),
                "Remove 1 record from your Discogs wantlist?",
            )
        },
        "the question",
    );
    rig.click_text("Cancel");
    assert_eq!(of_1001(&rig).len(), 2);
    assert!(memory(&rig).is_wanted(1001));
    // Remove: gone here and on Discogs.
    rig.app.apply(Action::RemoveSelected, &ctx);
    rig.until(|r| shows(&r.frame(Vec::new()), "Remove"), "asked again");
    key(&mut rig, Key::Enter);
    assert!(of_1001(&rig).is_empty());
    assert!(!memory(&rig).is_wanted(1001));
    let delete = (Method::Delete, "/users/digger/wants/1001".to_owned());
    rig.until(
        |_| fakes.changes().contains(&delete),
        "off the Discogs wantlist",
    );
}

#[test]
fn records_bought_elsewhere_leave_the_wantlist_after_a_sync() {
    let fakes = Fakes::new();
    // 1004 is on the wantlist, and now in the collection.
    fakes.transport.route(
        COLLECTION_PAGE,
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "items": 1},
            "releases": [{"id": 1004, "instance_id": 9,
                          "basic_information": {"id": 1004, "master_id": 0, "year": 1996,
                                                "labels": [{"catno": "LT-014"}]}}]}"#,
    );
    let mut rig = rig("dig-owned-wants", &fakes, with_token);
    play_release(&mut rig);
    let delete = (Method::Delete, "/users/digger/wants/1004".to_owned());
    rig.until(
        |_| fakes.changes().contains(&delete),
        "off the Discogs wantlist",
    );
    assert!(!memory(&rig).is_wanted(1004));
    let wl = wantlist(&rig).unwrap();
    assert!(clips_of(&rig, wl, 1004).is_empty());
}

#[test]
fn the_keepers_crate_becomes_the_wantlist_crate() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-keepers", &fakes, |_| {});
    let keepers = rig.app.crates.create("Keepers").unwrap();
    rig.app.dig.as_mut().unwrap().settings.wantlist_named = false;
    rig.app.dig.as_mut().unwrap().settings.wantlist = Some(keepers);
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.name(keepers), "Wantlist");
    assert_eq!(wantlist(&rig), Some(keepers));
    let saved = ::dig::config::load_settings(&rig.dir.join("config"));
    assert!(saved.wantlist_named);
    // Once only: a crate named Keepers later is left alone.
    rig.app.crates.rename(keepers, "Keepers").unwrap();
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.name(keepers), "Keepers");

    // When "Wantlist" is taken, Keepers keeps its name and still becomes the wantlist crate.
    let fakes = Fakes::new();
    let mut rig = self::rig("dig-keepers-taken", &fakes, |_| {});
    let keepers = rig.app.crates.create("Keepers").unwrap();
    let taken = rig.app.crates.create("Wantlist").unwrap();
    rig.app.dig.as_mut().unwrap().settings.wantlist_named = false;
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.name(keepers), "Keepers");
    assert_eq!(rig.app.crates.name(taken), "Wantlist");
    assert_eq!(wantlist(&rig), Some(keepers));
}

#[test]
fn owning_another_pressing_blocks_the_wantlist_but_not_the_collection() {
    let fakes = Fakes::new();
    // Release 7777 of master 98765, which release 1001 belongs to, is owned.
    fakes.transport.route(
        COLLECTION_PAGE,
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "items": 1},
            "releases": [{"id": 7777, "instance_id": 3,
                          "basic_information": {"id": 7777, "master_id": 98765, "year": 2019,
                                                "labels": [{"catno": "AF001R"}]}}]}"#,
    );
    let mut rig = rig("dig-owned-another", &fakes, with_token);
    play_release(&mut rig);
    rig.until(
        |r| r.app.dig.as_ref().is_some_and(|d| d.collection.is_some()),
        "the collection is synced",
    );
    key(&mut rig, Key::Y);
    assert!(!memory(&rig).is_wanted(1001));
    assert!(
        message(&rig).ends_with("is already in your collection (AF001R, 2019)"),
        "{}",
        message(&rig)
    );
    rig.click_with(rig.row(0), PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "In collection ✓"), "{:?}", text_list(&out));
    assert!(shows(&out, "Add to collection (own AF001R, 2019)"));
}

// ---- refresh from Discogs ------------------------------------------------------------------

/// Whether crate `c` holds an entry of release `r` (a clip, or its "no clip" entry).
fn holds(rig: &Rig, c: CrateId, r: u64) -> bool {
    rig.app.crates.get(c).is_some_and(|p| {
        p.entries()
            .iter()
            .any(|e| e.origin.as_ref().and_then(|o| o.release) == Some(r))
    })
}

/// Where `text` is drawn.
fn text_at(out: &egui::FullOutput, text: &str) -> Pos2 {
    texts(out)
        .into_iter()
        .find(|t| t.text == text)
        .unwrap_or_else(|| panic!("{text} in {:?}", text_list(out)))
        .rect
        .center()
}

#[test]
fn refresh_collection_brings_the_crate_in_line_with_discogs() {
    let fakes = Fakes::new();
    // On Discogs the collection is now release 1003 (1001 was sold).
    fakes.transport.route(
        COLLECTION_PAGE,
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "items": 1},
            "releases": [{"id": 1003, "instance_id": 9,
                          "basic_information": {"id": 1003, "master_id": 0, "year": 1995,
                                                "labels": [{"catno": "LT-013"}]}}]}"#,
    );
    let mut rig = rig("dig-refresh-collection", &fakes, with_token);
    let coll = rig.app.crates.create("Collection: digger").unwrap();
    rig.app.crates.set_collection(coll);
    rig.app.show_crate(coll);
    rig.app.dig_send(
        url::parse(RELEASE).unwrap(),
        SendMode::Crate("Collection: digger".into()),
        None,
    );
    rig.until(
        |r| clips_of(r, coll, 1001).len() == 3 && r.app.dig.as_ref().unwrap().collection.is_some(),
        "the crate, and a first sync",
    );
    // Wide enough for the sidebar: right-click the crate there.
    rig.app.settings.playlist_width = 700;
    let out = rig.frame(Vec::new());
    rig.click_with(
        text_at(&out, "Collection: digger"),
        PointerButton::Secondary,
    );
    rig.click_text("Refresh collection");
    rig.until(
        |r| message(r).starts_with("Collection:"),
        "the refresh says what changed",
    );
    assert_eq!(message(&rig), "Collection: 1 new, 1 gone");
    assert!(
        clips_of(&rig, coll, 1001).is_empty(),
        "the sold record leaves"
    );
    // It has no clip: it comes in as its "no clip" entry.
    rig.until(|r| holds(r, coll, 1003), "the new one comes in");
    assert!(fakes.changes().is_empty(), "Discogs is never changed by it");

    // Offline: nothing changes, and it says why.
    fakes.transport.set_offline(true);
    rig.app.dig_act(coll, DigAction::RefreshCollection);
    rig.until(|r| message(r).starts_with("Refresh failed"), "fails");
    assert!(message(&rig).contains("offline"), "{}", message(&rig));
    assert!(holds(&rig, coll, 1003));
}

#[test]
fn refresh_wantlist_reads_it_again_and_follows_it() {
    let fakes = Fakes::new();
    fakes
        .transport
        .route(COLLECTION_PAGE, 200, EMPTY_COLLECTION);
    let mut rig = rig("dig-refresh-wantlist", &fakes, with_token);
    play_release(&mut rig);
    rig.until(
        |r| wantlist(r).is_some_and(|wl| !clips_of(r, wl, 1006).is_empty()),
        "the wantlist crate follows Discogs",
    );
    let wl = wantlist(&rig).unwrap();
    // On discogs.com, 1002 and 1006 were removed from the wantlist.
    fakes.transport.route(
        "/users/digger/wants?page=1&per_page=100",
        200,
        r#"{"pagination": {"page": 1, "pages": 1, "items": 1}, "wants": [{"id": 1004}]}"#,
    );
    // The playlist is narrow: the title bar's crate menu has it.
    rig.click(rig.title_bar());
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Refresh wantlist"), "{:?}", text_list(&out));
    rig.click_text("Refresh wantlist");
    rig.until(
        |r| message(r).starts_with("Wantlist"),
        "the refresh says what changed",
    );
    assert_eq!(message(&rig), "Wantlist: 2 gone");
    assert!(clips_of(&rig, wl, 1006).is_empty() && clips_of(&rig, wl, 1002).is_empty());
    assert!(!memory(&rig).is_wanted(1006));
    assert!(memory(&rig).is_wanted(1004));
}

#[test]
fn without_a_token_there_is_nothing_to_refresh() {
    let fakes = Fakes::new();
    let mut rig = rig("dig-refresh-anon", &fakes, |_| {});
    let coll = rig.app.crates.create("Collection: digger").unwrap();
    rig.app.crates.set_collection(coll);
    rig.click(rig.title_bar());
    let out = rig.frame(Vec::new());
    assert!(
        text_list(&out)
            .iter()
            .any(|t| t.ends_with("Collection: digger")),
        "{:?}",
        text_list(&out)
    );
    assert!(!shows(&out, "Refresh collection"));
}
