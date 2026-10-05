//! The playlist model: entries, selection, reordering, totals, and play order.
//!
//! Entries have stable ids so the current track and pending metadata survive reordering.
//! An entry can exist before its audio does (`Waiting`), or be kept for the record without
//! ever having audio (`Unavailable`); neither ever reaches the engine, so the engine's
//! "track failed" keeps meaning "this file is broken".

use std::collections::{BTreeSet, HashMap, HashSet};

use platform::TrackRef;

use crate::columns::{Dir, Field};
use serde::{Deserialize, Serialize};

pub type EntryId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryStatus {
    /// Metadata not read yet.
    Pending,
    Ready,
    /// Could not be opened or decoded.
    Failed,
    /// The audio isn't local yet, and why.
    Waiting(WaitKind),
    /// Will never play; kept for the record.
    Unavailable(UnavailableKind),
}

/// Why an entry is waiting for its audio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitKind {
    /// Its record is listed; its details haven't arrived yet.
    Listed,
    /// Its preview will download when it nears the playhead.
    Queued,
    /// Its preview is downloading (percent).
    Downloading(u8),
    /// Previews can't download until yt-dlp is found.
    NeedsYtDlp,
    Other(String),
}

/// Why an entry will never play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnavailableKind {
    /// Its record has no usable clip.
    NoClip,
    /// Its clip failed to download twice.
    ClipFailed,
    Other(String),
}

impl std::fmt::Display for WaitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitKind::Listed => f.write_str("listed"),
            WaitKind::Queued => f.write_str("queued"),
            WaitKind::Downloading(p) => write!(f, "downloading {p}%"),
            WaitKind::NeedsYtDlp => f.write_str("needs yt-dlp"),
            WaitKind::Other(t) => f.write_str(t),
        }
    }
}

impl std::fmt::Display for UnavailableKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnavailableKind::NoClip => f.write_str("no clip"),
            UnavailableKind::ClipFailed => f.write_str("clip failed"),
            UnavailableKind::Other(t) => f.write_str(t),
        }
    }
}

/// The wording back to its kind: saved crates store the wording, and the dig crate reports
/// reasons as text. Anything unknown is kept as `Other`.
impl From<&str> for WaitKind {
    fn from(t: &str) -> Self {
        match t {
            "listed" => WaitKind::Listed,
            "queued" => WaitKind::Queued,
            "needs yt-dlp" => WaitKind::NeedsYtDlp,
            _ => t
                .strip_prefix("downloading ")
                .and_then(|p| p.strip_suffix('%'))
                .and_then(|p| p.parse().ok())
                .map_or_else(|| WaitKind::Other(t.to_owned()), WaitKind::Downloading),
        }
    }
}

impl From<String> for WaitKind {
    fn from(t: String) -> Self {
        t.as_str().into()
    }
}

impl From<&str> for UnavailableKind {
    fn from(t: &str) -> Self {
        match t {
            "no clip" => UnavailableKind::NoClip,
            "clip failed" => UnavailableKind::ClipFailed,
            _ => UnavailableKind::Other(t.to_owned()),
        }
    }
}

impl From<String> for UnavailableKind {
    fn from(t: String) -> Self {
        t.as_str().into()
    }
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

    /// What a waiting or unavailable entry is waiting for, or why it won't play, in words
    /// ("downloading 40%", "no clip").
    pub fn note(&self) -> Option<String> {
        match self {
            EntryStatus::Waiting(w) => Some(w.to_string()),
            EntryStatus::Unavailable(u) => Some(u.to_string()),
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
    /// The record's title: the album the entry belongs to.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub album: String,
    /// The address of the record's cover thumbnail.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cover: String,
    /// The record's Discogs styles, comma-separated ("Deep House, Minimal").
    #[serde(skip_serializing_if = "String::is_empty")]
    pub styles: String,
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
    /// Why it waits (usually queued).
    pub status: WaitKind,
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
    /// A local file's album tag (an entry with an origin uses its record's title instead).
    pub album: String,
    pub duration: Option<f64>,
    /// Tempo from analysis, folded into the DJ range (see [`crate::format::dj_bpm`]).
    pub bpm: Option<u16>,
    pub status: EntryStatus,
}

impl Entry {
    /// "(catno) Artist: Title (N BPM)", leaving out what isn't known (messages and the title
    /// line use it; rows add the album, see [`Self::row_name`]).
    pub fn display_name(&self) -> String {
        let catno = self.origin.as_ref().map_or("", |o| o.catno.as_str());
        crate::format::entry_name(catno, &self.artist, &self.title, "", self.bpm)
    }

    /// The playlist row's name: [`Self::display_name`] with the album after the title.
    pub fn row_name(&self) -> String {
        let catno = self.origin.as_ref().map_or("", |o| o.catno.as_str());
        crate::format::entry_name(catno, &self.artist, &self.title, self.album(), self.bpm)
    }

    /// The album it belongs to: its record's title, or a local file's album tag.
    pub fn album(&self) -> &str {
        match &self.origin {
            Some(o) => &o.album,
            None => &self.album,
        }
    }

    /// "Artist - Title", or just the title: the plain name other players expect (M3U).
    pub fn plain_name(&self) -> String {
        if self.artist.is_empty() {
            self.title.clone()
        } else {
            format!("{} - {}", self.artist, self.title)
        }
    }

    /// The album it belongs to, if any (see [`AlbumKey`]).
    pub fn album_key(&self) -> Option<AlbumKey> {
        match &self.origin {
            Some(o) => match (o.release, o.master) {
                (Some(r), _) => Some(AlbumKey::Release(r)),
                (None, Some(m)) => Some(AlbumKey::Master(m)),
                _ => None,
            },
            None if self.album.trim().is_empty() => None,
            None => Some(AlbumKey::Local(
                self.artist.trim().to_lowercase(),
                self.album.trim().to_lowercase(),
            )),
        }
    }

    /// The record's Discogs styles ("Deep House", "Minimal"), none for a local file.
    pub fn styles(&self) -> impl Iterator<Item = &str> {
        self.origin
            .as_ref()
            .map_or("", |o| o.styles.as_str())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
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
            album: self.album.clone(),
            duration: self.duration,
            bpm: self.bpm,
            source: self.source.clone(),
            origin: self.origin.clone(),
            status: match &self.status {
                EntryStatus::Waiting(w) => SavedStatus::Waiting(w.to_string()),
                EntryStatus::Unavailable(u) => SavedStatus::Unavailable(u.to_string()),
                _ => SavedStatus::Local,
            },
        }
    }
}

/// Which album an entry belongs to: entries with equal keys form one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlbumKey {
    /// A Discogs release: another pressing of the same master is another album.
    Release(u64),
    /// A master release, for an entry that has no release.
    Master(u64),
    /// A local file's artist and album tags, lower-cased, so two "Greatest Hits" stay apart.
    Local(String, String),
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
    /// The BPM filter's range, when narrower than the crate's tempos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bpm_range: Option<(u16, u16)>,
    /// The styles the style filter shows, when any is selected.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub styles: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedEntry {
    pub path: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub album: String,
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
    /// The keyboard cursor: an entry, so it stays put while entries arrive around it.
    cursor: Option<EntryId>,
    /// The field the entries were last sorted by, until they are reordered otherwise.
    sorted: Option<(Field, Dir)>,
    /// The BPM filter, as set (see [`Playlist::bpm_filter`] for what applies).
    bpm_range: Option<(u16, u16)>,
    /// The style filter, as set (see [`Playlist::style_filter`] for what applies).
    styles_on: BTreeSet<String>,
    next_id: EntryId,
    /// Shown grouped by record: each album's entries are kept together (see
    /// [`Playlist::gather`]).
    grouped: bool,
    /// Entries were added (or an album became known) while grouped: the next
    /// [`Playlist::settle`] places them with their record.
    unsettled: bool,
    /// Bumped whenever the entries, their order, their albums or the filter change, so views
    /// built from them know when to rebuild.
    rev: u64,
}

/// What the filters show: an entry whose tempo is in the BPM range (when one is set) and
/// that has one of the selected styles (when any is). An entry with no tempo, or no style,
/// shows only while that filter is off.
#[derive(Debug, Clone, Copy)]
pub struct Shown<'a> {
    bpm: Option<(u16, u16)>,
    styles: Option<&'a BTreeSet<String>>,
}

impl Shown<'_> {
    pub fn shows(&self, e: &Entry) -> bool {
        self.bpm
            .is_none_or(|(lo, hi)| e.bpm.is_some_and(|b| (lo..=hi).contains(&b)))
            && self
                .styles
                .is_none_or(|on| e.styles().any(|s| on.contains(s)))
    }

    pub fn is_filtered(&self) -> bool {
        self.bpm.is_some() || self.styles.is_some()
    }
}

/// How far the keyboard cursor moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMove {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
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

    /// Changes whenever the entries, their order, their albums or the BPM filter change.
    pub fn rev(&self) -> u64 {
        self.rev
    }

    fn changed(&mut self) {
        self.rev += 1;
    }

    // ---- grouped by record ---------------------------------------------------------------

    pub fn is_grouped(&self) -> bool {
        self.grouped
    }

    /// Groups (or ungroups) the entries by record. Grouping gathers them at once; ungrouping
    /// keeps the order. Returns whether the order changed.
    pub fn set_grouped(&mut self, on: bool) -> bool {
        self.grouped = on;
        self.unsettled = false;
        self.changed();
        let moved = on && self.gather();
        if moved {
            self.sorted = None; // like a sort by record
        }
        moved
    }

    /// While grouped, places entries added since the last call with their record. Returns
    /// whether the order changed (the play order follows it).
    pub fn settle(&mut self) -> bool {
        if !std::mem::take(&mut self.unsettled) || !self.grouped {
            return false;
        }
        let moved = self.gather();
        if moved {
            self.sorted = None;
        }
        moved
    }

    /// Moves each album's entries up to follow that album's first entry, keeping their order;
    /// entries of no album keep their place among the rest. A stable reorder, like a sort: the
    /// crate's order (play, save, export) follows it. Returns whether anything moved.
    pub fn gather(&mut self) -> bool {
        let mut slot: HashMap<AlbumKey, usize> = HashMap::new();
        let mut groups: Vec<Vec<usize>> = Vec::with_capacity(self.entries.len());
        for (i, e) in self.entries.iter().enumerate() {
            match e.album_key() {
                Some(k) => match slot.get(&k) {
                    Some(&g) => groups[g].push(i),
                    None => {
                        slot.insert(k, groups.len());
                        groups.push(vec![i]);
                    }
                },
                None => groups.push(vec![i]),
            }
        }
        if groups.len() == self.entries.len() {
            return false; // every album already in one piece, or none at all
        }
        let order: Vec<usize> = groups.into_iter().flatten().collect();
        if order.iter().enumerate().all(|(i, &j)| i == j) {
            return false;
        }
        let mut old: Vec<Option<Entry>> = std::mem::take(&mut self.entries)
            .into_iter()
            .map(Some)
            .collect();
        self.entries = order
            .into_iter()
            .map(|i| old[i].take().expect("each index once"))
            .collect();
        self.changed();
        true
    }

    /// Moves the entries `ids` (a record), in their order, to just before entry `before` (or
    /// to the end). Nothing happens when `before` is one of them.
    pub fn move_block(&mut self, ids: &[EntryId], before: Option<EntryId>) {
        if before.is_some_and(|b| ids.contains(&b)) || ids.is_empty() {
            return;
        }
        let (block, rest): (Vec<Entry>, Vec<Entry>) = std::mem::take(&mut self.entries)
            .into_iter()
            .partition(|e| ids.contains(&e.id));
        self.entries = rest;
        let at = before
            .and_then(|b| self.index_of(b))
            .unwrap_or(self.entries.len());
        self.entries.splice(at..at, block);
        self.sorted = None;
        self.changed();
    }

    /// Adds or removes `ids` from the selection as one: when all are selected they leave it,
    /// otherwise they all join it. The cursor goes to the first.
    pub fn toggle_ids(&mut self, ids: &[EntryId]) {
        if ids.iter().all(|id| self.selected.contains(id)) {
            for id in ids {
                self.selected.remove(id);
            }
        } else {
            self.selected.extend(ids.iter().copied());
        }
        self.cursor = ids.first().copied();
        self.anchor = self.cursor;
    }

    /// Selects every shown entry from the anchor to index `to`, and `ids` with them (Shift
    /// on a record: up to its end). The cursor goes to `cursor`.
    pub fn extend_to(&mut self, to: usize, ids: &[EntryId], cursor: EntryId) {
        let anchor = self.anchor.and_then(|a| self.index_of(a)).unwrap_or(to);
        let (a, b) = (anchor.min(to), anchor.max(to));
        self.selected = self.shown_ids_between(a, b);
        self.selected.extend(ids.iter().copied());
        self.cursor = Some(cursor);
        if self.anchor.is_none() {
            self.anchor = Some(cursor);
        }
    }

    pub fn index_of(&self, id: EntryId) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }

    pub fn get(&self, id: EntryId) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Every entry of `id`'s album, `id` included, in playlist order; empty when it belongs to
    /// none.
    pub fn album_of(&self, id: EntryId) -> Vec<EntryId> {
        let Some(key) = self.get(id).and_then(Entry::album_key) else {
            return Vec::new();
        };
        self.entries
            .iter()
            .filter(|e| e.album_key().as_ref() == Some(&key))
            .map(|e| e.id)
            .collect()
    }

    /// Appends tracks with file-name titles; returns the new entries for metadata lookup.
    pub fn add(&mut self, tracks: impl IntoIterator<Item = TrackRef>) -> Vec<(EntryId, TrackRef)> {
        let from = self.entries.len();
        let mut added = Vec::new();
        for track in tracks {
            let id = self.alloc_id();
            added.push((id, track.clone()));
            self.entries.push(Entry {
                id,
                title: track.stem().to_owned(),
                artist: String::new(),
                album: String::new(),
                duration: None,
                bpm: None,
                status: EntryStatus::Pending,
                source: None,
                origin: None,
                track,
            });
        }
        self.check_sorted(from);
        self.added();
        added
    }

    /// Appends saved entries (Send to crate) with fresh ids; returns the ones whose metadata
    /// still has to be read.
    pub fn add_saved(
        &mut self,
        saved: impl IntoIterator<Item = SavedEntry>,
    ) -> Vec<(EntryId, TrackRef)> {
        let from = self.entries.len();
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
        self.check_sorted(from);
        self.added();
        pending
    }

    /// Entries arrived: a grouped list places them at the next settle.
    fn added(&mut self) {
        self.unsettled |= self.grouped;
        self.changed();
    }

    fn alloc_id(&mut self) -> EntryId {
        self.next_id += 1;
        self.next_id
    }

    fn entry_mut(&mut self, id: EntryId) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    /// Metadata read from the file. An entry with an origin keeps its own artist, title and
    /// album (the record is the truth, not the file's tags) and takes only the duration.
    pub fn set_info(&mut self, id: EntryId, info: audio::TrackInfo) {
        if let Some(e) = self.entry_mut(id)
            && e.status.note().is_none()
        {
            if e.origin.is_none() {
                let album = e.album != info.album || e.artist != info.artist;
                e.title = info.title;
                e.artist = info.artist;
                e.album = info.album;
                if album {
                    self.added();
                }
            }
            if let Some(e) = self.entry_mut(id) {
                e.duration = info.duration_secs;
                e.status = EntryStatus::Ready;
            }
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
        if changed {
            self.changed();
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
        status: impl Into<WaitKind>,
    ) -> EntryId {
        let id = self.alloc_id();
        self.entries.push(Entry {
            id,
            track: TrackRef::new(""),
            source,
            origin,
            title: title.into(),
            artist: artist.into(),
            album: String::new(),
            duration: None,
            bpm: None,
            status: EntryStatus::Waiting(status.into()),
        });
        self.check_sorted(self.entries.len() - 1);
        self.added();
        id
    }

    /// Updates why a waiting entry waits (downloading 40%).
    pub fn set_status(&mut self, id: EntryId, kind: impl Into<WaitKind>) {
        if let Some(e) = self.entry_mut(id)
            && let EntryStatus::Waiting(w) = &mut e.status
        {
            *w = kind.into();
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
            album: String::new(),
            duration: n.duration,
            bpm: None,
            status: EntryStatus::Waiting(n.status),
        });
        self.entries.splice(at..=at, entries);
        self.changed();
        let first = ids.first().copied();
        if self.current == Some(id) {
            self.current = first;
        }
        if self.cursor == Some(id) {
            // Onto the entry now in its place.
            self.cursor = first.or_else(|| self.entry_near(at));
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
    pub fn set_waiting(&mut self, id: EntryId, kind: impl Into<WaitKind>) {
        if let Some(e) = self.entry_mut(id) {
            e.track = TrackRef::new("");
            e.status = EntryStatus::Waiting(kind.into());
        }
    }

    /// Every entry, for updates that touch many (marketplace numbers of a release).
    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut Entry> {
        self.changed();
        self.entries.iter_mut()
    }

    /// The entry will never have audio; it stays listed, dimmed, with `reason`.
    pub fn set_unavailable(&mut self, id: EntryId, reason: impl Into<UnavailableKind>) {
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

    /// Plain click selects one; Shift extends from the anchor; Cmd/Ctrl toggles. The keyboard
    /// cursor goes to the clicked entry.
    pub fn click(&mut self, index: usize, mods: ClickMods) {
        let Some(id) = self.entries.get(index).map(|e| e.id) else {
            return;
        };
        self.cursor = Some(id);
        if mods.shift {
            let anchor = self.anchor.and_then(|a| self.index_of(a)).unwrap_or(index);
            let (a, b) = (anchor.min(index), anchor.max(index));
            if !mods.command {
                self.selected.clear();
            }
            let ids = self.shown_ids_between(a, b);
            self.selected.extend(ids);
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

    /// Selects every entry the BPM filter shows.
    pub fn select_all(&mut self) {
        self.selected = self.shown_ids_between(0, self.entries.len().saturating_sub(1));
    }

    /// The shown entries from index `a` to `b` (inclusive).
    fn shown_ids_between(&self, a: usize, b: usize) -> BTreeSet<EntryId> {
        let shown = self.shown();
        self.entries
            .get(a..=b.min(self.entries.len().saturating_sub(1)))
            .unwrap_or_default()
            .iter()
            .filter(|e| shown.shows(e))
            .map(|e| e.id)
            .collect()
    }

    pub fn select_none(&mut self) {
        self.selected.clear();
    }

    /// Selects exactly `ids` (an album), with the cursor and the anchor on `cursor`.
    pub fn select_only(&mut self, ids: &[EntryId], cursor: EntryId) {
        self.selected = ids.iter().copied().collect();
        self.cursor = Some(cursor).filter(|&c| self.index_of(c).is_some());
        self.anchor = self.cursor;
    }

    /// Inverts the selection among the shown entries (hidden ones end up unselected).
    pub fn invert_selection(&mut self) {
        let shown = self.shown();
        self.selected = self
            .entries
            .iter()
            .filter(|e| shown.shows(e) && !self.selected.contains(&e.id))
            .map(|e| e.id)
            .collect();
    }

    // ---- keyboard cursor ---------------------------------------------------------------

    pub fn cursor(&self) -> Option<EntryId> {
        self.cursor
    }

    pub fn cursor_index(&self) -> Option<usize> {
        self.cursor.and_then(|c| self.index_of(c))
    }

    /// Puts the cursor on an entry without touching the selection.
    pub fn set_cursor(&mut self, id: Option<EntryId>) {
        self.cursor = id.filter(|&i| self.index_of(i).is_some());
    }

    /// Moves the cursor (`page` entries for a page) and selects what it passes: only the new
    /// entry, or with `extend` everything from the anchor to it. Without a cursor, ↑ or ↓ lands
    /// on `start` (the playing entry), or on the first entry; the other moves go from there.
    pub fn move_cursor(
        &mut self,
        mv: CursorMove,
        extend: bool,
        page: usize,
        start: Option<EntryId>,
    ) {
        // The cursor moves over the rows the BPM filter shows.
        let rows = self.shown_rows();
        if rows.is_empty() {
            return;
        }
        let last = rows.len() - 1;
        let page = page.max(1);
        // A row: the entry's own, or the nearest shown one after it (it may be hidden).
        let row_of = |id: EntryId| {
            self.index_of(id)
                .map(|i| rows.partition_point(|&r| r < i).min(last))
        };
        let to = match (self.cursor.and_then(row_of), mv) {
            (None, CursorMove::Up | CursorMove::Down) => start.and_then(row_of).unwrap_or(0),
            (i, _) => {
                let i = i.or_else(|| start.and_then(row_of)).unwrap_or(0);
                match mv {
                    CursorMove::Up => i.saturating_sub(1),
                    CursorMove::Down => (i + 1).min(last),
                    CursorMove::PageUp => i.saturating_sub(page),
                    CursorMove::PageDown => (i + page).min(last),
                    CursorMove::Home => 0,
                    CursorMove::End => last,
                }
            }
        };
        let to = rows[to];
        let id = self.entries[to].id;
        if extend {
            let from = self.anchor.or(self.cursor).unwrap_or(id);
            self.anchor = Some(from);
            let a = self.index_of(from).unwrap_or(to);
            let (a, b) = (a.min(to), a.max(to));
            self.selected = self.shown_ids_between(a, b);
        } else {
            self.selected = [id].into();
            self.anchor = Some(id);
        }
        self.cursor = Some(id);
    }

    /// The entry at `index`, or the last one when the list is shorter.
    fn entry_near(&self, index: usize) -> Option<EntryId> {
        self.entries
            .get(index)
            .or_else(|| self.entries.last())
            .map(|e| e.id)
    }

    // ---- editing -----------------------------------------------------------------------

    /// Removes the selected entries. The current track may be among them.
    pub fn remove_selected(&mut self) -> usize {
        let sel = std::mem::take(&mut self.selected);
        self.remove_set(&sel)
    }

    /// Removes these entries (an album); the rest of the selection stays. The current track
    /// may be among them.
    pub fn remove_ids(&mut self, ids: &[EntryId]) -> usize {
        let set: BTreeSet<EntryId> = ids.iter().copied().collect();
        self.selected.retain(|id| !set.contains(id));
        self.remove_set(&set)
    }

    fn remove_set(&mut self, sel: &BTreeSet<EntryId>) -> usize {
        let before = self.entries.len();
        if let Some(at) = self
            .cursor
            .filter(|c| sel.contains(c))
            .and_then(|c| self.index_of(c))
        {
            // Onto the first entry left after it, else the last one left before it.
            let kept = |e: &&Entry| !sel.contains(&e.id);
            self.cursor = self.entries[at..]
                .iter()
                .find(kept)
                .or_else(|| self.entries[..at].iter().rev().find(kept))
                .map(|e| e.id);
        }
        self.entries.retain(|e| !sel.contains(&e.id));
        self.changed();
        if self.current.is_some_and(|c| sel.contains(&c)) {
            self.current = None;
        }
        before - self.entries.len()
    }

    pub fn clear(&mut self) {
        self.changed();
        self.entries.clear();
        self.selected.clear();
        self.anchor = None;
        self.current = None;
        self.cursor = None;
    }

    /// Moves entry `from` so it ends up at index `to` (drag-to-reorder).
    pub fn move_entry(&mut self, from: usize, to: usize) {
        if from >= self.entries.len() || from == to {
            return;
        }
        let e = self.entries.remove(from);
        let to = to.min(self.entries.len());
        self.entries.insert(to, e);
        self.sorted = None;
        self.changed();
    }

    // ---- sorting -----------------------------------------------------------------------

    /// Reorders the entries by `field` (stable; entries without a value last). The new order
    /// is the crate's order: play order, saving and export follow it.
    pub fn sort_by(&mut self, field: Field, dir: Dir) {
        self.entries
            .sort_by(|a, b| crate::columns::compare(a, b, field, dir));
        // Grouped, each record follows its best-placed track; the mark stays.
        if self.grouped {
            self.gather();
        }
        self.sorted = Some((field, dir));
        self.changed();
    }

    /// What the entries were last sorted by, while that order holds.
    pub fn sorted(&self) -> Option<(Field, Dir)> {
        self.sorted
    }

    /// Entries appended from `from` on keep the sort mark only if they land in order.
    fn check_sorted(&mut self, from: usize) {
        let Some((field, dir)) = self.sorted else {
            return;
        };
        let start = from.max(1);
        if (start..self.entries.len()).any(|i| {
            crate::columns::compare(&self.entries[i - 1], &self.entries[i], field, dir)
                == std::cmp::Ordering::Greater
        }) {
            self.sorted = None;
        }
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
            bpm_range: self.bpm_filter(),
            styles: self.style_filter().cloned().unwrap_or_default(),
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
        pl.bpm_range = saved.bpm_range;
        pl.styles_on = saved.styles;
        (pl, pending)
    }

    // ---- BPM filter ----------------------------------------------------------------------

    /// The lowest and highest known tempo, when there are at least two different ones (a
    /// range means nothing otherwise).
    pub fn tempo_span(&self) -> Option<(u16, u16)> {
        let mut known = self.entries.iter().filter_map(|e| e.bpm);
        let first = known.next()?;
        let (lo, hi) = known.fold((first, first), |(lo, hi), b| (lo.min(b), hi.max(b)));
        (lo < hi).then_some((lo, hi))
    }

    /// The range that applies now: the one set, within the crate's tempos; `None` (no
    /// filter) when it covers them all, or misses them all (the tempos moved away).
    pub fn bpm_filter(&self) -> Option<(u16, u16)> {
        let (lo, hi) = self.bpm_range?;
        let (span_lo, span_hi) = self.tempo_span()?;
        let (lo, hi) = (lo.max(span_lo), hi.min(span_hi));
        (lo <= hi && (lo, hi) != (span_lo, span_hi)).then_some((lo, hi))
    }

    /// Sets the range (`None`, or the whole span, turns the filter off). Returns whether what
    /// applies changed.
    pub fn set_bpm_filter(&mut self, range: Option<(u16, u16)>) -> bool {
        let before = self.bpm_filter();
        self.bpm_range = range.map(|(a, b)| (a.min(b), a.max(b)));
        self.bpm_range = self.bpm_filter();
        self.changed();
        before != self.bpm_range
    }

    /// Whether the filters show `e` (see [`Shown`]). For many entries, take [`Self::shown`]
    /// once instead.
    pub fn shows(&self, e: &Entry) -> bool {
        self.shown().shows(e)
    }

    /// What the BPM and style filters show, worked out once for a pass over the entries.
    pub fn shown(&self) -> Shown<'_> {
        Shown {
            bpm: self.bpm_filter(),
            styles: self.style_filter(),
        }
    }

    /// Whether a filter (BPM or style) hides anything.
    pub fn is_filtered(&self) -> bool {
        self.shown().is_filtered()
    }

    /// Turns both filters off. Returns whether that shows more.
    pub fn clear_filters(&mut self) -> bool {
        let bpm = self.set_bpm_filter(None);
        self.clear_styles() || bpm
    }

    /// The crate indices of the entries the filters show, in order: row `r` of the list is
    /// entry `shown_rows()[r]`, still numbered by its crate position.
    pub fn shown_rows(&self) -> Vec<usize> {
        let shown = self.shown();
        if !shown.is_filtered() {
            return (0..self.entries.len()).collect();
        }
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| shown.shows(e))
            .map(|(i, _)| i)
            .collect()
    }

    // ---- style filter --------------------------------------------------------------------

    /// The crate's styles, each with the number of records (albums, and entries of no
    /// album) that have it: the most first, then by name.
    pub fn styles(&self) -> Vec<(String, usize)> {
        let mut seen: HashMap<&str, HashSet<AlbumKey>> = HashMap::new();
        let mut loose: HashMap<&str, usize> = HashMap::new();
        for e in &self.entries {
            let key = e.album_key();
            for s in e.styles() {
                match &key {
                    Some(k) => {
                        seen.entry(s).or_default().insert(k.clone());
                    }
                    None => *loose.entry(s).or_default() += 1,
                }
            }
        }
        let mut out: Vec<(String, usize)> = seen
            .keys()
            .chain(loose.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|&s| {
                let n = seen.get(s).map_or(0, HashSet::len) + loose.get(s).copied().unwrap_or(0);
                (s.to_owned(), n)
            })
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// The styles that filter now: the ones selected, when some entry still has one of them;
    /// `None` (no filter) otherwise.
    pub fn style_filter(&self) -> Option<&BTreeSet<String>> {
        let on = &self.styles_on;
        let any = !on.is_empty()
            && self
                .entries
                .iter()
                .any(|e| e.styles().any(|s| on.contains(s)));
        any.then_some(on)
    }

    /// Whether `style` is selected.
    pub fn style_on(&self, style: &str) -> bool {
        self.styles_on.contains(style)
    }

    /// Selects or unselects one style.
    pub fn set_style(&mut self, style: &str, on: bool) {
        let changed = if on {
            self.styles_on.insert(style.to_owned())
        } else {
            self.styles_on.remove(style)
        };
        if changed {
            self.changed();
        }
    }

    /// Unselects every style. Returns whether a style filtered.
    pub fn clear_styles(&mut self) -> bool {
        let was = self.style_filter().is_some();
        if !self.styles_on.is_empty() {
            self.styles_on.clear();
            self.changed();
        }
        was
    }

    /// Entries without a known tempo (hidden while a range is set).
    pub fn without_bpm(&self) -> usize {
        self.entries.iter().filter(|e| e.bpm.is_none()).count()
    }

    // ---- play order ----------------------------------------------------------------------

    /// Every entry that can eventually play (playable or waiting) and that the BPM filter
    /// shows, in playlist order or shuffled starting with `first`. The current entry stays in
    /// it even when the filter hides it, so it plays on and the next shown entry follows it.
    /// A waiting entry keeps its place when its audio arrives, because the set of entries
    /// shuffled doesn't change. Audio producers read what comes next from this order.
    pub fn play_order(&self, shuffle: bool, first: Option<EntryId>, seed: u64) -> Vec<EntryId> {
        let shown = self.shown();
        let ids: Vec<EntryId> = self
            .entries
            .iter()
            .filter(|e| e.status.in_play_order())
            .filter(|e| Some(e.id) == self.current || shown.shows(e))
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

/// File tags as the metadata worker reports them, for tests.
#[cfg(test)]
pub(crate) fn tags(title: String, artist: String, duration: Option<f64>) -> audio::TrackInfo {
    audio::TrackInfo {
        title,
        artist,
        duration_secs: duration,
        ..Default::default()
    }
}

impl Entry {
    fn from_saved(id: EntryId, s: SavedEntry) -> Self {
        let track = TrackRef::new(s.path);
        let known = !s.title.is_empty();
        let status = match s.status {
            SavedStatus::Waiting(t) => EntryStatus::Waiting(t.into()),
            SavedStatus::Unavailable(t) => EntryStatus::Unavailable(t.into()),
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
            album: s.album,
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
            tags("Midnight City".into(), "M83".into(), Some(243.0)),
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
        p.set_info(ids[0], tags("a".into(), "".into(), Some(60.0)));
        p.set_info(ids[1], tags("b".into(), "".into(), Some(30.5)));
        assert_eq!(p.total_duration(), (90.5, true));
        p.set_info(ids[2], tags("c".into(), "".into(), Some(9.5)));
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

    fn cursor_at(p: &Playlist) -> Option<usize> {
        p.cursor_index()
    }

    #[test]
    fn the_cursor_walks_pages_and_ends_and_selects_as_it_goes() {
        let mut p = pl(40);
        let playing = p.entries()[22].id;
        p.move_cursor(CursorMove::Down, false, 10, Some(playing));
        assert_eq!(
            cursor_at(&p),
            Some(22),
            "the first press lands on the playing entry"
        );
        assert_eq!(sel(&p), [22]);
        p.move_cursor(CursorMove::Down, false, 10, None);
        p.move_cursor(CursorMove::Down, false, 10, None);
        assert_eq!((cursor_at(&p), sel(&p)), (Some(24), vec![24]));
        p.move_cursor(CursorMove::PageUp, false, 10, None);
        assert_eq!(cursor_at(&p), Some(14));
        p.move_cursor(CursorMove::End, false, 10, None);
        assert_eq!(cursor_at(&p), Some(39));
        p.move_cursor(CursorMove::Down, false, 10, None);
        assert_eq!(cursor_at(&p), Some(39), "stops at the end");
        p.move_cursor(CursorMove::PageDown, false, 10, None);
        assert_eq!(cursor_at(&p), Some(39));
        p.move_cursor(CursorMove::Home, false, 10, None);
        p.move_cursor(CursorMove::Up, false, 10, None);
        assert_eq!(cursor_at(&p), Some(0), "stops at the start");

        let mut fresh = pl(3);
        fresh.move_cursor(CursorMove::Up, false, 10, None);
        assert_eq!(
            cursor_at(&fresh),
            Some(0),
            "nothing playing: the first entry"
        );
    }

    #[test]
    fn shift_extends_the_selection_from_the_anchor() {
        let mut p = pl(10);
        p.click(4, ClickMods::default());
        assert_eq!(cursor_at(&p), Some(4), "a click moves the cursor");
        for _ in 0..3 {
            p.move_cursor(CursorMove::Down, true, 10, None);
        }
        assert_eq!((cursor_at(&p), sel(&p)), (Some(7), vec![4, 5, 6, 7]));
        p.move_cursor(CursorMove::Up, true, 10, None);
        assert_eq!(sel(&p), [4, 5, 6]);
        p.move_cursor(CursorMove::Down, false, 10, None);
        assert_eq!(sel(&p), [7], "a plain move selects one again");
    }

    #[test]
    fn the_cursor_stays_on_its_entry_while_entries_change_around_it() {
        let mut p = pl(3);
        let listed = p.add_waiting("Nightcraft", "Glasshouse EP", None, None, "listed");
        let after = p.add_waiting("Nightcraft", "Undertow", None, None, "listed");
        p.move_cursor(CursorMove::End, false, 10, None);
        assert_eq!(p.cursor(), Some(after));
        // A listed record above it becomes three clips.
        let clip = |t: &str| NewEntry {
            artist: "Nightcraft".into(),
            title: t.into(),
            source: None,
            origin: None,
            duration: None,
            status: "queued".into(),
        };
        p.replace(listed, vec![clip("A1"), clip("A2"), clip("B1")]);
        assert_eq!((p.cursor(), cursor_at(&p)), (Some(after), Some(6)));
        // Reordering carries it along.
        p.move_entry(6, 0);
        assert_eq!((p.cursor(), cursor_at(&p)), (Some(after), Some(0)));
        // A replaced cursor entry hands over to its first replacement; a removed one to the
        // entry now in its place.
        let a1 = p.entries()[4].id;
        p.set_cursor(Some(a1));
        let a2 = p.entries()[5].id;
        p.remove(a1);
        assert_eq!(p.cursor(), Some(a2));
        p.click(4, ClickMods::default());
        let next = p.entries()[5].id;
        p.remove_selected();
        assert_eq!(p.cursor(), Some(next));
        p.clear();
        assert_eq!(p.cursor(), None);
    }

    #[test]
    fn statuses_read_back_from_their_wording() {
        for w in [
            WaitKind::Listed,
            WaitKind::Queued,
            WaitKind::Downloading(40),
            WaitKind::NeedsYtDlp,
            WaitKind::Other("paused by Discogs".into()),
        ] {
            assert_eq!(WaitKind::from(w.to_string()), w);
        }
        for u in [
            UnavailableKind::NoClip,
            UnavailableKind::ClipFailed,
            UnavailableKind::Other("not found".into()),
        ] {
            assert_eq!(UnavailableKind::from(u.to_string()), u);
        }
        assert_eq!(
            WaitKind::from("downloading lots%"),
            WaitKind::Other("downloading lots%".into())
        );
        assert_eq!(
            EntryStatus::Waiting(WaitKind::Downloading(7))
                .note()
                .as_deref(),
            Some("downloading 7%")
        );
    }

    #[test]
    fn a_crate_saved_with_status_words_loads_their_kinds() {
        let text = r#"(entries: [
            (path: "", title: "A", status: Waiting("listed")),
            (path: "", title: "B", status: Waiting("downloading 40%")),
            (path: "", title: "C", status: Waiting("needs yt-dlp")),
            (path: "", title: "D", status: Unavailable("no clip")),
            (path: "", title: "E", status: Unavailable("failed: 502")),
        ], current: None)"#;
        let (p, _) = Playlist::from_saved(ron::from_str(text).unwrap());
        let got: Vec<_> = p.entries().iter().map(|e| e.status.clone()).collect();
        assert_eq!(
            got,
            [
                EntryStatus::Waiting(WaitKind::Listed),
                EntryStatus::Waiting(WaitKind::Downloading(40)),
                EntryStatus::Waiting(WaitKind::NeedsYtDlp),
                EntryStatus::Unavailable(UnavailableKind::NoClip),
                EntryStatus::Unavailable(UnavailableKind::Other("failed: 502".into())),
            ]
        );
        // And they are written back in the same words.
        assert_eq!(
            ron::to_string(&p.to_saved()).unwrap(),
            ron::to_string(&ron::from_str::<SavedPlaylist>(text).unwrap()).unwrap()
        );
    }

    fn sorted_titles(p: &Playlist) -> Vec<String> {
        p.entries().iter().map(|e| e.title.clone()).collect()
    }

    /// Entries titled by name, with the given BPM, catalog number, side, year and for-sale
    /// snapshot (count, lowest cents).
    fn dig_entry(
        p: &mut Playlist,
        title: &str,
        bpm: Option<u16>,
        catno: &str,
        side: &str,
        year: Option<u16>,
        sale: Option<(u32, Option<u64>)>,
    ) -> EntryId {
        let id = p.add_waiting(
            "",
            title,
            None,
            Some(Origin {
                catno: catno.into(),
                position: side.into(),
                year,
                for_sale: sale.map(|(count, lowest_cents)| ForSale {
                    count,
                    lowest_cents,
                    currency: "EUR".into(),
                    fetched_at: 0,
                }),
                ..Default::default()
            }),
            "listed",
        );
        p.entries_mut().find(|e| e.id == id).unwrap().bpm = bpm;
        id
    }

    #[test]
    fn sorting_by_bpm_keeps_unknown_tempos_last_both_ways() {
        let mut p = Playlist::default();
        for (t, b) in [
            ("a", Some(128)),
            ("b", Some(122)),
            ("c", None),
            ("d", Some(140)),
        ] {
            dig_entry(&mut p, t, b, "", "", None, None);
        }
        p.sort_by(Field::Bpm, Dir::Asc);
        assert_eq!(sorted_titles(&p), ["b", "a", "d", "c"]);
        assert_eq!(p.sorted(), Some((Field::Bpm, Dir::Asc)));
        p.sort_by(Field::Bpm, Dir::Desc);
        assert_eq!(sorted_titles(&p), ["d", "a", "b", "c"]);
    }

    #[test]
    fn sorting_by_text_fields_is_natural_and_stable() {
        let mut p = Playlist::default();
        for (t, cat, side) in [
            ("x", "LT-10", "B1"),
            ("y", "LT-2", "A10"),
            ("z", "LT-1", "A2"),
            ("w", "LT-2", ""),
        ] {
            dig_entry(&mut p, t, None, cat, side, None, None);
        }
        p.sort_by(Field::CatNo, Dir::Asc);
        assert_eq!(
            sorted_titles(&p),
            ["z", "y", "w", "x"],
            "ties keep their order"
        );
        p.sort_by(Field::Side, Dir::Asc);
        assert_eq!(sorted_titles(&p), ["z", "y", "x", "w"]);
        p.sort_by(Field::Title, Dir::Desc);
        assert_eq!(sorted_titles(&p), ["z", "y", "x", "w"]);
    }

    #[test]
    fn sorting_by_year_time_and_artist() {
        let mut p = Playlist::default();
        let a = dig_entry(&mut p, "a", None, "", "", Some(1994), None);
        let b = dig_entry(&mut p, "b", None, "", "", Some(1989), None);
        let c = dig_entry(&mut p, "c", None, "", "", None, None);
        p.sort_by(Field::Year, Dir::Asc);
        assert_eq!(sorted_titles(&p), ["b", "a", "c"]);
        for (id, d, artist) in [
            (a, 300.0, "Mira Sol"),
            (b, 200.0, ""),
            (c, 100.0, "nightcraft"),
        ] {
            let e = p.entries_mut().find(|e| e.id == id).unwrap();
            e.duration = Some(d);
            e.artist = artist.into();
        }
        p.sort_by(Field::Time, Dir::Desc);
        assert_eq!(sorted_titles(&p), ["a", "b", "c"]);
        p.sort_by(Field::Artist, Dir::Asc);
        assert_eq!(
            sorted_titles(&p),
            ["a", "c", "b"],
            "case-insensitive, no artist last"
        );
    }

    #[test]
    fn for_sale_sorts_by_price_then_none_then_unknown() {
        let mut p = Playlist::default();
        for (t, sale) in [
            ("unknown", None),
            ("none", Some((0, None))),
            ("cheap", Some((3, Some(500)))),
            ("dear", Some((1, Some(4000)))),
            ("unpriced", Some((2, None))),
        ] {
            dig_entry(&mut p, t, None, "", "", None, sale);
        }
        p.sort_by(Field::ForSale, Dir::Asc);
        assert_eq!(
            sorted_titles(&p),
            ["cheap", "dear", "unpriced", "none", "unknown"]
        );
        p.sort_by(Field::ForSale, Dir::Desc);
        assert_eq!(
            sorted_titles(&p),
            ["dear", "cheap", "unpriced", "none", "unknown"]
        );
    }

    #[test]
    fn the_sort_mark_lasts_until_the_order_is_changed_otherwise() {
        let mut p = Playlist::default();
        for (t, b) in [("a", Some(128)), ("b", Some(122))] {
            dig_entry(&mut p, t, b, "", "", None, None);
        }
        p.sort_by(Field::Title, Dir::Asc);
        dig_entry(&mut p, "c", None, "", "", None, None);
        assert_eq!(
            p.sorted(),
            Some((Field::Title, Dir::Asc)),
            "an add in order keeps it"
        );
        dig_entry(&mut p, "0", None, "", "", None, None);
        assert_eq!(p.sorted(), None, "an add out of order clears it");
        p.sort_by(Field::Title, Dir::Asc);
        p.move_entry(0, 2);
        assert_eq!(p.sorted(), None, "a drag clears it");
    }

    #[test]
    fn sorting_a_thousand_entries_is_quick() {
        let mut p = Playlist::default();
        for i in 0..1000u32 {
            let cat = format!("LT-{}", (i * 7919) % 1000);
            dig_entry(
                &mut p,
                &format!("t{i}"),
                Some((i % 90 + 90) as u16),
                &cat,
                "A1",
                None,
                None,
            );
        }
        let t = std::time::Instant::now();
        p.sort_by(Field::CatNo, Dir::Asc);
        p.sort_by(Field::Bpm, Dir::Desc);
        // < 16 ms each in release; debug builds get a looser bound.
        let limit = if cfg!(debug_assertions) { 200 } else { 32 };
        assert!(t.elapsed().as_millis() < limit, "{:?}", t.elapsed());
    }

    #[test]
    fn save_and_restore() {
        let mut p = pl(3);
        let id = p.entries()[0].id;
        let mut info = tags("Echo".into(), "Crusher-P".into(), Some(230.0));
        info.album = "Vocaloid Hits".into();
        p.set_info(id, info);
        p.set_current(Some(p.entries()[2].id));
        let saved = p.to_saved();
        let text = ron::to_string(&saved).unwrap();
        assert_eq!(
            text.matches("album").count(),
            1,
            "an empty album isn't written"
        );
        let (restored, pending) = Playlist::from_saved(ron::from_str(&text).unwrap());
        assert_eq!(restored.len(), 3);
        assert_eq!(restored.entries()[0].album(), "Vocaloid Hits");
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
        p.set_info(id, tags("t".into(), "a".into(), Some(1.0)));
        assert_eq!(
            p.get(id).unwrap().status,
            EntryStatus::Waiting("listed".into())
        );
        assert!(p.set_audio(id, TrackRef::new("/cache/abcdefghijk.m4a")));
        assert_eq!(p.get(id).unwrap().status, EntryStatus::Pending);
        let mut info = tags(
            "glasshouse (vinyl rip)".into(),
            "Unknown".into(),
            Some(301.0),
        );
        info.album = "Rips".into();
        p.set_info(id, info);
        let e = p.get(id).unwrap();
        assert_eq!(e.display_name(), "(LT-012) Nightcraft: Glasshouse");
        assert_eq!(e.album(), "", "the record's (empty) album, not the tag's");
        assert_eq!((e.duration, &e.status), (Some(301.0), &EntryStatus::Ready));
        // Entries without an origin still take the tags.
        let plain = p.add([TrackRef::new("/m/x.mp3")])[0].0;
        let mut info = tags("Title".into(), "Artist".into(), None);
        info.album = "Album".into();
        p.set_info(plain, info);
        assert_eq!(p.get(plain).unwrap().plain_name(), "Artist - Title");
        assert_eq!(p.get(plain).unwrap().album(), "Album");
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

    #[test]
    fn albums_group_by_release_master_or_local_tags() {
        let mut p = Playlist::default();
        let rel = |r: Option<u64>, m: Option<u64>, clip: &str| Origin {
            release: r,
            master: m,
            clip: Some(clip.into()),
            ..Default::default()
        };
        let a1 = p.add_waiting("N", "a1", None, Some(rel(Some(1), Some(9), "a")), "queued");
        let other = p.add_waiting("N", "x", None, Some(rel(Some(2), Some(9), "b")), "queued");
        let a2 = p.add_waiting("N", "a2", None, Some(rel(Some(1), Some(9), "c")), "listed");
        p.set_unavailable(a2, "no clip");
        let m1 = p.add_waiting("N", "m1", None, Some(rel(None, Some(7), "d")), "queued");
        let m2 = p.add_waiting("N", "m2", None, Some(rel(None, Some(7), "e")), "queued");
        let none = p.add_waiting("N", "n", None, Some(rel(None, None, "f")), "queued");

        assert_eq!(
            p.album_of(a1),
            [a1, a2],
            "same release, unavailable included"
        );
        assert_eq!(p.album_of(other), [other], "another pressing of master 9");
        assert_eq!(p.album_of(m2), [m1, m2], "same master, no release");
        assert!(p.album_of(none).is_empty(), "no record: no album");

        let local: Vec<EntryId> = p
            .add((0..4).map(|i| TrackRef::new(format!("/m/{i}.mp3"))))
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        for (id, artist, album) in [
            (local[0], "Artist A", "Greatest Hits"),
            (local[1], "Artist B", "Greatest Hits"),
            (local[2], "artist a ", "greatest hits"),
            (local[3], "Artist A", ""),
        ] {
            let mut info = tags("t".into(), artist.into(), None);
            info.album = album.into();
            p.set_info(id, info);
        }
        assert_eq!(
            p.album_of(local[0]),
            [local[0], local[2]],
            "case and spaces aside"
        );
        assert_eq!(p.album_of(local[1]), [local[1]], "same name, other artist");
        assert!(p.album_of(local[3]).is_empty(), "no album tag");
    }

    /// Entries at these tempos (`None`: unknown), from /m/0.mp3 on.
    fn tempos(bpms: &[Option<u16>]) -> Playlist {
        let mut p = pl(bpms.len());
        for (e, b) in p.entries.iter_mut().zip(bpms) {
            e.bpm = *b;
        }
        p
    }

    #[test]
    fn a_bpm_range_shows_the_entries_inside_it() {
        let mut p = tempos(&[Some(124), Some(128), Some(137), Some(139)]);
        assert_eq!(p.tempo_span(), Some((124, 139)));
        assert_eq!(p.shown_rows(), [0, 1, 2, 3], "no filter: everything");
        assert!(p.set_bpm_filter(Some((140, 130))), "either order");
        assert_eq!(
            p.bpm_filter(),
            Some((130, 139)),
            "within the crate's tempos"
        );
        assert_eq!(p.shown_rows(), [2, 3]);
        assert!(p.set_bpm_filter(Some((100, 200))), "the whole span: off");
        assert_eq!(p.bpm_filter(), None);
        assert!(!p.set_bpm_filter(None), "already off");
    }

    #[test]
    fn entries_without_a_tempo_show_only_without_a_filter() {
        let mut p = tempos(&[Some(124), None, Some(139), None, None]);
        assert_eq!(p.without_bpm(), 3);
        assert_eq!(p.shown_rows().len(), 5);
        p.set_bpm_filter(Some((130, 140)));
        assert_eq!(p.shown_rows(), [2]);
        assert!(!p.shows(&p.entries[1].clone()));
    }

    #[test]
    fn a_range_needs_two_tempos_and_follows_them() {
        let mut p = tempos(&[Some(128), None]);
        assert_eq!(p.tempo_span(), None, "one tempo: no range");
        p.set_bpm_filter(Some((120, 125)));
        assert_eq!(p.bpm_filter(), None);
        let mut p = tempos(&[Some(124), Some(128), Some(137), Some(139)]);
        p.set_bpm_filter(Some((130, 139)));
        // The fast tracks go: the range no longer reaches any tempo, so it is off again.
        p.remove_ids(&[p.entries[2].id, p.entries[3].id]);
        assert_eq!(p.bpm_filter(), None);
        assert_eq!(p.shown_rows(), [0, 1]);
    }

    #[test]
    fn the_range_is_saved_with_the_crate() {
        let mut p = tempos(&[Some(124), Some(128), Some(137), Some(139)]);
        let text = ron::to_string(&p.to_saved()).unwrap();
        assert!(
            !text.contains("bpm_range"),
            "nothing saved without a filter"
        );
        p.set_bpm_filter(Some((130, 140)));
        let text = ron::to_string(&p.to_saved()).unwrap();
        let (restored, _) = Playlist::from_saved(ron::from_str(&text).unwrap());
        assert_eq!(restored.bpm_filter(), Some((130, 139)));
        // A hand-edited, upside-down or far-off range applies as nothing.
        let mut saved = p.to_saved();
        saved.bpm_range = Some((300, 400));
        assert_eq!(Playlist::from_saved(saved).0.bpm_filter(), None);
    }

    #[test]
    fn filtering_a_big_crate_is_quick() {
        let bpms: Vec<Option<u16>> = (0..1000)
            .map(|i| (i % 7 != 0).then_some(90 + (i % 80) as u16))
            .collect();
        let mut p = tempos(&bpms);
        let t = std::time::Instant::now();
        p.set_bpm_filter(Some((120, 140)));
        let rows = p.shown_rows();
        assert!(!rows.is_empty());
        assert!(
            t.elapsed() < std::time::Duration::from_millis(16),
            "{:?}",
            t.elapsed()
        );
    }

    #[test]
    fn the_play_order_keeps_to_the_filter_and_the_current_entry() {
        let mut p = tempos(&[Some(124), Some(134), Some(124), Some(124), Some(138)]);
        let ids: Vec<EntryId> = p.entries.iter().map(|e| e.id).collect();
        p.set_current(Some(ids[1]));
        p.set_bpm_filter(Some((130, 140)));
        assert_eq!(
            p.play_order(false, None, 0),
            [ids[1], ids[4]],
            "next after 2 is 5"
        );
        // The playing entry hidden by a new range plays on, and the next shown one follows.
        p.set_bpm_filter(Some((136, 140)));
        assert_eq!(p.play_order(false, None, 0), [ids[1], ids[4]]);
        let shuffled = p.play_order(true, Some(ids[1]), 7);
        assert_eq!(shuffled.len(), 2);
        assert_eq!(shuffled[0], ids[1]);
    }

    /// A crate of one entry per record, release `i + 1` with these styles ("" for none).
    fn styled(styles: &[&str]) -> (Playlist, Vec<EntryId>) {
        let mut p = Playlist::default();
        let ids = styles
            .iter()
            .enumerate()
            .map(|(i, st)| {
                let o = Origin {
                    styles: (*st).into(),
                    ..origin(i as u64 + 1, &format!("c{i}"))
                };
                p.add_waiting("A", format!("t{i}"), None, Some(o), "listed")
            })
            .collect();
        (p, ids)
    }

    #[test]
    fn a_style_filter_shows_records_with_any_selected_style() {
        let (mut p, ids) = styled(&["Deep House", "Minimal, Techno", "Electro", ""]);
        assert!(!p.is_filtered());
        p.set_style("Deep House", true);
        p.set_style("Minimal", true);
        assert_eq!(p.shown_rows(), [0, 1], "any of them; none for no style");
        assert!(p.is_filtered());
        assert!(!p.shows(&p.entries[3].clone()));
        // Both filters: a tempo in the range, and a selected style.
        for (e, b) in p.entries.iter_mut().zip([124, 134, 134, 134]) {
            e.bpm = Some(b);
        }
        p.set_bpm_filter(Some((130, 140)));
        assert_eq!(p.shown_rows(), [1]);
        p.set_current(Some(ids[0]));
        assert_eq!(
            p.play_order(false, None, 0),
            [ids[0], ids[1]],
            "the current plays on"
        );
        assert!(p.clear_filters());
        assert_eq!(p.shown_rows(), [0, 1, 2, 3]);
        assert!(!p.clear_filters(), "already off");
    }

    #[test]
    fn styles_are_counted_by_record_most_first() {
        // Two entries of release 1 count once.
        let (mut p, _) = styled(&["Minimal, Deep House", "Deep House", "Electro"]);
        let more = Origin {
            styles: "Minimal, Deep House".into(),
            ..origin(1, "c9")
        };
        p.add_waiting("A", "t9", None, Some(more), "listed");
        assert_eq!(
            p.styles(),
            [
                ("Deep House".to_owned(), 2),
                ("Electro".to_owned(), 1),
                ("Minimal".to_owned(), 1)
            ]
        );
    }

    #[test]
    fn a_style_gone_from_the_crate_stops_filtering() {
        let (mut p, ids) = styled(&["Deep House", "Electro"]);
        p.set_style("Electro", true);
        assert_eq!(p.shown_rows(), [1]);
        p.remove_ids(&[ids[1]]);
        assert_eq!(p.style_filter(), None);
        assert_eq!(p.shown_rows(), [0], "everything shows again");
    }

    #[test]
    fn the_style_filter_is_saved_with_the_crate() {
        let (mut p, _) = styled(&["Deep House", "Electro"]);
        let text = ron::to_string(&p.to_saved()).unwrap();
        assert!(
            !text.contains("styles: ["),
            "nothing saved without a filter"
        );
        p.set_style("Deep House", true);
        let text = ron::to_string(&p.to_saved()).unwrap();
        let (restored, _) = Playlist::from_saved(ron::from_str(&text).unwrap());
        assert!(restored.style_on("Deep House"));
        assert_eq!(restored.shown_rows(), [0]);
    }

    #[test]
    fn style_filtering_a_big_crate_is_quick() {
        let names = [
            "Deep House",
            "Minimal",
            "Techno",
            "Dub Techno",
            "Electro",
            "Ambient",
        ];
        let st: Vec<String> = (0..5000)
            .map(|i| format!("{}, {}", names[i % 6], names[(i / 6) % 6]))
            .collect();
        let refs: Vec<&str> = st.iter().map(String::as_str).collect();
        let (mut p, _) = styled(&refs);
        // The best of three, so a busy machine doesn't decide it.
        let best = |f: &mut dyn FnMut()| {
            (0..3)
                .map(|_| {
                    let t = std::time::Instant::now();
                    f();
                    t.elapsed()
                })
                .min()
                .unwrap()
        };
        let budget = if cfg!(debug_assertions) { 40 } else { 16 };
        let took = best(&mut || {
            p.set_style("Electro", false);
            p.set_style("Electro", true);
            let rows = p.shown_rows();
            let order = p.play_order(false, None, 0);
            assert!(!rows.is_empty() && !order.is_empty());
        });
        assert!(took < std::time::Duration::from_millis(budget), "{took:?}");
        // Counting styles runs again only when the crate changes, not per frame.
        let budget = if cfg!(debug_assertions) { 200 } else { 16 };
        let took = best(&mut || assert_eq!(p.styles().len(), 6));
        assert!(took < std::time::Duration::from_millis(budget), "{took:?}");
    }

    /// A crate of `releases` (one entry each, clip `c<i>`), plus a local file without an album
    /// where `None` is given.
    fn records(releases: &[Option<u64>]) -> (Playlist, Vec<EntryId>) {
        let mut p = Playlist::default();
        let ids = releases
            .iter()
            .enumerate()
            .map(|(i, r)| match r {
                Some(r) => p.add_waiting(
                    "A",
                    format!("t{i}"),
                    None,
                    Some(origin(*r, &format!("c{i}"))),
                    "listed",
                ),
                None => p.add([TrackRef::new(format!("/m/{i}.mp3"))])[0].0,
            })
            .collect();
        (p, ids)
    }

    fn order(p: &Playlist) -> Vec<String> {
        p.entries().iter().map(|e| e.title.clone()).collect()
    }

    #[test]
    fn gathering_moves_each_record_up_to_its_first_entry() {
        // 12, 13, 14 and 40 are one release (here t0, t1, t2, t5); t3 has no album.
        let (mut p, ids) = records(&[Some(1), Some(1), Some(1), None, Some(2), Some(1), Some(3)]);
        p.set_current(Some(ids[5]));
        let rev = p.rev();
        assert!(p.set_grouped(true));
        assert_eq!(
            order(&p),
            ["t0", "t1", "t2", "t5", "/m/3", "t4", "t6"]
                .map(|t| t.trim_start_matches("/m/").to_owned())
        );
        assert_eq!(
            p.current(),
            Some(ids[5]),
            "the playing entry is the same one"
        );
        assert!(p.rev() > rev);
        // Already gathered: nothing moves, and ungrouping keeps the order.
        assert!(!p.gather());
        p.set_grouped(false);
        assert_eq!(order(&p)[3], "t5");
    }

    #[test]
    fn entries_added_while_grouped_join_their_record_at_the_next_settle() {
        let (mut p, _) = records(&[Some(1), Some(2), Some(3)]);
        p.set_grouped(true);
        p.add_waiting("A", "t3", None, Some(origin(1, "c3")), "listed");
        assert_eq!(order(&p).last().unwrap(), "t3", "appended until settled");
        assert!(p.settle());
        assert_eq!(order(&p), ["t0", "t3", "t1", "t2"]);
        assert!(!p.settle(), "nothing more to place");
        // Flat crates never move.
        let (mut flat, _) = records(&[Some(1), Some(2)]);
        flat.add_waiting("A", "t2", None, Some(origin(1, "c2")), "listed");
        assert!(!flat.settle());
        assert_eq!(order(&flat), ["t0", "t1", "t2"]);
    }

    #[test]
    fn sorting_a_grouped_crate_gathers_after_sorting() {
        let (mut p, _) = records(&[Some(1), Some(2), Some(1)]);
        for (e, bpm) in p.entries.iter_mut().zip([130, 120, 110]) {
            e.bpm = Some(bpm);
        }
        p.set_grouped(true); // t0, t2, t1
        p.sort_by(Field::Bpm, Dir::Asc);
        // By tempo t2 (110) comes first, so its record leads: t2, t0, then t1.
        assert_eq!(order(&p), ["t2", "t0", "t1"]);
        assert_eq!(p.sorted(), Some((Field::Bpm, Dir::Asc)));
    }

    #[test]
    fn a_record_moves_as_a_block_and_selections_take_whole_records() {
        let (mut p, ids) = records(&[Some(1), Some(1), Some(2), Some(3)]);
        p.move_block(&[ids[0], ids[1]], Some(ids[3]));
        assert_eq!(order(&p), ["t2", "t0", "t1", "t3"]);
        p.move_block(&[ids[0], ids[1]], Some(ids[1]));
        assert_eq!(order(&p), ["t2", "t0", "t1", "t3"], "onto itself: nothing");
        p.move_block(&[ids[2]], None);
        assert_eq!(order(&p), ["t0", "t1", "t3", "t2"]);
        p.toggle_ids(&[ids[0], ids[1]]);
        assert_eq!(p.selected_ids(), [ids[0], ids[1]]);
        p.toggle_ids(&[ids[0], ids[1]]);
        assert!(p.selected_ids().is_empty());
        p.select_only(&[ids[0]], ids[0]);
        p.extend_to(3, &[ids[2]], ids[2]);
        assert_eq!(p.selected_ids().len(), 4);
    }

    #[test]
    fn gathering_five_thousand_entries_is_quick() {
        let releases: Vec<Option<u64>> = (0..5000u64).map(|i| Some(i % 1500)).collect();
        let (scattered, _) = records(&releases);
        // The best of a few runs (each from the same scattered order).
        let mut took = std::time::Duration::MAX;
        for _ in 0..5 {
            let mut p = Playlist::default();
            p.add_saved(
                scattered
                    .saved_entries(&scattered.entries().iter().map(|e| e.id).collect::<Vec<_>>()),
            );
            let t = std::time::Instant::now();
            assert!(p.set_grouped(true));
            took = took.min(t.elapsed());
        }
        let budget = if cfg!(debug_assertions) { 40 } else { 16 };
        assert!(took < std::time::Duration::from_millis(budget), "{took:?}");
    }
}
