//! The intake worker: every Discogs request, one at a time, on one low-priority thread.
//!
//! A send becomes a [`Job`]: first the page's name, then its listing, 100 records a request,
//! each record reported at once as "listed"; then each record's details, nearest the playhead
//! first, reported as its clip entries, "no clip", or left out by the vinyl filter. Jobs are
//! saved as they go, so a send interrupted by quitting resumes when its crate is next shown.
//!
//! [`Intake`] is the synchronous core (tests drive it step by step); [`IntakeHandle`] runs it
//! on a thread and talks to the UI over channels.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::Duration;

use platform::{Priority, Spawner};

use crate::collection::{self, Collection};
use crate::discogs::client::{ApiError, Client, Identity, path_segment};
use crate::discogs::expand::{self, PER_PAGE};
use crate::discogs::matching::{self, ClipEntry};
use crate::discogs::model::{ForSale, Listed, Record, RecordKey, Role};
use crate::discogs::transport::Method;
use crate::discogs::url::{Page, PageKind};
use crate::jobs::{Filters, Job, JobId, Jobs};

/// While Discogs doesn't answer, the same request is tried again this often.
pub const OFFLINE_RETRY: Duration = Duration::from_secs(15);
/// Progress is written to `jobs.ron` at most this often while details are fetched.
const SAVE_EVERY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Expand a page into crate `target`.
    Send {
        page: Page,
        target: u64,
        filters: Filters,
    },
    /// A crate was shown or played: its unfinished jobs go on.
    CrateActive(u64),
    /// A crate was deleted: its jobs stop and are forgotten.
    CrateDeleted(u64),
    /// Where the user is in crate `target`: the listed records from the focus on, wrapping
    /// around. Their details are fetched in that order.
    Focus { target: u64, order: Vec<RecordKey> },
    /// Fresh marketplace numbers for a release (its track started and they are a day old).
    Refresh(u64),
    /// A saved token (or none) to use from now on.
    SetToken(Option<String>),
    /// Check a token before it is saved; the current one stays in use.
    CheckToken(String),
    /// Add a kept release to the user's wantlist (unless it is already there).
    Keep(u64),
    /// Remove a release this app added to the wantlist.
    Unkeep(u64),
    /// Bring the user's collection up to date from this cached one (or from nothing). Runs
    /// only when no send is waiting.
    SyncCollection(Option<Box<Collection>>),
    /// Learn which release a marketplace item sells (for the browser's owned check). Runs
    /// only when no send is waiting; cached for good.
    ResolveShopItem(u64),
    /// Details of crate `target`'s records saved before entries carried them (the album and
    /// cover), from the disk cache only: never a request.
    Backfill { target: u64, keys: Vec<RecordKey> },
    /// A record's cover address stopped working: fetch its data again for a newer one.
    RefreshCover(RecordKey),
}

/// What became of a listed record.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// One entry per usable clip (possibly none left after a remix-credit filter).
    Clips(Vec<ClipEntry>),
    /// Unavailable, with the reason: "no clip", "not found".
    Unavailable(String),
    /// Not vinyl, with vinyl only on: it leaves the crate.
    Excluded,
}

/// A record's details, as the entries' origin carries them.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordInfo {
    pub key: RecordKey,
    pub release: Option<u64>,
    pub master: Option<u64>,
    pub artist: String,
    pub title: String,
    pub label: String,
    pub catno: String,
    pub year: Option<u16>,
    pub for_sale: Option<ForSale>,
    /// The record's thumbnail address (empty when unknown).
    pub cover: String,
}

impl RecordInfo {
    fn from_record(r: &Record) -> Self {
        Self {
            key: r.key,
            release: r.release,
            master: r.master,
            artist: r.artist.clone(),
            title: r.title.clone(),
            label: r.label.clone(),
            catno: r.catno.clone(),
            year: r.year,
            for_sale: r.for_sale.clone(),
            cover: r.cover.clone(),
        }
    }

    pub fn from_listed(l: &Listed) -> Self {
        let (release, master) = match l.key {
            RecordKey::Release(id) => (Some(id), None),
            RecordKey::Master(id) => (None, Some(id)),
        };
        Self {
            key: l.key,
            release,
            master,
            artist: l.artist.clone(),
            title: l.title.clone(),
            label: l.label.clone(),
            catno: l.catno.clone(),
            year: l.year,
            for_sale: None,
            cover: l.cover.clone(),
        }
    }
}

/// Which send an event belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct JobRef {
    pub id: JobId,
    pub target: u64,
    /// The page's address (entries' origin).
    pub page: String,
    pub name: String,
    pub filters: Filters,
}

// Events are few (one per record at most), so their size doesn't matter.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A send was accepted (with a provisional name).
    Started(JobRef),
    /// The page's name arrived from Discogs.
    Named(JobRef),
    /// Records from a listing page, in listing order; each becomes a "listed" entry.
    Listed(JobRef, Vec<Listed>),
    /// A listed record's details.
    Record(JobRef, RecordInfo, Outcome),
    /// Records finished of the listing's total.
    Progress(JobRef, usize, usize),
    Finished(JobRef),
    /// The send stopped (page not found, private, token needed); nothing more comes.
    Failed(JobRef, ApiError),
    /// Discogs stopped (true) or started (false) answering.
    Offline(bool),
    ForSale(u64, ForSale),
    /// The saved token's account, or why it can't be used.
    Identity(Result<Identity, ApiError>),
    TokenChecked(String, Result<Identity, ApiError>),
    /// A collection sync finished: the up-to-date collection, or why not (the previous one
    /// stays in use).
    Collection(Result<Box<Collection>, ApiError>),
    /// A marketplace item's release (`None`: Discogs doesn't know the item).
    ShopItem(u64, Option<u64>),
    /// Cached details for a crate's records (see [`Command::Backfill`]); uncached ones are
    /// left out.
    Backfill(u64, Vec<RecordInfo>),
    /// A record's cover address from fresh data (empty when it has no image any more, or the
    /// data couldn't be fetched).
    Cover(RecordKey, String),
    /// A wantlist change: `Ok(true)` if the app changed the wantlist, `Ok(false)` if the
    /// release was already there.
    Wantlist {
        release: u64,
        add: bool,
        result: Result<bool, ApiError>,
    },
}

pub struct Intake {
    client: Client,
    /// Where `jobs.ron` lives; `None` keeps jobs in memory.
    config: Option<PathBuf>,
    jobs: Jobs,
    /// Jobs whose crate was shown or played this session.
    active: HashSet<JobId>,
    focus: Option<u64>,
    identity_checked: bool,
    wants: Option<HashSet<u64>>,
    offline: bool,
    last_save: Option<Duration>,
    unsaved: bool,
    events: Vec<Event>,
    now: fn() -> u64,
    /// Waiting for a moment with no send: a collection sync, and items to resolve.
    collection_sync: Option<Option<Box<Collection>>>,
    shop_items: Vec<u64>,
}

impl Intake {
    /// Loads the unfinished jobs from `config` (none run until their crate is active).
    pub fn new(client: Client, config: Option<PathBuf>) -> Self {
        let jobs = config.as_deref().map(Jobs::load).unwrap_or_default();
        Self {
            client,
            config,
            jobs,
            active: HashSet::new(),
            focus: None,
            identity_checked: false,
            wants: None,
            offline: false,
            last_save: None,
            unsaved: false,
            events: Vec::new(),
            now: crate::now_secs,
            collection_sync: None,
            shop_items: Vec::new(),
        }
    }

    /// Epoch seconds for cache freshness, for tests.
    pub fn with_now(mut self, now: fn() -> u64) -> Self {
        self.now = now;
        self
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs.jobs
    }

    pub fn is_offline(&self) -> bool {
        self.offline
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn job_ref(job: &Job) -> JobRef {
        JobRef {
            id: job.id,
            target: job.target,
            page: job.page.url(),
            name: job.name.clone(),
            filters: job.filters,
        }
    }

    pub fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::Send {
                page,
                target,
                filters,
            } => {
                let id = self.jobs.alloc_id();
                let job = Job::new(id, page, target, filters);
                self.events.push(Event::Started(Self::job_ref(&job)));
                self.jobs.jobs.push(job);
                self.active.insert(id);
                self.save();
            }
            Command::CrateActive(target) => {
                for j in &self.jobs.jobs {
                    if j.target == target && self.active.insert(j.id) {
                        self.events.push(Event::Started(Self::job_ref(j)));
                    }
                }
            }
            Command::CrateDeleted(target) => {
                self.jobs.jobs.retain(|j| j.target != target);
                self.save();
            }
            Command::Focus { target, order } => {
                self.focus = Some(target);
                for j in self.jobs.jobs.iter_mut().filter(|j| j.target == target) {
                    reorder(&mut j.pending, &order);
                }
            }
            Command::Refresh(release) => {
                let now = (self.now)();
                self.ensure_identity();
                match expand::record(&mut self.client, RecordKey::Release(release), true, now) {
                    Ok(r) => {
                        self.online();
                        if let Some(fs) = r.for_sale {
                            self.events.push(Event::ForSale(release, fs));
                        }
                    }
                    Err(ApiError::Offline) => self.went_offline(),
                    Err(_) => {}
                }
            }
            Command::SetToken(token) => {
                self.client.set_token(token);
                self.identity_checked = false;
                self.wants = None;
            }
            Command::CheckToken(token) => {
                let saved = self.client.token().map(str::to_owned);
                self.client.set_token(Some(token.clone()));
                let result = self.client.check_identity();
                if result.is_ok() {
                    self.identity_checked = true;
                    self.wants = None;
                } else {
                    self.client.set_token(saved);
                    self.identity_checked = false;
                }
                self.events.push(Event::TokenChecked(token, result));
            }
            Command::Keep(release) => self.wantlist(release, true),
            Command::Unkeep(release) => self.wantlist(release, false),
            Command::SyncCollection(cached) => self.collection_sync = Some(cached),
            Command::ResolveShopItem(id) => {
                if !self.shop_items.contains(&id) {
                    self.shop_items.push(id);
                }
            }
            Command::RefreshCover(key) => {
                let now = (self.now)();
                self.ensure_identity();
                let cover = match expand::fresh_cover(&mut self.client, key, now) {
                    Ok(r) => {
                        self.online();
                        if let (Some(release), Some(fs)) = (r.release, r.for_sale.clone()) {
                            self.events.push(Event::ForSale(release, fs));
                        }
                        r.cover
                    }
                    Err(ApiError::Offline) => {
                        self.went_offline();
                        String::new()
                    }
                    Err(_) => String::new(),
                };
                self.events.push(Event::Cover(key, cover));
            }
            Command::Backfill { target, keys } => {
                let infos: Vec<RecordInfo> = keys
                    .into_iter()
                    .filter_map(|k| expand::cached_record(&self.client.cache, k))
                    .map(|r| RecordInfo::from_record(&r))
                    .collect();
                if !infos.is_empty() {
                    self.events.push(Event::Backfill(target, infos));
                }
            }
        }
    }

    /// Work that waits for a moment with no send: an item someone is looking at first, then
    /// the collection. False when there was none.
    fn background_step(&mut self) -> bool {
        let now = (self.now)();
        if let Some(id) = self.shop_items.first().copied() {
            match expand::shop_item_release(&mut self.client, id, now) {
                Err(ApiError::Offline) => {
                    self.went_offline();
                    self.client.sleep(OFFLINE_RETRY);
                    return true;
                }
                r => {
                    self.online();
                    self.shop_items.remove(0);
                    self.events.push(Event::ShopItem(id, r.ok()));
                }
            }
            return true;
        }
        let Some(cached) = self.collection_sync.take() else {
            return false;
        };
        self.ensure_identity();
        let Some(user) = self.client.identity().map(|i| i.username.clone()) else {
            self.events
                .push(Event::Collection(Err(ApiError::TokenNeeded)));
            return true;
        };
        let result = collection::sync(&mut self.client, &user, cached.as_deref(), now);
        match &result {
            Ok(_) => self.online(),
            Err(ApiError::Offline) => self.went_offline(),
            Err(_) => {}
        }
        self.events.push(Event::Collection(result.map(Box::new)));
        true
    }

    /// One request's worth of work. False when there is nothing to do.
    pub fn step(&mut self) -> bool {
        let Some(i) = self.next_job() else {
            self.save_if_due(true);
            return self.background_step();
        };
        let now = (self.now)();
        let job = &self.jobs.jobs[i];
        let result = if job.subject.is_none() {
            self.name_job(i, now)
        } else if !job.listing_done() {
            self.list_page(i, now)
        } else {
            self.expand_next(i, now)
        };
        match result {
            Ok(()) => self.online(),
            Err(ApiError::Offline) => {
                self.went_offline();
                self.save_if_due(true);
                self.client.sleep(OFFLINE_RETRY);
            }
            Err(e) => {
                let job = self.jobs.jobs.remove(i);
                self.events.push(Event::Failed(Self::job_ref(&job), e));
                self.save();
            }
        }
        if let Some(i) = self.jobs.jobs.iter().position(Job::is_finished) {
            let job = self.jobs.jobs.remove(i);
            self.events.push(Event::Finished(Self::job_ref(&job)));
            self.save();
        }
        self.save_if_due(false);
        true
    }

    /// Unfinished active jobs: the focused crate's first, then the oldest.
    fn next_job(&self) -> Option<usize> {
        let runnable = |j: &&Job| self.active.contains(&j.id) && !j.is_finished();
        // Every listing before any details, so a big page shows all its records early.
        let listing = self
            .jobs
            .jobs
            .iter()
            .position(|j| runnable(&j) && (j.subject.is_none() || !j.listing_done()));
        listing.or_else(|| {
            self.jobs
                .jobs
                .iter()
                .position(|j| runnable(&j) && Some(j.target) == self.focus)
                .or_else(|| self.jobs.jobs.iter().position(|j| runnable(&j)))
        })
    }

    fn name_job(&mut self, i: usize, now: u64) -> Result<(), ApiError> {
        let page = self.jobs.jobs[i].page.clone();
        if matches!(
            page.kind,
            PageKind::Release(_) | PageKind::Master(_) | PageKind::ShopItem(_)
        ) {
            self.ensure_identity();
        }
        let name = expand::page_name(&mut self.client, &page, now)?;
        let prefix = format!("{}: ", page.kind_name());
        let job = &mut self.jobs.jobs[i];
        job.subject = Some(name.strip_prefix(&prefix).unwrap_or(&name).to_owned());
        job.name = name;
        self.events.push(Event::Named(Self::job_ref(job)));
        Ok(())
    }

    fn list_page(&mut self, i: usize, now: u64) -> Result<(), ApiError> {
        let (page, n) = {
            let j = &self.jobs.jobs[i];
            (j.page.clone(), j.next_page)
        };
        let lp = match expand::listing(&mut self.client, &page, n, now) {
            Ok(lp) => lp,
            // A later page gone missing (the listing shrank): the listing is done.
            Err(ApiError::NotFound) if n > 1 => expand::ListingPage {
                items: Vec::new(),
                pages: n - 1,
                total: self.jobs.jobs[i].total,
            },
            Err(e) => return Err(e),
        };
        let job = &mut self.jobs.jobs[i];
        job.pages = Some(lp.pages);
        job.total = lp.total;
        job.next_page = n + 1;
        let (keep, out): (Vec<Listed>, Vec<Listed>) = lp
            .items
            .into_iter()
            .partition(|l| !(job.filters.vinyl_only && l.vinyl == Some(false)));
        job.done += out.len();
        job.pending.extend(keep.iter().cloned());
        let r = Self::job_ref(job);
        let (done, total) = (job.done, job.total);
        if !keep.is_empty() {
            self.events.push(Event::Listed(r.clone(), keep));
        }
        self.events.push(Event::Progress(r, done, total));
        self.save();
        Ok(())
    }

    fn expand_next(&mut self, i: usize, now: u64) -> Result<(), ApiError> {
        self.ensure_identity();
        let (listed, filters, subject) = {
            let j = &self.jobs.jobs[i];
            (
                j.pending[0].clone(),
                j.filters,
                j.subject.clone().unwrap_or_default(),
            )
        };
        let (info, outcome) = match expand::record(&mut self.client, listed.key, false, now) {
            Ok(rec) => {
                let info = RecordInfo::from_record(&rec);
                let outcome = if filters.vinyl_only && !rec.vinyl {
                    Outcome::Excluded
                } else {
                    let artist = if listed.role == Role::Remix {
                        subject.as_str()
                    } else {
                        ""
                    };
                    let entries = matching::entries(&rec, listed.role, artist);
                    if entries.is_empty() && (rec.clips.is_empty() || listed.role == Role::Main) {
                        Outcome::Unavailable("no clip".into())
                    } else {
                        Outcome::Clips(entries)
                    }
                };
                (info, outcome)
            }
            Err(ApiError::NotFound) => (
                RecordInfo::from_listed(&listed),
                Outcome::Unavailable("not found".into()),
            ),
            Err(ApiError::Other(e)) => (
                RecordInfo::from_listed(&listed),
                Outcome::Unavailable(format!("failed: {e}")),
            ),
            Err(e) => return Err(e),
        };
        let job = &mut self.jobs.jobs[i];
        job.pending.remove(0);
        job.done += 1;
        self.unsaved = true;
        let r = Self::job_ref(job);
        let (done, total) = (job.done, job.total);
        self.events.push(Event::Record(r.clone(), info, outcome));
        self.events.push(Event::Progress(r, done, total));
        Ok(())
    }

    /// With a token, the account's currency is needed before any price is asked for.
    fn ensure_identity(&mut self) {
        if self.identity_checked || self.client.token().is_none() {
            return;
        }
        match self.client.check_identity() {
            Ok(id) => {
                self.identity_checked = true;
                self.events.push(Event::Identity(Ok(id)));
            }
            Err(ApiError::Offline) => {}
            Err(e) => {
                // A bad token: carry on as without one, at the lower limit, and say so.
                self.identity_checked = true;
                self.client.set_token(None);
                self.events.push(Event::Identity(Err(e)));
            }
        }
    }

    fn wantlist(&mut self, release: u64, add: bool) {
        let result = self.change_wantlist(release, add);
        match &result {
            Err(ApiError::Offline) => self.went_offline(),
            Ok(_) => self.online(),
            _ => {}
        }
        self.events.push(Event::Wantlist {
            release,
            add,
            result,
        });
    }

    fn change_wantlist(&mut self, release: u64, add: bool) -> Result<bool, ApiError> {
        if self.client.token().is_none() {
            return Err(ApiError::TokenNeeded);
        }
        let user = self.client.ensure_identity()?.username;
        self.identity_checked = true;
        if self.wants.is_none() {
            self.wants = Some(self.read_wants(&user)?);
        }
        let path = format!("/users/{}/wants/{release}", path_segment(&user));
        if add {
            if self.wants.as_ref().is_some_and(|w| w.contains(&release)) {
                return Ok(false);
            }
            self.client.call(Method::Put, &path)?;
            self.wants.get_or_insert_default().insert(release);
        } else {
            match self.client.call(Method::Delete, &path) {
                Ok(_) | Err(ApiError::NotFound) => {}
                Err(e) => return Err(e),
            }
            self.wants.get_or_insert_default().remove(&release);
        }
        Ok(true)
    }

    /// The ids on the user's wantlist, once per session (100 a request).
    fn read_wants(&mut self, user: &str) -> Result<HashSet<u64>, ApiError> {
        let mut ids = HashSet::new();
        let mut n = 1;
        loop {
            let v = self.client.get_json(&format!(
                "/users/{}/wants?page={n}&per_page={PER_PAGE}",
                path_segment(user)
            ))?;
            ids.extend(
                v["wants"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|w| w["id"].as_u64()),
            );
            if n >= v["pagination"]["pages"].as_u64().unwrap_or(1) {
                return Ok(ids);
            }
            n += 1;
        }
    }

    fn went_offline(&mut self) {
        if !self.offline {
            self.offline = true;
            self.events.push(Event::Offline(true));
        }
    }

    fn online(&mut self) {
        if self.offline {
            self.offline = false;
            self.events.push(Event::Offline(false));
        }
    }

    fn save(&mut self) {
        self.unsaved = false;
        self.last_save = Some(self.client.clock().now());
        if let Some(c) = &self.config {
            let _ = self.jobs.save(c);
        }
    }

    fn save_if_due(&mut self, force: bool) {
        let due = self
            .last_save
            .is_none_or(|t| self.client.clock().now() >= t + SAVE_EVERY);
        if self.unsaved && (force || due) {
            self.save();
        }
    }

    /// Writes whatever is unsaved (the thread does this before it ends).
    pub fn flush(&mut self) {
        self.save_if_due(true);
    }
}

/// Moves the records named in `order` to the front, in that order; the rest keep theirs.
fn reorder(pending: &mut Vec<Listed>, order: &[RecordKey]) {
    let mut front = Vec::with_capacity(pending.len());
    for key in order {
        if let Some(i) = pending.iter().position(|l| l.key == *key) {
            front.push(pending.remove(i));
        }
    }
    front.append(pending);
    *pending = front;
}

/// The worker thread's end of the channels.
pub struct IntakeHandle {
    commands: Sender<Command>,
    events: Receiver<Event>,
}

impl IntakeHandle {
    /// Runs `intake` on a low-priority thread; `wake` is called when events are ready.
    pub fn start(
        spawner: &dyn Spawner,
        mut intake: Intake,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let (cmd_tx, cmd_rx) = channel::<Command>();
        let (ev_tx, ev_rx) = channel();
        spawner.spawn(
            "dig-intake",
            Priority::Low,
            Box::new(move || {
                loop {
                    loop {
                        match cmd_rx.try_recv() {
                            Ok(c) => intake.handle(c),
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => return intake.flush(),
                        }
                    }
                    let worked = intake.step();
                    let events = intake.take_events();
                    if !events.is_empty() {
                        for e in events {
                            if ev_tx.send(e).is_err() {
                                return intake.flush();
                            }
                        }
                        wake();
                    }
                    if !worked {
                        match cmd_rx.recv() {
                            Ok(c) => intake.handle(c),
                            Err(_) => return intake.flush(),
                        }
                    }
                }
            }),
        )?;
        Ok(Self {
            commands: cmd_tx,
            events: ev_rx,
        })
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.commands.send(cmd);
    }

    pub fn poll(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}
