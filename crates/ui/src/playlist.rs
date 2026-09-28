//! The playlist model: entries, selection, reordering, totals, and play order.
//!
//! Entries have stable ids so the current track and pending metadata survive reordering.
//! An entry can exist before its audio does (`Waiting`), or be kept for the record without
//! ever having audio (`Unavailable`); neither ever reaches the engine, so the engine's
//! "track failed" keeps meaning "this file is broken".

use std::collections::BTreeSet;

use platform::TrackRef;
use serde::{Deserialize, Serialize};

pub type EntryId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryStatus {
    /// Metadata not read yet.
    Pending,
    Ready,
    /// Could not be opened or decoded.
    Failed,
    /// The audio isn't local yet; the text says why ("listed", "downloading 40%").
    Waiting(String),
    /// Will never play ("no clip"); kept for the record.
    Unavailable(String),
}

impl EntryStatus {
    /// Has a local file the engine can be given.
    pub fn is_playable(&self) -> bool {
        matches!(self, EntryStatus::Pending | EntryStatus::Ready)
    }

    /// Playable now or once its audio arrives: it has a place in the play order.
    pub fn in_play_order(&self) -> bool {
        self.is_playable() || matches!(self, EntryStatus::Waiting(_))
    }

    /// The short text shown where a waiting or unavailable entry's duration would be.
    pub fn note(&self) -> Option<&str> {
        match self {
            EntryStatus::Waiting(t) | EntryStatus::Unavailable(t) => Some(t),
            _ => None,
        }
    }
}

/// The record an entry belongs to, for entries sent from a catalogue page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Origin {
    /// The page it was sent from.
    pub page: String,
    pub release: Option<u64>,
    pub master: Option<u64>,
    pub label: String,
    pub catno: String,
    pub year: Option<u16>,
    /// Side, e.g. "A1".
    pub position: String,
    /// The clip the audio comes from, e.g. a video id.
    pub clip: Option<String>,
    /// Copies for sale when the record was last looked up.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_sale: Option<ForSale>,
}

/// A marketplace snapshot: how many copies are for sale, and the cheapest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ForSale {
    pub count: u32,
    /// The lowest price in hundredths of `currency` (yen too, so every currency is alike).
    pub lowest_cents: Option<u64>,
    /// ISO code, e.g. "EUR".
    pub currency: String,
    /// Seconds since the Unix epoch.
    pub fetched_at: u64,
}

/// An entry to add in place of another (see [`Playlist::replace`]).
#[derive(Debug, Clone, PartialEq)]
pub struct NewEntry {
    pub artist: String,
    pub title: String,
    pub source: Option<String>,
    pub origin: Option<Origin>,
    /// A duration known before the file is read.
    pub duration: Option<f64>,
    /// The waiting note ("queued").
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: EntryId,
    /// The local file; empty while waiting.
    pub track: TrackRef,
    /// Where the audio comes from (a URL), for remote entries.
    pub source: Option<String>,
    pub origin: Option<Origin>,
    pub title: String,
    pub artist: String,
    pub duration: Option<f64>,
    /// Tempo from analysis, folded into the DJ range (see [`crate::format::dj_bpm`]).
    pub bpm: Option<u16>,
    pub status: EntryStatus,
}

impl Entry {
    /// "(catno) Artist: Title (N BPM)", leaving out what isn't known.
    pub fn display_name(&self) -> String {
        let catno = self.origin.as_ref().map_or("", |o| o.catno.as_str());
        crate::format::entry_name(catno, &self.artist, &self.title, self.bpm)
    }

    /// "Artist - Title", or just the title: the plain name other players expect (M3U).
    pub fn plain_name(&self) -> String {
        if self.artist.is_empty() {
            self.title.clone()
        } else {
            format!("{} - {}", self.artist, self.title)
        }
    }

    /// What Send to crate compares: the origin's clip when there is one, otherwise the file.
    pub fn duplicate_key(&self) -> DuplicateKey {
        match self.origin.as_ref().and_then(|o| o.clip.clone()) {
            Some(clip) => DuplicateKey::Clip(clip),
            None => DuplicateKey::File(self.track.0.clone()),
        }
    }

    fn to_saved(&self) -> SavedEntry {
        SavedEntry {
            path: self.track.0.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            duration: self.duration,
            bpm: self.bpm,
            source: self.source.clone(),
            origin: self.origin.clone(),
            status: match &self.status {
                EntryStatus::Waiting(t) => SavedStatus::Waiting(t.clone()),
                EntryStatus::Unavailable(t) => SavedStatus::Unavailable(t.clone()),
                _ => SavedStatus::Local,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DuplicateKey {
    Clip(String),
    File(String),
}

/// Saved form of the playlist (metadata cached so a restart shows titles immediately).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedPlaylist {
    pub entries: Vec<SavedEntry>,
    pub current: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedEntry {
    pub path: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bpm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
    #[serde(default, skip_serializing_if = "SavedStatus::is_local")]
    pub status: SavedStatus,
}

/// Whether a saved entry has local audio; Pending/Ready/Failed are worked out again at load.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedStatus {
    #[default]
    Local,
    Waiting(String),
    Unavailable(String),
}

impl SavedStatus {
    fn is_local(&self) -> bool {
        *self == SavedStatus::Local
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClickMods {
    pub shift: bool,
    /// Cmd on macOS, Ctrl elsewhere.
    pub command: bool,
}

#[derive(Debug, Default)]
pub struct Playlist {
    entries: Vec<Entry>,
    selected: BTreeSet<EntryId>,
    anchor: Option<EntryId>,
    current: Option<EntryId>,
    next_id: EntryId,
}

impl Playlist {
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn index_of(&self, id: EntryId) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }

    pub fn get(&self, id: EntryId) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Appends tracks with file-name titles; returns the new entries for metadata lookup.
    pub fn add(&mut self, tracks: impl IntoIterator<Item = TrackRef>) -> Vec<(EntryId, TrackRef)> {
        let mut added = Vec::new();
        for track in tracks {
            let id = self.alloc_id();
            added.push((id, track.clone()));
            self.entries.push(Entry {
                id,
                title: track.stem().to_owned(),
                artist: String::new(),
                duration: None,
                bpm: None,
                status: EntryStatus::Pending,
                source: None,
                origin: None,
                track,
            });
        }
        added
    }

    /// Appends saved entries (Send to crate) with fresh ids; returns the ones whose metadata
    /// still has to be read.
    pub fn add_saved(
        &mut self,
        saved: impl IntoIterator<Item = SavedEntry>,
    ) -> Vec<(EntryId, TrackRef)> {
        let mut pending = Vec::new();
        for s in saved {
            let id = self.alloc_id();
            let entry = Entry::from_saved(id, s);
            if entry.status == EntryStatus::Pending
                || (entry.status.is_playable() && entry.duration.is_none())
            {
                pending.push((id, entry.track.clone()));
            }
            self.entries.push(entry);
        }
        pending
    }

    fn alloc_id(&mut self) -> EntryId {
        self.next_id += 1;
        self.next_id
    }

    fn entry_mut(&mut self, id: EntryId) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    /// Metadata read from the file. An entry with an origin keeps its own artist and title
    /// (the record is the truth, not the file's tags) and takes only the duration.
    pub fn set_info(&mut self, id: EntryId, title: String, artist: String, duration: Option<f64>) {
        if let Some(e) = self.entry_mut(id)
            && e.status.note().is_none()
        {
            if e.origin.is_none() {
                e.title = title;
                e.artist = artist;
            }
            e.duration = duration;
            e.status = EntryStatus::Ready;
        }
    }

    /// Sets the tempo of every entry with this audio (a clip, or a local file); returns whether
    /// any entry changed.
    pub fn set_bpm(&mut self, key: &DuplicateKey, bpm: u16) -> bool {
        let mut changed = false;
        for e in &mut self.entries {
            if e.bpm != Some(bpm) && e.duplicate_key() == *key {
                e.bpm = Some(bpm);
                changed = true;
            }
        }
        changed
    }

    pub fn set_failed(&mut self, id: EntryId) {
        if let Some(e) = self.entry_mut(id)
            && e.status.note().is_none()
        {
            e.status = EntryStatus::Failed;
        }
    }

    // ---- entries waiting for their audio (the producer API) ----------------------------

    /// Appends an entry whose audio isn't local yet, with `status` as its short note.
    pub fn add_waiting(
        &mut self,
        artist: impl Into<String>,
        title: impl Into<String>,
        source: Option<String>,
        origin: Option<Origin>,
        status: impl Into<String>,
    ) -> EntryId {
        let id = self.alloc_id();
        self.entries.push(Entry {
            id,
            track: TrackRef::new(""),
            source,
            origin,
            title: title.into(),
            artist: artist.into(),
            duration: None,
            bpm: None,
            status: EntryStatus::Waiting(status.into()),
        });
        id
    }

    /// Updates a waiting entry's note ("downloading 40%").
    pub fn set_status(&mut self, id: EntryId, text: impl Into<String>) {
        if let Some(e) = self.entry_mut(id)
            && let EntryStatus::Waiting(t) = &mut e.status
        {
            *t = text.into();
        }
    }

    /// The entry's audio arrived at `track`: it becomes Pending, so its duration is read and
    /// it joins the engine queue. Returns false if there is no such entry.
    pub fn set_audio(&mut self, id: EntryId, track: TrackRef) -> bool {
        let Some(e) = self.entry_mut(id) else {
            return false;
        };
        e.track = track;
        e.status = EntryStatus::Pending;
        true
    }

    /// Replaces an entry, at its place, by new waiting entries (a listed record by its clips);
    /// with none it is removed. The current entry, the selection and the anchor move to the
    /// first new entry. Returns the new ids.
    pub fn replace(&mut self, id: EntryId, new: Vec<NewEntry>) -> Vec<EntryId> {
        let Some(at) = self.index_of(id) else {
            return Vec::new();
        };
        let ids: Vec<EntryId> = new.iter().map(|_| self.alloc_id()).collect();
        let entries = new.into_iter().zip(&ids).map(|(n, &nid)| Entry {
            id: nid,
            track: TrackRef::new(""),
            source: n.source,
            origin: n.origin,
            title: n.title,
            artist: n.artist,
            duration: n.duration,
            bpm: None,
            status: EntryStatus::Waiting(n.status),
        });
        self.entries.splice(at..=at, entries);
        let first = ids.first().copied();
        if self.current == Some(id) {
            self.current = first;
        }
        if self.selected.remove(&id)
            && let Some(f) = first
        {
            self.selected.insert(f);
        }
        if self.anchor == Some(id) {
            self.anchor = first;
        }
        ids
    }

    /// Removes one entry (the current one may be it).
    pub fn remove(&mut self, id: EntryId) -> bool {
        let found = self.index_of(id).is_some();
        self.replace(id, Vec::new());
        found
    }

    /// An entry whose audio went away (its preview was deleted from the cache) waits again.
    pub fn set_waiting(&mut self, id: EntryId, text: impl Into<String>) {
        if let Some(e) = self.entry_mut(id) {
            e.track = TrackRef::new("");
            e.status = EntryStatus::Waiting(text.into());
        }
    }

    /// Every entry, for updates that touch many (marketplace numbers of a release).
    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut Entry> {
        self.entries.iter_mut()
    }

    /// The entry will never have audio; it stays listed, dimmed, with `reason`.
    pub fn set_unavailable(&mut self, id: EntryId, reason: impl Into<String>) {
        if let Some(e) = self.entry_mut(id) {
            e.status = EntryStatus::Unavailable(reason.into());
        }
    }

    // ---- current track -----------------------------------------------------------------

    pub fn current(&self) -> Option<EntryId> {
        self.current
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current.and_then(|id| self.index_of(id))
    }

    pub fn set_current(&mut self, id: Option<EntryId>) {
        self.current = id;
    }

    // ---- selection ---------------------------------------------------------------------

    pub fn is_selected(&self, id: EntryId) -> bool {
        self.selected.contains(&id)
    }

    pub fn selected_ids(&self) -> Vec<EntryId> {
        self.entries
            .iter()
            .filter(|e| self.selected.contains(&e.id))
            .map(|e| e.id)
            .collect()
    }

    /// Plain click selects one; Shift extends from the anchor; Cmd/Ctrl toggles.
    pub fn click(&mut self, index: usize, mods: ClickMods) {
        let Some(id) = self.entries.get(index).map(|e| e.id) else {
            return;
        };
        if mods.shift {
            let anchor = self.anchor.and_then(|a| self.index_of(a)).unwrap_or(index);
            let (a, b) = (anchor.min(index), anchor.max(index));
            if !mods.command {
                self.selected.clear();
            }
            self.selected
                .extend(self.entries[a..=b].iter().map(|e| e.id));
            return;
        }
        if mods.command {
            if !self.selected.remove(&id) {
                self.selected.insert(id);
            }
        } else {
            self.selected.clear();
            self.selected.insert(id);
        }
        self.anchor = Some(id);
    }

    pub fn select_all(&mut self) {
        self.selected = self.entries.iter().map(|e| e.id).collect();
    }

    pub fn select_none(&mut self) {
        self.selected.clear();
    }

    pub fn invert_selection(&mut self) {
        self.selected = self
            .entries
            .iter()
            .map(|e| e.id)
            .filter(|id| !self.selected.contains(id))
            .collect();
    }

    // ---- editing -----------------------------------------------------------------------

    /// Removes the selected entries. The current track may be among them.
    pub fn remove_selected(&mut self) -> usize {
        let before = self.entries.len();
        let sel = std::mem::take(&mut self.selected);
        self.entries.retain(|e| !sel.contains(&e.id));
        if self.current.is_some_and(|c| sel.contains(&c)) {
            self.current = None;
        }
        before - self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.selected.clear();
        self.anchor = None;
        self.current = None;
    }

    /// Moves entry `from` so it ends up at index `to` (drag-to-reorder).
    pub fn move_entry(&mut self, from: usize, to: usize) {
        if from >= self.entries.len() || from == to {
            return;
        }
        let e = self.entries.remove(from);
        let to = to.min(self.entries.len());
        self.entries.insert(to, e);
    }

    // ---- totals ------------------------------------------------------------------------

    /// Sum of known durations and whether any duration is still unknown.
    pub fn total_duration(&self) -> (f64, bool) {
        sum(self.entries.iter())
    }

    pub fn selected_duration(&self) -> (f64, bool) {
        sum(self
            .entries
            .iter()
            .filter(|e| self.selected.contains(&e.id)))
    }

    // ---- persistence -------------------------------------------------------------------

    pub fn to_saved(&self) -> SavedPlaylist {
        SavedPlaylist {
            entries: self.entries.iter().map(Entry::to_saved).collect(),
            current: self.current_index(),
        }
    }

    /// The saved form of some entries, in playlist order (for Send to crate).
    pub fn saved_entries(&self, ids: &[EntryId]) -> Vec<SavedEntry> {
        self.entries
            .iter()
            .filter(|e| ids.contains(&e.id))
            .map(Entry::to_saved)
            .collect()
    }

    /// Restores a saved playlist; entries without cached metadata are returned for lookup.
    pub fn from_saved(saved: SavedPlaylist) -> (Self, Vec<(EntryId, TrackRef)>) {
        let mut pl = Playlist::default();
        let pending = pl.add_saved(saved.entries);
        pl.current = saved.current.and_then(|i| pl.entries.get(i)).map(|e| e.id);
        (pl, pending)
    }

    // ---- play order ----------------------------------------------------------------------

    /// Every entry that can eventually play (playable or waiting), in playlist order or
    /// shuffled starting with `first`. A waiting entry keeps its place when its audio arrives,
    /// because the set of entries shuffled doesn't change. Audio producers read what comes
    /// next from this order.
    pub fn play_order(&self, shuffle: bool, first: Option<EntryId>, seed: u64) -> Vec<EntryId> {
        let ids: Vec<EntryId> = self
            .entries
            .iter()
            .filter(|e| e.status.in_play_order())
            .map(|e| e.id)
            .collect();
        let first = first.and_then(|f| ids.iter().position(|&id| id == f));
        play_order(ids.len(), shuffle, first, seed)
            .into_iter()
            .map(|i| ids[i])
            .collect()
    }

    /// The engine queue: the play order without the entries still waiting for their audio.
    pub fn queue(
        &self,
        shuffle: bool,
        first: Option<EntryId>,
        seed: u64,
    ) -> Vec<(EntryId, TrackRef)> {
        self.play_order(shuffle, first, seed)
            .into_iter()
            .filter_map(|id| self.get(id))
            .filter(|e| e.status.is_playable())
            .map(|e| (e.id, e.track.clone()))
            .collect()
    }
}

impl Entry {
    fn from_saved(id: EntryId, s: SavedEntry) -> Self {
        let track = TrackRef::new(s.path);
        let known = !s.title.is_empty();
        let status = match s.status {
            SavedStatus::Waiting(t) => EntryStatus::Waiting(t),
            SavedStatus::Unavailable(t) => EntryStatus::Unavailable(t),
            SavedStatus::Local if known => EntryStatus::Ready,
            SavedStatus::Local => EntryStatus::Pending,
        };
        Entry {
            id,
            title: if known {
                s.title
            } else {
                track.stem().to_owned()
            },
            artist: s.artist,
            duration: s.duration,
            bpm: s.bpm,
            status,
            source: s.source,
            origin: s.origin,
            track,
        }
    }
}

fn sum<'a>(entries: impl Iterator<Item = &'a Entry>) -> (f64, bool) {
    entries.fold((0.0, false), |(total, unknown), e| match e.duration {
        Some(d) => (total + d, unknown),
        None => (total, true),
    })
}

/// Queue order for the engine: playlist order, or a shuffle that starts with `first`.
pub fn play_order(len: usize, shuffle: bool, first: Option<usize>, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..len).collect();
    if !shuffle || len < 2 {
        return order;
    }
    // Fisher–Yates with a small xorshift generator (no dependency needed for a shuffle).
    let mut state = seed | 1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in (1..len).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    if let Some(f) = first.filter(|&f| f < len) {
        let at = order.iter().position(|&i| i == f).expect("permutation");
        order.swap(0, at);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pl(n: usize) -> Playlist {
        let mut p = Playlist::default();
        p.add((0..n).map(|i| TrackRef::new(format!("/m/{i}.mp3"))));
        p
    }

    fn sel(p: &Playlist) -> Vec<usize> {
        p.selected_ids()
            .iter()
            .map(|id| p.index_of(*id).unwrap())
            .collect()
    }

    #[test]
    fn add_uses_file_names_until_metadata_arrives() {
        let mut p = Playlist::default();
        let added = p.add([TrackRef::new("/music/M83 - Midnight City.mp3")]);
        assert_eq!(p.entries()[0].title, "M83 - Midnight City");
        assert_eq!(p.entries()[0].status, EntryStatus::Pending);
        p.set_info(
            added[0].0,
            "Midnight City".into(),
            "M83".into(),
            Some(243.0),
        );
        assert_eq!(p.entries()[0].display_name(), "M83: Midnight City");
        assert_eq!(p.entries()[0].status, EntryStatus::Ready);
    }

    #[test]
    fn selection_click_shift_and_command() {
        let mut p = pl(10);
        p.click(2, ClickMods::default());
        assert_eq!(sel(&p), [2]);
        p.click(
            5,
            ClickMods {
                shift: true,
                command: false,
            },
        );
        assert_eq!(sel(&p), [2, 3, 4, 5]);
        p.click(
            8,
            ClickMods {
                shift: false,
                command: true,
            },
        );
        assert_eq!(sel(&p), [2, 3, 4, 5, 8]);
        p.click(
            3,
            ClickMods {
                shift: false,
                command: true,
            },
        );
        assert_eq!(sel(&p), [2, 4, 5, 8]);
        p.click(0, ClickMods::default());
        assert_eq!(sel(&p), [0]);
        p.invert_selection();
        assert_eq!(sel(&p).len(), 9);
        p.select_none();
        assert!(sel(&p).is_empty());
    }

    #[test]
    fn reorder_moves_entry_and_keeps_identity() {
        let mut p = pl(10);
        let seventh = p.entries()[6].id; // "entry 7"
        p.set_current(Some(seventh));
        p.move_entry(6, 1); // dragged above entry 2
        assert_eq!(p.entries()[1].id, seventh);
        assert_eq!(p.current_index(), Some(1), "current follows the entry");
        let order: Vec<_> = p
            .entries()
            .iter()
            .map(|e| e.track.stem().to_owned())
            .collect();
        assert_eq!(order, ["0", "6", "1", "2", "3", "4", "5", "7", "8", "9"]);
    }

    #[test]
    fn remove_selected_and_clear() {
        let mut p = pl(5);
        p.set_current(Some(p.entries()[1].id));
        p.click(1, ClickMods::default());
        p.click(
            3,
            ClickMods {
                shift: true,
                command: false,
            },
        );
        assert_eq!(p.remove_selected(), 3);
        assert_eq!(p.len(), 2);
        assert_eq!(p.current(), None, "removed current track is forgotten");
        p.clear();
        assert!(p.is_empty());
    }

    #[test]
    fn durations_sum_known_and_flag_unknown() {
        let mut p = pl(3);
        let ids: Vec<_> = p.entries().iter().map(|e| e.id).collect();
        p.set_info(ids[0], "a".into(), "".into(), Some(60.0));
        p.set_info(ids[1], "b".into(), "".into(), Some(30.5));
        assert_eq!(p.total_duration(), (90.5, true));
        p.set_info(ids[2], "c".into(), "".into(), Some(9.5));
        assert_eq!(p.total_duration(), (100.0, false));
        p.click(1, ClickMods::default());
        assert_eq!(p.selected_duration(), (30.5, false));
    }

    #[test]
    fn a_tempo_reaches_every_entry_with_that_audio_and_is_saved() {
        let mut p = pl(2);
        let clip = |c: &str| Origin {
            catno: "LT-012".into(),
            clip: Some(c.into()),
            ..Default::default()
        };
        let a = p.add_waiting(
            "Nightcraft",
            "Glasshouse",
            None,
            Some(clip("aaaaaaaaaaa")),
            "listed",
        );
        let b = p.add_waiting(
            "Nightcraft",
            "Glasshouse",
            None,
            Some(clip("aaaaaaaaaaa")),
            "listed",
        );
        let other = p.add_waiting(
            "Nightcraft",
            "Undertow",
            None,
            Some(clip("bbbbbbbbbbb")),
            "listed",
        );
        assert!(p.set_bpm(&DuplicateKey::Clip("aaaaaaaaaaa".into()), 124));
        assert!(
            !p.set_bpm(&DuplicateKey::Clip("aaaaaaaaaaa".into()), 124),
            "no change"
        );
        assert_eq!(p.get(a).unwrap().bpm, Some(124));
        assert_eq!(p.get(b).unwrap().bpm, Some(124));
        assert_eq!(p.get(other).unwrap().bpm, None);
        let file = p.entries()[0].track.clone();
        assert!(p.set_bpm(&DuplicateKey::File(file.0.clone()), 128));
        assert_eq!(p.entries()[0].bpm, Some(128));
        assert_eq!(p.entries()[1].bpm, None);

        let text = ron::to_string(&p.to_saved()).unwrap();
        let (restored, _) = Playlist::from_saved(ron::from_str(&text).unwrap());
        assert_eq!(
            restored.get(a).map(Entry::display_name).as_deref(),
            Some("(LT-012) Nightcraft: Glasshouse (124 BPM)")
        );
        assert_eq!(restored.entries()[0].bpm, Some(128));
        assert!(
            !ron::to_string(&restored.entries()[1].to_saved())
                .unwrap()
                .contains("bpm"),
            "an unknown tempo isn't written"
        );
    }

    #[test]
    fn save_and_restore() {
        let mut p = pl(3);
        let id = p.entries()[0].id;
        p.set_info(id, "Echo".into(), "Crusher-P".into(), Some(230.0));
        p.set_current(Some(p.entries()[2].id));
        let saved = p.to_saved();
        let text = ron::to_string(&saved).unwrap();
        let (restored, pending) = Playlist::from_saved(ron::from_str(&text).unwrap());
        assert_eq!(restored.len(), 3);
        assert_eq!(restored.entries()[0].display_name(), "Crusher-P: Echo");
        assert_eq!(restored.current_index(), Some(2));
        assert_eq!(
            pending.len(),
            2,
            "entries without cached metadata are re-read"
        );
    }

    #[test]
    fn play_order_is_a_permutation_starting_with_first() {
        assert_eq!(play_order(4, false, Some(2), 1), [0, 1, 2, 3]);
        let order = play_order(50, true, Some(17), 12345);
        assert_eq!(order[0], 17);
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
        assert_ne!(order, (0..50).collect::<Vec<_>>(), "actually shuffled");
        assert_eq!(
            play_order(50, true, Some(17), 12345),
            order,
            "deterministic per seed"
        );
    }

    fn origin(release: u64, clip: &str) -> Origin {
        Origin {
            page: "https://www.discogs.com/label/1".into(),
            release: Some(release),
            label: "Lowtide Tapes".into(),
            catno: "LT-012".into(),
            year: Some(1994),
            position: "A1".into(),
            clip: Some(clip.into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_playlist_file_in_the_current_format_still_loads() {
        // Written by the previous version: no source, origin or status fields.
        let text = r#"(
    entries: [
        (
            path: "/m/a.mp3",
            title: "Echo",
            artist: "Crusher-P",
            duration: Some(230.0),
        ),
        (
            path: "/m/b.flac",
        ),
    ],
    current: Some(1),
)"#;
        let saved: SavedPlaylist = ron::from_str(text).unwrap();
        let (p, pending) = Playlist::from_saved(saved.clone());
        assert_eq!(p.entries()[0].display_name(), "Crusher-P: Echo");
        assert_eq!(p.entries()[0].status, EntryStatus::Ready);
        assert_eq!(p.entries()[1].status, EntryStatus::Pending);
        assert!(
            p.entries()
                .iter()
                .all(|e| e.source.is_none() && e.origin.is_none())
        );
        assert_eq!(p.current_index(), Some(1));
        assert_eq!(pending.len(), 1);
        // And it saves back the same way (no new fields for local entries).
        assert_eq!(p.to_saved().entries[0], saved.entries[0]);
        assert!(!ron::to_string(&p.to_saved()).unwrap().contains("status"));
    }

    #[test]
    fn waiting_and_unavailable_entries_survive_a_restart() {
        let mut p = pl(1);
        let w = p.add_waiting(
            "Nightcraft",
            "Glasshouse",
            Some("https://www.youtube.com/watch?v=abcdefghijk".into()),
            Some(origin(123456, "abcdefghijk")),
            "listed",
        );
        p.set_status(w, "downloading 40%");
        let u = p.add_waiting(
            "Nightcraft",
            "Untitled",
            None,
            Some(origin(123456, "")),
            "listed",
        );
        p.set_unavailable(u, "no clip");
        let text = ron::to_string(&p.to_saved()).unwrap();
        let (r, pending) = Playlist::from_saved(ron::from_str(&text).unwrap());
        assert_eq!(pending.len(), 1, "only the local file is looked up");
        let e = &r.entries()[1];
        assert_eq!(e.status, EntryStatus::Waiting("downloading 40%".into()));
        assert_eq!(e.origin.as_ref().unwrap().catno, "LT-012");
        assert_eq!(
            e.source.as_deref(),
            Some("https://www.youtube.com/watch?v=abcdefghijk")
        );
        assert_eq!(
            r.entries()[2].status,
            EntryStatus::Unavailable("no clip".into())
        );
    }

    #[test]
    fn a_listed_record_is_replaced_in_place_by_its_clips() {
        let mut p = pl(2);
        let listed = p.add_waiting("Nightcraft", "Glasshouse EP", None, None, "listed");
        p.add([TrackRef::new("/m/after.mp3")]);
        p.set_current(Some(listed));
        p.click(2, ClickMods::default());
        let clip = |t: &str| NewEntry {
            artist: "Nightcraft".into(),
            title: t.into(),
            source: Some(format!("https://www.youtube.com/watch?v={t:x<11}")),
            origin: None,
            duration: Some(300.0),
            status: "queued".into(),
        };
        let ids = p.replace(listed, vec![clip("a"), clip("b")]);
        let titles: Vec<String> = p.entries().iter().map(|e| e.title.clone()).collect();
        assert_eq!(titles, ["0", "1", "a", "b", "after"]);
        assert_eq!(
            p.current(),
            Some(ids[0]),
            "the current entry moves to the first clip"
        );
        assert_eq!(p.selected_ids(), [ids[0]]);
        assert_eq!(
            p.get(ids[1]).unwrap().status,
            EntryStatus::Waiting("queued".into())
        );
        assert_eq!(p.get(ids[1]).unwrap().duration, Some(300.0));
        // A record with nothing to add leaves the crate.
        let gone = p.add_waiting("", "CD only", None, None, "listed");
        assert!(p.replace(gone, Vec::new()).is_empty());
        assert!(p.get(gone).is_none());
        assert!(p.remove(ids[1]));
        assert_eq!(p.len(), 4);
        // Audio that went away: waiting again.
        p.set_audio(ids[0], TrackRef::new("/cache/previews/a.m4a"));
        p.set_waiting(ids[0], "queued");
        assert_eq!(
            p.get(ids[0]).unwrap().status,
            EntryStatus::Waiting("queued".into())
        );
        assert!(p.get(ids[0]).unwrap().track.0.is_empty());
    }

    #[test]
    fn a_for_sale_snapshot_is_optional_in_saved_crates() {
        let mut o = origin(1, "x");
        let text = ron::to_string(&o).unwrap();
        assert!(!text.contains("for_sale"), "left out when unknown");
        o.for_sale = Some(ForSale {
            count: 6,
            lowest_cents: Some(900),
            currency: "EUR".into(),
            fetched_at: 1_790_000_000,
        });
        let back: Origin = ron::from_str(&ron::to_string(&o).unwrap()).unwrap();
        assert_eq!(back, o);
    }

    #[test]
    fn tags_never_overwrite_an_origin() {
        let mut p = Playlist::default();
        let id = p.add_waiting(
            "Nightcraft",
            "Glasshouse",
            None,
            Some(origin(1, "x")),
            "listed",
        );
        // Metadata for a waiting entry is ignored: it has no file yet.
        p.set_info(id, "t".into(), "a".into(), Some(1.0));
        assert_eq!(
            p.get(id).unwrap().status,
            EntryStatus::Waiting("listed".into())
        );
        assert!(p.set_audio(id, TrackRef::new("/cache/abcdefghijk.m4a")));
        assert_eq!(p.get(id).unwrap().status, EntryStatus::Pending);
        p.set_info(
            id,
            "glasshouse (vinyl rip)".into(),
            "Unknown".into(),
            Some(301.0),
        );
        let e = p.get(id).unwrap();
        assert_eq!(e.display_name(), "(LT-012) Nightcraft: Glasshouse");
        assert_eq!((e.duration, &e.status), (Some(301.0), &EntryStatus::Ready));
        // Entries without an origin still take the tags.
        let plain = p.add([TrackRef::new("/m/x.mp3")])[0].0;
        p.set_info(plain, "Title".into(), "Artist".into(), None);
        assert_eq!(p.get(plain).unwrap().display_name(), "Artist: Title");
    }

    /// Entries 0..6, with 3 waiting and 4 unavailable.
    fn mixed() -> (Playlist, Vec<EntryId>) {
        let mut p = pl(3);
        p.add_waiting("", "three", None, None, "listed");
        let u = p.add_waiting("", "four", None, None, "listed");
        p.set_unavailable(u, "no clip");
        p.add((5..7).map(|i| TrackRef::new(format!("/m/{i}.mp3"))));
        let ids = p.entries().iter().map(|e| e.id).collect();
        (p, ids)
    }

    #[test]
    fn the_engine_queue_skips_waiting_and_unavailable_entries() {
        let (p, ids) = mixed();
        assert_eq!(
            p.play_order(false, None, 1),
            [ids[0], ids[1], ids[2], ids[3], ids[5], ids[6]],
            "unavailable entries have no place in the order"
        );
        let queue: Vec<EntryId> = p.queue(false, None, 1).iter().map(|(id, _)| *id).collect();
        assert_eq!(queue, [ids[0], ids[1], ids[2], ids[5], ids[6]]);
        // Next/previous/repeat run over the engine queue: 2 is followed by 5, and 5 is preceded
        // by 2. Shuffle too: whatever the seed, the queue never holds 3 or 4.
        for seed in 1..50 {
            let q: Vec<EntryId> = p
                .queue(true, Some(ids[5]), seed)
                .iter()
                .map(|(id, _)| *id)
                .collect();
            assert_eq!(q[0], ids[5]);
            assert_eq!(q.len(), 5);
            assert!(!q.contains(&ids[3]) && !q.contains(&ids[4]));
        }
        // A waiting entry can't be the first of a queue.
        assert_eq!(p.queue(true, Some(ids[3]), 7).len(), 5);
    }

    #[test]
    fn an_entry_joins_the_queue_in_place_when_its_audio_arrives() {
        let (mut p, ids) = mixed();
        let order = p.play_order(true, Some(ids[0]), 99);
        let before: Vec<EntryId> = p
            .queue(true, Some(ids[0]), 99)
            .iter()
            .map(|(id, _)| *id)
            .collect();
        assert!(!before.contains(&ids[3]));
        p.set_audio(ids[3], TrackRef::new("/cache/three.m4a"));
        assert_eq!(
            p.play_order(true, Some(ids[0]), 99),
            order,
            "its place in the shuffled order doesn't change"
        );
        let after: Vec<EntryId> = p
            .queue(true, Some(ids[0]), 99)
            .iter()
            .map(|(id, _)| *id)
            .collect();
        let expected: Vec<EntryId> = order.iter().copied().filter(|&id| id != ids[4]).collect();
        assert_eq!(after, expected, "it is queued where the order puts it");
    }
}
