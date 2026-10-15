//! Juno in the player: records the browser extension read from the Juno page the user has
//! open, sent with the bridge (the app never reads Juno itself, see `dig::juno`).
//!
//! A send's tracks are merged into their crate as a Bandcamp album is: a track the crate plays
//! already is skipped, one with no preview ("no clip", "clip failed", "not found") gets the
//! Juno sample in place, and only what the crate lacks is added. A send can hold 500 records,
//! so the crate is indexed once by artist and title rather than scanned per track.
//!
//! Each record is then looked up on Discogs once (with a token), by artist, title and
//! catalogue number, and linked only when exactly one release fits: its entries get the
//! Discogs verbs (Y, I, ★, owned). What the page said about stock is kept per product and
//! shown on its entries, and on any Discogs entry of the linked release.

use dig::bandcamp::fan::DiscogsMatch;
use dig::discogs::client::ApiError;
use dig::discogs::search::Wanted;
use dig::intake::Command;
use dig::juno::{self, JunoMemory, JunoRecord};
use dig::preview::fetcher::preview_path;
use platform::TrackRef;
use std::collections::{HashMap, HashSet};

use super::DiggrApp;
use super::bandcamp::{Step, catno_key, match_key};
use super::digging::SendMode;
use crate::crates::CrateId;
use crate::playlist::{Entry, EntryId, EntryStatus, NewEntry, Origin, UnavailableKind, WaitKind};

/// Discogs lookups for Juno products are keyed `juno:‹product›`.
const FIND_PREFIX: &str = "juno:";

/// Juno's part of the dig state.
#[derive(Default)]
pub(super) struct JunoState {
    pub(super) memory: JunoMemory,
    /// Products being looked up on Discogs this session.
    finding: HashSet<String>,
    /// Where `juno.ron` snapshots go to be written (see `Dig::save_juno`).
    pub(super) saver: Option<std::sync::mpsc::Sender<JunoMemory>>,
}

impl JunoState {
    pub(super) fn new(memory: JunoMemory) -> Self {
        Self {
            memory,
            finding: HashSet::new(),
            saver: None,
        }
    }
}

/// An entry with no preview that a Juno sample can fix.
fn fixable(e: &Entry) -> bool {
    matches!(
        e.status,
        EntryStatus::Unavailable(
            UnavailableKind::NoClip | UnavailableKind::ClipFailed | UnavailableKind::NotFound
        )
    ) && e.origin.as_ref().is_some_and(|o| o.juno_clip.is_empty())
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

/// "1.2": side and place, or the place alone.
fn position(t: &juno::JunoTrack) -> String {
    match t.side.is_empty() {
        true => t.n.to_string(),
        false => format!("{}.{}", t.side, t.n),
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

impl DiggrApp {
    /// Records from a Juno page: merged where `mode` says, remembered, and looked up on
    /// Discogs once each.
    pub(super) fn juno_send(
        &mut self,
        records: Vec<JunoRecord>,
        mode: SendMode,
        skip_passed: bool,
    ) {
        if self.dig.is_none() || records.is_empty() {
            return;
        }
        let name = dig::bridge::juno_name(&records);
        let Some(target) = self.send_target(&mode, &name) else {
            return;
        };
        let (added, fixed, skipped) = self.juno_merge(target, &records, skip_passed);
        let Some(d) = &mut self.dig else { return };
        d.juno.memory.saw(&records, dig::now_secs());
        d.save_juno();
        self.juno_link_known(&records);
        self.juno_find(&records);
        self.notify(format!(
            "{name}: {added} added, {fixed} fixed, {skipped} skipped"
        ));
    }

    /// Merges the records' tracks into crate `target`; returns (added, fixed, skipped).
    fn juno_merge(
        &mut self,
        target: CrateId,
        records: &[JunoRecord],
        skip_passed: bool,
    ) -> (usize, usize, usize) {
        let Some(d) = &self.dig else {
            return (0, 0, 0);
        };
        let dir = d.previews_dir();
        let passed: HashSet<String> = records
            .iter()
            .flat_map(|r| r.tracks.iter().map(|t| juno::key(&r.product, &t.sample)))
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
            let page = juno::product_url(&r.slug, &r.product).unwrap_or_default();
            let names: Vec<(String, String)> = r
                .tracks
                .iter()
                .map(|t| {
                    let artist = match t.artist.is_empty() {
                        true => r.artist.clone(),
                        false => t.artist.clone(),
                    };
                    let title = match t.title.is_empty() {
                        true => r.title.clone(),
                        false => t.title.clone(),
                    };
                    (artist, title)
                })
                .collect();
            let keys: Vec<String> = names.iter().map(|(a, t)| match_key(a, t)).collect();
            let steps = ix.plan(&catno, &keys);
            let mut last_of_catno = ix.last.get(&catno).copied();
            for (((t, (artist, title)), key), step) in
                r.tracks.iter().zip(names).zip(keys).zip(steps)
            {
                let clip = juno::key(&r.product, &t.sample);
                match step {
                    Step::Skip => skipped += 1,
                    Step::Fix(id) => {
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
                        o.juno = r.product.clone();
                        o.juno_slug = r.slug.clone();
                        o.juno_clip = clip.clone();
                        o.clip = Some(clip.clone());
                        e.source = Some(page.clone());
                        if let Some(old) = old {
                            moved_passes.push((old, clip.clone()));
                        }
                        p.set_waiting(id, WaitKind::Queued);
                        ready.push((id, clip));
                        fixed += 1;
                    }
                    Step::Add => {
                        if passed.contains(&clip) {
                            skipped += 1;
                            continue;
                        }
                        let origin = Origin {
                            page: page.clone(),
                            label: r.label.clone(),
                            catno: r.catno.clone(),
                            position: position(t),
                            clip: Some(clip.clone()),
                            album: r.title.clone(),
                            cover: juno::cover_url(&r.image).unwrap_or_default(),
                            artist: r.artist.clone(),
                            formats: r.format.clone(),
                            juno: r.product.clone(),
                            juno_slug: r.slug.clone(),
                            juno_clip: clip.clone(),
                            ..Origin::default()
                        };
                        let new = NewEntry {
                            artist,
                            title,
                            source: Some(page.clone()),
                            origin: Some(origin),
                            duration: (t.secs > 0).then_some(f64::from(t.secs)),
                            status: WaitKind::Queued,
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
                        if !catno.is_empty() {
                            last_of_catno = Some(id);
                        }
                        ix.add(id, catno.clone(), key, false);
                        ready.push((id, clip));
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

    /// Products linked before: their entries take the release at once.
    fn juno_link_known(&mut self, records: &[JunoRecord]) {
        let Some(d) = &self.dig else { return };
        let links: Vec<(String, u64)> = records
            .iter()
            .filter_map(|r| {
                let release = d.juno.memory.products.get(&r.product)?.release?;
                Some((r.product.clone(), release))
            })
            .collect();
        for (product, release) in links {
            self.juno_link(&product, release);
        }
    }

    /// Products never looked up are looked up on Discogs (with a token), behind other work.
    fn juno_find(&mut self, records: &[JunoRecord]) {
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none() {
            return;
        }
        let mut finds = Vec::new();
        for r in records {
            let searched = d
                .juno
                .memory
                .products
                .get(&r.product)
                .is_some_and(|s| s.searched);
            if searched || !d.juno.finding.insert(r.product.clone()) {
                continue;
            }
            let w = Wanted::new(&r.artist, &r.title, &r.catno, true);
            finds.push((format!("{FIND_PREFIX}{}", r.product), w));
        }
        if !finds.is_empty() {
            d.send_cmd(Command::FindReleases(finds));
        }
    }

    /// Whether a lookup's key is a Juno product's.
    pub(super) fn is_juno_find(key: &str) -> bool {
        key.starts_with(FIND_PREFIX)
    }

    /// Discogs answered for a product: remembered; exactly one release links its entries.
    pub(super) fn juno_found(&mut self, key: &str, result: Result<DiscogsMatch, ApiError>) {
        let Some(product) = key.strip_prefix(FIND_PREFIX) else {
            return;
        };
        let Some(d) = &mut self.dig else { return };
        d.juno.finding.remove(product);
        // Offline or refused: looked up again at the next send.
        let Ok(m) = result else { return };
        let seen = d
            .juno
            .memory
            .products
            .entry(product.to_owned())
            .or_default();
        seen.searched = true;
        if let DiscogsMatch::Found(release) = m {
            seen.release = Some(release);
        }
        d.save_juno();
        if let DiscogsMatch::Found(release) = m {
            self.juno_link(product, release);
        }
    }

    /// Every loaded entry of `product` with no Discogs record takes `release`.
    fn juno_link(&mut self, product: &str, release: u64) {
        for c in self.crates.loaded_ids() {
            let Some(p) = self.crates.get_mut(c) else {
                continue;
            };
            let mut changed = false;
            for e in p.entries_mut() {
                if let Some(o) = e.origin.as_mut()
                    && o.juno == product
                    && o.release.is_none()
                    && o.master.is_none()
                {
                    o.release = Some(release);
                    changed = true;
                }
            }
            if changed {
                self.mark_crate(c);
            }
        }
    }

    // ---- the entry ----------------------------------------------------------------------

    /// What Juno last said of an entry's record: its own product, else the product linked
    /// to its release.
    pub(super) fn juno_note(&self, e: &Entry) -> Option<String> {
        let d = self.dig.as_ref()?;
        let o = e.origin.as_ref()?;
        let mem = &d.juno.memory;
        let seen = match o.juno.is_empty() {
            false => mem.products.get(&o.juno)?,
            true => mem.product_of(o.release?)?.1,
        };
        seen.note(dig::now_secs(), day_month)
    }

    /// The Juno product page of an entry's record, if it has one.
    pub(super) fn juno_page(&self, e: &Entry) -> Option<String> {
        let o = e.origin.as_ref()?;
        if !o.juno.is_empty() {
            return juno::product_url(&o.juno_slug, &o.juno);
        }
        let d = self.dig.as_ref()?;
        let (product, seen) = d.juno.memory.product_of(o.release?)?;
        juno::product_url(&seen.slug, product)
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
