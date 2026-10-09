//! Which previews to download, and when: only the horizon (the armed entry, the playing entry
//! and the next 3), 2 at a time, in priority order. A download that leaves the horizon is
//! cancelled and its partial file deleted; one that takes over 120 s is stopped and retried
//! once; a clip that fails twice is given up on.
//!
//! The UI computes the horizon ([`horizon`]) and sends it whenever it changes; this worker owns
//! the downloads and reports progress, finished files and failures.
//!
//! Besides the horizon there can be a background list (a label's Download all tracks): its
//! clips take the slots the horizon leaves free, in order, and never make the cache delete
//! anything. When the cache is full, the list pauses and [`PreviewEvent::CacheFull`] says so;
//! a new limit resumes it.
//!
//! Clips come from YouTube or Bandcamp (see [`super::clip`]); each source has its own wait
//! when it limits requests, so one limiting never stops the other. Bandcamp pages are read
//! here too, one at a time and at least [`READ_GAP`] apart, as they run the same yt-dlp.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use platform::{Priority, Spawner};

use super::clip::{Clip, Source, source_of, track_id_of};
use super::fetcher::{FetchError, Fetcher, preview_path};
use super::search::{self, Remembered, SearchRequest, SearchResult, Searches};
use super::store;
use crate::bandcamp::{BandcampAlbum, BandcampPage, Listing};

/// Entries kept ready after the playing one.
pub const AHEAD: usize = 3;
pub const SLOTS: usize = 2;
pub const TIMEOUT: Duration = Duration::from_secs(120);
/// How often yt-dlp is looked for again while entries wait for it.
pub const RECHECK: Duration = Duration::from_secs(30);
/// Tries per clip: the first and one retry.
const ATTEMPTS: u32 = 2;
/// The first wait when YouTube limits requests; each limited try after it doubles the wait,
/// up to [`MAX_LIMITED_WAITS`] times this.
pub const LIMITED_WAIT: Duration = Duration::from_secs(10 * 60);
const MAX_LIMITED_WAITS: u32 = 6;
/// A search that failed (offline, yt-dlp error) is tried again after this long.
const SEARCH_RETRY: Duration = Duration::from_secs(60);
/// The least time between two Bandcamp page reads.
pub const READ_GAP: Duration = Duration::from_secs(1);

/// The clips to have ready, in priority order: the armed entry's, then from `start` (the
/// playing entry, or the shown crate's current one when stopped) the next `AHEAD + 1` entries
/// of the play order. `order` is the play order's clips (`None` for local files).
pub fn horizon(order: &[Option<String>], start: usize, armed: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = armed.map(str::to_owned).into_iter().collect();
    for c in order.iter().skip(start).take(AHEAD + 1).flatten() {
        if !out.contains(c) {
            out.push(c.clone());
        }
    }
    out
}

/// Finds a fetcher (yt-dlp) for a configured path, with its version.
pub type Finder = Arc<dyn Fn(Option<&str>) -> Option<(Arc<dyn Fetcher>, String)> + Send + Sync>;

pub fn system_finder() -> Finder {
    Arc::new(|configured| {
        super::fetcher::find(configured).map(|(y, v)| (Arc::new(y) as Arc<dyn Fetcher>, v))
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewCommand {
    /// The horizon changed. `protected` are never evicted (the armed, playing and next ones).
    Want {
        wanted: Vec<String>,
        protected: Vec<String>,
    },
    /// The preview cache size, in bytes.
    SetLimit(u64),
    /// A configured yt-dlp path (or none: look on the PATH); looked for at once.
    SetProgram(Option<String>),
    /// Look for yt-dlp now (the dialog opened).
    CheckProgram,
    /// A preview started playing: it is now the most recently played.
    Played(PathBuf),
    /// The tracks to find in the window, best first (replacing the previous list): each is
    /// answered from the remembered results, or searched for, one at a time.
    Search(Vec<SearchRequest>),
    /// Clips to download after the horizon, in order (replacing the previous list; empty
    /// stops it). They never cause an eviction: the list pauses when the cache is full.
    Background(Vec<String>),
    /// While YouTube limits requests: try again now instead of at the end of the wait.
    TryNow,
    /// Try these again: clips given up on, and searches remembered as not found (or failed).
    Retry {
        clips: Vec<String>,
        searches: Vec<SearchRequest>,
    },
    /// Where Bandcamp clips are: key (`bc.‹id›`) → the track's checked page. Sent before the
    /// keys are wanted; a Bandcamp key with no page is given up on.
    Locate(Vec<(String, String)>),
    /// Read a Bandcamp page for job `job`: a label's albums are then read one by one,
    /// except those in `skip` (read before), and, when `only` has needles, only the albums
    /// whose address holds one (see [`crate::bandcamp::album_matches`]).
    ReadBandcamp {
        job: u64,
        page: BandcampPage,
        skip: Vec<String>,
        only: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewEvent {
    Progress(String, u8),
    Done(String, PathBuf),
    /// Given up on, with the reason ("clip failed").
    Failed(String, String),
    /// Deleted to keep the cache within its limit; downloaded again when needed.
    Evicted(String),
    /// yt-dlp can't be found; entries wait for it.
    NeedsYtDlp,
    /// yt-dlp was found, with its version.
    YtDlp(String),
    /// A search found a track's preview: `clip` is the video, `title` its title, `duration`
    /// its length (it answers the entries under `key` whose length agrees).
    Found {
        key: String,
        clip: String,
        title: String,
        duration: Option<f64>,
    },
    /// The background list paused: the cache is at its limit. A new limit resumes it.
    CacheFull,
    /// The source is limiting requests: nothing from it starts until `until` (Unix seconds).
    Limited {
        source: Source,
        until: u64,
    },
    /// The source answers normally again.
    Unlimited {
        source: Source,
    },
    /// A label page of job `job` lists this many albums and tracks, read next.
    BandcampListed {
        job: u64,
        albums: usize,
    },
    /// An album (or track) of job `job` was read.
    BandcampAlbum {
        job: u64,
        album: BandcampAlbum,
    },
    /// A page of job `job` couldn't be read, with yt-dlp's reason.
    BandcampFailed {
        job: u64,
        page: String,
        error: String,
    },
    /// Every page of job `job` was read (or failed).
    BandcampDone {
        job: u64,
    },
    /// A search found nothing usable for the entries under `key` of length `duration`.
    NotFound {
        key: String,
        duration: Option<f64>,
    },
}

pub struct Config {
    pub dir: PathBuf,
    /// Where search results are remembered (`searches.ron`); `None` keeps them in memory.
    pub searches: Option<PathBuf>,
    pub limit: u64,
    pub program: Option<String>,
    pub timeout: Duration,
    pub recheck: Duration,
    /// The first wait when a source limits requests.
    pub limited_wait: Duration,
    /// The least time between two Bandcamp page reads.
    pub read_gap: Duration,
}

impl Config {
    pub fn new(dir: PathBuf, limit: u64, program: Option<String>) -> Self {
        Self {
            dir,
            searches: None,
            limit,
            program,
            timeout: TIMEOUT,
            recheck: RECHECK,
            limited_wait: LIMITED_WAIT,
            read_gap: READ_GAP,
        }
    }
}

enum Msg {
    Cmd(PreviewCommand),
    Progress(String, u8),
    Result(String, Result<PathBuf, FetchError>),
    Searched(SearchRequest, Result<Vec<SearchResult>, FetchError>),
    Read(u64, BandcampPage, Result<Listing, FetchError>),
}

/// One source's wait while it limits requests.
#[derive(Debug, Default)]
struct Limit {
    /// Nothing from the source starts before this; past it, the next start is a try.
    until: Option<Instant>,
    /// The wait after the next limited answer.
    backoff: Duration,
}

struct Running {
    cancel: Arc<AtomicBool>,
    started: Instant,
    timed_out: bool,
}

struct Scheduler {
    cfg: Config,
    finder: Finder,
    spawner: Arc<dyn Spawner>,
    inbox: Sender<Msg>,
    fetcher: Option<Arc<dyn Fetcher>>,
    /// When yt-dlp was last looked for; `None`: never.
    looked: Option<Instant>,
    said_missing: bool,
    wanted: Vec<String>,
    protected: HashSet<String>,
    running: HashMap<String, Running>,
    failures: HashMap<String, u32>,
    given_up: HashSet<String>,
    delivered: HashSet<String>,
    events: Vec<PreviewEvent>,
    /// Search results remembered across sessions.
    remembered: Searches,
    /// The window's tracks to find, best first.
    to_search: Vec<SearchRequest>,
    /// The track being searched for.
    searching: Option<String>,
    /// Tracks whose search failed, and when (tried again after [`SEARCH_RETRY`]).
    search_failed: HashMap<String, Instant>,
    /// Clips to download after the horizon, in order.
    background: Vec<String>,
    /// The background list waits for a larger cache.
    paused: bool,
    /// Each source's wait while it limits requests.
    limits: HashMap<Source, Limit>,
    /// Bandcamp clips' pages, by key.
    urls: HashMap<String, String>,
    /// Bandcamp pages to read, in order, with their job.
    reads: VecDeque<(u64, BandcampPage)>,
    /// Per job: album pages its label listing leaves out.
    skips: HashMap<u64, HashSet<String>>,
    /// Per job: the needles its label listing is narrowed to.
    onlys: HashMap<u64, Vec<String>>,
    /// A page is being read.
    reading: bool,
    /// When the last read ended.
    last_read: Option<Instant>,
}

impl Scheduler {
    fn look(&mut self) {
        self.looked = Some(Instant::now());
        match (self.finder)(self.cfg.program.as_deref()) {
            Some((f, version)) => {
                if self.fetcher.is_none() || self.said_missing {
                    self.events.push(PreviewEvent::YtDlp(version));
                }
                self.fetcher = Some(f);
                self.said_missing = false;
            }
            None => {
                self.fetcher = None;
                if !self.said_missing {
                    self.said_missing = true;
                    self.events.push(PreviewEvent::NeedsYtDlp);
                }
            }
        }
    }

    fn handle(&mut self, cmd: PreviewCommand) {
        match cmd {
            PreviewCommand::Want { wanted, protected } => {
                for (clip, r) in &self.running {
                    if !wanted.contains(clip) && !self.background.contains(clip) {
                        r.cancel.store(true, Ordering::Release);
                    }
                }
                self.protected = protected
                    .into_iter()
                    .chain(wanted.iter().cloned())
                    .collect();
                self.wanted = wanted;
            }
            PreviewCommand::SetLimit(l) => {
                self.cfg.limit = l;
                self.paused = false;
                self.evict();
            }
            PreviewCommand::TryNow => {
                for l in self.limits.values_mut() {
                    if l.until.is_some() {
                        l.until = Some(Instant::now());
                    }
                }
            }
            PreviewCommand::Locate(urls) => self.urls.extend(urls),
            PreviewCommand::ReadBandcamp {
                job,
                page,
                skip,
                only,
            } => {
                if !skip.is_empty() {
                    self.skips.insert(job, skip.into_iter().collect());
                }
                if !only.is_empty() {
                    self.onlys.insert(job, only);
                }
                self.reads.push_back((job, page));
            }
            PreviewCommand::Retry { clips, searches } => {
                for c in &clips {
                    self.given_up.remove(c);
                    self.failures.remove(c);
                }
                for req in &searches {
                    self.remembered.forget(req);
                    self.search_failed.remove(&req.key);
                }
                if let Some(dir) = &self.cfg.searches
                    && !searches.is_empty()
                {
                    let _ = self.remembered.save(dir);
                }
            }
            PreviewCommand::Background(clips) => {
                for (clip, r) in &self.running {
                    if !clips.contains(clip) && !self.wanted.contains(clip) {
                        r.cancel.store(true, Ordering::Release);
                    }
                }
                self.background = clips;
                self.paused = false;
            }
            PreviewCommand::SetProgram(p) => {
                self.cfg.program = p;
                self.said_missing = false;
                self.fetcher = None;
                self.look();
            }
            PreviewCommand::CheckProgram => self.look(),
            PreviewCommand::Played(path) => store::touch(&path),
            PreviewCommand::Search(list) => self.to_search = list,
        }
    }

    /// Whether the source's wait is still on: nothing from it starts.
    fn waiting(&self, source: Source) -> bool {
        self.limits
            .get(&source)
            .and_then(|l| l.until)
            .is_some_and(|t| Instant::now() < t)
    }

    /// Whether any source is limited (its wait on, or its try not answered yet).
    fn any_limited(&self) -> bool {
        self.limits.values().any(|l| l.until.is_some())
    }

    /// A limited answer: wait, longer each time a try after a wait is limited too. Answers
    /// for requests already under way when the wait began change nothing.
    fn limited(&mut self, source: Source) {
        if self.waiting(source) {
            return;
        }
        let base = self.cfg.limited_wait;
        let l = self.limits.entry(source).or_default();
        l.backoff = if l.until.is_some() {
            (l.backoff * 2).min(base * MAX_LIMITED_WAITS)
        } else {
            base
        };
        l.until = Some(Instant::now() + l.backoff);
        let until = crate::now_secs() + l.backoff.as_secs();
        self.events.push(PreviewEvent::Limited { source, until });
    }

    /// A request to the source went through: it answers normally.
    fn answered(&mut self, source: Source) {
        if let Some(l) = self.limits.get_mut(&source)
            && l.until.take().is_some()
        {
            l.backoff = self.cfg.limited_wait;
            self.events.push(PreviewEvent::Unlimited { source });
        }
    }

    /// Reads the next Bandcamp page, when none is being read and the gap since the last is
    /// over.
    fn read_next(&mut self) {
        if self.reading
            || self.reads.is_empty()
            || self.waiting(Source::Bandcamp)
            || self
                .last_read
                .is_some_and(|t| t.elapsed() < self.cfg.read_gap)
        {
            return;
        }
        if self.fetcher.is_none() {
            if self.looked.is_none_or(|t| t.elapsed() >= self.cfg.recheck) {
                self.look();
            }
            if self.fetcher.is_none() {
                return;
            }
        }
        let Some((job, page)) = self.reads.pop_front() else {
            return;
        };
        let fetcher = self.fetcher.clone().expect("checked");
        let inbox = self.inbox.clone();
        self.reading = true;
        let (p2, j2) = (page.clone(), job);
        let spawned = self.spawner.spawn(
            "dig-bandcamp",
            Priority::Low,
            Box::new(move || {
                let result = fetcher.read_bandcamp(&p2);
                let _ = inbox.send(Msg::Read(j2, p2, result));
            }),
        );
        if spawned.is_err() {
            self.reading = false;
            self.reads.push_front((job, page));
        }
    }

    fn read_done(&mut self, job: u64, page: BandcampPage, result: Result<Listing, FetchError>) {
        self.reading = false;
        self.last_read = Some(Instant::now());
        match result {
            Ok(Listing::Albums(mut urls)) => {
                self.answered(Source::Bandcamp);
                if let Some(skip) = self.skips.remove(&job) {
                    urls.retain(|u| !skip.contains(u));
                }
                if let Some(only) = self.onlys.remove(&job) {
                    urls.retain(|u| crate::bandcamp::album_matches(u, &only));
                }
                self.events.push(PreviewEvent::BandcampListed {
                    job,
                    albums: urls.len(),
                });
                // A label's albums are read before any later send's pages.
                for url in urls.iter().rev() {
                    if let Ok(p) = crate::bandcamp::parse(url) {
                        self.reads.push_front((job, p));
                    }
                }
            }
            Ok(Listing::Album(album)) => {
                self.answered(Source::Bandcamp);
                self.events.push(PreviewEvent::BandcampAlbum { job, album });
            }
            // Not the page's fault: read again once the wait is over.
            Err(FetchError::Limited(_)) => {
                self.reads.push_front((job, page));
                return self.limited(Source::Bandcamp);
            }
            Err(FetchError::NoProgram) => {
                self.reads.push_front((job, page));
                self.fetcher = None;
                self.said_missing = false;
                self.look();
                return;
            }
            Err(e) => {
                let error = match e {
                    FetchError::Failed(t) => t,
                    other => format!("{other:?}"),
                };
                self.events.push(PreviewEvent::BandcampFailed {
                    job,
                    page: page.url(),
                    error,
                });
            }
        }
        if !self.reads.iter().any(|(j, _)| *j == job) {
            self.events.push(PreviewEvent::BandcampDone { job });
        }
    }

    /// Answers the window's tracks from the remembered results, and starts one search for the
    /// first that isn't remembered.
    fn search_next(&mut self) {
        let now = crate::now_secs();
        let todo = std::mem::take(&mut self.to_search);
        let mut left = Vec::new();
        for req in todo {
            match self.remembered.get(&req, now) {
                Some(Remembered::Found {
                    clip,
                    title,
                    duration,
                }) => self.events.push(PreviewEvent::Found {
                    key: req.key.clone(),
                    clip: clip.clone(),
                    title: title.clone(),
                    duration: *duration,
                }),
                Some(Remembered::NotFound { .. }) => self.events.push(PreviewEvent::NotFound {
                    key: req.key.clone(),
                    duration: req.duration,
                }),
                None => left.push(req),
            }
        }
        self.to_search = left;
        if self.searching.is_some() || self.waiting(Source::YouTube) {
            return;
        }
        let Some(i) = self.to_search.iter().position(|r| {
            self.search_failed
                .get(&r.key)
                .is_none_or(|t| t.elapsed() >= SEARCH_RETRY)
        }) else {
            return;
        };
        if self.fetcher.is_none() {
            if self.looked.is_none_or(|t| t.elapsed() >= self.cfg.recheck) {
                self.look();
            }
            if self.fetcher.is_none() {
                return;
            }
        }
        let req = self.to_search.remove(i);
        let fetcher = self.fetcher.clone().expect("checked");
        let inbox = self.inbox.clone();
        self.searching = Some(req.key.clone());
        let spawned = self.spawner.spawn(
            "dig-search",
            Priority::Low,
            Box::new(move || {
                let result = fetcher.search(&search::query(&req.artist, &req.title));
                let _ = inbox.send(Msg::Searched(req, result));
            }),
        );
        if spawned.is_err() {
            self.searching = None;
        }
    }

    fn searched(&mut self, req: SearchRequest, result: Result<Vec<SearchResult>, FetchError>) {
        self.searching = None;
        let found = match result {
            Ok(results) => {
                self.answered(Source::YouTube);
                search::best(&req, &results).cloned()
            }
            Err(FetchError::Limited(_)) => {
                // Not the track's fault: it is searched again once the wait is over.
                if !self.to_search.iter().any(|r| r.key == req.key) {
                    self.to_search.insert(0, req);
                }
                return self.limited(Source::YouTube);
            }
            Err(FetchError::NoProgram) => {
                self.fetcher = None;
                self.said_missing = false;
                self.look();
                return;
            }
            Err(_) => {
                self.search_failed.insert(req.key, Instant::now());
                return;
            }
        };
        self.search_failed.remove(&req.key);
        let remembered = match found {
            Some(r) => {
                self.events.push(PreviewEvent::Found {
                    key: req.key.clone(),
                    clip: r.id.clone(),
                    title: r.title.clone(),
                    duration: r.duration,
                });
                Remembered::Found {
                    clip: r.id,
                    title: r.title,
                    duration: r.duration,
                }
            }
            None => {
                self.events.push(PreviewEvent::NotFound {
                    key: req.key.clone(),
                    duration: req.duration,
                });
                Remembered::NotFound {
                    at: crate::now_secs(),
                    duration: req.duration,
                }
            }
        };
        self.remembered.put(&req, remembered);
        if let Some(dir) = &self.cfg.searches {
            let _ = self.remembered.save(dir);
        }
    }

    fn finished(&mut self, clip: String, result: Result<PathBuf, FetchError>) {
        let Some(run) = self.running.remove(&clip) else {
            return;
        };
        match result {
            Ok(path) => {
                self.answered(source_of(&clip));
                self.failures.remove(&clip);
                self.delivered.insert(clip.clone());
                let background = !self.protected.contains(&clip);
                self.background.retain(|c| *c != clip);
                self.events.push(PreviewEvent::Done(clip, path));
                // A background download never deletes anything: over the limit, the list
                // waits for a larger cache instead.
                if !background {
                    self.evict();
                } else if store::total_size(&self.cfg.dir) >= self.cfg.limit {
                    self.pause();
                }
            }
            Err(FetchError::Cancelled) if !run.timed_out => {}
            Err(FetchError::NoProgram) => {
                self.fetcher = None;
                self.said_missing = false;
                self.look();
            }
            Err(FetchError::InvalidId) => self.give_up(clip),
            // Not the clip's fault: it stays wanted, and starts again once the wait is over.
            Err(FetchError::Limited(_)) => self.limited(source_of(&clip)),
            Err(_) => {
                let n = self.failures.entry(clip.clone()).or_default();
                *n += 1;
                if *n >= ATTEMPTS {
                    self.give_up(clip);
                }
            }
        }
    }

    fn give_up(&mut self, clip: String) {
        self.background.retain(|c| *c != clip);
        self.given_up.insert(clip.clone());
        self.events
            .push(PreviewEvent::Failed(clip, "clip failed".into()));
    }

    fn evict(&mut self) {
        let mut keep = self.protected.clone();
        keep.extend(self.running.keys().cloned());
        for path in store::enforce_limit(&self.cfg.dir, self.cfg.limit, &keep) {
            if let Some(clip) = path.file_stem().map(|s| s.to_string_lossy().into_owned())
                && self.delivered.remove(&clip)
            {
                self.events.push(PreviewEvent::Evicted(clip));
            }
        }
    }

    fn check_timeouts(&mut self) {
        for r in self.running.values_mut() {
            if !r.timed_out && r.started.elapsed() > self.cfg.timeout {
                r.timed_out = true;
                r.cancel.store(true, Ordering::Release);
            }
        }
    }

    /// Starts downloads for the wanted clips, best first, while slots are free.
    fn fill(&mut self) {
        let todo: Vec<String> = self
            .wanted
            .iter()
            .filter(|c| !self.running.contains_key(*c) && !self.given_up.contains(*c))
            .filter(|c| !self.waiting(source_of(c)))
            .cloned()
            .collect();
        for clip in todo {
            let path = preview_path(&self.cfg.dir, &clip);
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > 0) {
                if self.delivered.insert(clip.clone()) {
                    self.events.push(PreviewEvent::Done(clip, path));
                }
                continue;
            }
            self.delivered.remove(&clip);
            if self.running.len() >= SLOTS {
                continue;
            }
            if self.fetcher.is_none() {
                if self.looked.is_none_or(|t| t.elapsed() >= self.cfg.recheck) {
                    self.look();
                }
                if self.fetcher.is_none() {
                    return;
                }
            }
            self.start(clip);
        }
    }

    fn pause(&mut self) {
        if !self.paused && !self.background.is_empty() {
            self.paused = true;
            self.events.push(PreviewEvent::CacheFull);
        }
    }

    /// Starts background downloads in the slots the horizon leaves free, while the cache is
    /// under its limit; clips already in the cache are reported done at once.
    fn fill_background(&mut self) {
        if self.paused {
            return;
        }
        let todo: Vec<String> = self
            .background
            .iter()
            .filter(|c| !self.running.contains_key(*c) && !self.given_up.contains(*c))
            .filter(|c| !self.waiting(source_of(c)))
            .cloned()
            .collect();
        for clip in todo {
            let path = preview_path(&self.cfg.dir, &clip);
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > 0) {
                self.background.retain(|c| *c != clip);
                self.delivered.insert(clip.clone());
                self.events.push(PreviewEvent::Done(clip, path));
                continue;
            }
            if self.running.len() >= SLOTS {
                return;
            }
            if self.fetcher.is_none() {
                if self.looked.is_none_or(|t| t.elapsed() >= self.cfg.recheck) {
                    self.look();
                }
                if self.fetcher.is_none() {
                    return;
                }
            }
            if store::total_size(&self.cfg.dir) >= self.cfg.limit {
                return self.pause();
            }
            self.start(clip);
        }
    }

    fn start(&mut self, clip: String) {
        let target = match track_id_of(&clip) {
            None => Clip::YouTube(clip.clone()),
            Some(track_id) => match self.urls.get(&clip) {
                Some(url) => Clip::Bandcamp {
                    track_id: track_id.to_owned(),
                    url: url.clone(),
                },
                None => return self.give_up(clip),
            },
        };
        let fetcher = self.fetcher.clone().expect("checked");
        let cancel = Arc::new(AtomicBool::new(false));
        let (c2, dir, inbox, id) = (
            cancel.clone(),
            self.cfg.dir.clone(),
            self.inbox.clone(),
            clip.clone(),
        );
        let spawned = self.spawner.spawn(
            "dig-preview",
            Priority::Low,
            Box::new(move || {
                let progress_inbox = inbox.clone();
                let progress_id = id.clone();
                let result = fetcher.fetch(
                    &target,
                    &dir,
                    &move |p| {
                        let _ = progress_inbox.send(Msg::Progress(progress_id.clone(), p));
                    },
                    &c2,
                );
                let _ = inbox.send(Msg::Result(id, result));
            }),
        );
        if spawned.is_ok() {
            self.running.insert(
                clip,
                Running {
                    cancel,
                    started: Instant::now(),
                    timed_out: false,
                },
            );
        }
    }
}

pub struct PreviewHandle {
    commands: Sender<Msg>,
    events: Receiver<PreviewEvent>,
}

impl PreviewHandle {
    /// Starts the scheduler on a low-priority thread; `wake` is called when events are ready.
    /// Nothing runs (yt-dlp isn't even looked for) until the first `Want`.
    pub fn start(
        spawner: Arc<dyn Spawner>,
        cfg: Config,
        finder: Finder,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, platform::PlatformError> {
        let (tx, rx) = channel::<Msg>();
        let (ev_tx, ev_rx) = channel();
        let mut s = Scheduler {
            cfg,
            finder,
            spawner: spawner.clone(),
            inbox: tx.clone(),
            fetcher: None,
            looked: None,
            said_missing: false,
            wanted: Vec::new(),
            protected: HashSet::new(),
            running: HashMap::new(),
            failures: HashMap::new(),
            given_up: HashSet::new(),
            delivered: HashSet::new(),
            events: Vec::new(),
            remembered: Searches::default(),
            to_search: Vec::new(),
            searching: None,
            search_failed: HashMap::new(),
            background: Vec::new(),
            paused: false,
            limits: HashMap::new(),
            urls: HashMap::new(),
            reads: VecDeque::new(),
            skips: HashMap::new(),
            onlys: HashMap::new(),
            reading: false,
            last_read: None,
        };
        spawner.spawn(
            "dig-previews",
            Priority::Low,
            Box::new(move || {
                if let Some(dir) = &s.cfg.searches {
                    s.remembered = Searches::load(dir);
                }
                loop {
                    let tick = if s.running.is_empty()
                        && s.fetcher.is_some()
                        && !s.any_limited()
                        && s.reads.is_empty()
                    {
                        Duration::from_secs(5)
                    } else {
                        Duration::from_millis(100).min(s.cfg.recheck)
                    };
                    match rx.recv_timeout(tick) {
                        Ok(Msg::Cmd(c)) => s.handle(c),
                        Ok(Msg::Progress(clip, p)) => {
                            if s.running.contains_key(&clip) {
                                s.events.push(PreviewEvent::Progress(clip, p));
                            }
                        }
                        Ok(Msg::Result(clip, r)) => s.finished(clip, r),
                        Ok(Msg::Searched(req, r)) => s.searched(req, r),
                        Ok(Msg::Read(job, page, r)) => s.read_done(job, page, r),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    s.check_timeouts();
                    // Searches first: a found track joins the downloads at the next horizon.
                    if !s.to_search.is_empty() {
                        s.search_next();
                    }
                    if !s.reads.is_empty() {
                        s.read_next();
                    }
                    if !s.wanted.is_empty() {
                        s.fill();
                    }
                    if !s.background.is_empty() {
                        s.fill_background();
                    }
                    if !s.events.is_empty() {
                        for e in s.events.drain(..) {
                            if ev_tx.send(e).is_err() {
                                return;
                            }
                        }
                        wake();
                    }
                }
                for r in s.running.values() {
                    r.cancel.store(true, Ordering::Release);
                }
            }),
        )?;
        Ok(Self {
            commands: tx,
            events: ev_rx,
        })
    }

    pub fn send(&self, cmd: PreviewCommand) {
        let _ = self.commands.send(Msg::Cmd(cmd));
    }

    pub fn poll(&self) -> Vec<PreviewEvent> {
        self.events.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler(wait: Duration) -> Scheduler {
        let mut cfg = Config::new(std::env::temp_dir().join("diggr-tests-unused"), 1, None);
        cfg.limited_wait = wait;
        let (tx, _rx) = channel();
        Scheduler {
            cfg,
            finder: Arc::new(|_| None),
            spawner: Arc::new(platform::native::NativeSpawner),
            inbox: tx,
            fetcher: None,
            looked: None,
            said_missing: false,
            wanted: Vec::new(),
            protected: HashSet::new(),
            running: HashMap::new(),
            failures: HashMap::new(),
            given_up: HashSet::new(),
            delivered: HashSet::new(),
            events: Vec::new(),
            remembered: Searches::default(),
            to_search: Vec::new(),
            searching: None,
            search_failed: HashMap::new(),
            background: Vec::new(),
            paused: false,
            limits: HashMap::new(),
            urls: HashMap::new(),
            reads: VecDeque::new(),
            skips: HashMap::new(),
            onlys: HashMap::new(),
            reading: false,
            last_read: None,
        }
    }

    #[test]
    fn limited_waits_double_up_to_six_times_and_reset_on_an_answer() {
        let base = Duration::from_secs(600);
        let mut s = scheduler(base);
        let mut waits = Vec::new();
        let yt = Source::YouTube;
        let backoff = |s: &Scheduler| s.limits[&yt].backoff;
        for _ in 0..5 {
            s.limited(yt);
            waits.push(backoff(&s));
            // A second limited answer within the wait (the other slot) changes nothing.
            s.limited(yt);
            assert_eq!(backoff(&s), *waits.last().unwrap());
            // The wait is over: the next answer is the try.
            s.limits.get_mut(&yt).unwrap().until = Some(Instant::now() - Duration::from_secs(1));
        }
        let min = |m: u64| Duration::from_secs(m * 60);
        assert_eq!(waits, [min(10), min(20), min(40), min(60), min(60)]);
        let limited = s
            .events
            .iter()
            .filter(|e| matches!(e, PreviewEvent::Limited { .. }))
            .count();
        assert_eq!(limited, 5, "one event per wait");
        s.answered(yt);
        assert_eq!(s.limits[&yt].until, None);
        assert_eq!(
            s.events.last(),
            Some(&PreviewEvent::Unlimited { source: yt })
        );
        s.limited(yt);
        assert_eq!(backoff(&s), min(10), "starts again at 10 minutes");
    }

    #[test]
    fn each_source_waits_on_its_own() {
        let mut s = scheduler(Duration::from_secs(600));
        s.limited(Source::Bandcamp);
        assert!(s.waiting(Source::Bandcamp));
        assert!(!s.waiting(Source::YouTube));
        assert_eq!(
            s.events,
            [PreviewEvent::Limited {
                source: Source::Bandcamp,
                until: crate::now_secs() + 600
            }]
        );
        s.answered(Source::YouTube);
        assert!(
            s.waiting(Source::Bandcamp),
            "a YouTube answer ends no Bandcamp wait"
        );
    }

    #[test]
    fn the_horizon_is_armed_then_playing_and_three_ahead() {
        let order: Vec<Option<String>> = (0..50)
            .map(|i| (i != 5).then(|| format!("clip{i:07}")))
            .collect();
        // Entry 3 plays (index 2): entries 3–6; entry 6 is a local file.
        assert_eq!(
            horizon(&order, 2, None),
            ["clip0000002", "clip0000003", "clip0000004"]
        );
        // Entry 30 armed: first.
        assert_eq!(horizon(&order, 2, Some("clip0000029"))[0], "clip0000029");
        assert_eq!(horizon(&order, 48, None), ["clip0000048", "clip0000049"]);
        assert_eq!(
            horizon(&order, 0, Some("clip0000001")),
            ["clip0000001", "clip0000000", "clip0000002", "clip0000003"],
            "no clip twice"
        );
    }
}
