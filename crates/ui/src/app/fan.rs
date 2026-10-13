//! A Bandcamp fan's collection and wishlist, imported into the collection and wantlist crates.
//!
//! A run reads both lists (preview worker, `dig::bandcamp::fan`), then the items not read
//! before as one Bandcamp send per list, merged like any album (so nothing comes in twice),
//! then, with a token, looks each new item up on Discogs (intake worker). An exact match is
//! added to the Discogs collection or wantlist unless it is there already, and sent into the
//! crate, where its record takes the Bandcamp entries over. Nothing is ever removed from
//! Discogs. Each wantlist (and collection) has two sources: an entry stays while either holds
//! it. The run ends with a report, kept for the crates' menus.

use std::collections::{HashMap, HashSet};

use dig::bandcamp::fan::{
    self, DiscogsMatch, FanList, FanLists, FanMemory, FanPage, FanReport, ItemKind, ListReport,
    ReportItem,
};
use dig::bandcamp::{self, BandcampAlbum};
use dig::discogs::client::ApiError;
use dig::discogs::search::Wanted;
use dig::discogs::url::{self, Page};
use dig::intake::Command;
use dig::jobs::JobId;
use dig::memory::WantOp;
use dig::preview::clip::bandcamp_key;
use dig::preview::scheduler::PreviewCommand;

use super::DiggrApp;
use super::bandcamp::{catno_key, match_key};
use super::digging::{WANTLIST, now_secs, release_of};
use crate::crates::{CrateId, MAX_NAME};

/// What reading a page that isn't an album or track says.
const NOT_A_RECORD: &str = "not an album or track page";

/// Where a run is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// The lists: this one, items read of those listed.
    Lists(FanList, usize, usize),
    /// The items' pages.
    Albums,
    /// Looking items up on Discogs: answered, asked.
    Matching(usize, usize),
}

/// An import or refresh running.
pub(super) struct FanRun {
    user: String,
    /// The lists it reads into their crates.
    lists: Vec<FanList>,
    read_job: JobId,
    step: Step,
    /// Each list's page reads still running.
    page_jobs: HashMap<JobId, FanList>,
    report: FanReport,
}

#[derive(Default)]
pub(super) struct FanUi {
    pub(super) run: Option<FanRun>,
    /// The report window is open.
    pub(super) report_open: bool,
    /// Collection adds of this import: they don't announce themselves one by one.
    pub(super) quiet: HashSet<u64>,
}

/// An item's lookup key for the intake worker: its list and Bandcamp key.
fn find_key(list: FanList, key: &str) -> String {
    match list {
        FanList::Collection => format!("c:{key}"),
        FanList::Wishlist => format!("w:{key}"),
    }
}

fn parse_find_key(k: &str) -> Option<(FanList, &str)> {
    match k.split_once(':')? {
        ("c", key) => Some((FanList::Collection, key)),
        ("w", key) => Some((FanList::Wishlist, key)),
        _ => None,
    }
}

impl FanRun {
    fn list_report(&mut self, list: FanList) -> &mut ListReport {
        let i = match self.report.lists.iter().position(|l| l.list == Some(list)) {
            Some(i) => i,
            None => {
                self.report.lists.push(ListReport {
                    list: Some(list),
                    ..ListReport::default()
                });
                self.report.lists.len() - 1
            }
        };
        &mut self.report.lists[i]
    }
}

impl DiggrApp {
    fn fan_memory(&self) -> Option<&FanMemory> {
        self.dig.as_ref()?.bandcamp.memory.fan.as_ref()
    }

    /// Whether a fan was imported (Refresh then reads their lists too).
    pub(super) fn fan_imported(&self) -> bool {
        self.fan_memory().is_some_and(|f| !f.user.is_empty())
    }

    /// Whether a run reads `list` now.
    pub(super) fn fan_running(&self, list: FanList) -> bool {
        self.dig
            .as_ref()
            .and_then(|d| d.bandcamp.fan.run.as_ref())
            .is_some_and(|r| r.lists.contains(&list))
    }

    pub(super) fn fan_has_report(&self) -> bool {
        self.fan_memory().is_some_and(|f| f.report.is_some())
    }

    pub(super) fn fan_show_report(&mut self) {
        if let Some(d) = &mut self.dig {
            d.bandcamp.fan.report_open = true;
        }
    }

    /// The track pages `list`'s items gave: entries holding one belong in its crate.
    pub(super) fn fan_tracks(&self, list: FanList) -> HashSet<String> {
        self.fan_memory()
            .map(|f| f.tracks(list).into_iter().map(str::to_owned).collect())
            .unwrap_or_default()
    }

    /// A fan's page, pasted or from the browser: both lists are imported (or refreshed).
    pub(super) fn fan_import(&mut self, page: FanPage) {
        self.fan_start(page, vec![FanList::Collection, FanList::Wishlist]);
    }

    /// Refresh collection or Refresh wantlist: the imported fan's list of the same kind.
    pub(super) fn fan_refresh(&mut self, list: FanList) {
        let Some(user) = self.fan_memory().map(|f| f.user.clone()) else {
            return;
        };
        self.fan_start(FanPage { user, list }, vec![list]);
    }

    /// The crate `list` goes into: the collection or wantlist crate, made (and marked) when
    /// there is none, named after the Bandcamp user.
    fn fan_crate(&mut self, list: FanList, user: &str) -> Result<CrateId, String> {
        match list {
            FanList::Collection => {
                if let Some(c) = self.crates.list().iter().find(|c| c.collection) {
                    return Ok(c.id);
                }
                let name: String = format!("Collection: {user}")
                    .chars()
                    .take(MAX_NAME)
                    .collect();
                let c = match self.crates.find(&name) {
                    Some(c) => c,
                    None => self.crates.create(&name)?,
                };
                self.crates.set_collection(c);
                Ok(c)
            }
            FanList::Wishlist => {
                let d = self.dig.as_ref().ok_or("Digging is off")?;
                if let Some(c) = d
                    .settings
                    .wantlist
                    .filter(|&c| self.crates.info(c).is_some())
                {
                    return Ok(c);
                }
                let name: String = format!("{WANTLIST}: {user}")
                    .chars()
                    .take(MAX_NAME)
                    .collect();
                let c = match self.crates.find(&name) {
                    Some(c) => c,
                    None => self.crates.create(&name)?,
                };
                self.crates.set_wantlist(c, true);
                if let Some(d) = &mut self.dig {
                    d.settings.wantlist = Some(c);
                    let e = d.save_settings();
                    self.dig_notify(e);
                }
                Ok(c)
            }
        }
    }

    fn fan_start(&mut self, page: FanPage, lists: Vec<FanList>) {
        let Some(d) = &self.dig else { return };
        if !d.started() {
            return self.notify("Bandcamp isn't ready yet");
        }
        if d.bandcamp.fan.run.is_some() {
            return self.notify("A Bandcamp import is running");
        }
        for &list in &lists {
            match self.fan_crate(list, &page.user) {
                Ok(c) => {
                    if !self.crates.load(c) {
                        return;
                    }
                }
                Err(e) => return self.notify(e),
            }
        }
        let Some(job) = self.bandcamp_next_job() else {
            return;
        };
        let Some(d) = &mut self.dig else { return };
        // One fan at a time: another one's lists start afresh (their crates stay).
        let fresh = d
            .bandcamp
            .memory
            .fan
            .as_ref()
            .is_none_or(|f| f.user != page.user);
        if fresh {
            d.bandcamp.memory.fan = Some(FanMemory {
                user: page.user.clone(),
                ..FanMemory::default()
            });
        }
        d.bandcamp.fan.run = Some(FanRun {
            user: page.user.clone(),
            lists,
            read_job: job,
            step: Step::Lists(page.list, 0, 0),
            page_jobs: HashMap::new(),
            report: FanReport {
                user: page.user.clone(),
                ..FanReport::default()
            },
        });
        let user = page.user.clone();
        d.preview(PreviewCommand::ReadFan { job, page });
        self.notify(format!("Reading {user}'s Bandcamp…"));
    }

    fn fan_run_mut(&mut self, job: JobId) -> Option<&mut FanRun> {
        self.dig
            .as_mut()?
            .bandcamp
            .fan
            .run
            .as_mut()
            .filter(|r| r.read_job == job)
    }

    pub(super) fn fan_progress(&mut self, job: JobId, list: FanList, done: usize, total: usize) {
        if let Some(r) = self.fan_run_mut(job) {
            r.step = Step::Lists(list, done, total);
        }
    }

    /// Both lists arrived (or didn't): the new items' pages are read next.
    pub(super) fn fan_lists_read(&mut self, job: JobId, result: Result<FanLists, String>) {
        let Some(run) = self.fan_run_mut(job) else {
            return;
        };
        let lists = match result {
            Ok(l) => l,
            Err(e) if e == fan::STOPPED => return self.fan_finish(true),
            Err(e) => {
                run.report.failed = e.clone();
                let user = run.user.clone();
                if let Some(d) = &mut self.dig {
                    d.bandcamp.fan.run = None;
                }
                return self.notify(format!("{user}'s Bandcamp couldn't be read: {e}"));
            }
        };
        let wanted = run.lists.clone();
        let mut gone_tracks: Vec<(FanList, HashSet<String>)> = Vec::new();
        let mut pages: Vec<(FanList, Vec<bandcamp::BandcampPage>)> = Vec::new();
        {
            let Some(d) = &mut self.dig else { return };
            let Some(run) = d.bandcamp.fan.run.as_mut() else {
                return;
            };
            let Some(mem) = d.bandcamp.memory.fan.as_mut() else {
                return;
            };
            mem.fan_id = lists.fan_id;
            mem.read_at = now_secs();
            for list in wanted {
                let items = match list {
                    FanList::Collection => lists.collection.clone(),
                    FanList::Wishlist => lists.wishlist.clone(),
                };
                let gone = mem.update(list, items);
                let tracks: HashSet<String> =
                    gone.iter().flat_map(|e| e.tracks.iter().cloned()).collect();
                run.list_report(list).gone = gone.len();
                gone_tracks.push((list, tracks));
                let mut todo = Vec::new();
                for e in mem.list_mut(list).iter_mut().filter(|e| !e.read) {
                    match bandcamp::parse(&e.item.page) {
                        Ok(p) => todo.push(p),
                        Err(_) => {
                            e.read = true;
                            e.error = NOT_A_RECORD.into();
                            run.report.unreadable.push(ReportItem {
                                list,
                                item: e.item.clone(),
                                why: NOT_A_RECORD.into(),
                            });
                        }
                    }
                }
                pages.push((list, todo));
            }
        }
        for (list, tracks) in gone_tracks {
            self.fan_let_go(list, &tracks);
        }
        let user = self
            .dig
            .as_ref()
            .and_then(|d| d.bandcamp.fan.run.as_ref())
            .map(|r| r.user.clone())
            .unwrap_or_default();
        for (list, todo) in pages {
            if todo.is_empty() {
                continue;
            }
            let Ok(target) = self.fan_crate(list, &user) else {
                continue;
            };
            if let Some(j) = self.bandcamp_pages_job(todo, target, list)
                && let Some(r) = self.dig.as_mut().and_then(|d| d.bandcamp.fan.run.as_mut())
            {
                r.page_jobs.insert(j, list);
                r.step = Step::Albums;
            }
        }
        self.fan_save();
        let idle = self
            .dig
            .as_ref()
            .and_then(|d| d.bandcamp.fan.run.as_ref())
            .is_some_and(|r| r.page_jobs.is_empty());
        if idle {
            self.fan_match();
        }
    }

    /// Items gone from `list`: their entries leave its crate, unless Discogs (or another item)
    /// still holds them.
    fn fan_let_go(&mut self, list: FanList, tracks: &HashSet<String>) {
        if tracks.is_empty() {
            return;
        }
        let user = self
            .fan_memory()
            .map(|f| f.user.clone())
            .unwrap_or_default();
        let Ok(c) = self.fan_crate(list, &user) else {
            return;
        };
        let still = self.fan_tracks(list);
        let Some(d) = &self.dig else { return };
        let kept = |release: Option<u64>| match (list, release) {
            (_, None) => false,
            (FanList::Collection, Some(r)) => d
                .collection
                .as_ref()
                .is_some_and(|c| c.releases.contains_key(&r)),
            (FanList::Wishlist, Some(r)) => d.memory.is_wanted(r),
        };
        let ids: Vec<_> = self
            .crates
            .get(c)
            .map(|p| {
                p.entries()
                    .iter()
                    .filter(|e| {
                        e.origin.as_ref().is_some_and(|o| {
                            tracks.contains(&o.bandcamp) && !still.contains(&o.bandcamp)
                        }) && !kept(release_of(e))
                    })
                    .map(|e| e.id)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(p) = self.crates.get_mut(c)
            && p.remove_ids(&ids) > 0
        {
            self.mark_crate(c);
        }
    }

    /// An item's page was read into its crate (`added` tracks it lacked).
    pub(super) fn fan_album_read(&mut self, list: FanList, album: &BandcampAlbum, added: usize) {
        self.fan_link(list, album);
        let Some(d) = &mut self.dig else { return };
        let Some(mem) = d.bandcamp.memory.fan.as_mut() else {
            return;
        };
        for e in mem
            .list_mut(list)
            .iter_mut()
            .filter(|e| e.item.page == album.url)
        {
            e.read = true;
            e.error.clear();
            e.tracks = album.tracks.iter().map(|t| t.url.clone()).collect();
        }
        if let Some(run) = d.bandcamp.fan.run.as_mut() {
            let r = run.list_report(list);
            r.read += 1;
            match added {
                0 => r.already += 1,
                _ => r.added += 1,
            }
        }
    }

    /// The crate's entries of the album's tracks that came from elsewhere (a Discogs record
    /// there first) get its Bandcamp track too, keeping what they play: the item holds them,
    /// and they can switch to Bandcamp.
    fn fan_link(&mut self, list: FanList, album: &BandcampAlbum) {
        let user = self
            .fan_memory()
            .map(|f| f.user.clone())
            .unwrap_or_default();
        let Ok(c) = self.fan_crate(list, &user) else {
            return;
        };
        let catno = catno_key(&album.catno);
        let Some(p) = self.crates.get_mut(c) else {
            return;
        };
        let narrowed = !catno.is_empty()
            && p.entries().iter().any(|e| {
                e.origin
                    .as_ref()
                    .is_some_and(|o| catno_key(&o.catno) == catno)
            });
        let mut linked = false;
        for t in album.tracks.iter().filter(|t| t.streamable) {
            let key = match_key(&t.artist, &t.title);
            for e in p.entries_mut() {
                if match_key(&e.artist, &e.title) != key {
                    continue;
                }
                let Some(o) = e.origin.as_mut() else { continue };
                if !o.bandcamp.is_empty() || (narrowed && catno_key(&o.catno) != catno) {
                    continue;
                }
                o.bandcamp = t.url.clone();
                o.bandcamp_clip = bandcamp_key(&t.track_id);
                linked = true;
            }
        }
        if linked {
            self.mark_crate(c);
        }
    }

    /// An item's page couldn't be read: reported, and read again next time.
    pub(super) fn fan_page_failed(&mut self, list: FanList, page: &str, error: &str) {
        let Some(d) = &mut self.dig else { return };
        let Some(mem) = d.bandcamp.memory.fan.as_mut() else {
            return;
        };
        let Some(e) = mem.list_mut(list).iter_mut().find(|e| e.item.page == page) else {
            return;
        };
        e.error = error.to_owned();
        let item = e.item.clone();
        if let Some(run) = d.bandcamp.fan.run.as_mut() {
            run.report.unreadable.push(ReportItem {
                list,
                item,
                why: error.to_owned(),
            });
        }
    }

    /// A list's pages are all read.
    pub(super) fn fan_pages_done(&mut self, job: JobId) {
        let Some(run) = self.dig.as_mut().and_then(|d| d.bandcamp.fan.run.as_mut()) else {
            return;
        };
        if run.page_jobs.remove(&job).is_some() && run.page_jobs.is_empty() {
            self.fan_save();
            self.fan_match();
        }
    }

    /// With a token, the items never looked up are looked up on Discogs; without, the run ends.
    fn fan_match(&mut self) {
        let Some(d) = &mut self.dig else { return };
        let Some(run) = d.bandcamp.fan.run.as_mut() else {
            return;
        };
        let Some(mem) = d.bandcamp.memory.fan.as_ref() else {
            return;
        };
        if d.token.is_none() {
            return self.fan_finish(false);
        }
        let mut finds = Vec::new();
        for &list in &run.lists {
            for e in mem.list(list) {
                if e.discogs != DiscogsMatch::Unsearched || e.item.title.is_empty() {
                    continue;
                }
                let physical = list == FanList::Wishlist || e.item.kind == ItemKind::Physical;
                let w = Wanted::new(&e.item.artist, &e.item.title, &e.item.catno, physical);
                finds.push((find_key(list, &e.item.key), w));
            }
        }
        if finds.is_empty() {
            return self.fan_finish(false);
        }
        run.step = Step::Matching(0, finds.len());
        d.send_cmd(Command::FindReleases(finds));
    }

    /// Discogs answered for an item.
    pub(super) fn fan_found(&mut self, key: &str, result: Result<DiscogsMatch, ApiError>) {
        let Some((list, key)) = parse_find_key(key) else {
            return;
        };
        let Some(d) = &mut self.dig else { return };
        let Some(run) = d.bandcamp.fan.run.as_mut() else {
            return;
        };
        if let Step::Matching(done, total) = &mut run.step {
            *done += 1;
            if *done >= *total {
                run.step = Step::Matching(*total, *total);
            }
        }
        let Some(mem) = d.bandcamp.memory.fan.as_mut() else {
            return;
        };
        let Some(e) = mem.list_mut(list).iter_mut().find(|e| e.item.key == key) else {
            return;
        };
        let Ok(m) = result else {
            return self.fan_matched_one();
        };
        e.discogs = m;
        let item = e.item.clone();
        let why = match m {
            DiscogsMatch::Found(release) => {
                self.fan_write(list, release);
                None
            }
            DiscogsMatch::NoMatch => Some("not on Discogs".to_owned()),
            DiscogsMatch::Ambiguous(n) => Some(format!("{n} candidates")),
            DiscogsMatch::Unsearched => None,
        };
        if let Some(why) = why
            && let Some(run) = self.dig.as_mut().and_then(|d| d.bandcamp.fan.run.as_mut())
        {
            run.report.not_added.push(ReportItem { list, item, why });
        }
        self.fan_matched_one();
    }

    /// Ends the run once every lookup has answered.
    fn fan_matched_one(&mut self) {
        let done = self
            .dig
            .as_ref()
            .and_then(|d| d.bandcamp.fan.run.as_ref())
            .is_some_and(|r| matches!(r.step, Step::Matching(a, b) if a >= b));
        if done {
            self.fan_finish(false);
        }
    }

    /// Release `release` is the item: onto the Discogs collection (or wantlist) unless it's
    /// there, and into the crate, where it takes the Bandcamp entries over.
    fn fan_write(&mut self, list: FanList, release: u64) {
        let user = self
            .fan_memory()
            .map(|f| f.user.clone())
            .unwrap_or_default();
        let Ok(c) = self.fan_crate(list, &user) else {
            return;
        };
        let Some(d) = &mut self.dig else { return };
        let already = match list {
            FanList::Collection => {
                let owned = d
                    .collection
                    .as_ref()
                    .is_some_and(|c| c.releases.contains_key(&release));
                if !owned && !d.collecting.contains_key(&release) {
                    d.collecting.insert(release, c);
                    d.collect_failed.remove(&release);
                    d.bandcamp.fan.quiet.insert(release);
                    d.send_cmd(Command::Collect {
                        release,
                        check: true,
                    });
                }
                owned
            }
            FanList::Wishlist => {
                let wanted = d.memory.is_wanted(release);
                if !wanted {
                    d.memory.wanted.insert(release);
                    d.memory.queue_want(release, WantOp::Add);
                    let err = d.save_memory();
                    self.dig_notify(err);
                    self.dig_retry_wantlist();
                }
                wanted
            }
        };
        if let Some(run) = self.dig.as_mut().and_then(|d| d.bandcamp.fan.run.as_mut()) {
            let r = run.list_report(list);
            match already {
                true => r.discogs_already += 1,
                false => r.discogs_added += 1,
            }
        }
        self.dig_fill(Page::new(url::PageKind::Release(release)), c);
    }

    /// Stop in the progress window: what was read stays, the rest waits for the next refresh.
    pub(super) fn fan_stop(&mut self) {
        let Some(d) = &self.dig else { return };
        let Some(run) = &d.bandcamp.fan.run else {
            return;
        };
        d.preview(PreviewCommand::StopBandcamp(run.read_job));
        for &j in run.page_jobs.keys() {
            d.preview(PreviewCommand::StopBandcamp(j));
        }
        d.send_cmd(Command::CancelFind);
        self.notify("Bandcamp import stopped");
        self.fan_finish(true);
    }

    /// The run ends: its report is kept and shown.
    fn fan_finish(&mut self, stopped: bool) {
        let names: HashMap<FanList, String> = [FanList::Collection, FanList::Wishlist]
            .into_iter()
            .filter_map(|l| {
                let c = match l {
                    FanList::Collection => self.crates.list().iter().find(|c| c.collection)?.id,
                    FanList::Wishlist => self.dig.as_ref()?.settings.wantlist?,
                };
                Some((l, self.crates.name(c).to_owned()))
            })
            .collect();
        let Some(d) = &mut self.dig else { return };
        let Some(mut run) = d.bandcamp.fan.run.take() else {
            return;
        };
        for &list in &run.lists.clone() {
            let unsearched = d.bandcamp.memory.fan.as_ref().map_or(0, |m| {
                m.list(list)
                    .iter()
                    .filter(|e| e.discogs == DiscogsMatch::Unsearched)
                    .count()
            });
            let r = run.list_report(list);
            r.unsearched = unsearched;
            r.crate_name = names.get(&list).cloned().unwrap_or_default();
        }
        run.report.stopped = stopped;
        run.report.at = now_secs();
        run.report.lists.sort_by_key(|l| l.list);
        if let Some(mem) = d.bandcamp.memory.fan.as_mut() {
            mem.report = Some(run.report);
        }
        d.bandcamp.fan.report_open = true;
        self.fan_save();
    }

    fn fan_save(&mut self) {
        let Some(d) = &self.dig else { return };
        if let Some(cfg) = d.config.as_deref()
            && let Err(e) = d.bandcamp.memory.save(cfg)
        {
            self.notify(format!("Could not save what Bandcamp read: {e}"));
        }
    }

    // ---- windows ---------------------------------------------------------------------------

    /// The run's progress, with Stop.
    pub(super) fn fan_progress_ui(&mut self, ctx: &egui::Context) {
        let Some(run) = self.dig.as_ref().and_then(|d| d.bandcamp.fan.run.as_ref()) else {
            return;
        };
        let (text, part) = match run.step {
            Step::Lists(list, done, total) => (
                format!(
                    "Reading your Bandcamp {}… {done} of {total} items",
                    list.name()
                ),
                done as f32 / total.max(1) as f32,
            ),
            Step::Albums => {
                let (done, total) = self
                    .dig
                    .as_ref()
                    .map(|d| {
                        run.page_jobs
                            .keys()
                            .filter_map(|j| d.jobs.get(j))
                            .fold((0, 0), |(a, b), v| (a + v.done, b + v.total))
                    })
                    .unwrap_or_default();
                (
                    format!("Reading albums… {done} of {total}"),
                    done as f32 / total.max(1) as f32,
                )
            }
            Step::Matching(done, total) => (
                format!("Matching on Discogs… {done} of {total}"),
                done as f32 / total.max(1) as f32,
            ),
        };
        let mut stop = false;
        egui::Window::new("Bandcamp import")
            .id(egui::Id::new("bandcamp-import"))
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.add(egui::ProgressBar::new(part).text(text));
                stop = ui.button("Stop").clicked();
            });
        if stop {
            self.fan_stop();
        }
    }

    /// The last run's report, until closed.
    pub(super) fn fan_report_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &self.dig else { return };
        if !d.bandcamp.fan.report_open {
            return;
        }
        let Some(report) = d
            .bandcamp
            .memory
            .fan
            .as_ref()
            .and_then(|f| f.report.clone())
        else {
            return;
        };
        let mut open = true;
        let mut close = false;
        let mut links: Vec<String> = Vec::new();
        egui::Window::new(format!("Bandcamp import: {}", report.user))
            .id(egui::Id::new("bandcamp-report"))
            .open(&mut open)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                if report.stopped {
                    ui.label("Stopped before the end: the rest comes with the next refresh.");
                }
                for l in &report.lists {
                    ui.strong(&l.crate_name);
                    ui.label(format!(
                        "{} added, {} already there, {} gone",
                        l.added, l.already, l.gone
                    ));
                    let (to, already) = match l.list {
                        Some(FanList::Wishlist) => ("wantlist", "already wanted"),
                        _ => ("collection", "already owned"),
                    };
                    ui.label(format!(
                        "Discogs {to}: {} added, {} {already}, {} not looked up",
                        l.discogs_added, l.discogs_already, l.unsearched
                    ));
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (title, items) in [
                            ("Not added to Discogs", &report.not_added),
                            ("Couldn't read", &report.unreadable),
                        ] {
                            if items.is_empty() {
                                continue;
                            }
                            egui::CollapsingHeader::new(format!("{title} ({})", items.len()))
                                .id_salt(title)
                                .show(ui, |ui| {
                                    for i in items {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(format!(
                                                "{} - {} ({})",
                                                i.item.artist, i.item.title, i.why
                                            ));
                                            if !i.item.page.is_empty()
                                                && ui.small_button("Open on Bandcamp").clicked()
                                            {
                                                links.push(i.item.page.clone());
                                            }
                                            if ui.small_button("Search Discogs").clicked() {
                                                links.push(fan::discogs_search_url(&i.item));
                                            }
                                        });
                                    }
                                });
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(report.text());
                    }
                    close = ui.button("Close").clicked();
                });
            });
        if let Some(d) = &mut self.dig
            && (!open || close)
        {
            d.bandcamp.fan.report_open = false;
        }
        for url in links {
            if let Some(d) = &self.dig
                && let Err(e) = d.open_url(&url)
            {
                self.notify(format!("Could not open the browser: {e}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_keys_carry_their_list() {
        assert_eq!(find_key(FanList::Wishlist, "a1"), "w:a1");
        assert_eq!(parse_find_key("c:p17"), Some((FanList::Collection, "p17")));
        assert_eq!(parse_find_key("x:p17"), None);
        assert_eq!(parse_find_key("p17"), None);
    }
}
