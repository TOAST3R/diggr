//! Bandcamp in the player: album, track and label pages sent like Discogs pages.
//!
//! A page is read by the preview worker (through yt-dlp, see `dig::bandcamp`), album by
//! album. Each album is merged into its crate rather than added blindly: a track the crate
//! plays already is skipped, one that had no preview ("no clip", "clip failed", "not found")
//! gets the Bandcamp audio in place, and only what the crate lacks is added. A Bandcamp label
//! is followed under LABELS like a Discogs one, and joins the followed Discogs label of the
//! same name (asking when the names are only close).

use std::collections::{HashMap, HashSet, VecDeque};

use dig::bandcamp::{
    self, BandcampAlbum, BandcampKind, BandcampMemory, BandcampPage, NameMatch, name_match,
};
use dig::discogs::matching::fold;
use dig::jobs::JobId;
use dig::preview::clip::bandcamp_key;
use dig::preview::fetcher::preview_path;
use dig::preview::scheduler::PreviewCommand;
use platform::TrackRef;

use super::DiggrApp;
use super::digging::{JobView, SendMode};
use crate::crates::{CrateId, MAX_NAME};
use crate::playlist::{
    ClipSource, Entry, EntryId, EntryStatus, NewEntry, Origin, UnavailableKind, WaitKind,
};

/// Bandcamp sends get job ids far from the intake's, so both fit in one list of sends.
const FIRST_JOB: JobId = 1 << 40;
/// What an entry that Bandcamp lists but doesn't stream says.
pub(super) const NOT_STREAMABLE: &str = "no clip (not streamable)";

/// One Bandcamp send being read.
struct Job {
    target: CrateId,
    /// The label the entries are credited to: the label crate's, else the subdomain.
    label_name: String,
    /// "The Ooze EP", "Label: Analogical Force": what messages call it.
    name: String,
    /// Following a label (or refreshing it): the summary counts new records.
    label: bool,
    skip_passed: bool,
    added: usize,
    fixed: usize,
    skipped: usize,
    /// Albums that brought at least one track.
    new_records: usize,
    failed: Vec<String>,
}

/// "Merge into Label: Siesta?", waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MergeAsk {
    page: BandcampPage,
    /// The Bandcamp label's name.
    name: String,
    into: CrateId,
}

/// Bandcamp's part of the dig state.
#[derive(Default)]
pub(super) struct BandcampState {
    jobs: HashMap<JobId, Job>,
    next: JobId,
    /// Bandcamp clips the preview worker knows the page of.
    located: HashSet<String>,
    pub(super) memory: BandcampMemory,
    /// Merge or Separate questions, oldest first; one is shown at a time.
    pub(super) asks: VecDeque<MergeAsk>,
}

/// How a Bandcamp track meets a crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    /// The crate has it, playing (or about to): left alone.
    Skip,
    /// The crate has it without a preview: it gets the Bandcamp audio.
    Fix(EntryId),
    /// The crate lacks it.
    Add,
}

/// What a track is matched by: "artist - title", folded, without "(original mix)" or a
/// "feat." credit.
pub(super) fn match_key(artist: &str, title: &str) -> String {
    let clean = |s: &str| -> String {
        let mut s = s.to_lowercase();
        for cut in [
            " feat. ",
            " feat ",
            " ft. ",
            " featuring ",
            "(feat. ",
            "(feat ",
            "(ft. ",
        ] {
            if let Some(at) = s.find(cut) {
                // A bracketed credit ends at its bracket; a bare one runs to the end.
                let rest = &s[at + cut.len()..];
                let end = if cut.starts_with('(') {
                    rest.find(')').map_or(s.len(), |i| at + cut.len() + i + 1)
                } else {
                    rest.find('(').map_or(s.len(), |i| at + cut.len() + i)
                };
                s.replace_range(at..end, " ");
            }
        }
        let s = s
            .replace("(original mix)", " ")
            .replace("original mix", " ");
        fold(&s).split_whitespace().collect::<Vec<_>>().join(" ")
    };
    format!("{} - {}", clean(artist), clean(title))
}

/// A catalogue number for comparing: letters and digits, upper case ("af-070" = "AF070").
pub(super) fn catno_key(catno: &str) -> String {
    catno
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn fixable(e: &Entry) -> bool {
    matches!(
        e.status,
        EntryStatus::Unavailable(
            UnavailableKind::NoClip | UnavailableKind::ClipFailed | UnavailableKind::NotFound
        )
    ) && e.origin.as_ref().is_some_and(|o| o.bandcamp.is_empty())
}

/// What each of an album's tracks does to a crate, in the album's order. With the album's
/// catalogue number held by some entries, only those are compared.
pub(super) fn plan_merge(entries: &[Entry], album: &BandcampAlbum) -> Vec<Step> {
    let catno = catno_key(&album.catno);
    let same_catno = |e: &&Entry| {
        e.origin
            .as_ref()
            .is_some_and(|o| catno_key(&o.catno) == catno)
    };
    let narrowed = !catno.is_empty() && entries.iter().any(|e| same_catno(&e));
    let candidates: Vec<&Entry> = entries
        .iter()
        .filter(|e| !narrowed || same_catno(e))
        .collect();
    let mut used: HashSet<EntryId> = HashSet::new();
    let mut seen: HashSet<String> = HashSet::new();
    album
        .tracks
        .iter()
        .map(|t| {
            let key = match_key(&t.artist, &t.title);
            if !seen.insert(key.clone()) {
                return Step::Skip; // listed twice
            }
            let matches: Vec<&&Entry> = candidates
                .iter()
                .filter(|e| match_key(&e.artist, &e.title) == key)
                .collect();
            if matches.is_empty() {
                return Step::Add;
            }
            if !t.streamable {
                return Step::Skip;
            }
            // One of them plays already: nothing to do.
            if matches.iter().any(|e| !fixable(e)) {
                return Step::Skip;
            }
            match matches.iter().find(|e| !used.contains(&e.id)) {
                Some(e) => {
                    used.insert(e.id);
                    Step::Fix(e.id)
                }
                None => Step::Skip,
            }
        })
        .collect()
}

impl BandcampState {
    pub(super) fn new(memory: BandcampMemory) -> Self {
        Self {
            memory,
            ..Default::default()
        }
    }
}

impl DiggrApp {
    // ---- sending ------------------------------------------------------------------------

    /// A Bandcamp page, pasted or from the browser. From the browser a label page follows
    /// the label; otherwise its tracks go where `mode` says, except that the albums of a
    /// followed label always go to its crate.
    pub(super) fn bandcamp_send(
        &mut self,
        page: BandcampPage,
        mode: SendMode,
        skip_passed: Option<bool>,
        title: Option<&str>,
        from_browser: bool,
    ) {
        if page.kind == BandcampKind::Label && from_browser {
            return self.bandcamp_follow(page, title);
        }
        let Some(d) = &self.dig else { return };
        if !d.started() {
            return self.notify("Bandcamp isn't ready yet");
        }
        let skip_passed = skip_passed.unwrap_or(d.settings.skip_passed);
        let name = page.provisional_name();
        if page.kind != BandcampKind::Label
            && let Some(c) = self.crates.find_bandcamp(&page.name)
        {
            if !self.crates.load(c) {
                return;
            }
            return self.bandcamp_start(page, c, name, false, skip_passed);
        }
        let target = match &mode {
            SendMode::Enqueue => {
                let c = self.crates.shown_id();
                if self.refuse_discogs_insert(c) {
                    return;
                }
                c
            }
            SendMode::Play | SendMode::Crate(_) => {
                let crate_name = match &mode {
                    SendMode::Crate(n) => n.clone(),
                    _ => name.clone(),
                };
                let crate_name: String = crate_name.trim().chars().take(MAX_NAME).collect();
                let (id, created) = match self.crates.find(&crate_name) {
                    Some(id) => (id, false),
                    None => match self.crates.create(&crate_name) {
                        Ok(id) => (id, true),
                        Err(e) => return self.notify(e),
                    },
                };
                if self.refuse_discogs_insert(id) || !self.crates.load(id) {
                    return;
                }
                if mode == SendMode::Play || created {
                    self.show_crate(id);
                    if !self.settings.show_playlist {
                        self.settings.show_playlist = true;
                        self.mark_settings();
                    }
                }
                if mode == SendMode::Play
                    && let Some(d) = &mut self.dig
                {
                    d.play_when_ready = Some(id);
                }
                id
            }
        };
        self.bandcamp_start(page, target, name, false, skip_passed);
    }

    /// Reads `page` into `target`.
    fn bandcamp_start(
        &mut self,
        page: BandcampPage,
        target: CrateId,
        name: String,
        label: bool,
        skip_passed: bool,
    ) {
        let to = self.crates.name(target).to_owned();
        let label_name = match self.crates.is_label(target) {
            true => to.strip_prefix("Label: ").unwrap_or(&to).to_owned(),
            false => page.name.clone(),
        };
        let skip: Vec<String> = match label {
            true => self.bandcamp_read_albums(target),
            false => Vec::new(),
        };
        let Some(d) = &mut self.dig else { return };
        let bc = &mut d.bandcamp;
        bc.next = bc.next.max(FIRST_JOB) + 1;
        let job = bc.next;
        bc.jobs.insert(
            job,
            Job {
                target,
                label_name,
                name: name.clone(),
                label,
                skip_passed,
                added: 0,
                fixed: 0,
                skipped: 0,
                new_records: 0,
                failed: Vec::new(),
            },
        );
        d.jobs.insert(
            job,
            JobView {
                name: format!("Bandcamp: {name}"),
                target,
                done: 0,
                total: 0,
            },
        );
        d.preview(PreviewCommand::ReadBandcamp { job, page, skip });
        if !label {
            self.notify(if to == name {
                format!("Digging {to} on Bandcamp")
            } else {
                format!("{name} › {to}")
            });
        }
    }

    /// The album pages a label crate has read.
    fn bandcamp_read_albums(&self, c: CrateId) -> Vec<String> {
        self.dig
            .as_ref()
            .and_then(|d| d.bandcamp.memory.albums.get(&c))
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// A Bandcamp label from the browser: refreshed when followed; else it joins the
    /// followed label of the same name, asks when a name is close, or gets its own crate.
    pub(super) fn bandcamp_follow(&mut self, page: BandcampPage, title: Option<&str>) {
        let Some(d) = &self.dig else { return };
        if !d.started() {
            return self.notify("Bandcamp isn't ready yet");
        }
        if let Some(c) = self.crates.find_bandcamp(&page.name) {
            return self.bandcamp_refresh(c);
        }
        let name = bandcamp::label_name(title, &page);
        let mut same = None;
        let mut close = None;
        for c in self.crates.labels().filter(|c| c.bandcamp.is_none()) {
            if d.bandcamp
                .memory
                .separate
                .contains(&(page.name.clone(), c.id))
            {
                continue;
            }
            for theirs in [name.as_str(), page.name.as_str()] {
                match name_match(&c.name, theirs) {
                    NameMatch::Same => same = same.or(Some(c.id)),
                    NameMatch::Close => close = close.or(Some(c.id)),
                    NameMatch::Different => {}
                }
            }
        }
        if let Some(c) = same {
            return self.bandcamp_merge_into(page, c);
        }
        if let Some(into) = close {
            let ask = MergeAsk { page, name, into };
            if let Some(d) = &mut self.dig
                && !d.bandcamp.asks.contains(&ask)
            {
                d.bandcamp.asks.push_back(ask);
            }
            return;
        }
        self.bandcamp_new_label(page, &name);
    }

    /// The Bandcamp label joins label crate `c`, and its albums are merged in.
    fn bandcamp_merge_into(&mut self, page: BandcampPage, c: CrateId) {
        if !self.crates.load(c) {
            return;
        }
        self.crates.set_bandcamp(c, &page.name);
        let name = self.crates.name(c).to_owned();
        self.notify(format!("Merged into {name}"));
        self.bandcamp_start(page, c, name, true, true);
    }

    /// A crate of its own under LABELS for a Bandcamp label.
    fn bandcamp_new_label(&mut self, page: BandcampPage, name: &str) {
        let base: String = format!("Label: {name}").chars().take(MAX_NAME).collect();
        let crate_name = (1..)
            .map(|n| match n {
                1 => base.clone(),
                n => format!("{base} ({n})"),
            })
            .find(|n| self.crates.find(n).is_none())
            .unwrap_or(base);
        let c = match self.crates.create(&crate_name) {
            Ok(c) => c,
            Err(e) => return self.notify(e),
        };
        self.crates.set_bandcamp(c, &page.name);
        if !self.crates.load(c) {
            return;
        }
        self.notify(format!("Added label {name}"));
        self.bandcamp_start(page, c, crate_name, true, true);
    }

    /// Refresh label's Bandcamp half: the label's albums not read before.
    pub(super) fn bandcamp_refresh(&mut self, c: CrateId) {
        let Some(name) = self.crates.bandcamp_of(c).map(str::to_owned) else {
            return;
        };
        if !self.crates.load(c) {
            return;
        }
        let page = BandcampPage {
            name,
            kind: BandcampKind::Label,
        };
        let crate_name = self.crates.name(c).to_owned();
        self.bandcamp_start(page, c, crate_name, true, true);
    }

    /// Whether crate `c` is reading from Bandcamp.
    pub(super) fn bandcamp_reading(&self, c: CrateId) -> bool {
        self.dig
            .as_ref()
            .is_some_and(|d| d.bandcamp.jobs.values().any(|j| j.target == c))
    }

    // ---- what the preview worker read --------------------------------------------------

    pub(super) fn bandcamp_listed(&mut self, job: JobId, albums: usize) {
        if let Some(v) = self.dig.as_mut().and_then(|d| d.jobs.get_mut(&job)) {
            v.total = albums;
        }
    }

    pub(super) fn bandcamp_failed(&mut self, job: JobId, page: &str, error: &str) {
        let Some(j) = self
            .dig
            .as_mut()
            .and_then(|d| d.bandcamp.jobs.get_mut(&job))
        else {
            return;
        };
        j.failed.push(format!("{page}: {error}"));
        if let Some(v) = self.dig.as_mut().and_then(|d| d.jobs.get_mut(&job)) {
            v.done += 1;
        }
    }

    /// An album read: merged into its job's crate.
    pub(super) fn bandcamp_album(&mut self, job: JobId, album: BandcampAlbum) {
        let Some(d) = &mut self.dig else { return };
        let Some(j) = d.bandcamp.jobs.get(&job) else {
            return;
        };
        let (target, skip_passed, label) = (j.target, j.skip_passed, j.label_name.clone());
        if let Some(v) = d.jobs.get_mut(&job) {
            v.done += 1;
        }
        d.bandcamp
            .memory
            .albums
            .entry(target)
            .or_default()
            .insert(album.url.clone());
        let Some(p) = self.crates.get_mut(target) else {
            return;
        };
        let steps = plan_merge(p.entries(), &album);
        let Some(d) = &mut self.dig else { return };
        let dir = d.previews_dir();
        let catno = catno_key(&album.catno);
        let mut last_of_catno = (!catno.is_empty())
            .then(|| {
                p.entries()
                    .iter()
                    .rev()
                    .find(|e| {
                        e.origin
                            .as_ref()
                            .is_some_and(|o| catno_key(&o.catno) == catno)
                    })
                    .map(|e| e.id)
            })
            .flatten();
        let (mut added, mut fixed, mut skipped) = (0, 0, 0);
        let mut ready: Vec<(EntryId, String)> = Vec::new();
        let mut unstreamable: Vec<EntryId> = Vec::new();
        for (t, step) in album.tracks.iter().zip(steps) {
            let key = bandcamp_key(&t.track_id);
            match step {
                Step::Skip => skipped += 1,
                Step::Fix(id) => {
                    let Some(e) = p.entries_mut().find(|e| e.id == id) else {
                        continue;
                    };
                    let Some(o) = e.origin.as_mut() else { continue };
                    let old = o.clip.clone();
                    o.bandcamp = t.url.clone();
                    o.bandcamp_clip = key.clone();
                    o.youtube_clip = old.clone().unwrap_or_default();
                    o.clip = Some(key.clone());
                    e.source = Some(t.url.clone());
                    // A pass follows the track to its new clip.
                    if let Some(old) = old
                        && let Some(&at) = d.memory.passed.get(&old)
                    {
                        d.memory.passed.insert(key.clone(), at);
                    }
                    p.set_waiting(id, WaitKind::Queued);
                    ready.push((id, key));
                    fixed += 1;
                }
                Step::Add => {
                    if skip_passed && d.memory.is_passed(&key) {
                        skipped += 1;
                        continue;
                    }
                    let origin = Origin {
                        page: album.url.clone(),
                        label: label.clone(),
                        catno: album.catno.clone(),
                        year: album.year,
                        clip: t.streamable.then(|| key.clone()),
                        album: album.title.clone(),
                        cover: album.cover.clone(),
                        artist: album.artist.clone(),
                        bandcamp: t.url.clone(),
                        bandcamp_clip: key.clone(),
                        ..Origin::default()
                    };
                    let new = NewEntry {
                        artist: t.artist.clone(),
                        title: t.title.clone(),
                        source: t.streamable.then(|| t.url.clone()),
                        origin: Some(origin),
                        duration: t.duration,
                        status: WaitKind::Queued,
                    };
                    let id = match last_of_catno {
                        Some(after) => p.insert_after(after, new),
                        None => {
                            let end = p.entries().last().map(|e| e.id);
                            match end {
                                Some(after) => p.insert_after(after, new),
                                None => {
                                    let n = new;
                                    let id = p.add_waiting(
                                        n.artist, n.title, n.source, n.origin, n.status,
                                    );
                                    if let Some(e) = p.entries_mut().find(|e| e.id == id) {
                                        e.duration = n.duration;
                                    }
                                    id
                                }
                            }
                        }
                    };
                    if !catno.is_empty() {
                        last_of_catno = Some(id);
                    }
                    if t.streamable {
                        ready.push((id, key));
                    } else {
                        unstreamable.push(id);
                    }
                    added += 1;
                }
            }
        }
        for id in unstreamable {
            p.set_unavailable(id, NOT_STREAMABLE);
        }
        if let Some(j) = d.bandcamp.jobs.get_mut(&job) {
            j.added += added;
            j.fixed += fixed;
            j.skipped += skipped;
            if added > 0 {
                j.new_records += 1;
            }
        }
        if fixed > 0 {
            let err = d.save_memory();
            self.dig_notify(err);
        }
        self.mark_crate(target);
        // Previews already in the cache play at once.
        for (id, key) in ready {
            let path = preview_path(&dir, &key);
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > 0) {
                self.set_audio(target, id, TrackRef::new(path.to_string_lossy()));
            }
        }
    }

    /// Every page of a job read: the summary.
    pub(super) fn bandcamp_done(&mut self, job: JobId) {
        let Some(d) = &mut self.dig else { return };
        d.jobs.remove(&job);
        let Some(j) = d.bandcamp.jobs.remove(&job) else {
            return;
        };
        if let Some(cfg) = d.config.as_deref()
            && let Err(e) = d.bandcamp.memory.save(cfg)
        {
            self.notify(format!("Could not save what Bandcamp read: {e}"));
        }
        let name = if j.label {
            self.crates.name(j.target).to_owned()
        } else {
            j.name.clone()
        };
        let read_nothing = j.added + j.fixed + j.skipped == 0;
        let text = if read_nothing && !j.failed.is_empty() {
            format!("{name}: Bandcamp couldn't be read ({})", j.failed[0])
        } else if j.label {
            // Beside a Discogs label, which has its own summary: this one says it is Bandcamp's.
            let on = match self.crates.label_of(j.target) {
                Some(_) => " on Bandcamp",
                None => "",
            };
            match j.new_records {
                0 if j.fixed == 0 => format!("{name}: up to date{on}"),
                0 => format!("{name}: {} tracks fixed{on}", j.fixed),
                1 => format!("{name}: 1 new record{on}"),
                n => format!("{name}: {n} new records{on}"),
            }
        } else {
            format!(
                "{name}: {} added, {} fixed, {} skipped",
                j.added, j.fixed, j.skipped
            )
        };
        self.notify(text);
    }

    /// The Bandcamp clips among `keys` the preview worker doesn't know the page of yet: told
    /// before they are wanted.
    pub(super) fn bandcamp_locate(&mut self, keys: &[String]) {
        let Some(d) = &self.dig else { return };
        let missing: Vec<&String> = keys
            .iter()
            .filter(|k| ClipSource::of(k) == ClipSource::Bandcamp)
            .filter(|k| !d.bandcamp.located.contains(*k))
            .collect();
        if missing.is_empty() {
            return;
        }
        let mut urls: HashMap<String, String> = HashMap::new();
        for c in self.crates.loaded_ids() {
            let Some(p) = self.crates.get(c) else {
                continue;
            };
            for e in p.entries() {
                if let Some(o) = &e.origin
                    && !o.bandcamp.is_empty()
                    && missing.contains(&&o.bandcamp_clip)
                {
                    urls.insert(o.bandcamp_clip.clone(), o.bandcamp.clone());
                }
            }
        }
        let Some(d) = &mut self.dig else { return };
        d.bandcamp.located.extend(urls.keys().cloned());
        d.preview(PreviewCommand::Locate(urls.into_iter().collect()));
    }

    // ---- the entry ----------------------------------------------------------------------

    /// Play from Bandcamp / Play from YouTube: the entry's clip switches, and its preview is
    /// downloaded (the other stays in the cache).
    pub(super) fn bandcamp_switch(&mut self, c: CrateId, id: EntryId, to: ClipSource) {
        let Some(p) = self.crates.get_mut(c) else {
            return;
        };
        let Some(e) = p.entries_mut().find(|e| e.id == id) else {
            return;
        };
        let Some(o) = e.origin.as_mut() else { return };
        let old = o.clip.clone();
        if !o.switch_to(to) {
            return;
        }
        let new = o.clip.clone().unwrap_or_default();
        e.source = match to {
            ClipSource::Bandcamp => Some(o.bandcamp.clone()),
            ClipSource::YouTube => Some(dig::preview::fetcher::clip_url(&new)),
        };
        let playing = self.crates.playing_id() == c && self.crates.playing().current() == Some(id);
        let Some(p) = self.crates.get_mut(c) else {
            return;
        };
        // The playing entry keeps its audio until it next starts.
        if !playing {
            p.set_waiting(id, WaitKind::Queued);
        }
        if let Some(d) = &mut self.dig
            && let Some(old) = old
            && let Some(&at) = d.memory.passed.get(&old)
        {
            d.memory.passed.insert(new.clone(), at);
            let err = d.save_memory();
            self.dig_notify(err);
        }
        self.mark_crate(c);
        if let Some(d) = &self.dig {
            let path = preview_path(&d.previews_dir(), &new);
            if !playing && std::fs::metadata(&path).is_ok_and(|m| m.len() > 0) {
                self.set_audio(c, id, TrackRef::new(path.to_string_lossy()));
            }
        }
    }

    // ---- Merge or Separate ----------------------------------------------------------------

    /// The question for the oldest close name, if any.
    pub(super) fn bandcamp_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(ask) = self
            .dig
            .as_ref()
            .and_then(|d| d.bandcamp.asks.front().cloned())
        else {
            return;
        };
        let into = self.crates.name(ask.into).to_owned();
        let mut choice = None;
        egui::Modal::new(egui::Id::new("bandcamp-merge")).show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.strong(format!("Merge {} into {into}?", ask.name));
            ui.label(format!(
                "The Bandcamp label {} ({}.bandcamp.com) has a name close to {into}. Merged, \
                 one crate follows both; separate, each has its own.",
                ask.name, ask.page.name
            ));
            ui.horizontal(|ui| {
                if ui.button("Merge").clicked() {
                    choice = Some(true);
                }
                if ui.button("Separate").clicked() {
                    choice = Some(false);
                }
            });
        });
        let Some(merge) = choice else { return };
        let Some(d) = &mut self.dig else { return };
        d.bandcamp.asks.pop_front();
        if merge {
            return self.bandcamp_merge_into(ask.page, ask.into);
        }
        d.bandcamp
            .memory
            .separate
            .insert((ask.page.name.clone(), ask.into));
        if let Some(cfg) = d.config.as_deref() {
            let _ = d.bandcamp.memory.save(cfg);
        }
        self.bandcamp_new_label(ask.page, &ask.name);
    }

    /// A deleted crate: its reads stop counting, and what it read is forgotten (a later
    /// follow reads every album again).
    pub(super) fn bandcamp_crate_deleted(&mut self, c: CrateId) {
        let Some(d) = &mut self.dig else { return };
        let bc = &mut d.bandcamp;
        bc.jobs.retain(|_, j| j.target != c);
        bc.asks.retain(|a| a.into != c);
        let known = bc.memory.albums.remove(&c).is_some();
        let before = bc.memory.separate.len();
        bc.memory.separate.retain(|(_, id)| *id != c);
        if (known || bc.memory.separate.len() != before)
            && let Some(cfg) = d.config.as_deref()
        {
            let _ = bc.memory.save(cfg);
        }
    }

    /// Following a Discogs label that isn't followed yet: the Bandcamp-only label crate of
    /// the same name, if there is one, follows it too.
    pub(super) fn bandcamp_crate_for_discogs(&self, name: &str) -> Option<CrateId> {
        self.crates
            .labels()
            .filter(|c| c.label.is_none() && c.bandcamp.is_some())
            .find(|c| name_match(&c.name, name) == NameMatch::Same)
            .map(|c| c.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dig::bandcamp::BandcampTrack;

    fn entry(id: EntryId, artist: &str, title: &str, catno: &str, status: EntryStatus) -> Entry {
        Entry {
            id,
            track: TrackRef::new(""),
            source: None,
            origin: Some(Origin {
                catno: catno.into(),
                clip: Some(format!("clip{id:07}")),
                ..Origin::default()
            }),
            title: title.into(),
            artist: artist.into(),
            album: String::new(),
            duration: None,
            bpm: None,
            status,
        }
    }

    fn track(id: &str, title: &str) -> BandcampTrack {
        BandcampTrack {
            track_id: id.into(),
            url: format!("https://analogicalforce.bandcamp.com/track/{id}"),
            artist: "Patricia".into(),
            title: title.into(),
            duration: Some(400.0),
            streamable: true,
        }
    }

    fn album(catno: &str, tracks: Vec<BandcampTrack>) -> BandcampAlbum {
        BandcampAlbum {
            url: "https://analogicalforce.bandcamp.com/album/af070".into(),
            title: "The Ooze EP".into(),
            catno: catno.into(),
            artist: "Patricia".into(),
            cover: String::new(),
            year: Some(2026),
            tracks,
        }
    }

    const NOT_FOUND: EntryStatus = EntryStatus::Unavailable(UnavailableKind::NotFound);

    #[test]
    fn match_keys_ignore_what_says_nothing() {
        let k = match_key("Patricia", "The Ooze");
        for (a, t) in [
            ("PATRICIA", "The Ooze (Original Mix)"),
            ("Patricia", "The  Ooze"),
            ("Patricia feat. Somebody", "The Ooze"),
            ("Patricia", "The Ooze (feat. Somebody)"),
            ("Patricia", "The Ooze!"),
        ] {
            assert_eq!(match_key(a, t), k, "{a} - {t}");
        }
        assert_eq!(match_key("Désirée", "Café"), match_key("Desiree", "Cafe"));
        assert_ne!(
            match_key("Patricia", "The Ooze (Dub)"),
            k,
            "a remix is another track"
        );
    }

    #[test]
    fn every_row_of_the_merge() {
        let entries = vec![
            entry(1, "Patricia", "The Ooze", "AF070", NOT_FOUND),
            entry(2, "Patricia", "Swamp", "AF-070", EntryStatus::Ready),
            entry(
                3,
                "Patricia",
                "Gone",
                "AF070",
                EntryStatus::Waiting(WaitKind::Queued),
            ),
            entry(
                4,
                "Patricia",
                "Dead",
                "AF070",
                EntryStatus::Unavailable(UnavailableKind::ClipFailed),
            ),
        ];
        let a = album(
            "AF070",
            vec![
                track("1", "The Ooze"),
                track("2", "Swamp"),
                track("3", "Gone"),
                track("4", "Dead (Original Mix)"),
                track("5", "Bonus"),
            ],
        );
        assert_eq!(
            plan_merge(&entries, &a),
            [
                Step::Fix(1),
                Step::Skip,
                Step::Skip,
                Step::Fix(4),
                Step::Add
            ]
        );
    }

    #[test]
    fn the_same_title_on_another_record_is_another_track() {
        let entries = vec![
            entry(1, "Patricia", "Swamp", "AF012", NOT_FOUND),
            entry(2, "Patricia", "Other", "AF070", EntryStatus::Ready),
        ];
        let a = album("AF070", vec![track("2", "Swamp")]);
        assert_eq!(plan_merge(&entries, &a), [Step::Add]);
        // Without catalogue numbers, every entry is compared.
        let a = album("", vec![track("2", "Swamp")]);
        assert_eq!(plan_merge(&entries, &a), [Step::Fix(1)]);
    }

    #[test]
    fn a_track_not_streamed_fixes_nothing() {
        let entries = vec![entry(1, "Patricia", "The Ooze", "AF070", NOT_FOUND)];
        let mut t = track("1", "The Ooze");
        t.streamable = false;
        assert_eq!(plan_merge(&entries, &album("AF070", vec![t])), [Step::Skip]);
    }

    #[test]
    fn an_origin_saved_before_bandcamp_is_youtube_and_switches_both_ways() {
        let old: Origin = ron::from_str(r#"(page: "p", clip: Some("OOZEclip001"))"#).unwrap();
        assert_eq!(old.source(), Some(ClipSource::YouTube));
        assert_eq!(old.other_source(), None, "nothing to switch to");
        let mut o = Origin {
            bandcamp: "https://a.bandcamp.com/track/x".into(),
            bandcamp_clip: "bc.12".into(),
            ..old
        };
        assert!(o.switch_to(ClipSource::Bandcamp));
        assert_eq!(
            (o.clip.as_deref(), o.youtube()),
            (Some("bc.12"), Some("OOZEclip001"))
        );
        let saved = ron::to_string(&o).unwrap();
        let mut o: Origin = ron::from_str(&saved).unwrap();
        assert_eq!(
            o.source(),
            Some(ClipSource::Bandcamp),
            "kept across a restart"
        );
        assert!(!o.switch_to(ClipSource::Bandcamp));
        assert!(o.switch_to(ClipSource::YouTube));
        assert_eq!(o.clip.as_deref(), Some("OOZEclip001"));
        assert_eq!(o.other_source(), Some(ClipSource::Bandcamp));
    }
}
