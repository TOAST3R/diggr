//! Digging in the player: Discogs pages become crates of previews, and Y / N / I decide.
//!
//! The `dig` workers do the work on their own low-priority threads; this module turns their
//! results into crate entries through the crates producer API (waiting entries, `set_audio`,
//! `set_unavailable`), tells them what the user is near (the focus, the preview horizon), and
//! keeps the dig memory. Nothing here waits on a worker, and no worker is started before the
//! window has shown its first frame, so launch never waits for Discogs.
//!
//! The browser bridge starts with the workers: its sends take the same path as a paste, and a
//! snapshot of the crates, what plays and the sends in progress is kept up to date for it, so
//! it answers the extension without waiting for a frame.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio::PlayState;
use dig::bridge::{
    self, BridgeCommand, BridgeHandle, CrateState, Mode, Pairing, SendState, Shared, Snapshot,
};
use dig::browser::{Browser, SystemBrowser, sell_url};
use dig::clock::{Clock, RealClock};
use dig::config::{self as digconf, DigSettings};
use dig::discogs::cache::DiskCache;
use dig::discogs::client::{ApiError, Client, Identity};
use dig::discogs::model::RecordKey;
use dig::discogs::transport::{Transport, UreqTransport};
use dig::discogs::url::{self, Page, Refused};
use dig::intake::{Command, Event, Intake, IntakeHandle, JobRef, Outcome, RecordInfo};
use dig::jobs::{Filters, JobId};
use dig::memory::{DigMemory, Kept, WantOp};
use dig::prepare::PrepareHandle;
use dig::preview::fetcher::{clip_url, preview_path};
use dig::preview::scheduler::{
    self, Finder, PreviewCommand, PreviewEvent, PreviewHandle, system_finder,
};
use dig::preview::store;
use platform::{FileSource, Spawner, TrackRef};

use super::{Action, WinampApp};
use crate::crates::{CrateId, MAX_NAME};
use crate::playlist::{Entry, EntryId, EntryStatus, ForSale, NewEntry, Origin};

/// Pending wantlist changes are sent again this often.
const WANTLIST_RETRY: Duration = Duration::from_secs(60);
/// Failed clips in a row before suggesting that yt-dlp needs an update.
const FAILS_BEFORE_UPDATE_HINT: u32 = 3;
pub const KEEPERS: &str = "Keepers";
const INSTALL_HINT: &str =
    "Previews need yt-dlp: install it (brew install yt-dlp); it is found within 30 s";

/// What the host provides for digging (fakes in tests).
pub struct DigSetup {
    pub transport: Arc<dyn Transport>,
    pub clock: Arc<dyn Clock>,
    pub finder: Finder,
    pub browser: Arc<dyn Browser>,
    /// The app's cache folder: Discogs responses, previews, scores and overviews.
    pub cache_root: Option<PathBuf>,
    pub bridge: BridgeSetup,
}

/// Whether the browser bridge runs, and on which port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeSetup {
    Off,
    /// On the port from `bridge.ron` (47800 by default).
    Configured,
    /// On this port (0 picks a free one, for tests).
    Port(u16),
}

impl DigSetup {
    /// Real HTTP, yt-dlp from the PATH (or the configured path), the default browser.
    pub fn system(cache_root: Option<PathBuf>) -> Self {
        Self {
            transport: Arc::new(UreqTransport::default()),
            clock: Arc::new(RealClock::default()),
            finder: system_finder(),
            browser: Arc::new(SystemBrowser),
            cache_root,
            bridge: BridgeSetup::Configured,
        }
    }
}

/// How a page's tracks go into crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendMode {
    /// To the end of the shown crate.
    Enqueue,
    /// Into a new crate named after the page, shown, playing its first track when ready.
    Play,
    /// Into the named crate, created if needed.
    Crate(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigAction {
    Keep(EntryId),
    Pass(EntryId),
    UndoPass(EntryId),
    OpenForSale(EntryId),
    OpenDialog,
    OpenBrowserDialog,
}

/// What OPT ▸ Browser… asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BridgeAction {
    ForgetBrowsers,
    SetPort(u16),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum YtDlpState {
    Unknown,
    Missing,
    Found(String),
}

struct JobView {
    name: String,
    target: CrateId,
    done: usize,
    total: usize,
}

/// OPT ▸ Discogs…
#[derive(Default)]
pub(super) struct DigDialog {
    token: String,
    /// The last token check: the username, or why it failed.
    check: Option<Result<String, String>>,
    checking: bool,
    ytdlp_path: String,
}

pub(super) struct Dig {
    setup: DigSetup,
    spawner: Arc<dyn Spawner>,
    files: Arc<dyn FileSource>,
    /// The app's config folder (`dig/` goes in it); `None` keeps nothing.
    config: Option<PathBuf>,
    pub(super) settings: DigSettings,
    pub(super) memory: DigMemory,
    token: Option<String>,
    identity: Option<Identity>,
    intake: Option<IntakeHandle>,
    previews: Option<PreviewHandle>,
    prepare: Option<PrepareHandle>,
    ytdlp: YtDlpState,
    hint_shown: bool,
    offline: bool,
    jobs: BTreeMap<JobId, JobView>,
    /// The clips last asked for, in priority order.
    horizon: Vec<String>,
    prepared_list: Vec<TrackRef>,
    /// Per crate being expanded: the focus entry and how many records were still listed.
    focus: HashMap<CrateId, (Option<EntryId>, usize)>,
    /// Crates whose unfinished sends were resumed this session.
    active: HashSet<CrateId>,
    /// Crates a send created: named after the page once Discogs gives its name, and deleted
    /// again if the page turns out not to exist.
    created: HashSet<CrateId>,
    refreshed: HashSet<u64>,
    last_retry: Option<Instant>,
    fail_streak: u32,
    /// A Play send's crate: its first entry is armed as soon as it has one.
    play_when_ready: Option<CrateId>,
    last_playing: Option<(CrateId, EntryId)>,
    pub(super) dialog: Option<DigDialog>,
    /// Shared with the bridge thread once digging has started (unless the bridge is off).
    pub(super) bridge_shared: Option<Arc<Shared>>,
    bridge: Option<BridgeHandle>,
    /// Why the bridge isn't running (the port is taken…).
    bridge_error: Option<String>,
    /// What the bridge was last given to answer with.
    snapshot: Snapshot,
    pub(super) bridge_dialog: Option<BridgeDialog>,
    wake: egui::Context,
}

/// OPT ▸ Browser…
pub(super) struct BridgeDialog {
    port: u16,
    /// A browser was paired while the dialog was open.
    paired: bool,
}

impl Dig {
    /// Reads the dig settings, memory and token (small files); starts nothing.
    pub(super) fn new(
        setup: DigSetup,
        spawner: Arc<dyn Spawner>,
        files: Arc<dyn FileSource>,
        config: Option<PathBuf>,
        wake: egui::Context,
    ) -> Self {
        let settings = config
            .as_deref()
            .map(digconf::load_settings)
            .unwrap_or_default();
        let memory = config.as_deref().map(DigMemory::load).unwrap_or_default();
        let token = config.as_deref().and_then(digconf::load_token);
        Self {
            setup,
            spawner,
            files,
            config,
            settings,
            memory,
            token,
            identity: None,
            intake: None,
            previews: None,
            prepare: None,
            ytdlp: YtDlpState::Unknown,
            hint_shown: false,
            offline: false,
            jobs: BTreeMap::new(),
            horizon: Vec::new(),
            prepared_list: Vec::new(),
            focus: HashMap::new(),
            active: HashSet::new(),
            created: HashSet::new(),
            refreshed: HashSet::new(),
            last_retry: None,
            fail_streak: 0,
            play_when_ready: None,
            last_playing: None,
            dialog: None,
            bridge_shared: None,
            bridge: None,
            bridge_error: None,
            snapshot: Snapshot::default(),
            bridge_dialog: None,
            wake,
        }
    }

    fn started(&self) -> bool {
        self.intake.is_some()
    }

    fn previews_dir(&self) -> PathBuf {
        match &self.setup.cache_root {
            Some(root) => store::dir(root),
            None => std::env::temp_dir().join("winamp_rust").join(store::DIR),
        }
    }

    /// Starts the workers (once the window is interactive).
    fn start(&mut self) {
        let client = Client::new(
            self.setup.transport.clone(),
            self.setup.clock.clone(),
            self.token.clone(),
            DiskCache::new(self.setup.cache_root.as_deref()),
        );
        let wake = self.wake.clone();
        self.intake = IntakeHandle::start(
            &*self.spawner,
            Intake::new(client, self.config.clone()),
            move || wake.request_repaint(),
        )
        .ok();
        let wake = self.wake.clone();
        self.previews = PreviewHandle::start(
            self.spawner.clone(),
            scheduler::Config::new(
                self.previews_dir(),
                self.settings.cache_bytes(),
                self.settings.ytdlp_path.clone(),
            ),
            self.setup.finder.clone(),
            move || wake.request_repaint(),
        )
        .ok();
        if let Some(root) = &self.setup.cache_root {
            let wake = self.wake.clone();
            self.prepare = PrepareHandle::start(
                &*self.spawner,
                self.files.clone(),
                root.clone(),
                move || wake.request_repaint(),
            )
            .ok();
        }
        if self.setup.bridge != BridgeSetup::Off {
            let pairing = Pairing::load(self.config.clone(), self.setup.clock.clone());
            self.bridge_shared = Some(Shared::new(pairing));
            self.start_bridge();
        }
    }

    /// (Re)starts the bridge on its port; a failure is kept for OPT ▸ Browser….
    fn start_bridge(&mut self) {
        let Some(shared) = self.bridge_shared.clone() else {
            return;
        };
        self.bridge = None; // frees the old port first
        let port = match self.setup.bridge {
            BridgeSetup::Port(p) => p,
            _ => shared.pairing().port(),
        };
        let wake = self.wake.clone();
        match BridgeHandle::start(&*self.spawner, shared, port, move || wake.request_repaint()) {
            Ok(h) => {
                self.bridge = Some(h);
                self.bridge_error = None;
            }
            Err(e) => self.bridge_error = Some(e),
        }
    }

    /// The port the bridge listens on, while it runs.
    pub(super) fn bridge_port(&self) -> Option<u16> {
        self.bridge.as_ref().map(BridgeHandle::port)
    }

    fn send(&self, cmd: Command) {
        if let Some(i) = &self.intake {
            i.send(cmd);
        }
    }

    fn preview(&self, cmd: PreviewCommand) {
        if let Some(p) = &self.previews {
            p.send(cmd);
        }
    }

    pub(super) fn save_memory(&mut self) -> Option<String> {
        let c = self.config.as_deref()?;
        self.memory
            .save(c)
            .err()
            .map(|e| format!("Could not save the dig memory: {e}"))
    }

    fn save_settings(&self) -> Option<String> {
        let c = self.config.as_deref()?;
        digconf::save_settings(c, &self.settings)
            .err()
            .map(|e| format!("Could not save the dig settings: {e}"))
    }

    /// The status line: offline, or the sends in progress.
    pub(super) fn line(&self) -> Option<String> {
        if self.offline {
            return Some("Discogs offline: waiting for it to answer".into());
        }
        let parts: Vec<String> = self
            .jobs
            .values()
            .filter(|j| j.total > 0)
            .map(|j| {
                let name = j.name.split_once(": ").map_or(j.name.as_str(), |(_, n)| n);
                format!("{name}: {} of {} releases", j.done, j.total)
            })
            .collect();
        (!parts.is_empty()).then(|| parts.join(" · "))
    }

    /// Kept, passed, and a wantlist change still to be sent, for an entry's row.
    pub(super) fn marks(&self, e: &Entry) -> (bool, bool, bool) {
        let key = key_of(e);
        let kept = self.memory.kept.get(&key);
        let pending = kept
            .and_then(|k| k.release)
            .is_some_and(|r| self.memory.is_want_pending(r));
        (kept.is_some(), self.memory.is_passed(&key), pending)
    }
}

/// Keep and pass work on any entry: a clip by its id, a local file by its path.
fn key_of(e: &Entry) -> String {
    e.origin
        .as_ref()
        .and_then(|o| o.clip.clone())
        .unwrap_or_else(|| e.track.0.clone())
}

fn clip_of(e: &Entry) -> Option<&str> {
    e.source.as_ref()?;
    e.origin.as_ref()?.clip.as_deref()
}

/// The record a "listed" placeholder stands for.
fn listed_key(e: &Entry) -> Option<RecordKey> {
    if e.status != EntryStatus::Waiting("listed".into()) {
        return None;
    }
    let o = e.origin.as_ref().filter(|o| o.clip.is_none())?;
    match (o.release, o.master) {
        (Some(r), _) => Some(RecordKey::Release(r)),
        (None, Some(m)) => Some(RecordKey::Master(m)),
        _ => None,
    }
}

fn for_sale(fs: &Option<dig::discogs::model::ForSale>) -> Option<ForSale> {
    fs.as_ref().map(|f| ForSale {
        count: f.count,
        lowest_cents: f.lowest.map(|p| (p * 100.0).round().max(0.0) as u64),
        currency: f.currency.clone(),
        fetched_at: f.fetched_at,
    })
}

fn origin(page: &str, info: &RecordInfo) -> Origin {
    Origin {
        page: page.to_owned(),
        release: info.release,
        master: info.master,
        label: info.label.clone(),
        catno: info.catno.clone(),
        year: info.year,
        position: String::new(),
        clip: None,
        for_sale: for_sale(&info.for_sale),
    }
}

fn now_secs() -> u64 {
    dig::now_secs()
}

impl WinampApp {
    fn dig_notify(&mut self, text: Option<String>) {
        if let Some(t) = text {
            self.notify(t);
        }
    }

    // ---- sending pages ---------------------------------------------------------------------

    /// Cmd+V / Ctrl+V: a Discogs address goes to the shown crate; other text is ignored.
    pub(super) fn dig_paste(&mut self, text: &str) {
        match url::parse(text) {
            Ok(page) => self.dig_send(page, SendMode::Enqueue, None),
            Err(Refused::Unsupported) => self.notify(url::SUPPORTED),
            Err(Refused::NotDiscogs) => {}
        }
    }

    /// Sends a page's tracks into a crate. `filters` overrides the defaults.
    pub(crate) fn dig_send(&mut self, page: Page, mode: SendMode, filters: Option<Filters>) {
        let Some(d) = &self.dig else { return };
        if !d.started() {
            self.notify("Discogs isn't ready yet");
            return;
        }
        let filters = filters.unwrap_or(Filters {
            vinyl_only: d.settings.vinyl_only,
            skip_passed: d.settings.skip_passed,
        });
        let (target, created, play) = match mode {
            SendMode::Enqueue => (self.crates.shown_id(), false, false),
            SendMode::Play | SendMode::Crate(_) => {
                let name = match &mode {
                    SendMode::Crate(n) => n.clone(),
                    _ => page.provisional_name(),
                };
                let name: String = name.trim().chars().take(MAX_NAME).collect();
                let (id, created) = match self.crates.find(&name) {
                    Some(id) => (id, false),
                    None => match self.crates.create(&name) {
                        Ok(id) => (id, true),
                        Err(e) => return self.notify(e),
                    },
                };
                if !self.crates.load(id) {
                    return self.notify(format!("Crate \"{name}\" can't be read"));
                }
                let play = mode == SendMode::Play;
                if play {
                    self.show_crate(id);
                }
                (id, created, play)
            }
        };
        let Some(d) = &mut self.dig else { return };
        if play {
            d.play_when_ready = Some(target);
        }
        if created {
            d.created.insert(target);
        }
        d.active.insert(target);
        d.send(Command::Send {
            page,
            target,
            filters,
        });
    }

    // ---- each frame --------------------------------------------------------------------

    pub(super) fn dig_update(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if !d.started() {
            // Launch never waits for Discogs: nothing starts before the first frame is shown.
            if !self.first_frame_done {
                return;
            }
            d.start();
        }
        for id in [self.crates.shown_id(), self.crates.playing_id()] {
            if d.active.insert(id) {
                d.send(Command::CrateActive(id));
            }
        }
        let events = d
            .intake
            .as_ref()
            .map(IntakeHandle::poll)
            .unwrap_or_default();
        for e in events {
            self.dig_intake_event(e);
        }
        let Some(d) = &self.dig else { return };
        let events = d
            .previews
            .as_ref()
            .map(PreviewHandle::poll)
            .unwrap_or_default();
        for e in events {
            self.dig_preview_event(e);
        }
        let prepared = self
            .dig
            .as_ref()
            .and_then(|d| d.prepare.as_ref())
            .map(PrepareHandle::poll)
            .unwrap_or_default();
        for p in prepared {
            if let Some(bpm) = p.bpm {
                self.set_track_bpm(&p.track, bpm);
            }
        }
        let commands = self
            .dig
            .as_ref()
            .and_then(|d| d.bridge.as_ref())
            .map(BridgeHandle::poll)
            .unwrap_or_default();
        for c in commands {
            self.dig_bridge_command(c);
        }
        self.dig_play_when_ready();
        self.dig_horizon();
        self.dig_focus();
        self.dig_playing_changed();
        self.dig_retry_wantlist();
        self.dig_bridge_snapshot();
    }

    // ---- the browser bridge ----------------------------------------------------------------

    fn dig_bridge_command(&mut self, c: BridgeCommand) {
        match c {
            BridgeCommand::Send {
                page,
                mode,
                filters,
            } => {
                let mode = match mode {
                    Mode::Play => SendMode::Play,
                    Mode::Enqueue => SendMode::Enqueue,
                    Mode::Crate(name) => SendMode::Crate(name),
                };
                self.dig_send(page, mode, Some(filters));
            }
            BridgeCommand::Paired => {
                if let Some(dialog) = self.dig.as_mut().and_then(|d| d.bridge_dialog.as_mut()) {
                    dialog.paired = true;
                }
                self.notify("A browser was paired");
            }
        }
    }

    /// What the bridge answers with: the crates, what plays, and the sends in progress. Given
    /// to it only when it changes.
    fn dig_bridge_snapshot(&mut self) {
        let Some(d) = &self.dig else { return };
        let Some(shared) = &d.bridge_shared else {
            return;
        };
        let (shown, playing_id) = (self.crates.shown_id(), self.crates.playing_id());
        let active = self.position.state != PlayState::Stopped;
        let crates = self
            .crates
            .list()
            .iter()
            .map(|c| CrateState {
                name: c.name.clone(),
                shown: c.id == shown,
                playing: active && c.id == playing_id,
            })
            .collect();
        let playing = active
            .then(|| {
                let p = self.crates.playing();
                let e = p.get(p.current()?)?;
                let title = if e.title.is_empty() {
                    e.display_name()
                } else {
                    e.title.clone()
                };
                Some(bridge::Playing {
                    artist: e.artist.clone(),
                    title,
                    crate_name: self.crates.name(playing_id).to_owned(),
                })
            })
            .flatten();
        let sends = d
            .jobs
            .values()
            .map(|j| SendState {
                page: j.name.clone(),
                done: j.done,
                total: j.total,
            })
            .collect();
        let snap = Snapshot {
            crates,
            playing,
            sends,
        };
        if snap != d.snapshot {
            shared.snapshot.store(Arc::new(snap.clone()));
            if let Some(d) = &mut self.dig {
                d.snapshot = snap;
            }
        }
    }

    fn dig_intake_event(&mut self, e: Event) {
        match e {
            Event::Started(j) => {
                let Some(d) = &mut self.dig else { return };
                d.jobs.insert(
                    j.id,
                    JobView {
                        name: j.name.clone(),
                        target: j.target,
                        done: 0,
                        total: 0,
                    },
                );
                let to = self.crates.name(j.target).to_owned();
                self.notify(if to == j.name {
                    format!("Digging {to}")
                } else {
                    format!("{} → {to}", j.name)
                });
            }
            Event::Named(j) => {
                let Some(d) = &mut self.dig else { return };
                if let Some(v) = d.jobs.get_mut(&j.id) {
                    v.name = j.name.clone();
                }
                // A crate named from the address takes the page's real name.
                if d.created.remove(&j.target) {
                    let name: String = j.name.chars().take(MAX_NAME).collect();
                    let _ = self.crates.rename(j.target, &name);
                }
            }
            Event::Listed(j, items) => {
                if !self.crates.load(j.target) {
                    return;
                }
                let Some(p) = self.crates.get_mut(j.target) else {
                    return;
                };
                for l in items {
                    let info = RecordInfo {
                        key: l.key,
                        release: match l.key {
                            RecordKey::Release(r) => Some(r),
                            RecordKey::Master(_) => None,
                        },
                        master: match l.key {
                            RecordKey::Master(m) => Some(m),
                            RecordKey::Release(_) => None,
                        },
                        artist: l.artist.clone(),
                        title: l.title.clone(),
                        label: l.label.clone(),
                        catno: l.catno.clone(),
                        year: l.year,
                        for_sale: None,
                    };
                    let title = if l.title.is_empty() {
                        match l.key {
                            RecordKey::Release(r) => format!("Release {r}"),
                            RecordKey::Master(m) => format!("Master {m}"),
                        }
                    } else {
                        l.title.clone()
                    };
                    p.add_waiting(
                        l.artist,
                        title,
                        None,
                        Some(origin(&j.page, &info)),
                        "listed",
                    );
                }
                self.mark_crate(j.target);
            }
            Event::Record(j, info, outcome) => self.dig_record(&j, &info, outcome),
            Event::Progress(j, done, total) => {
                if let Some(v) = self.dig.as_mut().and_then(|d| d.jobs.get_mut(&j.id)) {
                    (v.done, v.total) = (done, total);
                }
            }
            Event::Finished(j) => {
                if let Some(d) = &mut self.dig {
                    d.jobs.remove(&j.id);
                }
            }
            Event::Failed(j, err) => {
                let Some(d) = &mut self.dig else { return };
                d.jobs.remove(&j.id);
                if d.play_when_ready == Some(j.target) {
                    d.play_when_ready = None;
                }
                // A crate made for a page that turned out not to exist goes again.
                if d.created.remove(&j.target) && self.crates.entry_count(j.target) == 0 {
                    self.delete_crate(j.target);
                }
                self.notify(format!("{}: {}", j.name, err.message()));
            }
            Event::Offline(off) => {
                if let Some(d) = &mut self.dig {
                    d.offline = off;
                }
            }
            Event::ForSale(release, fs) => {
                let fs = for_sale(&Some(fs));
                for c in self.crates.loaded_ids() {
                    let Some(p) = self.crates.get_mut(c) else {
                        continue;
                    };
                    let mut changed = false;
                    for e in p.entries_mut() {
                        if let Some(o) = e.origin.as_mut().filter(|o| o.release == Some(release)) {
                            o.for_sale = fs.clone();
                            changed = true;
                        }
                    }
                    if changed {
                        self.crates.touch(c);
                    }
                }
            }
            Event::Identity(result) => match result {
                Ok(id) => {
                    if let Some(d) = &mut self.dig {
                        d.identity = Some(id);
                    }
                }
                Err(e) => self.notify(e.message()),
            },
            Event::TokenChecked(token, result) => self.dig_token_checked(token, result),
            Event::Wantlist {
                release,
                add,
                result,
            } => self.dig_wantlist_result(release, add, result),
        }
    }

    /// A listed record's details: its placeholder becomes its clips, "no clip", or goes.
    fn dig_record(&mut self, j: &JobRef, info: &RecordInfo, outcome: Outcome) {
        let Some(p) = self.crates.get_mut(j.target) else {
            return;
        };
        let Some(placeholder) = p
            .entries()
            .iter()
            .find(|e| listed_key(e) == Some(info.key))
            .map(|e| e.id)
        else {
            return; // removed by the user meanwhile
        };
        let Some(d) = &self.dig else { return };
        let dir = d.previews_dir();
        let base = origin(&j.page, info);
        let new: Vec<NewEntry> = match outcome {
            Outcome::Excluded => Vec::new(),
            Outcome::Unavailable(reason) => {
                if let Some(e) = p.entries_mut().find(|e| e.id == placeholder) {
                    e.origin = Some(base);
                    if !info.artist.is_empty() {
                        e.artist = info.artist.clone();
                    }
                    if !info.title.is_empty() {
                        e.title = info.title.clone();
                    }
                }
                p.set_unavailable(placeholder, reason);
                self.mark_crate(j.target);
                return;
            }
            Outcome::Clips(entries) => {
                let mut have: HashSet<String> = p
                    .entries()
                    .iter()
                    .filter_map(|e| e.origin.as_ref()?.clip.clone())
                    .collect();
                entries
                    .into_iter()
                    .filter(|c| !(j.filters.skip_passed && d.memory.is_passed(&c.clip)))
                    .filter(|c| have.insert(c.clip.clone()))
                    .map(|c| NewEntry {
                        source: Some(clip_url(&c.clip)),
                        origin: Some(Origin {
                            position: c.position,
                            clip: Some(c.clip),
                            ..base.clone()
                        }),
                        artist: c.artist,
                        title: c.title,
                        duration: c.duration,
                        status: "queued".into(),
                    })
                    .collect()
            }
        };
        let ids = p.replace(placeholder, new);
        // Previews already in the cache play at once.
        let ready: Vec<(EntryId, TrackRef)> = ids
            .iter()
            .filter_map(|&id| {
                let clip = p.get(id)?.origin.as_ref()?.clip.clone()?;
                let path = preview_path(&dir, &clip);
                std::fs::metadata(&path)
                    .is_ok_and(|m| m.len() > 0)
                    .then(|| (id, TrackRef::new(path.to_string_lossy())))
            })
            .collect();
        if self.armed == Some((j.target, placeholder)) {
            self.armed = ids.first().map(|&id| (j.target, id));
        }
        self.mark_crate(j.target);
        for (id, track) in ready {
            self.set_audio(j.target, id, track);
        }
    }

    fn dig_preview_event(&mut self, e: PreviewEvent) {
        match e {
            PreviewEvent::Progress(clip, pct) => {
                self.dig_each_clip(&clip, |p, id| {
                    p.set_status(id, format!("downloading {pct}%"));
                });
            }
            PreviewEvent::Done(clip, path) => {
                if let Some(d) = &mut self.dig {
                    d.fail_streak = 0;
                }
                let track = TrackRef::new(path.to_string_lossy());
                let mut hits = Vec::new();
                for c in self.crates.loaded_ids() {
                    let Some(p) = self.crates.get(c) else {
                        continue;
                    };
                    hits.extend(
                        p.entries()
                            .iter()
                            .filter(|e| {
                                clip_of(e) == Some(clip.as_str())
                                    && matches!(e.status, EntryStatus::Waiting(_))
                            })
                            .map(|e| (c, e.id)),
                    );
                }
                for (c, id) in hits {
                    self.set_audio(c, id, track.clone());
                }
            }
            PreviewEvent::Failed(clip, reason) => {
                self.dig_each_clip(&clip, |p, id| p.set_unavailable(id, reason.clone()));
                let Some(d) = &mut self.dig else { return };
                d.fail_streak += 1;
                if d.fail_streak == FAILS_BEFORE_UPDATE_HINT {
                    self.notify("Clips keep failing: yt-dlp may need an update (yt-dlp -U)");
                }
            }
            PreviewEvent::Evicted(clip) => {
                let playing = self.crates.playing().current();
                self.dig_each_clip(&clip, |p, id| {
                    if Some(id) != playing && p.get(id).is_some_and(|e| e.status.is_playable()) {
                        p.set_waiting(id, "queued");
                    }
                });
            }
            PreviewEvent::NeedsYtDlp => {
                let Some(d) = &mut self.dig else { return };
                d.ytdlp = YtDlpState::Missing;
                let first = !d.hint_shown;
                d.hint_shown = true;
                self.dig_mark_waiting("queued", "needs yt-dlp");
                if first {
                    self.notify(INSTALL_HINT);
                }
            }
            PreviewEvent::YtDlp(version) => {
                if let Some(d) = &mut self.dig {
                    d.ytdlp = YtDlpState::Found(version);
                }
                self.dig_mark_waiting("needs yt-dlp", "queued");
            }
        }
    }

    /// Runs `f` on every entry of a loaded crate whose clip is `clip`, marking changed crates.
    fn dig_each_clip(
        &mut self,
        clip: &str,
        mut f: impl FnMut(&mut crate::playlist::Playlist, EntryId),
    ) {
        for c in self.crates.loaded_ids() {
            let Some(p) = self.crates.get_mut(c) else {
                continue;
            };
            let ids: Vec<EntryId> = p
                .entries()
                .iter()
                .filter(|e| clip_of(e) == Some(clip))
                .map(|e| e.id)
                .collect();
            if ids.is_empty() {
                continue;
            }
            for id in ids {
                f(p, id);
            }
            self.mark_crate(c);
        }
    }

    /// Horizon entries waiting with note `from` get note `to` ("queued" ↔ "needs yt-dlp").
    fn dig_mark_waiting(&mut self, from: &str, to: &str) {
        let Some(d) = &self.dig else { return };
        let clips = d.horizon.clone();
        for clip in clips {
            self.dig_each_clip(&clip, |p, id| {
                if p.get(id).map(|e| &e.status) == Some(&EntryStatus::Waiting(from.into())) {
                    p.set_status(id, to);
                }
            });
        }
    }

    fn dig_play_when_ready(&mut self) {
        let Some(c) = self.dig.as_ref().and_then(|d| d.play_when_ready) else {
            return;
        };
        let Some(first) = self
            .crates
            .get(c)
            .and_then(|p| p.play_order(false, None, 0).first().copied())
        else {
            return;
        };
        if let Some(d) = &mut self.dig {
            d.play_when_ready = None;
        }
        // Plays now if its preview is there, or is armed to start once it is.
        self.play_entry(c, first);
    }

    /// The previews to have ready: the armed entry, then from the playing entry (or the shown
    /// crate's current one when stopped) the next few of the play order. Also what to prepare.
    fn dig_horizon(&mut self) {
        let playing = self.position.state != PlayState::Stopped;
        let cid = if playing {
            self.crates.playing_id()
        } else {
            self.crates.shown_id()
        };
        let Some(p) = self.crates.get(cid) else {
            return;
        };
        let start = p.current();
        let first = if self.settings.shuffle && cid == self.queue_crate {
            self.queue.first().copied()
        } else {
            start
        };
        let order = p.play_order(self.settings.shuffle, first, self.shuffle_seed);
        let clips: Vec<Option<String>> = order
            .iter()
            .map(|&id| p.get(id).and_then(clip_of).map(str::to_owned))
            .collect();
        let at = start
            .and_then(|s| order.iter().position(|&id| id == s))
            .unwrap_or(0);
        let armed = self
            .armed
            .and_then(|(c, id)| self.crates.get(c)?.get(id))
            .and_then(clip_of)
            .map(str::to_owned);
        let wanted = scheduler::horizon(&clips, at, armed.as_deref());
        // Downloaded previews near the playhead, to prepare (not the playing one: the
        // analyzer is on it already).
        let playing_id = if playing { start } else { None };
        let prepare: Vec<TrackRef> = order
            .iter()
            .skip(at)
            .take(scheduler::AHEAD + 1)
            .filter(|&&id| Some(id) != playing_id)
            .filter_map(|&id| p.get(id))
            .filter(|e| e.status.is_playable() && clip_of(e).is_some())
            .map(|e| e.track.clone())
            .collect();
        let gate = !playing
            || self.analysis.is_none()
            || self
                .score
                .as_ref()
                .is_some_and(|s| s.complete || s.downbeats.len() >= 32);
        let Some(d) = &mut self.dig else { return };
        if let Some(pr) = &d.prepare {
            pr.set_gate(gate);
            if prepare != d.prepared_list {
                pr.prepare(prepare.clone());
                d.prepared_list = prepare;
            }
        }
        if wanted != d.horizon {
            d.preview(PreviewCommand::Want {
                wanted: wanted.clone(),
                protected: wanted.clone(),
            });
            d.horizon = wanted;
            if d.ytdlp == YtDlpState::Missing {
                self.dig_mark_waiting("queued", "needs yt-dlp");
            }
        }
    }

    /// Tells the intake where the user is in each crate being expanded.
    fn dig_focus(&mut self) {
        let Some(d) = &self.dig else { return };
        let targets: HashSet<CrateId> = d.jobs.values().map(|j| j.target).collect();
        let mut sends = Vec::new();
        for c in targets {
            let Some(p) = self.crates.get(c) else {
                continue;
            };
            let focus =
                if c == self.crates.playing_id() && self.position.state != PlayState::Stopped {
                    p.current()
                } else {
                    p.selected_ids().first().copied().or(p.current())
                };
            let listed = p
                .entries()
                .iter()
                .filter(|e| listed_key(e).is_some())
                .count();
            if d.focus.get(&c) == Some(&(focus, listed)) {
                continue;
            }
            let at = focus.and_then(|f| p.index_of(f)).unwrap_or(0);
            let n = p.len();
            let order: Vec<RecordKey> = (0..n)
                .filter_map(|i| listed_key(&p.entries()[(at + i) % n]))
                .collect();
            sends.push((c, focus, listed, order));
        }
        let Some(d) = &mut self.dig else { return };
        for (c, focus, listed, order) in sends {
            d.focus.insert(c, (focus, listed));
            if focus.is_some() {
                d.send(Command::Focus { target: c, order });
            }
        }
    }

    /// A new track started: mark its preview as just played, and refresh its marketplace
    /// numbers once they are a day old.
    fn dig_playing_changed(&mut self) {
        let now = (self.position.state != PlayState::Stopped)
            .then(|| {
                self.crates
                    .playing()
                    .current()
                    .map(|id| (self.crates.playing_id(), id))
            })
            .flatten();
        let Some(d) = &mut self.dig else { return };
        if now == d.last_playing {
            return;
        }
        d.last_playing = now;
        let Some(e) = now.and_then(|(c, id)| self.crates.get(c)?.get(id)) else {
            return;
        };
        if clip_of(e).is_some() && e.status.is_playable() {
            d.preview(PreviewCommand::Played(PathBuf::from(&e.track.0)));
        }
        if let Some(o) = &e.origin
            && let (Some(r), Some(fs)) = (o.release, &o.for_sale)
            && now_secs().saturating_sub(fs.fetched_at) >= dig::discogs::cache::FRESH_SECS
            && d.refreshed.insert(r)
        {
            d.send(Command::Refresh(r));
        }
    }

    fn dig_retry_wantlist(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none()
            || d.memory.wantlist_pending.is_empty()
            || d.last_retry.is_some_and(|t| t.elapsed() < WANTLIST_RETRY)
        {
            return;
        }
        d.last_retry = Some(Instant::now());
        for (release, op) in d.memory.wantlist_pending.clone() {
            d.send(match op {
                WantOp::Add => Command::Keep(release),
                WantOp::Remove => Command::Unkeep(release),
            });
        }
    }

    // ---- verdicts ----------------------------------------------------------------------

    /// Y, N and I act on the playing or paused entry, and do nothing while stopped.
    pub(super) fn dig_key(&mut self, key: egui::Key) -> bool {
        let action: fn(EntryId) -> DigAction = match key {
            egui::Key::Y => DigAction::Keep,
            egui::Key::N => DigAction::Pass,
            egui::Key::I => DigAction::OpenForSale,
            _ => return false,
        };
        if self.dig.is_none() {
            return false;
        }
        if self.position.state == PlayState::Stopped {
            return true;
        }
        let c = self.crates.playing_id();
        if let Some(id) = self.crates.playing().current() {
            self.dig_act(c, action(id));
        }
        true
    }

    pub(super) fn dig_act(&mut self, c: CrateId, a: DigAction) {
        match a {
            DigAction::Keep(id) => self.dig_keep(c, id),
            DigAction::Pass(id) => self.dig_pass(c, id),
            DigAction::UndoPass(id) => {
                let Some(key) = self.crates.get(c).and_then(|p| p.get(id)).map(key_of) else {
                    return;
                };
                let Some(d) = &mut self.dig else { return };
                if d.memory.passed.remove(&key).is_some() {
                    let e = d.save_memory();
                    self.dig_notify(e);
                }
            }
            DigAction::OpenForSale(id) => {
                let Some(e) = self.crates.get(c).and_then(|p| p.get(id)) else {
                    return;
                };
                let release = e.origin.as_ref().and_then(|o| o.release);
                let name = e.display_name();
                let Some(d) = &self.dig else { return };
                match release {
                    Some(r) => {
                        if let Err(err) = d.setup.browser.open(&sell_url(r)) {
                            self.notify(format!("Could not open the browser: {err}"));
                        }
                    }
                    None => self.notify(format!("{name} isn't from Discogs")),
                }
            }
            DigAction::OpenDialog => self.dig_open_dialog(),
            DigAction::OpenBrowserDialog => self.dig_open_bridge_dialog(),
        }
    }

    /// The Keepers crate: the one in the settings, else one named "Keepers", else a new one.
    fn keepers(&mut self) -> Result<CrateId, String> {
        let Some(d) = &self.dig else {
            return Err("Digging is off".into());
        };
        if let Some(id) = d
            .settings
            .keepers
            .filter(|&id| self.crates.info(id).is_some())
        {
            return Ok(id);
        }
        let id = match self.crates.find(KEEPERS) {
            Some(id) => id,
            None => self.crates.create(KEEPERS)?,
        };
        if let Some(d) = &mut self.dig {
            d.settings.keepers = Some(id);
            let e = d.save_settings();
            self.dig_notify(e);
        }
        Ok(id)
    }

    fn dig_keep(&mut self, c: CrateId, id: EntryId) {
        let Some(e) = self.crates.get(c).and_then(|p| p.get(id)).cloned() else {
            return;
        };
        let key = key_of(&e);
        if key.is_empty() {
            return;
        }
        if self.dig.as_ref().is_some_and(|d| d.memory.is_kept(&key)) {
            return self.dig_unkeep(&key, &e);
        }
        let keepers = match self.keepers() {
            Ok(k) => k,
            Err(err) => return self.notify(err),
        };
        if c != keepers {
            if let Err(err) = self.crates.send(c, &[id], keepers) {
                return self.notify(err);
            }
            self.mark_crate(keepers);
        }
        let release = e.origin.as_ref().and_then(|o| o.release);
        let Some(d) = &mut self.dig else { return };
        d.memory.kept.insert(
            key,
            Kept {
                release,
                added_to_wantlist: false,
                at: now_secs(),
            },
        );
        let mut text = format!("Kept {}", e.display_name());
        if let Some(r) = release {
            if d.token.is_some() {
                if d.memory.is_want_pending(r) {
                    d.memory.queue_want(r, WantOp::Add);
                } else {
                    d.send(Command::Keep(r));
                }
            } else {
                text += " (a Discogs token is needed for the wantlist: OPT ▸ Discogs…)";
            }
        }
        let err = d.save_memory();
        self.notify(text);
        self.dig_notify(err);
    }

    fn dig_unkeep(&mut self, key: &str, e: &Entry) {
        let keepers = self.dig.as_ref().and_then(|d| d.settings.keepers);
        if let Some(k) = keepers
            && self.crates.load(k)
            && let Some(p) = self.crates.get_mut(k)
        {
            let ids: Vec<EntryId> = p
                .entries()
                .iter()
                .filter(|x| key_of(x) == key)
                .map(|x| x.id)
                .collect();
            for id in ids {
                p.remove(id);
            }
            self.mark_crate(k);
        }
        let Some(d) = &mut self.dig else { return };
        let Some(kept) = d.memory.kept.remove(key) else {
            return;
        };
        if let Some(r) = kept.release
            && !d.memory.release_kept_elsewhere(r, key)
        {
            if d.memory.wantlist_pending.contains(&(r, WantOp::Add)) {
                d.memory.queue_want(r, WantOp::Remove); // cancels the add
            } else if kept.added_to_wantlist {
                d.send(Command::Unkeep(r));
            }
        }
        let err = d.save_memory();
        self.notify(format!("No longer kept: {}", e.display_name()));
        self.dig_notify(err);
    }

    fn dig_wantlist_result(&mut self, release: u64, add: bool, result: Result<bool, ApiError>) {
        let Some(d) = &mut self.dig else { return };
        let op = if add { WantOp::Add } else { WantOp::Remove };
        let message = match result {
            Ok(changed) => {
                d.memory.want_done(release, op);
                if add && changed {
                    for k in d.memory.kept.values_mut() {
                        if k.release == Some(release) {
                            k.added_to_wantlist = true;
                        }
                    }
                }
                None
            }
            Err(e @ (ApiError::TokenNeeded | ApiError::TokenRejected)) => {
                d.memory.want_done(release, op);
                Some(e.message())
            }
            Err(ApiError::Offline | ApiError::Other(_)) => {
                d.memory.queue_want(release, op);
                None
            }
            Err(e) => {
                d.memory.want_done(release, op);
                Some(e.message())
            }
        };
        let err = d.save_memory();
        self.dig_notify(message);
        self.dig_notify(err);
    }

    fn dig_pass(&mut self, c: CrateId, id: EntryId) {
        let Some(e) = self.crates.get(c).and_then(|p| p.get(id)).cloned() else {
            return;
        };
        let key = key_of(&e);
        let Some(d) = &mut self.dig else { return };
        if d.memory.is_kept(&key) {
            return self.notify(format!("{} is kept (Y undoes the keep)", e.display_name()));
        }
        d.memory.passed.insert(key, now_secs());
        let err = d.save_memory();
        self.dig_notify(err);
        let playing = self.position.state != PlayState::Stopped
            && c == self.crates.playing_id()
            && self.crates.playing().current() == Some(id);
        if playing {
            self.armed = None;
            self.with_engine(|e| e.next());
        }
    }

    // ---- the entry menu ------------------------------------------------------------------

    pub(super) fn dig_entry_menu(&self, ui: &mut egui::Ui, e: &Entry, actions: &mut Vec<Action>) {
        let Some(d) = &self.dig else { return };
        let (kept, passed, _) = d.marks(e);
        ui.separator();
        let items = [
            (
                if kept { "Undo keep" } else { "Keep (Y)" },
                DigAction::Keep(e.id),
            ),
            if passed {
                ("Undo pass", DigAction::UndoPass(e.id))
            } else {
                ("Pass (N)", DigAction::Pass(e.id))
            },
            ("Open for-sale page (I)", DigAction::OpenForSale(e.id)),
        ];
        for (label, a) in items {
            if ui.button(label).clicked() {
                actions.push(Action::Dig(a));
                ui.close();
            }
        }
    }

    // ---- OPT ▸ Discogs… ------------------------------------------------------------------

    fn dig_open_dialog(&mut self) {
        let Some(d) = &mut self.dig else { return };
        d.dialog = Some(DigDialog {
            ytdlp_path: d.settings.ytdlp_path.clone().unwrap_or_default(),
            ..Default::default()
        });
        d.preview(PreviewCommand::CheckProgram);
    }

    fn dig_token_checked(&mut self, token: String, result: Result<Identity, ApiError>) {
        let Some(d) = &mut self.dig else { return };
        let outcome = match result {
            Ok(id) => {
                let saved = d
                    .config
                    .as_deref()
                    .map(|c| digconf::save_token(c, Some(&token)));
                if let Some(Err(e)) = saved {
                    Err(format!("Could not save the token: {e}"))
                } else {
                    d.token = Some(token);
                    let name = id.username.clone();
                    d.identity = Some(id);
                    d.last_retry = None;
                    Ok(name)
                }
            }
            Err(ApiError::TokenRejected) => {
                Err("Discogs rejected the token; it wasn't saved".into())
            }
            Err(e) => Err(e.message()),
        };
        if let Some(dialog) = &mut d.dialog {
            dialog.checking = false;
            if outcome.is_ok() {
                dialog.token.clear();
            }
            dialog.check = Some(outcome);
        }
    }

    pub(super) fn dig_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.dig else { return };
        let Some(dialog) = &mut d.dialog else { return };
        let mut open = true;
        let mut commands: Vec<Command> = Vec::new();
        let mut preview_cmds: Vec<PreviewCommand> = Vec::new();
        let mut remove_token = false;
        let mut settings_changed = false;
        egui::Window::new("Discogs")
            .id(egui::Id::new("dig-dialog"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                ui.strong("Account");
                match (&d.token, &d.identity) {
                    (Some(t), Some(id)) => {
                        ui.label(format!(
                            "Connected as {} ({})",
                            id.username,
                            digconf::masked(t)
                        ));
                    }
                    (Some(t), None) => {
                        ui.label(format!("Token {}", digconf::masked(t)));
                    }
                    (None, _) => {
                        ui.label("No token: pages still work, at Discogs' lower rate limit");
                    }
                }
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.token)
                            .password(true)
                            .hint_text("Personal access token")
                            .desired_width(200.0),
                    );
                    let can = !dialog.token.trim().is_empty() && !dialog.checking;
                    if ui
                        .add_enabled(can, egui::Button::new("Check and save"))
                        .clicked()
                    {
                        dialog.checking = true;
                        dialog.check = None;
                        commands.push(Command::CheckToken(dialog.token.trim().to_owned()));
                    }
                });
                if d.token.is_some() && ui.button("Remove token").clicked() {
                    remove_token = true;
                }
                if dialog.checking {
                    ui.label("Checking…");
                }
                match &dialog.check {
                    Some(Ok(name)) => {
                        ui.label(format!("Connected as {name}"));
                    }
                    Some(Err(e)) => {
                        ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e);
                    }
                    None => {}
                }
                ui.label("Create one at discogs.com ▸ Settings ▸ Developers.");

                ui.separator();
                ui.strong("Previews");
                match &d.ytdlp {
                    YtDlpState::Found(v) => ui.label(format!("yt-dlp {v}")),
                    YtDlpState::Missing => {
                        ui.label("yt-dlp not found: install it with brew install yt-dlp")
                    }
                    YtDlpState::Unknown => ui.label("Looking for yt-dlp…"),
                };
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.ytdlp_path)
                            .hint_text("yt-dlp path (empty: on the PATH)")
                            .desired_width(200.0),
                    );
                    if ui.button("Use").clicked() {
                        let p = dialog.ytdlp_path.trim();
                        d.settings.ytdlp_path = (!p.is_empty()).then(|| p.to_owned());
                        preview_cmds
                            .push(PreviewCommand::SetProgram(d.settings.ytdlp_path.clone()));
                        settings_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Preview cache");
                    let r = ui.add(
                        egui::DragValue::new(&mut d.settings.cache_gb)
                            .range(0.5..=100.0)
                            .speed(0.1)
                            .suffix(" GB"),
                    );
                    if r.changed() {
                        preview_cmds.push(PreviewCommand::SetLimit(d.settings.cache_bytes()));
                        settings_changed = true;
                    }
                });
                ui.label("Previews are for listening only; they are never exported.");

                ui.separator();
                ui.strong("Every send");
                settings_changed |= ui
                    .checkbox(&mut d.settings.vinyl_only, "Vinyl only")
                    .changed();
                settings_changed |= ui
                    .checkbox(&mut d.settings.skip_passed, "Skip what I've passed")
                    .changed();
            });
        if !open {
            d.dialog = None;
        }
        if remove_token {
            if let Some(c) = d.config.as_deref() {
                let _ = digconf::save_token(c, None);
            }
            d.token = None;
            d.identity = None;
            commands.push(Command::SetToken(None));
        }
        for c in commands {
            d.send(c);
        }
        for c in preview_cmds {
            d.preview(c);
        }
        if settings_changed {
            let e = d.save_settings();
            self.dig_notify(e);
        }
    }

    // ---- OPT ▸ Browser… ------------------------------------------------------------------

    fn dig_open_bridge_dialog(&mut self) {
        let Some(d) = &mut self.dig else { return };
        let port = d
            .bridge_port()
            .or_else(|| d.bridge_shared.as_ref().map(|s| s.pairing().port()))
            .unwrap_or(bridge::DEFAULT_PORT);
        d.bridge_dialog = Some(BridgeDialog {
            port,
            paired: false,
        });
    }

    pub(crate) fn dig_bridge_act(&mut self, a: BridgeAction) {
        let Some(d) = &mut self.dig else { return };
        match a {
            BridgeAction::ForgetBrowsers => {
                let Some(shared) = &d.bridge_shared else {
                    return;
                };
                let result = shared.pairing().forget_all();
                if let Some(dialog) = &mut d.bridge_dialog {
                    dialog.paired = false;
                }
                self.notify(match result {
                    Ok(()) => "Browsers forgotten: each one needs pairing again".into(),
                    Err(e) => e,
                });
            }
            BridgeAction::SetPort(port) => {
                let Some(shared) = &d.bridge_shared else {
                    return;
                };
                let saved = shared.pairing().set_port(port);
                d.setup.bridge = BridgeSetup::Configured;
                d.start_bridge();
                if let Err(e) = saved {
                    self.notify(e);
                }
            }
            BridgeAction::Close => {
                // No code works while the dialog is closed.
                if let Some(shared) = &d.bridge_shared {
                    shared.pairing().clear_code();
                }
                d.bridge_dialog = None;
            }
        }
    }

    pub(super) fn dig_bridge_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.dig else { return };
        let Some(dialog) = &mut d.bridge_dialog else {
            return;
        };
        let shared = d.bridge_shared.clone();
        let running = d.bridge.as_ref().map(BridgeHandle::port);
        let error = d.bridge_error.clone();
        let mut open = true;
        let mut actions = Vec::new();
        egui::Window::new("Browser")
            .id(egui::Id::new("bridge-dialog"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                let Some(shared) = &shared else {
                    ui.label("The browser bridge starts once the player is ready.");
                    return;
                };
                match (running, &error) {
                    (_, Some(e)) => {
                        ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e);
                        ui.label("The player works without it; choose another port below.");
                    }
                    (Some(p), None) => {
                        ui.label(format!(
                            "Listening on 127.0.0.1:{p}, for this computer only"
                        ));
                    }
                    (None, None) => {
                        ui.label("Starting…");
                    }
                }

                ui.separator();
                ui.strong("Pair a browser");
                let mut pairing = shared.pairing();
                if let Some(wait) = pairing.locked_for() {
                    ui.label(format!(
                        "Too many wrong codes: pairing resumes in {} s",
                        wait.as_secs_f32().ceil()
                    ));
                } else {
                    match pairing.ensure_code() {
                        Ok((code, left)) => {
                            ui.label(
                                egui::RichText::new(format!("{} {}", &code[..3], &code[3..]))
                                    .monospace()
                                    .size(28.0),
                            );
                            let secs = left.as_secs_f32().ceil() as u64;
                            ui.label(format!("Valid for {}:{:02}", secs / 60, secs % 60));
                        }
                        Err(e) => {
                            ui.colored_label(egui::Color32::from_rgb(230, 90, 90), e);
                        }
                    }
                }
                ui.label("Enter this code in the extension's options.");
                if dialog.paired {
                    ui.label("A browser was paired.");
                }

                ui.separator();
                let n = pairing.paired();
                drop(pairing);
                ui.label(match n {
                    0 => "No paired browsers".to_owned(),
                    1 => "1 paired browser".to_owned(),
                    n => format!("{n} paired browsers"),
                });
                if ui
                    .add_enabled(n > 0, egui::Button::new("Forget browsers"))
                    .clicked()
                {
                    actions.push(BridgeAction::ForgetBrowsers);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Port");
                    ui.add(egui::DragValue::new(&mut dialog.port).range(1024..=65535));
                    let changed = Some(dialog.port) != running || error.is_some();
                    if ui.add_enabled(changed, egui::Button::new("Use")).clicked() {
                        actions.push(BridgeAction::SetPort(dialog.port));
                    }
                });
                ui.label("The extension's options need the same port.");
            });
        // The code's countdown.
        ctx.request_repaint_after(Duration::from_millis(500));
        if !open {
            actions.push(BridgeAction::Close);
        }
        for a in actions {
            self.dig_bridge_act(a);
        }
    }

    /// A crate was deleted: its sends stop.
    pub(super) fn dig_crate_deleted(&mut self, id: CrateId) {
        let Some(d) = &mut self.dig else { return };
        d.send(Command::CrateDeleted(id));
        d.jobs.retain(|_, j| j.target != id);
        d.active.remove(&id);
        d.focus.remove(&id);
        if d.settings.keepers == Some(id) {
            d.settings.keepers = None;
            let _ = d.save_settings();
        }
    }
}
