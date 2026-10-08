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
use dig::browser::{Browser, SystemBrowser, record_url, sell_url};
use dig::clock::{Clock, RealClock};
use dig::collection::{Collection, Owned, Pressing};
use dig::config::{self as digconf, DigSettings};
use dig::cover::{CoverHandle, Covers, ImageSource, UreqImages};
use dig::discogs::cache::DiskCache;
use dig::discogs::cart::CartSnapshot;
use dig::discogs::client::{ApiError, Client, Identity};
use dig::discogs::matching::ClipEntry;
use dig::discogs::model::{Format, RecordKey};
use dig::discogs::transport::{Transport, UreqTransport};
use dig::discogs::url::{self, Page, Refused};
use dig::intake::{Command, Discarded, Event, Intake, IntakeHandle, JobRef, Outcome, RecordInfo};
use dig::jobs::{Filters, JobId};
use dig::memory::{DigMemory, WantOp};
use dig::prepare::PrepareHandle;
use dig::preview::fetcher::{clip_url, preview_path};
use dig::preview::scheduler::{
    self, Finder, PreviewCommand, PreviewEvent, PreviewHandle, system_finder,
};
use dig::preview::search::SearchRequest;
use dig::preview::store;
use dig::sellers::SellerList;
use platform::{FileSource, Spawner, TrackRef};

use super::covers::CoverCache;
use super::{Action, DiggrApp, records_label};
use crate::crates::{CrateId, MAX_NAME, PLAYLIST};
use crate::playlist::{
    Entry, EntryId, EntryStatus, ForSale, NewEntry, Origin, Playlist, UnavailableKind, WaitKind,
};

/// Failed clips in a row before suggesting that yt-dlp needs an update.
const FAILS_BEFORE_UPDATE_HINT: u32 = 3;
/// The wantlist crate's name before an account is connected ("Wantlist: ‹user›" after).
pub const WANTLIST: &str = "Wantlist";
/// What the wantlist crate was called when wanting was keeping.
const KEEPERS: &str = "Keepers";
/// Up to this many records missing from the wantlist crate are fetched one by one; more,
/// and the whole wantlist page is sent into it.
const FILL_ONE_BY_ONE: usize = 20;
const INSTALL_HINT: &str =
    "Previews need yt-dlp: install it (brew install yt-dlp); it is found within 30 s";

/// What the host provides for digging (fakes in tests).
pub struct DigSetup {
    pub transport: Arc<dyn Transport>,
    /// Where record covers come from (Discogs' image host).
    pub images: Arc<dyn ImageSource>,
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
            images: Arc::new(UreqImages::default()),
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
    /// Add the records of these entries to the wantlist.
    Want(Vec<EntryId>),
    /// Take the records of these entries off the wantlist.
    Unwant(Vec<EntryId>),
    /// Add one copy of each record of these entries to the collection.
    Collect(Vec<EntryId>),
    /// Send a failed wantlist change again, from the start of its schedule.
    RetryWant(EntryId),
    /// Add again after a failure, asking Discogs first whether the add went through.
    RetryCollect(EntryId),
    /// Ask before taking one copy of the entry's record out of the collection.
    Discard(EntryId),
    /// Send a failed collection removal again, from the start of its schedule.
    RetryDiscard(EntryId),
    /// Make the collection crate match the Discogs collection.
    RefreshCollection,
    /// Make the wantlist crate match the Discogs wantlist.
    RefreshWantlist,
    Pass(EntryId),
    UndoPass(EntryId),
    OpenForSale(EntryId),
    /// The entry's release (or master) page on Discogs.
    OpenRecord(EntryId),
    OpenDialog,
    OpenBrowserDialog,
    /// Top Sellers and the cart.
    Seller(super::sellers::SellerAction),
}

/// What Options ▸ Browser… asks for.
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

/// Options ▸ Discogs…
#[derive(Default)]
pub(super) struct DigDialog {
    token: String,
    /// The last token check: the username, or why it failed.
    check: Option<Result<String, String>>,
    checking: bool,
    ytdlp_path: String,
    /// Put the keyboard in the token field (Connect… in the Connect to Discogs dialog): asked
    /// for over this many more frames that it holds, as the closing dialog can take it back.
    pub(super) focus_token: u8,
}

impl DigDialog {
    /// The last token check succeeded.
    #[cfg(test)]
    pub(super) fn connected(&self) -> bool {
        matches!(self.check, Some(Ok(_)))
    }
}

/// The Connect to Discogs dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConnectDialog {
    /// Opened by Add to wantlist (which worked locally): it can be turned off.
    pub(super) from_wantlist: bool,
}

/// "Remove 1 copy of ‹record› from your Discogs collection?", waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscardConfirm {
    release: u64,
    /// "Nightcraft – Glasshouse EP (LT-012, 1994)".
    name: String,
}

/// What the dig side marks an entry with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Marks {
    pub(super) wanted: bool,
    pub(super) passed: bool,
    pub(super) wantlist_pending: bool,
    pub(super) wantlist_failed: Option<String>,
    pub(super) collection_failed: Option<String>,
    pub(super) discard_pending: bool,
    pub(super) discard_failed: Option<String>,
}

pub(super) struct Dig {
    setup: DigSetup,
    spawner: Arc<dyn Spawner>,
    files: Arc<dyn FileSource>,
    /// The app's config folder (`dig/` goes in it); `None` keeps nothing.
    config: Option<PathBuf>,
    pub(super) settings: DigSettings,
    pub(super) memory: DigMemory,
    pub(super) token: Option<String>,
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
    /// The tracks last asked to be searched for, best first.
    searching: Vec<SearchRequest>,
    prepared_list: Vec<TrackRef>,
    /// Per crate being expanded: the focus entry and how many records were still listed.
    focus: HashMap<CrateId, (Option<EntryId>, usize)>,
    /// Crates whose unfinished sends were resumed this session.
    active: HashSet<CrateId>,
    /// Crates a send created: named after the page once Discogs gives its name, and deleted
    /// again if the page turns out not to exist.
    created: HashSet<CrateId>,
    refreshed: HashSet<u64>,
    /// Wantlist changes sent and not answered yet.
    want_inflight: HashSet<(u64, WantOp)>,
    /// Collection removals sent and not answered yet.
    discard_inflight: HashSet<u64>,
    /// Tracks of a digital twin that were playing when its vinyl release took over: they
    /// leave the crate once they stop.
    twin_leftovers: Vec<(CrateId, EntryId)>,
    /// Remove from collection…, asking.
    pub(super) confirm_discard: Option<DiscardConfirm>,
    /// Collection adds on their way: release → the crate they were asked from.
    collecting: HashMap<u64, CrateId>,
    /// Collection adds that failed this session: release → (its crate, why).
    collect_failed: HashMap<u64, (CrateId, String)>,
    /// The account the wantlist crate follows this session.
    connected: Option<String>,
    /// Whose token it is was asked for this session (when no dig asked first).
    identify_asked: bool,
    /// Refresh collection / Refresh wantlist asked: the crate follows when the answer comes.
    refreshing_collection: bool,
    refreshing_wantlist: bool,
    /// Crates filled without announcing each send (the wantlist crate).
    quiet: HashSet<CrateId>,
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
    /// The user's collection (loaded from the cache when digging starts; kept up to date by
    /// a sync on the intake worker).
    pub(super) collection: Option<Arc<Collection>>,
    collection_syncing: bool,
    /// Copies this app added since the running sync started (its result doesn't know them).
    local_adds: Vec<(u64, u64, Pressing)>,
    /// Copies removed by the app while a sync ran: (instance, release, copies left).
    local_discards: Vec<(Option<u64>, u64, usize)>,
    /// Why the last sync failed, for Options ▸ Discogs….
    collection_error: Option<String>,
    /// When a sync was last asked for without being needed again (retries wait an hour).
    collection_tried: Option<Instant>,
    /// The Connect to Discogs dialog, while open.
    pub(super) connect: Option<ConnectDialog>,
    /// "Add a Discogs token…" was said this session.
    token_hint_shown: bool,
    /// Record covers for entry tooltips.
    pub(super) covers: CoverCache,
    /// Top Sellers (`sellers.ron`).
    pub(super) sellers: SellerList,
    /// The user's Discogs cart as last read (`cart.ron` in the cache).
    pub(super) cart: CartSnapshot,
    /// Top Sellers' dialogs and running refreshes.
    pub(super) seller_ui: super::sellers::SellerUi,
}

/// Where a Discogs personal access token is generated ("Generate new token").
pub const TOKEN_PAGE: &str = "https://www.discogs.com/settings/developers";

/// A failed collection sync is tried again after this long (when still needed).
const COLLECTION_RETRY: Duration = Duration::from_secs(3600);

/// Options ▸ Browser…
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
        let sellers = config.as_deref().map(SellerList::load).unwrap_or_default();
        let cart = setup
            .cache_root
            .as_deref()
            .map(CartSnapshot::load)
            .unwrap_or_default();
        Self {
            sellers,
            cart,
            seller_ui: Default::default(),
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
            searching: Vec::new(),
            prepared_list: Vec::new(),
            focus: HashMap::new(),
            active: HashSet::new(),
            created: HashSet::new(),
            refreshed: HashSet::new(),
            want_inflight: HashSet::new(),
            discard_inflight: HashSet::new(),
            twin_leftovers: Vec::new(),
            confirm_discard: None,
            collecting: HashMap::new(),
            collect_failed: HashMap::new(),
            connected: None,
            identify_asked: false,
            refreshing_collection: false,
            refreshing_wantlist: false,
            quiet: HashSet::new(),
            fail_streak: 0,
            play_when_ready: None,
            last_playing: None,
            dialog: None,
            bridge_shared: None,
            bridge: None,
            bridge_error: None,
            snapshot: Snapshot::default(),
            collection: None,
            collection_syncing: false,
            local_adds: Vec::new(),
            local_discards: Vec::new(),
            collection_error: None,
            collection_tried: None,
            connect: None,
            token_hint_shown: false,
            covers: CoverCache::default(),
            bridge_dialog: None,
            wake,
        }
    }

    pub(super) fn has_token(&self) -> bool {
        self.token.is_some()
    }

    pub(super) fn save_sellers(&self) -> Option<String> {
        let c = self.config.as_deref()?;
        self.sellers
            .save(c)
            .err()
            .map(|e| format!("Could not save Top Sellers: {e}"))
    }

    pub(super) fn save_cart(&self) {
        if let Some(root) = &self.setup.cache_root {
            let _ = self.cart.save(root);
        }
    }

    /// With a token: the first Top Sellers list (once ever) and the cart, behind other work.
    fn read_account_lists(&self) {
        if self.token.is_none() {
            return;
        }
        if !self.sellers.seeded {
            self.send(Command::FirstSellers);
        }
        self.send(Command::ReadCart { soon: false });
    }

    /// A send into this crate is listed or expanding.
    pub(super) fn has_job_for(&self, target: CrateId) -> bool {
        self.jobs.values().any(|j| j.target == target)
    }

    pub(super) fn send_cmd(&self, cmd: Command) {
        self.send(cmd);
    }

    pub(super) fn open_url(&self, url: &str) -> Result<(), String> {
        self.setup.browser.open(url)
    }

    fn started(&self) -> bool {
        self.intake.is_some()
    }

    /// The clips asked for ahead of the playhead.
    #[cfg(test)]
    pub(super) fn horizon(&self) -> &[String] {
        &self.horizon
    }

    fn previews_dir(&self) -> PathBuf {
        match &self.setup.cache_root {
            Some(root) => store::dir(root),
            None => std::env::temp_dir()
                .join(platform::APP_DIR)
                .join(store::DIR),
        }
    }

    /// Starts the workers (once the window is interactive).
    fn start(&mut self) {
        self.collection = self
            .setup
            .cache_root
            .as_deref()
            .and_then(Collection::load)
            .map(Arc::new);
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
            scheduler::Config {
                // Search results are remembered beside the previews' folder, not in it, so
                // clearing previews keeps them.
                searches: self.setup.cache_root.clone(),
                ..scheduler::Config::new(
                    self.previews_dir(),
                    self.settings.cache_bytes(),
                    self.settings.ytdlp_path.clone(),
                )
            },
            self.setup.finder.clone(),
            move || wake.request_repaint(),
        )
        .ok();
        let wake = self.wake.clone();
        self.covers.handle = CoverHandle::start(
            &*self.spawner,
            Covers::new(
                self.setup.cache_root.as_deref(),
                self.setup.images.clone(),
                self.setup.clock.clone(),
            ),
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
            self.bridge_shared = Some(Shared::with_cache(
                pairing,
                self.setup.cache_root.as_deref(),
            ));
            self.share_collection();
            self.start_bridge();
        }
        self.read_account_lists();
    }

    /// (Re)starts the bridge on its port; a failure is kept for Options ▸ Browser….
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
                let (kind, name) = j.name.split_once(": ").unwrap_or(("", j.name.as_str()));
                // A seller's job counts the records its copies are of.
                let what = if kind == "Seller" {
                    "records"
                } else {
                    "releases"
                };
                format!("{name}: {} of {} {what}", j.done, j.total)
            })
            .collect();
        (!parts.is_empty()).then(|| parts.join(" · "))
    }

    /// Whether the entry's record is in the user's collection (only with a token).
    pub(super) fn owned(&self, e: &Entry) -> Option<Owned> {
        self.token.as_ref()?;
        let o = e.origin.as_ref()?;
        self.collection.as_ref()?.owned(o.release, o.master)
    }

    /// Asks for a collection sync when there is something to mark (`wanted`), a token, and
    /// the cached collection is missing, someone else's or a week old. A failure waits an
    /// hour before the next try.
    /// Returns a hint to show once per session when there is something to mark but no
    /// token to know the collection with.
    fn maybe_sync_collection(&mut self, wanted: bool) -> Option<String> {
        if wanted && self.token.is_none() && !self.token_hint_shown {
            self.token_hint_shown = true;
            return Some(
                "Add a Discogs token (Options › Discogs…) to mark the records you already own"
                    .into(),
            );
        }
        if !wanted || self.token.is_none() || self.collection_syncing {
            return None;
        }
        if self
            .collection_tried
            .is_some_and(|t| t.elapsed() < COLLECTION_RETRY)
        {
            return None;
        }
        let now = now_secs();
        let stale = match (&self.collection, &self.identity) {
            (None, _) => true,
            (Some(c), Some(id)) => c.is_stale(&id.username, now),
            (Some(c), None) => c.is_stale(&c.username, now),
        };
        if stale {
            self.sync_collection();
        }
        None
    }

    /// Syncs now (Options ▸ Discogs… ▸ Refresh collection): only what changed since the cache.
    pub(super) fn sync_collection(&mut self) {
        if self.token.is_none() || self.collection_syncing {
            return;
        }
        self.collection_syncing = true;
        self.collection_tried = Some(Instant::now());
        let cached = self.collection.as_deref().cloned().map(Box::new);
        self.send(Command::SyncCollection(cached));
    }

    /// A sync finished: the new collection is saved and shared (with the bridge too).
    fn collection_synced(&mut self, result: Result<Box<Collection>, ApiError>) -> Option<String> {
        self.collection_syncing = false;
        let adds = std::mem::take(&mut self.local_adds);
        let discards = std::mem::take(&mut self.local_discards);
        match result {
            Ok(mut c) => {
                for (instance, release, pressing) in adds {
                    c.insert(instance, release, pressing);
                }
                for (instance, release, remaining) in discards {
                    c.discard(instance, release, remaining);
                }
                self.collection_error = None;
                self.collection_tried = None;
                let saved = self
                    .setup
                    .cache_root
                    .as_deref()
                    .and_then(|root| c.save(root).err())
                    .map(|e| format!("Could not save the collection: {e}"));
                self.collection = Some(Arc::new(*c));
                self.share_collection();
                saved
            }
            Err(e) => {
                self.collection_error = Some(match e {
                    ApiError::TokenNeeded | ApiError::TokenRejected => {
                        "needs a valid token".to_owned()
                    }
                    ApiError::Private => "private".to_owned(),
                    ApiError::Offline => "Discogs offline".to_owned(),
                    other => other.message(),
                });
                None
            }
        }
    }

    /// Hands the bridge the current collection, for the browser's owned check.
    fn share_collection(&self) {
        if let Some(s) = &self.bridge_shared {
            let c = self.token.as_ref().and(self.collection.clone());
            s.collection.store(Arc::new(c));
            s.has_token
                .store(self.token.is_some(), std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Options ▸ Discogs…: "1,234 records, updated 2 h ago", or why there is none.
    fn collection_line(&self) -> String {
        if self.token.is_none() {
            return "Needs a token".into();
        }
        let mut line = match &self.collection {
            Some(c) => format!(
                "{} record{}, updated {}",
                c.len(),
                if c.len() == 1 { "" } else { "s" },
                crate::format::ago(now_secs().saturating_sub(c.fetched_at))
            ),
            None => "Not fetched yet (it is, once a crate from Discogs is shown)".into(),
        };
        if self.collection_syncing {
            line += " · syncing…";
        } else if let Some(e) = &self.collection_error {
            line += &format!(" · last sync failed: {e}");
        }
        line
    }

    /// Wanted, passed, and how its wantlist or collection change is going, for a row.
    pub(super) fn marks(&self, e: &Entry) -> Marks {
        let release = release_of(e);
        Marks {
            wanted: release.is_some_and(|r| self.memory.is_wanted(r)),
            passed: self.memory.is_passed(&key_of(e)),
            wantlist_pending: release.is_some_and(|r| self.memory.is_want_pending(r)),
            wantlist_failed: release
                .and_then(|r| self.memory.want_failure(r))
                .map(str::to_owned),
            collection_failed: release
                .and_then(|r| self.collect_failed.get(&r))
                .map(|(_, why)| why.clone()),
            discard_pending: release.is_some_and(|r| self.memory.is_discard_pending(r)),
            discard_failed: release
                .and_then(|r| self.memory.discard_failure(r))
                .map(str::to_owned),
        }
    }
}

/// The release an entry's record is (a master's entries carry its main release).
pub(super) fn release_of(e: &Entry) -> Option<u64> {
    e.origin.as_ref()?.release
}

/// A record in messages: its album, else the entry's name.
fn record_name(e: &Entry) -> String {
    if e.album().is_empty() {
        e.display_name()
    } else {
        e.album().to_owned()
    }
}

/// "Collection: 3 new, 1 gone", "Wantlist up to date".
fn changes_line(what: &str, new: usize, gone: usize) -> String {
    match (new, gone) {
        (0, 0) => format!("{what} up to date"),
        (n, 0) => format!("{what}: {n} new"),
        (0, g) => format!("{what}: {g} gone"),
        (n, g) => format!("{what}: {n} new, {g} gone"),
    }
}

/// "this pressing", "another pressing (AF001R, 2019)".
pub(super) fn owned_text(o: &Owned) -> String {
    match o {
        Owned::ThisPressing => "this pressing".to_owned(),
        Owned::Another { .. } => {
            let short = owned_short(&Some(o.clone()));
            if short.is_empty() {
                "another pressing".to_owned()
            } else {
                format!("another pressing ({short})")
            }
        }
    }
}

/// The owned pressing's catalog number and year: "AF001R, 2019".
fn owned_short(o: &Option<Owned>) -> String {
    match o {
        Some(Owned::Another { catno, year }) => [
            catno.clone(),
            year.map(|y| y.to_string()).unwrap_or_default(),
        ]
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", "),
        _ => String::new(),
    }
}

/// Keep and pass work on any entry: a clip by its id, a local file by its path.
pub(super) fn key_of(e: &Entry) -> String {
    e.origin
        .as_ref()
        .and_then(|o| o.clip.clone())
        .unwrap_or_else(|| e.track.0.clone())
}

fn clip_of(e: &Entry) -> Option<&str> {
    e.source.as_ref()?;
    e.origin.as_ref()?.clip.as_deref()
}

/// The Discogs page of the record an entry comes from, if it comes from one.
fn entry_record_url(e: &Entry) -> Option<String> {
    let o = e.origin.as_ref()?;
    record_url(o.release, o.master)
}

/// The record a "listed" placeholder stands for.
fn listed_key(e: &Entry) -> Option<RecordKey> {
    if e.status != EntryStatus::Waiting(WaitKind::Listed) {
        return None;
    }
    record_key(e.origin.as_ref().filter(|o| o.clip.is_none())?)
}

/// What pairs releases of one record across formats: its master release, and its catalog
/// number with its title (lower-cased), as far as each is known.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Twin {
    Master(u64),
    Named(String, String),
}

fn twin_keys(master: Option<u64>, catno: &str, title: &str) -> Vec<Twin> {
    let (catno, title) = (catno.trim().to_lowercase(), title.trim().to_lowercase());
    let named = (!catno.is_empty() && !title.is_empty()).then_some(Twin::Named(catno, title));
    master.map(Twin::Master).into_iter().chain(named).collect()
}

fn origin_twins(o: &Origin) -> Vec<Twin> {
    twin_keys(o.master, &o.catno, &o.album)
}

/// Whether formats kept on an origin ("Vinyl, CD") include vinyl.
fn has_vinyl(formats: &str) -> bool {
    formats.split(',').any(|f| f.trim() == Format::Vinyl.name())
}

/// A release known to be in some format, but not on vinyl.
fn not_vinyl(o: &Origin) -> bool {
    !o.formats.is_empty() && !has_vinyl(&o.formats)
}

/// The record an origin names: its release, else its master release.
fn record_key(o: &Origin) -> Option<RecordKey> {
    match (o.release, o.master) {
        (Some(r), _) => Some(RecordKey::Release(r)),
        (None, Some(m)) => Some(RecordKey::Master(m)),
        _ => None,
    }
}

/// The records of a crate's Discogs entries saved without an album or styles, once each.
fn records_to_backfill(p: &Playlist) -> Vec<RecordKey> {
    let mut seen = HashSet::new();
    p.entries()
        .iter()
        .filter_map(|e| e.origin.as_ref())
        .filter(|o| {
            o.album.is_empty() || o.styles.is_empty() || o.artist.is_empty() || o.formats.is_empty()
        })
        .filter_map(record_key)
        .filter(|k| seen.insert(*k))
        .collect()
}

/// Fills the album, cover, styles, record artist and formats of entries saved without them;
/// true if any changed.
fn backfill(p: &mut Playlist, infos: &[RecordInfo]) -> bool {
    let by_key: HashMap<RecordKey, &RecordInfo> = infos.iter().map(|i| (i.key, i)).collect();
    let mut changed = false;
    for e in p.entries_mut() {
        let Some(o) = e.origin.as_mut() else {
            continue;
        };
        let Some(info) = record_key(o).and_then(|k| by_key.get(&k)) else {
            continue;
        };
        if o.album.is_empty() && !info.title.is_empty() {
            o.album = info.title.clone();
            changed = true;
        }
        if o.cover.is_empty() && !info.cover.is_empty() {
            o.cover = info.cover.clone();
            changed = true;
        }
        if o.styles.is_empty() && !info.styles.is_empty() {
            o.styles = info.styles.clone();
            changed = true;
        }
        if o.artist.is_empty() && !info.artist.is_empty() {
            o.artist = info.artist.clone();
            changed = true;
        }
        if o.formats.is_empty() && !info.formats.is_empty() {
            o.formats = formats_text(&info.formats);
            changed = true;
        }
    }
    changed
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
        album: info.title.clone(),
        cover: info.cover.clone(),
        styles: info.styles.clone(),
        artist: info.artist.clone(),
        formats: formats_text(&info.formats),
        search_key: String::new(),
        found: String::new(),
    }
}

/// "Vinyl", "Vinyl, CD": what an entry's origin keeps of a record's formats.
fn formats_text(formats: &[Format]) -> String {
    formats
        .iter()
        .map(|f| f.name())
        .collect::<Vec<_>>()
        .join(", ")
}

fn now_secs() -> u64 {
    dig::now_secs()
}

impl DiggrApp {
    pub(super) fn dig_notify(&mut self, text: Option<String>) {
        if let Some(t) = text {
            self.notify(t);
        }
    }

    // ---- sending pages ---------------------------------------------------------------------

    /// A send the user asked for (a paste, the browser): never into a Discogs crate, which
    /// only the Discogs items fill.
    fn dig_send_asked(&mut self, page: Page, mode: SendMode, filters: Option<Filters>) {
        if matches!(page.kind, url::PageKind::Seller(_)) {
            return self.dig_send(page, mode, filters);
        }
        let target = match &mode {
            SendMode::Enqueue => Some(self.crates.shown_id()),
            SendMode::Crate(name) => self.crates.find(name),
            SendMode::Play => None,
        };
        if target.is_some_and(|c| self.refuse_discogs_insert(c)) {
            return;
        }
        self.dig_send(page, mode, filters);
    }

    /// Cmd+V / Ctrl+V: a Discogs address goes to the shown crate; other text is ignored.
    pub(super) fn dig_paste(&mut self, text: &str) {
        match url::parse(text) {
            Ok(page) => self.dig_send_asked(page, SendMode::Enqueue, None),
            Err(Refused::Unsupported) => self.notify(url::SUPPORTED),
            Err(Refused::NotDiscogs) => {}
        }
    }

    /// A page's tracks into the crate `target` (a seller's), announced like any send.
    pub(super) fn dig_send_to(&mut self, page: Page, target: CrateId) {
        let Some(d) = &mut self.dig else { return };
        if !d.started() {
            return;
        }
        let filters = Filters {
            skip_passed: d.settings.skip_passed,
        };
        d.active.insert(target);
        d.send(Command::Send {
            page,
            target,
            filters,
        });
    }

    /// Sends a page's tracks into a crate. `filters` overrides the defaults. A seller's page
    /// adds the seller to Top Sellers instead (or refreshes it).
    pub(crate) fn dig_send(&mut self, page: Page, mode: SendMode, filters: Option<Filters>) {
        if let url::PageKind::Seller(name) = &page.kind {
            let name = name.clone();
            self.seller_page(&name);
            return;
        }
        let Some(d) = &self.dig else { return };
        if !d.started() {
            self.notify("Discogs isn't ready yet");
            return;
        }
        let filters = filters.unwrap_or(Filters {
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
                // A crate the send just made (or plays) comes on screen, with the playlist
                // opened if it was hidden; sending to an existing crate leaves the view alone.
                if play || created {
                    self.show_crate(id);
                    if !self.settings.show_playlist {
                        self.settings.show_playlist = true;
                        self.mark_settings();
                    }
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
        self.dig_migrate_keepers();
        self.dig_connect_wantlist();
        let Some(d) = &mut self.dig else { return };
        for id in [self.crates.shown_id(), self.crates.playing_id()] {
            if d.active.insert(id) {
                d.send(Command::CrateActive(id));
                let keys = self
                    .crates
                    .get(id)
                    .map(records_to_backfill)
                    .unwrap_or_default();
                if !keys.is_empty() {
                    d.send(Command::Backfill { target: id, keys });
                }
            }
        }
        // The collection is only worth syncing when a crate from Discogs is on screen.
        let from_discogs = self
            .crates
            .shown()
            .entries()
            .iter()
            .any(|e| e.origin.is_some());
        let hint = d.maybe_sync_collection(from_discogs);
        self.dig_notify(hint);
        let Some(d) = &mut self.dig else { return };
        let events = d
            .intake
            .as_ref()
            .map(IntakeHandle::poll)
            .unwrap_or_default();
        for e in events {
            self.dig_intake_event(e);
        }
        self.seller_dialog_tick();
        self.seller_add_tick();
        self.seller_frame();
        let Some(d) = &mut self.dig else { return };
        let ctx = d.wake.clone();
        for key in d.covers.poll(&ctx) {
            d.send(Command::RefreshCover(key));
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
        self.dig_retry_discards();
        self.dig_bridge_snapshot();
    }

    // ---- the browser bridge ----------------------------------------------------------------

    pub(super) fn dig_bridge_command(&mut self, c: BridgeCommand) {
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
                let seller = matches!(page.kind, url::PageKind::Seller(_));
                self.dig_send_asked(page, mode, Some(filters));
                // A seller added from the browser: the app asks to come forward (the system
                // may only draw attention to it instead, as macOS does for a background app).
                if seller && let Some(d) = &self.dig {
                    d.wake.send_viewport_cmd(egui::ViewportCommand::Focus);
                    d.wake
                        .send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Informational,
                        ));
                }
            }
            BridgeCommand::ResolveShopItem(id) => {
                if let Some(d) = &self.dig {
                    d.send(Command::ResolveShopItem(id));
                }
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
        let sellers = d
            .sellers
            .sellers
            .iter()
            .map(|s| s.username.clone())
            .collect();
        let snap = Snapshot {
            crates,
            playing,
            sends,
            sellers,
        };
        if snap != d.snapshot {
            shared.snapshot.store(Arc::new(snap.clone()));
            if let Some(d) = &mut self.dig {
                d.snapshot = snap;
            }
        }
    }

    pub(super) fn dig_intake_event(&mut self, e: Event) {
        match e {
            Event::Copies(j, copies) => self.seller_copies(j.target, copies),
            Event::FirstSellers(result) => self.dig_first_sellers(result),
            Event::Cart(result) => self.dig_cart_read(result),
            Event::SellerCount {
                seller,
                query,
                result,
            } => self.seller_counted(&seller, &query, result),
            Event::SellerLookup { name, result } => self.seller_looked_up(&name, result),
            Event::Scanned {
                seller,
                copies,
                read,
                pages,
                total,
                done,
            } => self.seller_scanned(&seller, copies, (read, pages, total, done)),
            Event::ScanFailed { seller, error } => self.seller_scan_failed(&seller, &error),
            Event::CartAdded { listings, result } => self.cart_added(&listings, result),
            Event::CartRemoved { listing, result } => self.cart_removed(listing, result),
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
                if d.quiet.contains(&j.target) {
                    return;
                }
                let to = self.crates.name(j.target).to_owned();
                self.notify(if to == j.name {
                    format!("Digging {to}")
                } else {
                    format!("{} › {to}", j.name)
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
                // The user's own collection gets its place in the sidebar.
                let own = self.dig.as_ref().and_then(|d| d.identity.as_ref()).is_some_and(|id| {
                    url::parse(&j.page).is_ok_and(|p| {
                        matches!(&p.kind, url::PageKind::Collection(u) if u.eq_ignore_ascii_case(&id.username))
                    })
                });
                if own {
                    self.crates.set_collection(j.target);
                }
                let Some(p) = self.crates.get_mut(j.target) else {
                    return;
                };
                for l in items {
                    let info = RecordInfo::from_listed(&l);
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
                        WaitKind::Listed,
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
                self.seller_job_ended(j.target, true);
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
                self.seller_job_ended(j.target, false);
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
            Event::Collection(result) => {
                let failed = result.as_ref().err().map(ApiError::message);
                let err = self.dig.as_mut().and_then(|d| d.collection_synced(result));
                self.dig_notify(err);
                self.dig_collection_refreshed(failed);
            }
            // The bridge reads the item's release from the disk cache the lookup filled.
            Event::ShopItem(..) => {}
            Event::Cover(key, url) => {
                let mut changed = false;
                for c in self.crates.loaded_ids() {
                    let Some(p) = self.crates.get_mut(c) else {
                        continue;
                    };
                    let mut touched = false;
                    for e in p.entries_mut() {
                        if let Some(o) = e.origin.as_mut()
                            && record_key(o) == Some(key)
                            && !url.is_empty()
                            && o.cover != url
                        {
                            o.cover = url.clone();
                            touched = true;
                        }
                    }
                    if touched {
                        changed = true;
                        self.crates.touch(c);
                    }
                }
                if let Some(d) = &mut self.dig {
                    d.covers.refreshed(key, changed);
                }
            }
            Event::Backfill(target, infos) => {
                if let Some(p) = self.crates.get_mut(target)
                    && backfill(p, &infos)
                {
                    self.crates.touch(target);
                }
            }
            Event::Wantlist {
                release,
                add,
                result,
            } => self.dig_wantlist_result(release, add, result),
            Event::Collected { release, result } => self.dig_collected(release, result),
            Event::Discarded { release, result } => self.dig_discarded(release, result),
            Event::Wants(result) => self.dig_wants(result),
            Event::OwnedWants(releases) => self.dig_owned_wants(releases),
        }
    }

    /// A listed record's details: its placeholder becomes its clips, "no clip", or goes.
    fn dig_record(&mut self, j: &JobRef, info: &RecordInfo, outcome: Outcome) {
        // Vinyl first, in crates filled from pages (never the wantlist or collection crates).
        let twins = !self.crates.is_discogs(j.target)
            && self
                .dig
                .as_ref()
                .is_some_and(|d| d.settings.wantlist != Some(j.target));
        let playing = (self.crates.playing_id() == j.target)
            .then(|| self.crates.playing().current())
            .flatten();
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
        let keys = twin_keys(info.master, &info.catno, &info.title);
        let twin_of = |o: &Origin| origin_twins(o).iter().any(|k| keys.contains(k));
        // A release not on vinyl whose record is on vinyl here adds nothing: unless it brings
        // tunes the vinyl release has none of.
        let vinyl_twins: Vec<bool> = p
            .entries()
            .iter()
            .filter(|e| e.id != placeholder)
            .filter_map(|e| e.origin.as_ref())
            .filter(|o| has_vinyl(&o.formats) && twin_of(o))
            .map(|o| o.clip.is_some())
            .collect();
        // No clip of its own (a "no clip" record, or tracks still to search for).
        let silent = matches!(outcome, Outcome::Unavailable(_) | Outcome::Tracks(_));
        if twins
            && not_vinyl(&base)
            && !vinyl_twins.is_empty()
            && (silent || vinyl_twins.iter().any(|&clip| clip))
        {
            p.remove(placeholder);
            self.mark_crate(j.target);
            return;
        }
        // A vinyl release: its twins in other formats that came back with no tunes leave (those
        // still waiting for their details are decided when those arrive).
        if twins && has_vinyl(&base.formats) {
            let silent_twins: Vec<EntryId> = p
                .entries()
                .iter()
                .filter(|e| e.id != placeholder && listed_key(e).is_none())
                .filter(|e| {
                    e.origin
                        .as_ref()
                        .is_some_and(|o| o.clip.is_none() && not_vinyl(o) && twin_of(o))
                })
                .map(|e| e.id)
                .collect();
            p.remove_ids(&silent_twins);
        }
        let mut leftovers = Vec::new();
        let new: Vec<NewEntry> = match outcome {
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
                // The dig crate reports reasons in words ("no clip", "not found").
                p.set_unavailable(placeholder, reason.as_str());
                self.mark_crate(j.target);
                return;
            }
            Outcome::Tracks(tracks) => tracks
                .into_iter()
                .map(|t| NewEntry {
                    source: None,
                    origin: Some(Origin {
                        position: t.position,
                        search_key: t.search_key,
                        ..base.clone()
                    }),
                    artist: t.artist,
                    title: t.title,
                    duration: t.duration,
                    status: WaitKind::Search,
                })
                .collect(),
            Outcome::Clips(entries) => {
                // The vinyl release takes over the tracks its non-vinyl twins brought, in
                // place; their other tracks (and their waiting placeholders) leave.
                if twins && has_vinyl(&base.formats) {
                    let by_clip: HashMap<&str, &ClipEntry> =
                        entries.iter().map(|c| (c.clip.as_str(), c)).collect();
                    let twin_ids: Vec<EntryId> = p
                        .entries()
                        .iter()
                        .filter(|e| e.id != placeholder)
                        .filter(|e| {
                            e.origin
                                .as_ref()
                                .is_some_and(|o| not_vinyl(o) && twin_of(o))
                        })
                        .map(|e| e.id)
                        .collect();
                    let mut gone = Vec::new();
                    for id in twin_ids {
                        let Some(e) = p.entries_mut().find(|e| e.id == id) else {
                            continue;
                        };
                        let clip = e.origin.as_ref().and_then(|o| o.clip.clone());
                        match clip.as_deref().and_then(|c| by_clip.get(c)) {
                            Some(c) => {
                                e.origin = Some(Origin {
                                    position: c.position.clone(),
                                    clip: Some(c.clip.clone()),
                                    ..base.clone()
                                });
                                e.artist = c.artist.clone();
                                e.title = c.title.clone();
                            }
                            None if playing == Some(id) => leftovers.push((j.target, id)),
                            None => gone.push(id),
                        }
                    }
                    p.remove_ids(&gone);
                }
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
                        status: WaitKind::Queued,
                    })
                    .collect()
            }
        };
        let ids = p.replace(placeholder, new);
        if let Some(d) = &mut self.dig {
            d.twin_leftovers.extend(leftovers);
        }
        let Some(p) = self.crates.get_mut(j.target) else {
            return;
        };
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

    /// Every loaded crate's entries still waiting to be searched for under `key`, changed by
    /// `f`; returns them.
    fn dig_each_searched(
        &mut self,
        key: &str,
        mut f: impl FnMut(&mut Entry),
    ) -> Vec<(CrateId, EntryId)> {
        let mut hits = Vec::new();
        for c in self.crates.loaded_ids() {
            let before = hits.len();
            if let Some(p) = self.crates.get_mut(c) {
                for e in p.entries_mut() {
                    let waiting = e.status == EntryStatus::Waiting(WaitKind::Search);
                    if waiting && e.origin.as_ref().is_some_and(|o| o.search_key == key) {
                        f(e);
                        hits.push((c, e.id));
                    }
                }
            }
            if hits.len() > before {
                self.mark_crate(c);
            }
        }
        hits
    }

    fn dig_preview_event(&mut self, e: PreviewEvent) {
        match e {
            PreviewEvent::Found { key, clip, title } => {
                let found = self.dig_each_searched(&key, |e| {
                    let o = e.origin.as_mut().expect("searched entries have an origin");
                    o.clip = Some(clip.clone());
                    o.found = title.clone();
                    e.source = Some(clip_url(&clip));
                    e.status = EntryStatus::Waiting(WaitKind::Queued);
                });
                // A preview downloaded already (for another crate) plays at once.
                let Some(d) = &self.dig else { return };
                let path = preview_path(&d.previews_dir(), &clip);
                if std::fs::metadata(&path).is_ok_and(|m| m.len() > 0) {
                    let track = TrackRef::new(path.to_string_lossy());
                    for (c, id) in found {
                        self.set_audio(c, id, track.clone());
                    }
                }
            }
            PreviewEvent::NotFound(key) => {
                let _ = self.dig_each_searched(&key, |e| {
                    e.status = EntryStatus::Unavailable(UnavailableKind::NotFound);
                });
            }
            PreviewEvent::Progress(clip, pct) => {
                self.dig_each_clip(&clip, |p, id| {
                    p.set_status(id, WaitKind::Downloading(pct));
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
                        p.set_waiting(id, WaitKind::Queued);
                    }
                });
            }
            PreviewEvent::NeedsYtDlp => {
                let Some(d) = &mut self.dig else { return };
                d.ytdlp = YtDlpState::Missing;
                let first = !d.hint_shown;
                d.hint_shown = true;
                self.dig_mark_waiting(WaitKind::Queued, WaitKind::NeedsYtDlp);
                if first {
                    self.notify(INSTALL_HINT);
                }
            }
            PreviewEvent::YtDlp(version) => {
                if let Some(d) = &mut self.dig {
                    d.ytdlp = YtDlpState::Found(version);
                }
                self.dig_mark_waiting(WaitKind::NeedsYtDlp, WaitKind::Queued);
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

    /// Horizon entries waiting for `from` wait for `to` instead (queued ↔ needs yt-dlp).
    fn dig_mark_waiting(&mut self, from: WaitKind, to: WaitKind) {
        let Some(d) = &self.dig else { return };
        let clips = d.horizon.clone();
        for clip in clips {
            self.dig_each_clip(&clip, |p, id| {
                if p.get(id).map(|e| &e.status) == Some(&EntryStatus::Waiting(from.clone())) {
                    p.set_status(id, to.clone());
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
        // Tracks of records with no clip, in the same window: searched for, best first.
        let armed_entry = self.armed.filter(|(c, _)| *c == cid).map(|(_, id)| id);
        let searches: Vec<SearchRequest> = armed_entry
            .into_iter()
            .chain(order.iter().skip(at).take(scheduler::AHEAD + 1).copied())
            .filter_map(|id| p.get(id))
            .filter(|e| e.status == EntryStatus::Waiting(WaitKind::Search))
            .filter_map(|e| {
                let o = e.origin.as_ref()?;
                (!o.search_key.is_empty()).then(|| SearchRequest {
                    key: o.search_key.clone(),
                    artist: e.artist.clone(),
                    record_artist: o.artist.clone(),
                    title: e.title.clone(),
                    duration: e.duration,
                })
            })
            .fold(Vec::new(), |mut v: Vec<SearchRequest>, r| {
                if !v.iter().any(|x| x.key == r.key) {
                    v.push(r);
                }
                v
            });
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
        if searches != d.searching {
            d.preview(PreviewCommand::Search(searches.clone()));
            d.searching = searches;
        }
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
                self.dig_mark_waiting(WaitKind::Queued, WaitKind::NeedsYtDlp);
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
    pub(super) fn dig_playing_changed(&mut self) {
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
        // Twin tracks kept while they played leave once something else plays.
        let (stopped, still): (Vec<_>, Vec<_>) = std::mem::take(&mut d.twin_leftovers)
            .into_iter()
            .partition(|&left| Some(left) != now);
        d.twin_leftovers = still;
        for (c, id) in stopped {
            if let Some(p) = self.crates.get_mut(c)
                && p.remove(id)
            {
                self.mark_crate(c);
            }
        }
        let Some(d) = &mut self.dig else { return };
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

    /// Sends the wantlist changes that are due (each one once until its answer comes back).
    /// Sends the collection removals that are due.
    fn dig_retry_discards(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none() || !d.started() || d.memory.discards.is_empty() {
            return;
        }
        for release in d.memory.discards_due(now_secs()) {
            if d.discard_inflight.insert(release) {
                d.send(Command::Discard(release));
            }
        }
    }

    fn dig_retry_wantlist(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none() || !d.started() || d.memory.wantlist_pending.is_empty() {
            return;
        }
        for (release, op) in d.memory.due(now_secs()) {
            if d.want_inflight.insert((release, op)) {
                d.send(match op {
                    WantOp::Add => Command::Want(release),
                    WantOp::Remove => Command::Unwant(release),
                });
            }
        }
    }

    // ---- verdicts ----------------------------------------------------------------------

    /// Y, N and I act on the playing or paused entry, and do nothing while stopped.
    pub(super) fn dig_key(&mut self, key: egui::Key) -> bool {
        if !matches!(key, egui::Key::Y | egui::Key::N | egui::Key::I) || self.dig.is_none() {
            return false;
        }
        if self.position.state == PlayState::Stopped {
            return true;
        }
        let c = self.crates.playing_id();
        let Some(id) = self.crates.playing().current() else {
            return true;
        };
        let action = match key {
            egui::Key::Y => {
                let wanted = self
                    .crates
                    .playing()
                    .get(id)
                    .and_then(release_of)
                    .is_some_and(|r| self.dig.as_ref().is_some_and(|d| d.memory.is_wanted(r)));
                if wanted {
                    DigAction::Unwant(vec![id])
                } else {
                    DigAction::Want(vec![id])
                }
            }
            egui::Key::N => DigAction::Pass(id),
            _ => DigAction::OpenForSale(id),
        };
        self.dig_act(c, action);
        true
    }

    pub(super) fn dig_act(&mut self, c: CrateId, a: DigAction) {
        match a {
            DigAction::Seller(a) => self.seller_act(a),
            DigAction::Want(ids) => self.dig_want(c, &ids),
            DigAction::Unwant(ids) => self.dig_unwant(c, &ids),
            DigAction::Collect(ids) => self.dig_collect(c, &ids, false),
            DigAction::RetryCollect(id) => self.dig_collect(c, &[id], true),
            DigAction::Discard(id) => self.dig_ask_discard(c, id),
            DigAction::RetryDiscard(id) => {
                let Some(r) = self
                    .crates
                    .get(c)
                    .and_then(|p| p.get(id))
                    .and_then(release_of)
                else {
                    return;
                };
                let Some(d) = &mut self.dig else { return };
                d.memory.retry_discard(r);
                let e = d.save_memory();
                self.dig_notify(e);
                self.dig_retry_discards();
            }
            DigAction::RetryWant(id) => {
                let Some(r) = self
                    .crates
                    .get(c)
                    .and_then(|p| p.get(id))
                    .and_then(release_of)
                else {
                    return;
                };
                let Some(d) = &mut self.dig else { return };
                d.memory.retry_want(r);
                let e = d.save_memory();
                self.dig_notify(e);
                self.dig_retry_wantlist();
            }
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
            DigAction::OpenRecord(id) => {
                let url = self
                    .crates
                    .get(c)
                    .and_then(|p| p.get(id))
                    .and_then(entry_record_url);
                let Some(d) = &self.dig else { return };
                if let Some(url) = url
                    && let Err(err) = d.setup.browser.open(&url)
                {
                    self.notify(format!("Could not open the browser: {err}"));
                }
            }
            DigAction::RefreshCollection => {
                let Some(d) = &mut self.dig else { return };
                if d.token.is_none() || d.refreshing_collection {
                    return;
                }
                d.refreshing_collection = true;
                // A sync already on its way does: the crate follows its answer.
                d.sync_collection();
            }
            DigAction::RefreshWantlist => {
                let Some(d) = &mut self.dig else { return };
                if d.token.is_none() || d.refreshing_wantlist || d.settings.wantlist.is_none() {
                    return;
                }
                d.refreshing_wantlist = true;
                d.send(Command::ReadWants { fresh: true });
            }
            DigAction::OpenDialog => self.dig_open_dialog(),
            DigAction::OpenBrowserDialog => self.dig_open_bridge_dialog(),
        }
    }

    /// Refresh wantlist / Refresh collection for one of the user's Discogs crates, in a crate
    /// menu (only with a token; disabled while it runs).
    pub(super) fn dig_refresh_item(
        &self,
        ui: &mut egui::Ui,
        c: &crate::crates::CrateInfo,
        actions: &mut Vec<Action>,
    ) {
        let Some(d) = self.dig.as_ref().filter(|d| d.token.is_some()) else {
            return;
        };
        let (label, running, action) = if c.wantlist {
            (
                "Refresh wantlist",
                d.refreshing_wantlist,
                DigAction::RefreshWantlist,
            )
        } else if c.collection {
            (
                "Refresh collection",
                d.refreshing_collection,
                DigAction::RefreshCollection,
            )
        } else {
            return;
        };
        let label = if running { "Refreshing…" } else { label };
        if ui
            .add_enabled(!running, egui::Button::new(label))
            .on_hover_text("Make this crate match Discogs")
            .clicked()
        {
            actions.push(Action::Dig(action));
            ui.close();
        }
    }

    /// After Refresh collection's sync: the collection crate takes in the records it lacks and
    /// lets go of those no longer in the collection; the main window says what changed.
    fn dig_collection_refreshed(&mut self, failed: Option<String>) {
        let Some(d) = &mut self.dig else { return };
        if !std::mem::take(&mut d.refreshing_collection) {
            return;
        }
        if let Some(why) = failed {
            return self.notify(format!("Refresh failed: {why}"));
        }
        let owned: Option<HashSet<u64>> = d
            .collection
            .as_ref()
            .map(|c| c.releases.keys().copied().collect());
        let (Some(coll), Some(owned)) = (self.collection_crate(), owned) else {
            return;
        };
        if !self.crates.load(coll) {
            return;
        }
        let present: HashSet<u64> = self
            .crates
            .get(coll)
            .map(|p| p.entries().iter().filter_map(release_of).collect())
            .unwrap_or_default();
        let gone: HashSet<u64> = present.difference(&owned).copied().collect();
        let new: Vec<u64> = owned.difference(&present).copied().collect();
        let ids = self.entries_of(coll, &gone);
        if let Some(p) = self.crates.get_mut(coll)
            && p.remove_ids(&ids) > 0
        {
            self.mark_crate(coll);
        }
        let user = self
            .dig
            .as_ref()
            .and_then(|d| d.identity.as_ref())
            .map(|i| i.username.clone());
        match user {
            Some(u) if new.len() > FILL_ONE_BY_ONE => {
                self.dig_fill(Page::new(url::PageKind::Collection(u)), coll);
            }
            _ => {
                for &r in &new {
                    self.dig_fill(Page::new(url::PageKind::Release(r)), coll);
                }
            }
        }
        self.notify(changes_line("Collection", new.len(), gone.len()));
    }

    /// The records of entries `ids` of crate `c`: each release once, with its first entry,
    /// in crate order; and how many entries have no release (local files).
    fn dig_records(&self, c: CrateId, ids: &[EntryId]) -> (Vec<(u64, Entry)>, usize) {
        let Some(p) = self.crates.get(c) else {
            return (Vec::new(), 0);
        };
        let mut seen = HashSet::new();
        let mut records = Vec::new();
        let mut local = 0;
        for e in p.entries().iter().filter(|e| ids.contains(&e.id)) {
            match release_of(e) {
                Some(r) => {
                    if seen.insert(r) {
                        records.push((r, e.clone()));
                    }
                }
                None => local += 1,
            }
        }
        (records, local)
    }

    /// The wantlist crate: the one in the settings, else one named "Wantlist" (or, connected,
    /// "Wantlist: ‹user›"), else a new one.
    fn wantlist_crate(&mut self) -> Result<CrateId, String> {
        let Some(d) = &self.dig else {
            return Err("Digging is off".into());
        };
        if let Some(id) = d
            .settings
            .wantlist
            .filter(|&id| self.crates.info(id).is_some())
        {
            return Ok(id);
        }
        let user = d
            .token
            .as_ref()
            .and(d.identity.as_ref())
            .map(|i| i.username.clone());
        let name = match &user {
            Some(u) => format!("{WANTLIST}: {u}"),
            None => WANTLIST.to_owned(),
        };
        let id = match self.crates.find(&name) {
            Some(id) => id,
            None => self.crates.create(&name)?,
        };
        if user.is_some() {
            self.crates.set_wantlist(id, true);
        }
        if let Some(d) = &mut self.dig {
            d.settings.wantlist = Some(id);
            let e = d.save_settings();
            self.dig_notify(e);
        }
        Ok(id)
    }

    /// The crate made from the user's collection, if there is one.
    fn collection_crate(&self) -> Option<CrateId> {
        self.crates
            .list()
            .iter()
            .find(|c| c.collection)
            .map(|c| c.id)
    }

    /// The entries of `releases` in crate `c` (loaded first).
    fn entries_of(&mut self, c: CrateId, releases: &HashSet<u64>) -> Vec<EntryId> {
        if !self.crates.load(c) {
            return Vec::new();
        }
        self.crates
            .get(c)
            .map(|p| {
                p.entries()
                    .iter()
                    .filter(|e| release_of(e).is_some_and(|r| releases.contains(&r)))
                    .map(|e| e.id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Takes the entries of `releases` out of the wantlist crate.
    fn leave_wantlist_crate(&mut self, releases: &HashSet<u64>) {
        let Some(wl) = self.dig.as_ref().and_then(|d| d.settings.wantlist) else {
            return;
        };
        let ids = self.entries_of(wl, releases);
        if let Some(p) = self.crates.get_mut(wl)
            && p.remove_ids(&ids) > 0
        {
            self.mark_crate(wl);
        }
    }

    /// Fetches a record's clips into a crate it may only partly be in (no request when the
    /// release is cached), without announcing it.
    fn dig_fill(&mut self, page: Page, target: CrateId) {
        let Some(d) = &mut self.dig else { return };
        if !d.started() {
            return;
        }
        let filters = Filters {
            skip_passed: d.settings.skip_passed,
        };
        d.quiet.insert(target);
        d.active.insert(target);
        d.send(Command::Send {
            page,
            target,
            filters,
        });
    }

    /// Add to wantlist: each record (not owned, not wanted yet) is wanted, goes to the
    /// wantlist crate and, with a token, to the Discogs wantlist.
    fn dig_want(&mut self, c: CrateId, ids: &[EntryId]) {
        let (records, local) = self.dig_records(c, ids);
        if records.is_empty() {
            if local > 0 && ids.len() == 1 {
                let name = self
                    .crates
                    .get(c)
                    .and_then(|p| p.get(ids[0]))
                    .map(Entry::display_name)
                    .unwrap_or_default();
                self.notify(format!("{name} isn't from Discogs"));
            }
            return;
        }
        let mut added = Vec::new();
        let (mut owned, mut already) = (Vec::new(), 0);
        for (r, e) in &records {
            if let Some(o) = self.dig.as_ref().and_then(|d| d.owned(e)) {
                let what = match owned_short(&Some(o.clone())) {
                    short if !short.is_empty() => short,
                    _ => owned_text(&o),
                };
                owned.push((record_name(e), what));
            } else if self.dig.as_ref().is_some_and(|d| d.memory.is_wanted(*r)) {
                already += 1;
            } else {
                added.push((*r, e.clone()));
            }
        }
        if !added.is_empty() {
            let wl = match self.wantlist_crate() {
                Ok(k) => k,
                Err(err) => return self.notify(err),
            };
            let set: HashSet<u64> = added.iter().map(|(r, _)| *r).collect();
            if c != wl {
                let copies = self.entries_of(c, &set);
                if let Err(err) = self.crates.send(c, &copies, wl) {
                    return self.notify(err);
                }
                self.mark_crate(wl);
            }
            for (r, _) in &added {
                self.dig_fill(Page::new(url::PageKind::Release(*r)), wl);
            }
            let Some(d) = &mut self.dig else { return };
            for (r, _) in &added {
                d.memory.wanted.insert(*r);
                if d.token.is_some() {
                    d.memory.queue_want(*r, WantOp::Add);
                }
            }
            let err = d.save_memory();
            self.dig_notify(err);
            self.dig_retry_wantlist();
        }
        let Some(d) = &mut self.dig else { return };
        let anon = d.token.is_none();
        let text = if records.len() == 1 {
            match (added.first(), owned.first()) {
                (Some((_, e)), _) if anon && d.settings.connect_hint_dismissed => format!(
                    "Added {} to your wantlist (a Discogs token would add it to your Discogs \
                     wantlist too: Options › Discogs…)",
                    record_name(e)
                ),
                (Some((_, e)), _) => format!("Added {} to your wantlist", record_name(e)),
                (None, Some((name, what))) => {
                    format!("{name} is already in your collection ({what})")
                }
                (None, None) => format!("{} is on your wantlist", record_name(&records[0].1)),
            }
        } else {
            let mut text = format!("Added {} to your wantlist", records_label(added.len()));
            let mut skipped = Vec::new();
            if !owned.is_empty() {
                skipped.push(format!(
                    "{} skipped (already in your collection)",
                    owned.len()
                ));
            }
            if already > 0 {
                skipped.push(format!("{already} already on it"));
            }
            if local > 0 {
                skipped.push(format!("{local} not from Discogs"));
            }
            if !skipped.is_empty() {
                text += "; ";
                text += &skipped.join(", ");
            }
            text
        };
        if anon && !added.is_empty() && !d.settings.connect_hint_dismissed {
            d.connect = Some(ConnectDialog {
                from_wantlist: true,
            });
        }
        self.notify(text);
    }

    /// Remove from wantlist: the records leave the wantlist crate and the Discogs wantlist,
    /// whoever put them there.
    fn dig_unwant(&mut self, c: CrateId, ids: &[EntryId]) {
        let (records, _) = self.dig_records(c, ids);
        let Some(d) = &self.dig else { return };
        let gone: Vec<(u64, Entry)> = records
            .into_iter()
            .filter(|(r, _)| d.memory.is_wanted(*r))
            .collect();
        if gone.is_empty() {
            return;
        }
        let set: HashSet<u64> = gone.iter().map(|(r, _)| *r).collect();
        self.leave_wantlist_crate(&set);
        self.dig_forget_wanted(&set);
        self.notify(match gone.as_slice() {
            [(_, e)] => format!("Removed {} from your wantlist", record_name(e)),
            _ => format!("Removed {} from your wantlist", records_label(gone.len())),
        });
    }

    /// `releases` are no longer wanted: here, and (with a token) on Discogs.
    fn dig_forget_wanted(&mut self, releases: &HashSet<u64>) {
        let Some(d) = &mut self.dig else { return };
        for &r in releases {
            if d.memory.wanted.remove(&r) && d.token.is_some() {
                d.memory.queue_want(r, WantOp::Remove);
            }
        }
        let err = d.save_memory();
        self.dig_notify(err);
        self.dig_retry_wantlist();
    }

    fn dig_wantlist_result(&mut self, release: u64, add: bool, result: Result<bool, ApiError>) {
        let Some(d) = &mut self.dig else { return };
        let op = if add { WantOp::Add } else { WantOp::Remove };
        d.want_inflight.remove(&(release, op));
        let message = match result {
            Ok(_) => {
                d.memory.want_done(release, op);
                // The user changed their mind while it was on its way.
                let wanted = d.memory.is_wanted(release);
                if add != wanted && !d.memory.is_want_pending(release) {
                    d.memory
                        .queue_want(release, if wanted { WantOp::Add } else { WantOp::Remove });
                }
                None
            }
            Err(ApiError::Offline) => {
                d.memory
                    .want_failed(release, op, now_secs(), true, "Discogs offline");
                None
            }
            Err(e @ ApiError::Other(_)) => d
                .memory
                .want_failed(release, op, now_secs(), false, &e.message())
                .map(|err| format!("Wantlist change failed for release {release}: {err}")),
            Err(e) => {
                d.memory.want_done(release, op);
                Some(e.message())
            }
        };
        let err = d.save_memory();
        self.dig_notify(message);
        self.dig_notify(err);
        self.dig_retry_wantlist();
    }

    /// Add to collection: one copy of each record not owned in this pressing (with `check`,
    /// a retry that first asks Discogs whether the add went through).
    fn dig_collect(&mut self, c: CrateId, ids: &[EntryId], check: bool) {
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none() {
            d.connect = Some(ConnectDialog {
                from_wantlist: false,
            });
            return;
        }
        let (records, _) = self.dig_records(c, ids);
        let Some(d) = &self.dig else { return };
        let todo: Vec<(u64, Entry)> = records
            .into_iter()
            .filter(|(r, e)| {
                !matches!(d.owned(e), Some(Owned::ThisPressing)) && !d.collecting.contains_key(r)
            })
            .collect();
        let Some(d) = &mut self.dig else { return };
        for (r, _) in &todo {
            d.collecting.insert(*r, c);
            d.collect_failed.remove(r);
            d.send(Command::Collect { release: *r, check });
        }
        match todo.as_slice() {
            [] => {}
            [(_, e)] => self.notify(format!("Adding {} to your collection…", record_name(e))),
            _ => self.notify(format!(
                "Adding {} to your collection…",
                records_label(todo.len())
            )),
        }
    }

    /// Discogs took (or refused) a collection add.
    fn dig_collected(&mut self, release: u64, result: Result<(u64, Pressing), ApiError>) {
        let Some(d) = &mut self.dig else { return };
        let from = d.collecting.remove(&release);
        let set = HashSet::from([release]);
        let name = from
            .and_then(|c| {
                let p = self.crates.get(c)?;
                p.entries().iter().find(|e| release_of(e) == Some(release))
            })
            .map(record_name)
            .unwrap_or_else(|| format!("release {release}"));
        let Some(d) = &mut self.dig else { return };
        let (instance, pressing) = match result {
            Ok(ok) => ok,
            Err(e) => {
                let msg = e.message();
                d.collect_failed
                    .insert(release, (from.unwrap_or(PLAYLIST), msg.clone()));
                return self.notify(format!("Could not add {name} to your collection: {msg}"));
            }
        };
        // Owned from now on, without a sync.
        let user = d.identity.as_ref().map(|i| i.username.clone());
        let mut c = d
            .collection
            .as_deref()
            .cloned()
            .unwrap_or_else(|| Collection::new(&user.unwrap_or_default(), now_secs()));
        if d.collection_syncing {
            d.local_adds.push((instance, release, pressing.clone()));
        }
        c.insert(instance, release, pressing);
        let saved = d
            .setup
            .cache_root
            .as_deref()
            .and_then(|root| c.save(root).err())
            .map(|e| format!("Could not save the collection: {e}"));
        d.collection = Some(Arc::new(c));
        d.share_collection();
        self.dig_notify(saved);
        // Bought: off the wantlist, out of its crate, into the collection's.
        let copies = from.map(|c| (c, self.entries_of(c, &set)));
        if let (Some(coll), Some((src, ids))) = (self.collection_crate(), copies)
            && src != coll
            && self.crates.send(src, &ids, coll).is_ok_and(|n| n > 0)
        {
            self.mark_crate(coll);
        }
        self.leave_wantlist_crate(&set);
        self.dig_forget_wanted(&set);
        self.notify(format!("Added {name} to your collection"));
    }

    /// Remove from collection…: asks first, naming the record and its pressing.
    fn dig_ask_discard(&mut self, c: CrateId, id: EntryId) {
        let Some(e) = self.crates.get(c).and_then(|p| p.get(id)) else {
            return;
        };
        let Some(release) = release_of(e) else { return };
        let mut name = if e.artist.is_empty() {
            record_name(e)
        } else {
            format!("{} – {}", e.artist, record_name(e))
        };
        let pressing: Vec<String> = e
            .origin
            .iter()
            .flat_map(|o| {
                [
                    o.catno.clone(),
                    o.year.map(|y| y.to_string()).unwrap_or_default(),
                ]
            })
            .filter(|s| !s.is_empty())
            .collect();
        if !pressing.is_empty() {
            name += &format!(" ({})", pressing.join(", "));
        }
        if let Some(d) = &mut self.dig {
            d.confirm_discard = Some(DiscardConfirm { release, name });
        }
    }

    /// Discogs answered a collection removal: the cached collection follows at once, and the
    /// record leaves the collection crate when no copy is left.
    fn dig_discarded(&mut self, release: u64, result: Result<Discarded, ApiError>) {
        let Some(d) = &mut self.dig else { return };
        d.discard_inflight.remove(&release);
        let name = self
            .collection_crate()
            .and_then(|c| {
                let p = self.crates.get(c)?;
                p.entries().iter().find(|e| release_of(e) == Some(release))
            })
            .map(record_name)
            .unwrap_or_else(|| format!("release {release}"));
        let Some(d) = &mut self.dig else { return };
        let done = match result {
            Ok(done) => done,
            Err(ApiError::Offline) => {
                d.memory
                    .discard_failed(release, now_secs(), true, "Discogs offline");
                let e = d.save_memory();
                return self.dig_notify(e);
            }
            Err(e @ ApiError::Other(_)) => {
                let failed = d
                    .memory
                    .discard_failed(release, now_secs(), false, &e.message())
                    .map(|err| format!("Could not remove {name} from your collection: {err}"));
                let e = d.save_memory();
                self.dig_notify(failed);
                return self.dig_notify(e);
            }
            Err(e) => {
                d.memory.discard_done(release);
                let err = d.save_memory();
                self.dig_notify(err);
                return self.notify(format!(
                    "Could not remove {name} from your collection: {}",
                    e.message()
                ));
            }
        };
        d.memory.discard_done(release);
        let err = d.save_memory();
        let user = d.identity.as_ref().map(|i| i.username.clone());
        let mut c = d
            .collection
            .as_deref()
            .cloned()
            .unwrap_or_else(|| Collection::new(&user.unwrap_or_default(), now_secs()));
        if d.collection_syncing {
            d.local_discards
                .push((done.instance, release, done.remaining));
        }
        c.discard(done.instance, release, done.remaining);
        let saved = d
            .setup
            .cache_root
            .as_deref()
            .and_then(|root| c.save(root).err())
            .map(|e| format!("Could not save the collection: {e}"));
        d.collection = Some(Arc::new(c));
        d.share_collection();
        self.dig_notify(err);
        self.dig_notify(saved);
        if done.remaining > 0 {
            let left = done.remaining;
            return self.notify(format!(
                "Removed a copy of {name} from your collection ({left} left)"
            ));
        }
        if let Some(coll) = self.collection_crate() {
            let ids = self.entries_of(coll, &HashSet::from([release]));
            if let Some(p) = self.crates.get_mut(coll)
                && p.remove_ids(&ids) > 0
            {
                self.mark_crate(coll);
            }
        }
        self.notify(format!("Removed {name} from your collection"));
    }

    fn dig_pass(&mut self, c: CrateId, id: EntryId) {
        let Some(e) = self.crates.get(c).and_then(|p| p.get(id)).cloned() else {
            return;
        };
        let key = key_of(&e);
        let Some(d) = &mut self.dig else { return };
        if release_of(&e).is_some_and(|r| d.memory.is_wanted(r)) {
            return self.notify(format!(
                "{} is on your wantlist (Y removes it)",
                record_name(&e)
            ));
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

    // ---- the wantlist crate and the account ------------------------------------------------

    /// Once, on the first launch after wanting replaced keeping: the Keepers crate becomes
    /// the wantlist crate, named "Wantlist" unless that name is taken.
    fn dig_migrate_keepers(&mut self) {
        let Some(d) = &self.dig else { return };
        if d.settings.wantlist_named {
            return;
        }
        let id = d
            .settings
            .wantlist
            .filter(|&id| self.crates.info(id).is_some())
            .or_else(|| self.crates.find(KEEPERS));
        if let Some(id) = id
            && self.crates.name(id).eq_ignore_ascii_case(KEEPERS)
            && self.crates.find(WANTLIST).is_none()
        {
            let _ = self.crates.rename(id, WANTLIST);
        }
        let Some(d) = &mut self.dig else { return };
        d.settings.wantlist = id;
        d.settings.wantlist_named = true;
        let e = d.save_settings();
        self.dig_notify(e);
    }

    /// Once the account is known (each session, and after a new token): the wantlist crate
    /// is "Wantlist: ‹user›" in the DISCOGS group (merged into one sent before), and the
    /// Discogs wantlist is read to follow it.
    fn dig_connect_wantlist(&mut self) {
        let shown = self.crates.shown_id();
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none() || !d.started() {
            return;
        }
        // Something to follow (wanted records, changes waiting, the crate on screen), and no
        // dig has asked whose token it is yet: ask, once.
        let something = !d.memory.wanted.is_empty()
            || !d.memory.wantlist_pending.is_empty()
            || d.settings.wantlist == Some(shown);
        if d.identity.is_none() && something && !d.identify_asked {
            d.identify_asked = true;
            d.send(Command::Identify);
        }
        let Some(id) = &d.identity else {
            return;
        };
        if !d.started() || d.connected.as_deref() == Some(id.username.as_str()) {
            return;
        }
        let user = id.username.clone();
        let name: String = format!("{WANTLIST}: {user}")
            .chars()
            .take(MAX_NAME)
            .collect();
        let local = d
            .settings
            .wantlist
            .filter(|&id| self.crates.info(id).is_some());
        let existing = self.crates.find(&name);
        let wl = match (local, existing) {
            (Some(l), Some(e)) if l != e => {
                let ids: Vec<EntryId> = if self.crates.load(l) {
                    self.crates
                        .get(l)
                        .map(|p| p.entries().iter().map(|e| e.id).collect())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                match self.crates.send(l, &ids, e) {
                    Ok(_) => {
                        self.mark_crate(e);
                        // The merged entries are on disk before their old crate goes (one
                        // that plays stays, unmarked).
                        let failed = self.crates.save_due(true, Duration::ZERO);
                        if failed.is_empty() && l != self.crates.playing_id() {
                            self.delete_crate(l);
                        }
                    }
                    Err(err) => self.notify(err),
                }
                e
            }
            (Some(l), _) => {
                if self.crates.name(l) != name {
                    let _ = self.crates.rename(l, &name);
                }
                l
            }
            (None, Some(e)) => e,
            (None, None) => match self.crates.create(&name) {
                Ok(id) => id,
                Err(err) => return self.notify(err),
            },
        };
        self.crates.set_wantlist(wl, true);
        let Some(d) = &mut self.dig else { return };
        d.connected = Some(user);
        d.settings.wantlist = Some(wl);
        let e = d.save_settings();
        d.send(Command::ReadWants { fresh: false });
        self.dig_notify(e);
    }

    /// The Discogs wantlist arrived. For a newly connected account, the records wanted here
    /// are added to it; otherwise the crate follows it (records removed on discogs.com
    /// leave). Records missing from the crate are fetched into it.
    fn dig_wants(&mut self, result: Result<Vec<u64>, ApiError>) {
        let refresh = self
            .dig
            .as_mut()
            .is_some_and(|d| std::mem::take(&mut d.refreshing_wantlist));
        let wants = match result {
            Ok(w) => w,
            Err(e) => {
                if refresh {
                    self.notify(format!("Refresh failed: {}", e.message()));
                }
                return;
            }
        };
        let Some(wl) = self.dig.as_ref().and_then(|d| d.settings.wantlist) else {
            return;
        };
        if !self.crates.load(wl) {
            return;
        }
        let remote: HashSet<u64> = wants.into_iter().collect();
        let masters: HashMap<u64, Option<u64>> = self
            .crates
            .get(wl)
            .map(|p| {
                p.entries()
                    .iter()
                    .filter_map(|e| Some((release_of(e)?, e.origin.as_ref()?.master)))
                    .collect()
            })
            .unwrap_or_default();
        let Some(d) = &mut self.dig else { return };
        let Some(user) = d.identity.as_ref().map(|i| i.username.clone()) else {
            return;
        };
        let first = d.memory.synced_user.as_deref() != Some(user.as_str());
        let local: Vec<u64> = d.memory.wanted.iter().copied().collect();
        let mut gone = HashSet::new();
        for r in local.into_iter().filter(|r| !remote.contains(r)) {
            if d.memory.is_want_pending(r) || d.memory.want_failure(r).is_some() {
                continue;
            }
            let owned = d.collection.as_ref().is_some_and(|c| {
                c.owned(Some(r), masters.get(&r).copied().flatten())
                    .is_some()
            });
            if first && !owned {
                d.memory.queue_want(r, WantOp::Add);
            } else {
                d.memory.wanted.remove(&r);
                gone.insert(r);
            }
        }
        let removing: HashSet<u64> = d
            .memory
            .wantlist_pending
            .iter()
            .filter(|p| p.op == WantOp::Remove)
            .map(|p| p.release)
            .collect();
        d.memory
            .wanted
            .extend(remote.iter().filter(|r| !removing.contains(r)));
        d.memory.synced_user = Some(user.clone());
        let err = d.save_memory();
        self.dig_notify(err);
        self.leave_wantlist_crate(&gone);
        // Records wanted but not in the crate: fetched into it (the whole wantlist when many).
        let present: HashSet<u64> = self
            .crates
            .get(wl)
            .map(|p| p.entries().iter().filter_map(release_of).collect())
            .unwrap_or_default();
        let Some(d) = &self.dig else { return };
        let missing: Vec<u64> = d
            .memory
            .wanted
            .iter()
            .copied()
            .filter(|r| !present.contains(r))
            .collect();
        let new = missing.len();
        if missing.len() > FILL_ONE_BY_ONE {
            self.dig_fill(Page::new(url::PageKind::Wantlist(user)), wl);
        } else {
            for r in missing {
                self.dig_fill(Page::new(url::PageKind::Release(r)), wl);
            }
        }
        self.dig_retry_wantlist();
        if refresh {
            self.notify(changes_line("Wantlist", new, gone.len()));
        }
    }

    /// A sync found wanted records the user now owns: they leave the wantlist.
    fn dig_owned_wants(&mut self, releases: Vec<u64>) {
        let set: HashSet<u64> = releases.into_iter().collect();
        if let Some(d) = &mut self.dig {
            // On Discogs even if this app didn't know they were wanted.
            for &r in &set {
                d.memory.wanted.insert(r);
            }
        }
        self.leave_wantlist_crate(&set);
        self.dig_forget_wanted(&set);
        self.notify(format!(
            "Removed {} you now own from your wantlist",
            records_label(set.len())
        ));
    }

    /// Before entries of the shown crate are removed. Without a token, the wantlist crate is
    /// the user's own: a record that leaves it entirely is no longer wanted. (Connected, it
    /// takes no hand removals: see [`DiggrApp::refuse_discogs_edit`].)
    pub(super) fn dig_before_remove(&mut self, ids: &[EntryId]) {
        let c = self.crates.shown_id();
        let Some(d) = &self.dig else { return };
        if d.token.is_some() || d.settings.wantlist != Some(c) {
            return;
        }
        let p = self.crates.shown();
        let leaving: HashSet<u64> = p
            .entries()
            .iter()
            .filter(|e| ids.contains(&e.id))
            .filter_map(release_of)
            .filter(|&r| d.memory.is_wanted(r))
            .filter(|&r| {
                p.entries()
                    .iter()
                    .all(|e| release_of(e) != Some(r) || ids.contains(&e.id))
            })
            .collect();
        if !leaving.is_empty() {
            self.dig_forget_wanted(&leaving);
        }
    }

    // ---- the entry menu ------------------------------------------------------------------

    pub(super) fn dig_entry_menu(&self, ui: &mut egui::Ui, e: &Entry, actions: &mut Vec<Action>) {
        let Some(d) = &self.dig else { return };
        let shown = self.crates.shown();
        let ids = if shown.is_selected(e.id) {
            shown.selected_ids()
        } else {
            vec![e.id]
        };
        let c = self.crates.shown_id();
        let (records, _) = self.dig_records(c, &ids);
        let marks = d.marks(e);
        // In the collection crate everything is owned: nothing to want or collect.
        let collection = self.crates.is_collection(c);
        let discogs = self.crates.is_discogs(c);
        ui.separator();
        let mut item = |ui: &mut egui::Ui, label: String, a: Option<DigAction>, tip: &str| {
            let r = ui.add_enabled(a.is_some(), egui::Button::new(label));
            let r = if tip.is_empty() {
                r
            } else {
                r.on_hover_text(tip).on_disabled_hover_text(tip)
            };
            if r.clicked()
                && let Some(a) = a
            {
                actions.push(Action::Dig(a));
                ui.close();
            }
        };
        if let Some(r) = release_of(e).filter(|_| !collection) {
            let owned = d.owned(e);
            let what = owned.as_ref().map(owned_text).unwrap_or_default();
            if records.len() > 1 {
                let n = records.len();
                if marks.wanted {
                    let label = format!("Remove {} from wantlist", records_label(n));
                    item(ui, label, Some(DigAction::Unwant(ids.clone())), "");
                } else {
                    let label = format!("Add {} to wantlist", records_label(n));
                    item(ui, label, Some(DigAction::Want(ids.clone())), "");
                }
                let label = format!("Add {} to collection", records_label(n));
                item(ui, label, Some(DigAction::Collect(ids.clone())), "");
            } else {
                if owned.is_some() {
                    item(ui, "In collection".into(), None, &format!("Owned: {what}"));
                } else if marks.wantlist_failed.is_some() {
                    item(
                        ui,
                        "Retry wantlist".into(),
                        Some(DigAction::RetryWant(e.id)),
                        "",
                    );
                } else if marks.wanted {
                    let a = DigAction::Unwant(vec![e.id]);
                    item(ui, "Remove from wantlist (Y)".into(), Some(a), "");
                } else {
                    let a = DigAction::Want(vec![e.id]);
                    item(ui, "Add to wantlist (Y)".into(), Some(a), "");
                }
                match &owned {
                    Some(Owned::ThisPressing) => {}
                    _ if d.collect_failed.contains_key(&r) => {
                        let a = DigAction::RetryCollect(e.id);
                        item(ui, "Retry add to collection".into(), Some(a), "");
                    }
                    _ if d.collecting.contains_key(&r) => {
                        item(ui, "Adding to collection…".into(), None, "");
                    }
                    Some(Owned::Another { .. }) => {
                        let label = format!("Add to collection (own {})", owned_short(&owned));
                        item(ui, label, Some(DigAction::Collect(vec![e.id])), "");
                    }
                    None => {
                        let a = DigAction::Collect(vec![e.id]);
                        item(ui, "Add to collection".into(), Some(a), "");
                    }
                }
            }
        }
        // The collection crate: one record at a time comes out, one copy, after a question.
        if collection
            && d.token.is_some()
            && records.len() <= 1
            && let Some(r) = release_of(e)
        {
            if let Some(why) = d.memory.discard_failure(r) {
                let a = DigAction::RetryDiscard(e.id);
                let tip = format!("The removal failed: {why}");
                item(ui, "Retry remove from collection".into(), Some(a), &tip);
            } else if d.memory.is_discard_pending(r) {
                item(ui, "Removing from collection…".into(), None, "");
            } else {
                let a = DigAction::Discard(e.id);
                item(ui, "Remove from collection…".into(), Some(a), "");
            }
        }
        // Passing on a record you want or own means nothing.
        if !discogs {
            if marks.passed {
                item(ui, "Undo pass".into(), Some(DigAction::UndoPass(e.id)), "");
            } else {
                item(ui, "Pass (N)".into(), Some(DigAction::Pass(e.id)), "");
            }
        }
        let a = DigAction::OpenForSale(e.id);
        item(ui, "Open for-sale page (I)".into(), Some(a), "");
        if let Some(url) = entry_record_url(e) {
            if ui.button("Open release on Discogs").clicked() {
                actions.push(Action::Dig(DigAction::OpenRecord(e.id)));
                ui.close();
            }
            if ui.button("Copy Discogs link").clicked() {
                ui.ctx().copy_text(url);
                ui.close();
            }
        }
    }

    // ---- Options ▸ Discogs… ------------------------------------------------------------------

    fn dig_open_dialog(&mut self) {
        let Some(d) = &mut self.dig else { return };
        d.dialog = Some(DigDialog {
            ytdlp_path: d.settings.ytdlp_path.clone().unwrap_or_default(),
            ..Default::default()
        });
        d.preview(PreviewCommand::CheckProgram);
    }

    pub(super) fn dig_token_checked(&mut self, token: String, result: Result<Identity, ApiError>) {
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
                    d.connected = None;
                    d.share_collection();
                    d.read_account_lists();
                    Ok(name)
                }
            }
            Err(ApiError::TokenRejected) => {
                Err("Discogs rejected the token; it wasn't saved".into())
            }
            Err(e) => Err(e.message()),
        };
        let user = outcome.as_ref().ok().cloned();
        if let Some(dialog) = &mut d.dialog {
            dialog.checking = false;
            if outcome.is_ok() {
                dialog.token.clear();
            }
            dialog.check = Some(outcome);
        }
        // A new account: its whole collection becomes a crate, on screen (once; an existing
        // "Collection: …" crate is left as it is).
        if let Some(user) = user {
            let page = Page::new(dig::discogs::url::PageKind::Collection(user));
            let name = page.provisional_name();
            if self.crates.find(&name).is_none() {
                self.dig_send(page, SendMode::Crate(name.clone()), None);
            }
            if let Some(c) = self.crates.find(&name) {
                self.crates.set_collection(c);
            }
        }
    }

    /// "Connect to Discogs": what a token gives, and how to get one.
    fn dig_connect_ui(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dig.as_ref().and_then(|d| d.connect.clone()) else {
            return;
        };
        let mut dismiss = false;
        let (mut close, mut connect, mut open_page) = (false, false, false);
        egui::Modal::new(egui::Id::new("connect-discogs")).show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.strong("Connect to Discogs");
            if dialog.from_wantlist {
                ui.label("Added to your wantlist here.");
            }
            ui.label(
                "If you have a collection on Discogs, you can keep it up to date from here: \
                 add records to your wantlist or collection while you dig, and see at a glance \
                 what you already own. It only takes connecting your account:",
            );
            ui.horizontal(|ui| {
                ui.label("1. Open your Discogs developer settings");
                if ui.button("Open ⬈").clicked() {
                    open_page = true;
                }
            });
            ui.label("2. Click \"Generate new token\" and copy it");
            ui.label("3. Paste it in Options › Discogs…");
            ui.separator();
            ui.horizontal(|ui| {
                if dialog.from_wantlist {
                    ui.checkbox(&mut dismiss, "Don't show this again");
                }
                if ui.button("Later").clicked() {
                    close = true;
                }
                if ui.button("Connect…").clicked() {
                    connect = true;
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if open_page {
            let err = self
                .dig
                .as_ref()
                .and_then(|d| d.setup.browser.open(TOKEN_PAGE).err());
            self.dig_notify(err.map(|e| format!("Could not open the browser: {e}")));
        }
        let Some(d) = &mut self.dig else { return };
        if dismiss {
            d.settings.connect_hint_dismissed = true;
            let e = d.save_settings();
            d.connect = None;
            self.dig_notify(e);
        } else if close || connect {
            d.connect = None;
        }
        if connect {
            self.dig_open_dialog();
            if let Some(dialog) = self.dig.as_mut().and_then(|d| d.dialog.as_mut()) {
                dialog.focus_token = 3;
            }
        }
    }

    /// "Remove 1 copy of ‹record› from your Discogs collection?": Remove (Enter) or Cancel
    /// (Esc).
    fn dig_confirm_discard_ui(&mut self, ctx: &egui::Context) {
        let Some(ask) = self.dig.as_ref().and_then(|d| d.confirm_discard.clone()) else {
            return;
        };
        let mut choice = None;
        egui::Modal::new(egui::Id::new("confirm-discard")).show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.strong(format!(
                "Remove 1 copy of {} from your Discogs collection?",
                ask.name
            ));
            ui.label("Its notes and rating on Discogs are lost.");
            ui.horizontal(|ui| {
                if ui.button("Remove").clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            choice = Some(true);
        } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            choice = Some(false);
        }
        let Some(remove) = choice else { return };
        let Some(d) = &mut self.dig else { return };
        d.confirm_discard = None;
        if remove {
            d.memory.queue_discard(ask.release);
            let e = d.save_memory();
            self.dig_notify(e);
            self.notify(format!("Removing {} from your collection…", ask.name));
            self.dig_retry_discards();
        }
    }

    /// A question is open (connect, or removing from the collection): shortcuts wait.
    pub(super) fn dig_asking(&self) -> bool {
        self.dig.as_ref().is_some_and(|d| {
            d.connect.is_some() || d.confirm_discard.is_some() || d.seller_ui.dialog.is_some()
        })
    }

    pub(super) fn dig_dialog_ui(&mut self, ctx: &egui::Context) {
        self.dig_connect_ui(ctx);
        self.dig_confirm_discard_ui(ctx);
        self.seller_dialogs_ui(ctx);
        let Some(d) = &mut self.dig else { return };
        let collection_line = d.collection_line();
        let collection_syncing = d.collection_syncing;
        let Some(dialog) = &mut d.dialog else { return };
        let mut open = true;
        let mut commands: Vec<Command> = Vec::new();
        let mut preview_cmds: Vec<PreviewCommand> = Vec::new();
        let mut remove_token = false;
        let mut settings_changed = false;
        let mut refresh_collection = false;
        let mut open_token_page = false;
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
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut dialog.token)
                            .password(true)
                            .hint_text("Personal access token")
                            .desired_width(200.0),
                    );
                    if dialog.focus_token > 0 {
                        field.request_focus();
                        if field.has_focus() {
                            dialog.focus_token -= 1;
                        }
                    }
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
                ui.horizontal(|ui| {
                    ui.label("Get one at discogs.com › Settings › Developers:");
                    if ui.button("Open that page").clicked() {
                        open_token_page = true;
                    }
                });

                ui.separator();
                ui.strong("Collection");
                ui.label(&collection_line);
                if d.token.is_some()
                    && ui
                        .add_enabled(!collection_syncing, egui::Button::new("Refresh collection"))
                        .on_hover_text("Fetches only what changed since the last sync")
                        .clicked()
                {
                    refresh_collection = true;
                }
                ui.label("Records you own are marked OWNED in the playlist.");

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
            d.connected = None;
            commands.push(Command::SetToken(None));
            d.share_collection();
            // Unconnected, the wantlist crate is one of the user's own again.
            if let Some(wl) = d.settings.wantlist {
                self.crates.set_wantlist(wl, false);
            }
        }
        if refresh_collection {
            d.sync_collection();
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
        if open_token_page {
            let err = self
                .dig
                .as_ref()
                .and_then(|d| d.setup.browser.open(TOKEN_PAGE).err());
            self.dig_notify(err.map(|e| format!("Could not open the browser: {e}")));
        }
    }

    // ---- Options ▸ Browser… ------------------------------------------------------------------

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
        d.quiet.remove(&id);
        if d.settings.wantlist == Some(id) {
            d.settings.wantlist = None;
            let _ = d.save_settings();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(release: u64, artist: &str) -> RecordInfo {
        RecordInfo {
            key: RecordKey::Release(release),
            release: Some(release),
            master: None,
            artist: artist.into(),
            title: "Night Moves".into(),
            label: "Lowtide Tapes".into(),
            catno: "LT-020".into(),
            year: Some(1996),
            for_sale: None,
            cover: String::new(),
            styles: "Deep House".into(),
            formats: vec![Format::Vinyl],
        }
    }

    #[test]
    fn every_track_of_a_compilation_carries_the_record_artist() {
        let base = origin("https://www.discogs.com/release/7", &info(7, "Various"));
        let mut p = Playlist::default();
        for (artist, clip) in [("Nightcraft", "a"), ("Lumen", "b")] {
            let o = Origin {
                clip: Some(clip.into()),
                ..base.clone()
            };
            p.add_waiting(artist, "t", None, Some(o), "queued");
        }
        for (e, own) in p.entries().iter().zip(["Nightcraft", "Lumen"]) {
            assert_eq!(e.origin.as_ref().unwrap().artist, "Various");
            assert_eq!(e.artist, own, "the track's own credit stays");
        }
    }

    #[test]
    fn the_record_artist_is_backfilled_only_where_missing() {
        let mut p = Playlist::default();
        for (release, artist) in [(7, ""), (8, "Kept")] {
            let o = Origin {
                release: Some(release),
                album: "Album".into(),
                styles: "Techno".into(),
                artist: artist.into(),
                formats: "Vinyl".into(),
                ..Default::default()
            };
            p.add_waiting("X", "t", None, Some(o), "queued");
        }
        assert_eq!(records_to_backfill(&p), [RecordKey::Release(7)]);
        assert!(backfill(&mut p, &[info(7, "Various"), info(8, "Other")]));
        let artists: Vec<&str> = p
            .entries()
            .iter()
            .map(|e| e.origin.as_ref().unwrap().artist.as_str())
            .collect();
        assert_eq!(artists, ["Various", "Kept"]);
    }

    #[test]
    fn formats_are_kept_as_text_and_backfilled() {
        assert_eq!(formats_text(&[Format::Vinyl, Format::Cd]), "Vinyl, CD");
        let o = origin("p", &info(7, "Various"));
        assert_eq!(o.formats, "Vinyl");
        let mut p = Playlist::default();
        let saved = Origin {
            release: Some(9),
            album: "A".into(),
            styles: "Techno".into(),
            artist: "X".into(),
            ..Default::default()
        };
        p.add_waiting("X", "t", None, Some(saved), "queued");
        assert_eq!(records_to_backfill(&p), [RecordKey::Release(9)]);
        let mut digital = info(9, "X");
        digital.formats = vec![Format::File];
        assert!(backfill(&mut p, &[digital]));
        assert_eq!(p.entries()[0].origin.as_ref().unwrap().formats, "File");
    }
}
