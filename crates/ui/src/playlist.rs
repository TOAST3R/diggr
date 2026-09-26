//! The playlist model: entries, selection, reordering, totals, and play order.
//!
//! Entries have stable ids so the current track and pending metadata survive reordering.

use std::collections::BTreeSet;

use platform::TrackRef;
use serde::{Deserialize, Serialize};

pub type EntryId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStatus {
    /// Metadata not read yet.
    Pending,
    Ready,
    /// Could not be opened or decoded.
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: EntryId,
    pub track: TrackRef,
    pub title: String,
    pub artist: String,
    pub duration: Option<f64>,
    pub status: EntryStatus,
}

impl Entry {
    /// "Artist - Title", or just the title.
    pub fn display_name(&self) -> String {
        if self.artist.is_empty() {
            self.title.clone()
        } else {
            format!("{} - {}", self.artist, self.title)
        }
    }
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
                status: EntryStatus::Pending,
                track,
            });
        }
        added
    }

    fn alloc_id(&mut self) -> EntryId {
        self.next_id += 1;
        self.next_id
    }

    pub fn set_info(&mut self, id: EntryId, title: String, artist: String, duration: Option<f64>) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.title = title;
            e.artist = artist;
            e.duration = duration;
            e.status = EntryStatus::Ready;
        }
    }

    pub fn set_failed(&mut self, id: EntryId) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.status = EntryStatus::Failed;
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
            entries: self
                .entries
                .iter()
                .map(|e| SavedEntry {
                    path: e.track.0.clone(),
                    title: e.title.clone(),
                    artist: e.artist.clone(),
                    duration: e.duration,
                })
                .collect(),
            current: self.current_index(),
        }
    }

    /// Restores a saved playlist; entries without cached metadata are returned for lookup.
    pub fn from_saved(saved: SavedPlaylist) -> (Self, Vec<(EntryId, TrackRef)>) {
        let mut pl = Playlist::default();
        let mut pending = Vec::new();
        for s in saved.entries {
            let id = pl.alloc_id();
            let track = TrackRef::new(s.path);
            let known = !s.title.is_empty();
            if !known || s.duration.is_none() {
                pending.push((id, track.clone()));
            }
            pl.entries.push(Entry {
                id,
                title: if known {
                    s.title
                } else {
                    track.stem().to_owned()
                },
                artist: s.artist,
                duration: s.duration,
                status: if known {
                    EntryStatus::Ready
                } else {
                    EntryStatus::Pending
                },
                track,
            });
        }
        pl.current = saved.current.and_then(|i| pl.entries.get(i)).map(|e| e.id);
        (pl, pending)
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
        assert_eq!(p.entries()[0].display_name(), "M83 - Midnight City");
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
    fn save_and_restore() {
        let mut p = pl(3);
        let id = p.entries()[0].id;
        p.set_info(id, "Echo".into(), "Crusher-P".into(), Some(230.0));
        p.set_current(Some(p.entries()[2].id));
        let saved = p.to_saved();
        let text = ron::to_string(&saved).unwrap();
        let (restored, pending) = Playlist::from_saved(ron::from_str(&text).unwrap());
        assert_eq!(restored.len(), 3);
        assert_eq!(restored.entries()[0].display_name(), "Crusher-P - Echo");
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
}
