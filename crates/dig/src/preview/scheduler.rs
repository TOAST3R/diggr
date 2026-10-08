//! Which previews to download, and when: only the horizon (the armed entry, the playing entry
//! and the next 3), 2 at a time, in priority order. A download that leaves the horizon is
//! cancelled and its partial file deleted; one that takes over 120 s is stopped and retried
//! once; a clip that fails twice is given up on.
//!
//! The UI computes the horizon ([`horizon`]) and sends it whenever it changes; this worker owns
//! the downloads and reports progress, finished files and failures.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use platform::{Priority, Spawner};

use super::fetcher::{FetchError, Fetcher, preview_path};
use super::search::{self, Remembered, SearchRequest, SearchResult, Searches};
use super::store;

/// Entries kept ready after the playing one.
pub const AHEAD: usize = 3;
pub const SLOTS: usize = 2;
pub const TIMEOUT: Duration = Duration::from_secs(120);
/// How often yt-dlp is looked for again while entries wait for it.
pub const RECHECK: Duration = Duration::from_secs(30);
/// Tries per clip: the first and one retry.
const ATTEMPTS: u32 = 2;
/// A search that failed (offline, yt-dlp error) is tried again after this long.
const SEARCH_RETRY: Duration = Duration::from_secs(60);

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
        }
    }
}

enum Msg {
    Cmd(PreviewCommand),
    Progress(String, u8),
    Result(String, Result<PathBuf, FetchError>),
    Searched(SearchRequest, Result<Vec<SearchResult>, FetchError>),
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
                    if !wanted.contains(clip) {
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
                self.evict();
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
        if self.searching.is_some() {
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
            Ok(results) => search::best(&req, &results).cloned(),
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
                self.failures.remove(&clip);
                self.delivered.insert(clip.clone());
                self.events.push(PreviewEvent::Done(clip, path));
                self.evict();
            }
            Err(FetchError::Cancelled) if !run.timed_out => {}
            Err(FetchError::NoProgram) => {
                self.fetcher = None;
                self.said_missing = false;
                self.look();
            }
            Err(FetchError::InvalidId) => self.give_up(clip),
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

    fn start(&mut self, clip: String) {
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
                    &id,
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
        };
        spawner.spawn(
            "dig-previews",
            Priority::Low,
            Box::new(move || {
                if let Some(dir) = &s.cfg.searches {
                    s.remembered = Searches::load(dir);
                }
                loop {
                    let tick = if s.running.is_empty() && s.fetcher.is_some() {
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
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    s.check_timeouts();
                    // Searches first: a found track joins the downloads at the next horizon.
                    if !s.to_search.is_empty() {
                        s.search_next();
                    }
                    if !s.wanted.is_empty() {
                        s.fill();
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
