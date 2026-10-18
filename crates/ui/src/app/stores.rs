//! Stores in the player: records the browser extension read from a Juno, Beatport, Traxsource
//! or Bleep page the user has open, sent with the bridge (the app never reads those pages
//! itself, see `dig::stores`).
//!
//! A send's tracks are merged into their crate as a Bandcamp album is: a track the crate plays
//! already is skipped, one with no preview ("no clip", "clip failed", "not found") gets the
//! store's sample in place, and only what the crate lacks is added. A Traxsource track has no
//! sample: it waits to be searched for, as a Discogs track without a clip does. A send can
//! hold 500 records, so the crate is indexed once by artist and title rather than scanned per
//! track. Beatport and Traxsource tracks bring their BPM (shown until the analyser has one)
//! and their key.
//!
//! Each record is then looked up on Discogs once (with a token), by artist, title and
//! catalogue number, and linked only when exactly one release fits: its entries get the
//! Discogs verbs (Y, I, ★, owned). What the page said about stock and price is kept per
//! record and shown on its entries, and on any Discogs entry of the linked release.
//!
//! A Bleep list shows its records without tracks: each stands as one entry with no position
//! until it is linked, when its release's tracks come in from Discogs and it goes.
//!
//! Each store entry has a cart pill. Juno and Bleep baskets are reached through the browser
//! extension (`dig::carts`): a click queues the action and the pill changes at once; an answer
//! other than added, already in, removed or sold out opens the store's page instead. Beatport
//! and Traxsource baskets can't be reached, so their pill opens the page straight away.

use dig::bandcamp::fan::DiscogsMatch;
use dig::carts::{CartKind, CartResult, Outcome, StoreCarts};
use dig::discogs::client::ApiError;
use dig::discogs::search::Wanted;
use dig::discogs::url::{Page, PageKind};
use dig::intake::Command;
use dig::juno;
use dig::preview::fetcher::preview_path;
use dig::stores::{Store, StoreMemory, StoreRecord};
use platform::TrackRef;
use std::collections::{HashMap, HashSet};

use super::bandcamp::{Step, catno_key, match_key};
use super::digging::SendMode;
use super::{DiggrApp, StorePill};
use crate::crates::CrateId;
use crate::playlist::{Entry, EntryId, EntryStatus, NewEntry, Origin, UnavailableKind, WaitKind};

/// Discogs lookups for a store's records are keyed `‹store›:‹record›` (`juno:1178909-01`).
fn find_key(store: Store, id: &str) -> String {
    format!("{}:{id}", store.name().to_ascii_lowercase())
}

/// The store and record a lookup's key names.
fn parse_find(key: &str) -> Option<(Store, &str)> {
    let (name, id) = key.split_once(':')?;
    let store = Store::ALL
        .into_iter()
        .find(|s| s.name().eq_ignore_ascii_case(name))?;
    Some((store, id))
}

/// The stores' part of the dig state.
#[derive(Default)]
pub(super) struct StoreState {
    memories: HashMap<Store, StoreMemory>,
    /// Records being looked up on Discogs this session.
    finding: HashSet<(Store, String)>,
    /// Where `juno.ron`, `beatport.ron` and `traxsource.ron` snapshots go to be written (see
    /// `Dig::save_store`).
    pub(super) saver: Option<std::sync::mpsc::Sender<(Store, StoreMemory)>>,
    /// The Juno and Bleep baskets as last known (`store-carts.ron`).
    pub(super) carts: StoreCarts,
    /// Cart actions queued and not answered yet, by action id.
    pending: HashMap<u64, Pending>,
    /// Items a store said were sold out this session.
    sold: HashSet<(Store, String)>,
}

/// A cart action on its way: what the pill shows meanwhile, and the page to open if it fails.
struct Pending {
    store: Store,
    item: String,
    kind: CartKind,
    page: String,
    price: String,
}

/// What a click on an entry's pill does.
enum CartClick {
    Queue(Store, CartKind, String),
    /// Beatport, Traxsource, or a Bleep record whose formats weren't sent: the page opens.
    Page(Store),
}

impl StoreState {
    pub(super) fn load(cache: Option<&std::path::Path>) -> Self {
        let memories = Store::ALL
            .into_iter()
            .map(|s| {
                (
                    s,
                    cache.map(|c| StoreMemory::load(c, s)).unwrap_or_default(),
                )
            })
            .collect();
        Self {
            memories,
            carts: cache.map(StoreCarts::load).unwrap_or_default(),
            ..Self::default()
        }
    }

    /// Whether the item is in the basket: as the pending action will leave it, else as known.
    fn in_cart(&self, store: Store, item: &str) -> bool {
        let pending = self
            .pending
            .values()
            .filter(|p| p.store == store && p.item == item)
            .last()
            .map(|p| p.kind == CartKind::Add);
        pending.unwrap_or_else(|| self.carts.get(store, item).is_some())
    }

    pub(super) fn memory(&self, store: Store) -> &StoreMemory {
        self.memories.get(&store).expect("every store has a memory")
    }

    pub(super) fn memory_mut(&mut self, store: Store) -> &mut StoreMemory {
        self.memories.entry(store).or_default()
    }
}

/// The id of the store's record an entry came from, if it came from that store.
fn record_of(o: &Origin, store: Store) -> &str {
    match store {
        Store::Juno => &o.juno,
        Store::Beatport => &o.beatport,
        Store::Traxsource => &o.traxsource,
        Store::Bleep => &o.bleep,
    }
}

/// An entry with no preview that a store's sample can fix.
fn fixable(e: &Entry) -> bool {
    matches!(
        e.status,
        EntryStatus::Unavailable(
            UnavailableKind::NoClip | UnavailableKind::ClipFailed | UnavailableKind::NotFound
        )
    ) && e
        .origin
        .as_ref()
        .is_some_and(|o| o.juno_clip.is_empty() && o.beatport_clip.is_empty())
}

/// A crate's entries by match key, built once per send.
struct Index {
    /// Per entry: its id, catalogue key and whether a sample can fix it.
    entries: Vec<(EntryId, String, bool)>,
    by_key: HashMap<String, Vec<usize>>,
    /// Per catalogue number, its last entry in the crate (new tracks go after it).
    last: HashMap<String, EntryId>,
    /// Entries fixed by this send (each once).
    used: HashSet<EntryId>,
}

impl Index {
    fn new(entries: &[Entry]) -> Self {
        let mut ix = Index {
            entries: Vec::with_capacity(entries.len()),
            by_key: HashMap::new(),
            last: HashMap::new(),
            used: HashSet::new(),
        };
        for e in entries {
            let catno = e
                .origin
                .as_ref()
                .map(|o| catno_key(&o.catno))
                .unwrap_or_default();
            ix.add(e.id, catno, match_key(&e.artist, &e.title), fixable(e));
        }
        ix
    }

    fn add(&mut self, id: EntryId, catno: String, key: String, fixable: bool) {
        if !catno.is_empty() {
            self.last.insert(catno.clone(), id);
        }
        self.by_key.entry(key).or_default().push(self.entries.len());
        self.entries.push((id, catno, fixable));
    }

    /// What each of a record's tracks does to the crate, in the record's order (the same
    /// rules as a Bandcamp album: with its catalogue number held by some entries, only those
    /// are compared).
    fn plan(&mut self, catno: &str, keys: &[String]) -> Vec<Step> {
        let narrowed = !catno.is_empty() && self.last.contains_key(catno);
        let mut seen = HashSet::new();
        keys.iter()
            .map(|key| {
                if !seen.insert(key.as_str()) {
                    return Step::Skip;
                }
                let matches: Vec<&(EntryId, String, bool)> = self
                    .by_key
                    .get(key)
                    .into_iter()
                    .flatten()
                    .map(|&i| &self.entries[i])
                    .filter(|(_, c, _)| !narrowed || c == catno)
                    .collect();
                if matches.is_empty() {
                    return Step::Add;
                }
                if matches.iter().any(|(_, _, fixable)| !fixable) {
                    return Step::Skip;
                }
                match matches.iter().find(|(id, _, _)| !self.used.contains(id)) {
                    Some((id, _, _)) => {
                        let id = *id;
                        self.used.insert(id);
                        Step::Fix(id)
                    }
                    None => Step::Skip,
                }
            })
            .collect()
    }
}

/// "14 Oct" for a Unix time (UTC).
pub(super) fn day_month(secs: u64) -> String {
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{day} {}", MONTHS[(month - 1) as usize])
}

/// Sets what a store's track tells on an entry's origin: where it came from, its sample (the
/// clip to switch to), its page and key.
fn tag(o: &mut Origin, r: &StoreRecord, t: &dig::stores::StoreTrack) {
    let clip = t.clip.clone().unwrap_or_default();
    match r.store {
        Store::Juno => {
            o.juno = r.id.clone();
            o.juno_slug = r.slug.clone();
            o.juno_clip = clip;
        }
        Store::Beatport => {
            o.beatport = r.id.clone();
            o.beatport_page = t.page.clone();
            o.beatport_clip = clip;
        }
        Store::Traxsource => {
            o.traxsource = r.id.clone();
            o.traxsource_page = t.page.clone();
        }
        Store::Bleep => {
            o.bleep = r.id.clone();
            o.bleep_slug = r.slug.clone();
        }
    }
    if let Some(k) = &t.key {
        o.key = k.clone();
    }
}

impl DiggrApp {
    /// Records from a store's page: merged where `mode` says, remembered, and looked up on
    /// Discogs once each.
    pub(super) fn store_send(
        &mut self,
        records: Vec<StoreRecord>,
        mode: SendMode,
        skip_passed: bool,
    ) {
        if self.dig.is_none() || records.is_empty() {
            return;
        }
        let name = dig::stores::send_name(&records);
        let Some(target) = self.send_target(&mode) else {
            return;
        };
        let (added, fixed, skipped) = self.store_merge(target, &records, skip_passed);
        self.play_send_ended(target);
        let Some(d) = &mut self.dig else { return };
        for store in Store::ALL {
            let of: Vec<StoreRecord> = records
                .iter()
                .filter(|r| r.store == store)
                .cloned()
                .collect();
            if !of.is_empty() {
                d.stores.memory_mut(store).saw(&of, dig::now_secs());
                d.save_store(store);
            }
        }
        self.store_link_known(&records);
        self.store_find(&records);
        self.notify(format!(
            "{name}: {added} added, {fixed} fixed, {skipped} skipped"
        ));
    }

    /// Merges the records' tracks into crate `target`; returns (added, fixed, skipped).
    fn store_merge(
        &mut self,
        target: CrateId,
        records: &[StoreRecord],
        skip_passed: bool,
    ) -> (usize, usize, usize) {
        let Some(d) = &self.dig else {
            return (0, 0, 0);
        };
        let dir = d.previews_dir();
        let passed: HashSet<String> = records
            .iter()
            .flat_map(|r| r.tracks.iter().filter_map(|t| t.clip.clone()))
            .filter(|k| skip_passed && d.memory.is_passed(k))
            .collect();
        let Some(p) = self.crates.get_mut(target) else {
            return (0, 0, 0);
        };
        let mut ix = Index::new(p.entries());
        let (mut added, mut fixed, mut skipped) = (0, 0, 0);
        let mut ready: Vec<(EntryId, String)> = Vec::new();
        let mut moved_passes: Vec<(String, String)> = Vec::new();
        for r in records {
            let catno = catno_key(&r.catno);
            let keys: Vec<String> = r
                .tracks
                .iter()
                .map(|t| match_key(&t.artist, &t.title))
                .collect();
            let steps = ix.plan(&catno, &keys);
            let mut last_of_catno = ix.last.get(&catno).copied();
            for ((t, key), step) in r.tracks.iter().zip(keys).zip(steps) {
                let page = match t.page.is_empty() {
                    true => r.page.clone(),
                    false => t.page.clone(),
                };
                match (step, &t.clip) {
                    (Step::Skip, _) => skipped += 1,
                    // A track without a sample can't fix anything.
                    (Step::Fix(_), None) => skipped += 1,
                    (Step::Fix(id), Some(clip)) => {
                        let Some(e) = p.entries_mut().find(|e| e.id == id) else {
                            continue;
                        };
                        let Some(o) = e.origin.as_mut() else { continue };
                        let old = o.clip.clone();
                        if let Some(old) = &old
                            && crate::playlist::ClipSource::of(old)
                                == crate::playlist::ClipSource::YouTube
                        {
                            o.youtube_clip = old.clone();
                        }
                        tag(o, r, t);
                        o.clip = Some(clip.clone());
                        e.source = Some(page.clone());
                        if e.bpm.is_none() && t.bpm.is_some() {
                            e.bpm = t.bpm;
                            if let Some(o) = e.origin.as_mut() {
                                o.store_bpm = true;
                            }
                        }
                        if let Some(old) = old {
                            moved_passes.push((old, clip.clone()));
                        }
                        p.set_waiting(id, WaitKind::Queued);
                        ready.push((id, clip.clone()));
                        fixed += 1;
                    }
                    (Step::Add, clip) => {
                        if clip.as_ref().is_some_and(|c| passed.contains(c)) {
                            skipped += 1;
                            continue;
                        }
                        let mut origin = Origin {
                            page: page.clone(),
                            label: r.label.clone(),
                            catno: r.catno.clone(),
                            position: t.position.clone(),
                            clip: clip.clone(),
                            album: r.title.clone(),
                            cover: r.cover.clone(),
                            artist: r.artist.clone(),
                            formats: r.format.clone(),
                            store_bpm: t.bpm.is_some(),
                            ..Origin::default()
                        };
                        tag(&mut origin, r, t);
                        // No sample: found by searching, like a Discogs track without a clip.
                        let status = match clip {
                            Some(_) => WaitKind::Queued,
                            None => {
                                origin.search_key =
                                    format!("{}/{}", r.store.name().to_ascii_lowercase(), t.track);
                                WaitKind::Search
                            }
                        };
                        let new = NewEntry {
                            artist: t.artist.clone(),
                            title: t.title.clone(),
                            source: Some(page.clone()),
                            origin: Some(origin),
                            duration: (t.secs > 0).then_some(f64::from(t.secs)),
                            status,
                        };
                        let after = last_of_catno.or_else(|| p.entries().last().map(|e| e.id));
                        let id = match after {
                            Some(after) => p.insert_after(after, new),
                            None => {
                                let id = p.add_waiting(
                                    new.artist, new.title, new.source, new.origin, new.status,
                                );
                                if let Some(e) = p.entries_mut().find(|e| e.id == id) {
                                    e.duration = new.duration;
                                }
                                id
                            }
                        };
                        if let Some(e) = p.entries_mut().find(|e| e.id == id) {
                            e.bpm = t.bpm;
                        }
                        if !catno.is_empty() {
                            last_of_catno = Some(id);
                        }
                        ix.add(id, catno.clone(), key, false);
                        if let Some(clip) = clip {
                            ready.push((id, clip.clone()));
                        }
                        added += 1;
                    }
                }
            }
        }
        // A pass follows the track to its new clip.
        if let Some(d) = &mut self.dig
            && !moved_passes.is_empty()
        {
            let mut moved = false;
            for (old, new) in moved_passes {
                if let Some(&at) = d.memory.passed.get(&old) {
                    d.memory.passed.insert(new, at);
                    moved = true;
                }
            }
            if moved {
                let err = d.save_memory();
                self.dig_notify(err);
            }
        }
        self.mark_crate(target);
        // Samples already in the cache play at once. The folder is listed once: a send can
        // add thousands of tracks.
        let cached: HashSet<std::ffi::OsString> = std::fs::read_dir(&dir)
            .map(|rd| rd.flatten().map(|e| e.file_name()).collect())
            .unwrap_or_default();
        for (id, key) in ready {
            let path = preview_path(&dir, &key);
            if path.file_name().is_some_and(|n| cached.contains(n))
                && std::fs::metadata(&path).is_ok_and(|m| m.len() > 0)
            {
                self.set_audio(target, id, TrackRef::new(path.to_string_lossy()));
            }
        }
        (added, fixed, skipped)
    }

    // ---- Discogs ------------------------------------------------------------------------

    /// Records linked before: their entries take the release at once.
    fn store_link_known(&mut self, records: &[StoreRecord]) {
        let Some(d) = &self.dig else { return };
        let links: Vec<(Store, String, u64)> = records
            .iter()
            .filter_map(|r| {
                let release = d.stores.memory(r.store).products.get(&r.id)?.release?;
                Some((r.store, r.id.clone(), release))
            })
            .collect();
        for (store, id, release) in links {
            self.store_link(store, &id, release);
        }
    }

    /// Records never looked up are looked up on Discogs (with a token), behind other work.
    fn store_find(&mut self, records: &[StoreRecord]) {
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none() {
            return;
        }
        let mut finds = Vec::new();
        for r in records {
            let searched = d
                .stores
                .memory(r.store)
                .products
                .get(&r.id)
                .is_some_and(|s| s.searched);
            if searched || !d.stores.finding.insert((r.store, r.id.clone())) {
                continue;
            }
            let w = Wanted::new(&r.artist, &r.title, &r.catno, true);
            finds.push((find_key(r.store, &r.id), w));
        }
        if !finds.is_empty() {
            d.send_cmd(Command::FindReleases(finds));
        }
    }

    /// Whether a lookup's key is a store record's.
    pub(super) fn is_store_find(key: &str) -> bool {
        parse_find(key).is_some()
    }

    /// Discogs answered for a record: remembered; exactly one release links its entries.
    pub(super) fn store_found(&mut self, key: &str, result: Result<DiscogsMatch, ApiError>) {
        let Some((store, id)) = parse_find(key) else {
            return;
        };
        let Some(d) = &mut self.dig else { return };
        d.stores.finding.remove(&(store, id.to_owned()));
        // Offline or refused: looked up again at the next send.
        let Ok(m) = result else { return };
        let seen = d
            .stores
            .memory_mut(store)
            .products
            .entry(id.to_owned())
            .or_default();
        seen.searched = true;
        if let DiscogsMatch::Found(release) = m {
            seen.release = Some(release);
        }
        d.save_store(store);
        if let DiscogsMatch::Found(release) = m {
            self.store_link(store, id, release);
        }
    }

    /// Every loaded entry of the store's record `id` with no Discogs record takes `release`.
    /// An entry standing for a whole Bleep record (no position) gives way to the release's
    /// tracks, brought into its crate.
    fn store_link(&mut self, store: Store, id: &str, release: u64) {
        for c in self.crates.loaded_ids() {
            let Some(p) = self.crates.get_mut(c) else {
                continue;
            };
            let mut changed = false;
            let mut standing = Vec::new();
            for e in p.entries_mut() {
                if let Some(o) = e.origin.as_mut()
                    && record_of(o, store) == id
                    && o.release.is_none()
                    && o.master.is_none()
                {
                    if store == Store::Bleep && o.position.is_empty() {
                        standing.push(e.id);
                        continue;
                    }
                    o.release = Some(release);
                    changed = true;
                }
            }
            if !standing.is_empty() {
                p.remove_ids(&standing);
                self.dig_fill(Page::new(PageKind::Release(release)), c);
                changed = true;
            }
            if changed {
                self.mark_crate(c);
            }
        }
    }

    // ---- the entry ----------------------------------------------------------------------

    /// What each store last said of an entry's record ("Juno", "in stock, €14.78 (seen 14
    /// Oct)"): its own record, else the record linked to its release.
    pub(super) fn store_notes(&self, e: &Entry) -> Vec<(&'static str, String)> {
        let (Some(d), Some(o)) = (self.dig.as_ref(), e.origin.as_ref()) else {
            return Vec::new();
        };
        Store::ALL
            .into_iter()
            .filter_map(|store| {
                let mem = d.stores.memory(store);
                let id = record_of(o, store);
                let seen = match id.is_empty() {
                    false => mem.products.get(id)?,
                    true => mem.product_of(o.release?)?.1,
                };
                Some((store.name(), seen.note(dig::now_secs(), day_month)?))
            })
            .collect()
    }

    // ---- baskets --------------------------------------------------------------------------

    /// The entry's store, item and what can be bought of it: a Juno product, a Bleep
    /// release's formats, or (Beatport, Traxsource) only its page.
    fn store_item(&self, e: &Entry) -> Option<(Store, Vec<(String, String)>)> {
        let (d, o) = (self.dig.as_ref()?, e.origin.as_ref()?);
        let price = |store, id: &str| {
            d.stores
                .memory(store)
                .products
                .get(id)
                .map(|s| s.price.clone())
                .unwrap_or_default()
        };
        if !o.juno.is_empty() {
            return Some((
                Store::Juno,
                vec![(o.juno.clone(), price(Store::Juno, &o.juno))],
            ));
        }
        if !o.bleep.is_empty() {
            let buy = d.stores.memory(Store::Bleep).products.get(&o.bleep);
            return Some((Store::Bleep, buy.map(|s| s.buy.clone()).unwrap_or_default()));
        }
        [
            (Store::Beatport, &o.beatport_page),
            (Store::Traxsource, &o.traxsource_page),
        ]
        .into_iter()
        .find(|(_, page)| !page.is_empty())
        .map(|(s, _)| (s, Vec::new()))
    }

    /// The entry's cart pill, when it came from a store.
    pub(crate) fn store_pill_of(&self, e: &Entry) -> Option<StorePill> {
        let (store, items) = self.store_item(e)?;
        let s = &self.dig.as_ref()?.stores;
        let name = store.name();
        let held = items.iter().find(|(id, _)| s.in_cart(store, id));
        let sold = items
            .iter()
            .any(|(id, _)| s.sold.contains(&(store, id.clone())));
        // The price as the store shows it: the basket's, else the page's.
        let price = |id: &str, shown: &str| {
            let p = match s.carts.get(store, id) {
                Some(b) if !b.price.is_empty() => b.price.as_str(),
                _ => shown,
            };
            match p.is_empty() {
                true => String::new(),
                false => format!(", {p}"),
            }
        };
        let tip = match (held, items.as_slice()) {
            (Some((id, shown)), _) => {
                format!("In your {name} cart{}: click to remove", price(id, shown))
            }
            (None, [(id, shown)]) => format!("Add to your {name} cart{}", price(id, shown)),
            (None, []) => format!("Open on {name} to buy it there"),
            (None, _) => format!("Add to your {name} cart: choose a format"),
        };
        Some(StorePill {
            in_cart: held.is_some(),
            sold,
            choose: match held {
                None if items.len() > 1 => items.clone(),
                _ => Vec::new(),
            },
            tip,
        })
    }

    /// A click on the pill of entry `id` in crate `c` (`format`: the Bleep format chosen):
    /// queues the add or remove, or opens the store's page.
    pub(super) fn store_cart_click(&mut self, c: CrateId, id: EntryId, format: Option<String>) {
        let Some(e) = self.crates.get(c).and_then(|p| p.get(id)).cloned() else {
            return;
        };
        let Some((store, items)) = self.store_item(&e) else {
            return;
        };
        let page = self
            .store_pages(&e)
            .into_iter()
            .find(|(s, _)| *s == store)
            .map(|(_, p)| p)
            .unwrap_or_default();
        let Some(d) = self.dig.as_mut() else { return };
        let held = items.iter().find(|(i, _)| d.stores.in_cart(store, i));
        let click = match (held, format, items.as_slice()) {
            _ if !dig::carts::reachable(store) => CartClick::Page(store),
            (Some((item, _)), _, _) => CartClick::Queue(store, CartKind::Remove, item.clone()),
            (None, Some(f), _) if items.iter().any(|(i, _)| *i == f) => {
                CartClick::Queue(store, CartKind::Add, f)
            }
            (None, _, [(item, _)]) => CartClick::Queue(store, CartKind::Add, item.clone()),
            _ => CartClick::Page(store),
        };
        let queued = match &click {
            CartClick::Queue(store, kind, item) => d
                .bridge_shared
                .as_ref()
                .and_then(|sh| {
                    sh.cart()
                        .push(*store, *kind, item, std::time::Instant::now())
                })
                .map(|action| (action, *store, *kind, item.clone())),
            CartClick::Page(_) => None,
        };
        match (queued, click) {
            (Some((action, store, kind, item)), _) => {
                let price = items
                    .iter()
                    .find(|(i, _)| *i == item)
                    .map(|(_, p)| p.clone())
                    .unwrap_or_default();
                d.stores.pending.insert(
                    action,
                    Pending {
                        store,
                        item,
                        kind,
                        page,
                        price,
                    },
                );
            }
            (None, CartClick::Page(s)) if !dig::carts::reachable(s) => {
                self.cart_fallback(
                    &page,
                    format!("{}'s cart can't be reached from Diggr", s.name()),
                );
            }
            // A Bleep record sent from a list: its tiles name no format to add.
            (None, CartClick::Page(s)) if items.is_empty() => {
                let why = format!(
                    "Send the record's own page from {} to choose its format here",
                    s.name()
                );
                self.cart_fallback(&page, why);
            }
            (None, _) => {
                self.cart_fallback(&page, format!("Couldn't reach your {} cart", store.name()));
            }
        }
    }

    /// Opens the item's page on its store, saying why.
    fn cart_fallback(&mut self, page: &str, why: String) {
        let Some(d) = &self.dig else { return };
        match page.is_empty() {
            true => self.notify(why),
            false => match d.open_url(page) {
                Ok(()) => self.notify(format!("{why}: opened the page instead")),
                Err(err) => self.notify(format!("{why}, and could not open the browser: {err}")),
            },
        }
    }

    /// A cart action ended: the basket and pill follow, the main window says how it went,
    /// and anything but added, already in, removed or sold out opens the store's page.
    pub(super) fn store_cart_result(&mut self, r: CartResult) {
        let Some(d) = self.dig.as_mut() else { return };
        let pending = d.stores.pending.remove(&r.action.id);
        let price = pending.as_ref().map_or("", |p| p.price.as_str());
        if d.stores.carts.apply(&r, price) {
            d.save_store_carts();
        }
        // A read that didn't happen says nothing: there was nothing to do.
        let Some(p) = pending else { return };
        let name = p.store.name();
        match r.outcome {
            Outcome::Added => self.notify(format!("Added to your {name} cart")),
            Outcome::AlreadyIn => self.notify(format!("Already in your {name} cart")),
            Outcome::Removed => self.notify(format!("Removed from your {name} cart")),
            Outcome::SoldOut => {
                d.stores.sold.insert((p.store, p.item));
                self.notify(format!("Sold out on {name}"));
            }
            Outcome::NotLoggedIn => {
                self.cart_fallback(&p.page, format!("Log in to {name} to use its cart"));
            }
            Outcome::Failed | Outcome::Read | Outcome::NoExtension | Outcome::TimedOut => {
                self.cart_fallback(&p.page, format!("Couldn't reach your {name} cart"));
            }
        }
    }

    /// The stores whose page an entry can open, with the page.
    pub(super) fn store_pages(&self, e: &Entry) -> Vec<(Store, String)> {
        let Some(o) = e.origin.as_ref() else {
            return Vec::new();
        };
        let juno = match o.juno.is_empty() {
            false => juno::product_url(&o.juno_slug, &o.juno),
            true => self.dig.as_ref().and_then(|d| {
                let (product, seen) = d.stores.memory(Store::Juno).product_of(o.release?)?;
                juno::product_url(&seen.slug, product)
            }),
        };
        let bleep = match o.bleep.is_empty() {
            false => dig::bleep::release_url(&o.bleep, &o.bleep_slug),
            true => self.dig.as_ref().and_then(|d| {
                let (id, seen) = d.stores.memory(Store::Bleep).product_of(o.release?)?;
                dig::bleep::release_url(id, &seen.slug)
            }),
        };
        [
            (Store::Juno, juno),
            (Store::Beatport, Some(o.beatport_page.clone())),
            (Store::Traxsource, Some(o.traxsource_page.clone())),
            (Store::Bleep, bleep),
        ]
        .into_iter()
        .filter_map(|(s, page)| Some((s, page.filter(|p| !p.is_empty())?)))
        .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_and_month() {
        assert_eq!(day_month(0), "1 Jan");
        // 2026-10-14 12:00 UTC.
        assert_eq!(day_month(1_791_979_200), "14 Oct");
        // 2024-02-29.
        assert_eq!(day_month(1_709_164_800), "29 Feb");
    }
}
