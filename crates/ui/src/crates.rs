//! Crates: named playlists. The playlist window shows one crate and playback follows another
//! (usually the same one), so you can look through a crate without interrupting what plays.
//!
//! Each crate is a [`Playlist`] saved as `crates/<id>.ron` in the config folder; the list of
//! crates is `crates/index.ron`. Only the index and the shown crate are read at launch, others
//! when they are first shown, played or sent to, so many large crates don't slow launch. A crate
//! file that can't be read is reported once, left on disk and never overwritten.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use platform::TrackRef;
use serde::{Deserialize, Serialize};

use crate::playlist::{EntryId, Playlist, SavedPlaylist};
use crate::settings::{PLAYLIST_FILE, Store};

pub type CrateId = u64;

/// The scratch crate: files opened with Eject or on the command line replace it. It always
/// exists and can't be renamed or deleted.
pub const PLAYLIST: CrateId = 1;
pub const PLAYLIST_NAME: &str = "Playlist";
pub const CRATES_DIR: &str = "crates";
pub const INDEX_FILE: &str = "index.ron";
pub const MAX_NAME: usize = 40;
/// The name the app gives the crate it makes from the user's Discogs collection.
pub const COLLECTION_PREFIX: &str = "Collection: ";

/// Metadata lookups are keyed by crate as well as entry: entry ids are per crate.
pub type MetaKey = (CrateId, EntryId);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrateInfo {
    pub id: CrateId,
    pub name: String,
    /// As of the last save (the menu and the delete confirmation use it for unloaded crates).
    pub entries: usize,
    /// Seconds since the Unix epoch.
    pub created: u64,
    /// Made from the user's Discogs collection (the sidebar pins it under DISCOGS).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub collection: bool,
    /// The user's Discogs wantlist, once a token is set (pinned under DISCOGS, above the
    /// collection).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wantlist: bool,
    /// A Top Sellers crate: the seller's username (pinned under TOP SELLERS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller: Option<String>,
    /// A followed label's crate: the label's Discogs id (pinned under LABELS). It fills only
    /// from that label's page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<u64>,
    /// A followed label's Bandcamp (`analogicalforce` for analogicalforce.bandcamp.com), alone
    /// or beside its Discogs id: the crate fills from that page too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandcamp: Option<String>,
    /// Shown grouped by record, once toggled; `None` follows the crate's kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouped: Option<bool>,
    /// Records as of the last save (albums, and entries of no album), for the sidebar of a
    /// grouped crate that isn't loaded; 0 when not known yet.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub records: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Records in a playlist: its albums, and each entry that belongs to none.
pub fn count_records(p: &Playlist) -> usize {
    let mut albums = HashSet::new();
    let mut loose = 0;
    for e in p.entries() {
        match e.album_key() {
            Some(k) => {
                albums.insert(k);
            }
            None => loose += 1,
        }
    }
    albums.len() + loose
}

impl CrateInfo {
    /// One of the user's own Discogs crates (the wantlist or the collection).
    pub fn discogs(&self) -> bool {
        self.collection || self.wantlist
    }

    /// A label crate (LABELS): following a Discogs label, a Bandcamp one, or both.
    pub fn is_label(&self) -> bool {
        self.label.is_some() || self.bandcamp.is_some()
    }

    /// Grouped by record: as toggled, else the user's Discogs crates, seller crates and label
    /// crates are and others aren't.
    pub fn is_grouped(&self) -> bool {
        self.grouped
            .unwrap_or_else(|| self.discogs() || self.seller.is_some() || self.is_label())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Index {
    next_id: CrateId,
    shown: CrateId,
    /// In creation order, Playlist first.
    crates: Vec<CrateInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CrateFile {
    name: String,
    playlist: SavedPlaylist,
}

pub struct Crates {
    /// The `crates/` folder; `None` keeps everything in memory.
    store: Option<Store>,
    index: Index,
    loaded: HashMap<CrateId, Playlist>,
    unreadable: HashSet<CrateId>,
    dirty: HashMap<CrateId, Instant>,
    index_dirty: Option<Instant>,
    playing: CrateId,
    /// Records per loaded crate, with the playlist revision they were counted at.
    records: std::cell::RefCell<HashMap<CrateId, (u64, usize)>>,
    pending_meta: Vec<(MetaKey, TrackRef)>,
    messages: Vec<String>,
}

impl Crates {
    /// Crates kept in memory only (no config folder).
    pub fn in_memory() -> Self {
        let mut c = Self::empty(None);
        c.loaded.insert(PLAYLIST, Playlist::default());
        c
    }

    fn empty(store: Option<Store>) -> Self {
        Self {
            store,
            index: Index {
                next_id: PLAYLIST + 1,
                shown: PLAYLIST,
                crates: vec![CrateInfo {
                    id: PLAYLIST,
                    name: PLAYLIST_NAME.into(),
                    entries: 0,
                    created: now_secs(),
                    collection: false,
                    wantlist: false,
                    seller: None,
                    label: None,
                    bandcamp: None,
                    grouped: None,
                    records: 0,
                }],
            },
            loaded: HashMap::new(),
            unreadable: HashSet::new(),
            dirty: HashMap::new(),
            index_dirty: None,
            playing: PLAYLIST,
            records: std::cell::RefCell::new(HashMap::new()),
            pending_meta: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// Opens the crates in `config`'s `crates/` folder: reads the index (rebuilding it from the
    /// crate files if it is damaged, or migrating the single `playlist.ron` on first launch)
    /// and the crate that was shown.
    pub fn open(config: &Store) -> Self {
        let store = Store::new(config.dir().join(CRATES_DIR));
        let mut c = Self::empty(Some(store.clone()));
        match store.try_load::<Index>(INDEX_FILE) {
            Ok(Some(index)) => c.index = index,
            Ok(None) if crate_files(&store).is_empty() => c.migrate(config),
            Ok(None) => c.rebuild_index(),
            Err(_) => {
                c.rebuild_index();
                c.messages
                    .push("The list of crates was damaged and has been rebuilt".into());
            }
        }
        c.sanitize_index();
        if !c.load(c.index.shown) {
            c.index.shown = PLAYLIST;
            c.load_playlist();
        }
        c.playing = c.index.shown;
        c
    }

    /// First launch with crates: the single playlist becomes the Playlist crate. The old file
    /// is only read, so the previous version still finds it as it was.
    fn migrate(&mut self, config: &Store) {
        let saved = match config.try_load::<SavedPlaylist>(PLAYLIST_FILE) {
            Ok(Some(saved)) => saved,
            Ok(None) => return self.mark_index(), // fresh install
            Err(e) => {
                self.messages
                    .push(format!("The old playlist could not be read: {e}"));
                return self.mark_index();
            }
        };
        let file = CrateFile {
            name: PLAYLIST_NAME.into(),
            playlist: saved,
        };
        self.index.crates[0].entries = file.playlist.entries.len();
        if let Some(store) = &self.store
            && let Err(e) = store
                .save(&file_name(PLAYLIST), &file)
                .and_then(|()| store.save(INDEX_FILE, &self.index))
        {
            self.messages
                .push(format!("Could not save the Playlist crate: {e}"));
        }
    }

    /// Rebuilds the index from the name stored in each crate file.
    fn rebuild_index(&mut self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        self.index.crates.clear();
        for id in crate_files(&store) {
            let name = match store.try_load::<CrateFile>(&file_name(id)) {
                Ok(Some(f)) => f.name,
                _ => {
                    self.unreadable.insert(id);
                    self.messages.push(unreadable_message(id, None));
                    if id == PLAYLIST {
                        PLAYLIST_NAME.into()
                    } else {
                        format!("Crate {id}")
                    }
                }
            };
            self.index.crates.push(CrateInfo {
                id,
                name,
                entries: 0,
                created: id,
                collection: false,
                wantlist: false,
                seller: None,
                label: None,
                bandcamp: None,
                grouped: None,
                records: 0,
            });
        }
        self.index.shown = PLAYLIST;
        self.mark_index();
    }

    /// Makes a hand-edited or rebuilt index usable: Playlist first, unique ids, a shown crate
    /// that exists.
    fn sanitize_index(&mut self) {
        let mut seen = HashSet::new();
        self.index.crates.retain(|c| seen.insert(c.id));
        match self.index.crates.iter().position(|c| c.id == PLAYLIST) {
            Some(0) => {}
            Some(i) => {
                let p = self.index.crates.remove(i);
                self.index.crates.insert(0, p);
            }
            None => {
                self.index.crates.insert(
                    0,
                    CrateInfo {
                        id: PLAYLIST,
                        name: PLAYLIST_NAME.into(),
                        entries: 0,
                        created: now_secs(),
                        collection: false,
                        wantlist: false,
                        seller: None,
                        label: None,
                        bandcamp: None,
                        grouped: None,
                        records: 0,
                    },
                );
                self.mark_index();
            }
        }
        self.index.crates[0].name = PLAYLIST_NAME.into();
        // Collection crates made before the flag existed are known by their name.
        for c in &mut self.index.crates {
            if !c.collection && c.name.starts_with(COLLECTION_PREFIX) {
                c.collection = true;
                self.index_dirty.get_or_insert_with(Instant::now);
            }
        }
        let max = self
            .index
            .crates
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(PLAYLIST);
        self.index.next_id = self.index.next_id.max(max + 1);
        if self.info(self.index.shown).is_none() {
            self.index.shown = PLAYLIST;
        }
    }

    /// Loads the Playlist crate, which must exist: if its file can't be read, it is kept next
    /// to the others as `1.ron.unreadable` and an empty one takes its place.
    pub fn load_playlist(&mut self) {
        if self.load(PLAYLIST) {
            return;
        }
        if let Some(store) = &self.store {
            let from = store.dir().join(file_name(PLAYLIST));
            let to = store
                .dir()
                .join(format!("{}.unreadable", file_name(PLAYLIST)));
            if std::fs::rename(&from, &to).is_ok() {
                self.messages.push(format!(
                    "The Playlist crate could not be read; it was kept as {}",
                    to.display()
                ));
            }
        }
        self.unreadable.remove(&PLAYLIST);
        self.loaded.insert(PLAYLIST, Playlist::default());
    }

    // ---- loading ---------------------------------------------------------------------------

    /// Loads a crate if it isn't yet. False if it doesn't exist or can't be read.
    pub fn load(&mut self, id: CrateId) -> bool {
        if self.loaded.contains_key(&id) {
            return true;
        }
        if self.unreadable.contains(&id) || self.info(id).is_none() {
            return false;
        }
        let file = match &self.store {
            Some(store) => store.try_load::<CrateFile>(&file_name(id)),
            None => Ok(None),
        };
        let saved = match file {
            Ok(Some(f)) => f.playlist,
            // Listed but never saved (created just before a crash): empty.
            Ok(None) => SavedPlaylist::default(),
            Err(_) => {
                self.unreadable.insert(id);
                let name = self.info(id).map(|c| c.name.clone());
                self.messages.push(unreadable_message(id, name.as_deref()));
                return false;
            }
        };
        let (mut playlist, pending) = Playlist::from_saved(saved);
        self.pending_meta
            .extend(pending.into_iter().map(|(e, t)| ((id, e), t)));
        let gathered = self
            .info(id)
            .is_some_and(|c| c.is_grouped() && playlist.set_grouped(true));
        self.loaded.insert(id, playlist);
        if gathered {
            self.touch(id);
        }
        true
    }

    pub fn is_grouped(&self, id: CrateId) -> bool {
        self.info(id).is_some_and(CrateInfo::is_grouped)
    }

    /// Groups a crate by record (gathering it) or shows it flat, remembered across launches.
    pub fn set_grouped(&mut self, id: CrateId, on: bool) {
        if let Some(c) = self.index.crates.iter_mut().find(|c| c.id == id) {
            c.grouped = Some(on);
            self.mark_index();
        }
        self.apply_grouped(id);
    }

    /// The loaded playlist takes the crate's grouping (after its kind or setting changed).
    fn apply_grouped(&mut self, id: CrateId) {
        let on = self.is_grouped(id);
        if let Some(p) = self.loaded.get_mut(&id)
            && p.is_grouped() != on
            && p.set_grouped(on)
        {
            self.touch(id);
        }
    }

    pub fn is_loaded(&self, id: CrateId) -> bool {
        self.loaded.contains_key(&id)
    }

    pub fn is_unreadable(&self, id: CrateId) -> bool {
        self.unreadable.contains(&id)
    }

    /// Entries whose metadata has to be read, from crates loaded since the last call.
    pub fn take_pending_meta(&mut self) -> Vec<(MetaKey, TrackRef)> {
        std::mem::take(&mut self.pending_meta)
    }

    /// Problems to show the user (unreadable crates), once each.
    pub fn take_messages(&mut self) -> Vec<String> {
        std::mem::take(&mut self.messages)
    }

    // ---- the list --------------------------------------------------------------------------

    /// Every crate, Playlist first, then in creation order.
    pub fn list(&self) -> &[CrateInfo] {
        &self.index.crates
    }

    pub fn info(&self, id: CrateId) -> Option<&CrateInfo> {
        self.index.crates.iter().find(|c| c.id == id)
    }

    pub fn name(&self, id: CrateId) -> &str {
        self.info(id).map_or("", |c| c.name.as_str())
    }

    /// Current entry count, or the saved one for a crate that isn't loaded.
    pub fn entry_count(&self, id: CrateId) -> usize {
        match self.loaded.get(&id) {
            Some(p) => p.len(),
            None => self.info(id).map_or(0, |c| c.entries),
        }
    }

    /// Records in a crate (albums, and entries of no album): counted for a loaded crate
    /// (again only when it changed), else as of its last save, else its entries.
    pub fn record_count(&self, id: CrateId) -> usize {
        let Some(p) = self.loaded.get(&id) else {
            return self
                .info(id)
                .map_or(0, |c| if c.records > 0 { c.records } else { c.entries });
        };
        let mut cache = self.records.borrow_mut();
        match cache.get(&id) {
            Some(&(rev, n)) if rev == p.rev() => n,
            _ => {
                let n = count_records(p);
                cache.insert(id, (p.rev(), n));
                n
            }
        }
    }

    // ---- shown and playing -----------------------------------------------------------------

    pub fn shown_id(&self) -> CrateId {
        self.index.shown
    }

    pub fn playing_id(&self) -> CrateId {
        self.playing
    }

    /// Shows another crate in the window. Playback isn't touched.
    pub fn show(&mut self, id: CrateId) -> bool {
        if !self.load(id) {
            return false;
        }
        if self.index.shown != id {
            // A search lasts while its crate is shown.
            if let Some(p) = self.loaded.get_mut(&self.index.shown) {
                p.set_search("");
            }
            self.index.shown = id;
            self.mark_index();
        }
        true
    }

    /// Makes `id` the crate the engine queue is built from.
    pub fn set_playing(&mut self, id: CrateId) -> bool {
        if !self.load(id) {
            return false;
        }
        self.playing = id;
        true
    }

    pub fn shown(&self) -> &Playlist {
        &self.loaded[&self.index.shown]
    }

    pub fn shown_mut(&mut self) -> &mut Playlist {
        self.loaded
            .get_mut(&self.index.shown)
            .expect("the shown crate is loaded")
    }

    pub fn playing(&self) -> &Playlist {
        &self.loaded[&self.playing]
    }

    pub fn playing_mut(&mut self) -> &mut Playlist {
        self.loaded
            .get_mut(&self.playing)
            .expect("the playing crate is loaded")
    }

    pub fn get(&self, id: CrateId) -> Option<&Playlist> {
        self.loaded.get(&id)
    }

    pub fn get_mut(&mut self, id: CrateId) -> Option<&mut Playlist> {
        self.loaded.get_mut(&id)
    }

    /// The crates read so far (in no particular order).
    pub fn loaded_ids(&self) -> Vec<CrateId> {
        self.loaded.keys().copied().collect()
    }

    /// A crate by name, ignoring case.
    pub fn find(&self, name: &str) -> Option<CrateId> {
        let lower = name.trim().to_lowercase();
        self.index
            .crates
            .iter()
            .find(|c| c.name.to_lowercase() == lower)
            .map(|c| c.id)
    }

    // ---- create, rename, delete ------------------------------------------------------------

    /// Checks a crate name: 1 to 40 characters, unique ignoring case (`except` is the crate
    /// being renamed). Returns it trimmed.
    pub fn validate_name(&self, name: &str, except: Option<CrateId>) -> Result<String, String> {
        let name = name.trim();
        let len = name.chars().count();
        if len == 0 || len > MAX_NAME {
            return Err(format!("A crate name needs 1 to {MAX_NAME} characters"));
        }
        let lower = name.to_lowercase();
        if let Some(other) = self
            .index
            .crates
            .iter()
            .find(|c| Some(c.id) != except && c.name.to_lowercase() == lower)
        {
            return Err(format!("A crate named \"{}\" already exists", other.name));
        }
        Ok(name.to_owned())
    }

    /// Whether the crate was made from the user's Discogs collection.
    pub fn is_collection(&self, id: CrateId) -> bool {
        self.info(id).is_some_and(|c| c.collection)
    }

    /// Marks a crate as made from the user's Discogs collection.
    pub fn set_collection(&mut self, id: CrateId) {
        if let Some(c) = self
            .index
            .crates
            .iter_mut()
            .find(|c| c.id == id && !c.collection)
        {
            c.collection = true;
            self.mark_index();
        }
        self.apply_grouped(id);
    }

    /// Whether the crate is the user's Discogs wantlist.
    pub fn is_wantlist(&self, id: CrateId) -> bool {
        self.info(id).is_some_and(|c| c.wantlist)
    }

    /// The user's Discogs wantlist or collection crate: it mirrors the account, so it takes
    /// no hand edits and offers the style filter.
    pub fn is_discogs(&self, id: CrateId) -> bool {
        self.is_wantlist(id) || self.is_collection(id)
    }

    /// Marks (or unmarks) a crate as the user's Discogs wantlist; only one crate is.
    pub fn set_wantlist(&mut self, id: CrateId, on: bool) {
        let mut changed = false;
        for c in &mut self.index.crates {
            let want = on && c.id == id;
            if c.wantlist != want && (c.id == id || on) {
                c.wantlist = want;
                changed = true;
            }
        }
        if changed {
            self.mark_index();
            let ids: Vec<CrateId> = self.loaded.keys().copied().collect();
            for c in ids {
                self.apply_grouped(c);
            }
        }
    }

    /// The seller whose Top Sellers crate this is.
    pub fn seller_of(&self, id: CrateId) -> Option<&str> {
        self.info(id).and_then(|c| c.seller.as_deref())
    }

    /// Takes no hand edits: the user's Discogs crates mirror the account, and a label crate
    /// fills only from its label.
    pub fn is_locked(&self, id: CrateId) -> bool {
        self.is_discogs(id) || self.is_label(id)
    }

    /// A label crate (LABELS), following Discogs, Bandcamp or both.
    pub fn is_label(&self, id: CrateId) -> bool {
        self.info(id).is_some_and(CrateInfo::is_label)
    }

    /// The Bandcamp this label crate follows.
    pub fn bandcamp_of(&self, id: CrateId) -> Option<&str> {
        self.info(id).and_then(|c| c.bandcamp.as_deref())
    }

    /// The crate following the Bandcamp `name` (its subdomain), if any.
    pub fn find_bandcamp(&self, name: &str) -> Option<CrateId> {
        self.index
            .crates
            .iter()
            .find(|c| c.bandcamp.as_deref() == Some(name))
            .map(|c| c.id)
    }

    /// Marks a crate as following the Bandcamp `name` too (LABELS).
    pub fn set_bandcamp(&mut self, id: CrateId, name: &str) {
        if let Some(c) = self.index.crates.iter_mut().find(|c| c.id == id)
            && c.bandcamp.as_deref() != Some(name)
        {
            c.bandcamp = Some(name.to_owned());
            self.mark_index();
        }
        self.apply_grouped(id);
    }

    /// The Discogs label whose crate this is (LABELS).
    pub fn label_of(&self, id: CrateId) -> Option<u64> {
        self.info(id).and_then(|c| c.label)
    }

    /// The crate following `label`, if any.
    pub fn find_label(&self, label: u64) -> Option<CrateId> {
        self.index
            .crates
            .iter()
            .find(|c| c.label == Some(label))
            .map(|c| c.id)
    }

    /// The label crates, in the order they were followed (created).
    pub fn labels(&self) -> impl Iterator<Item = &CrateInfo> {
        self.index.crates.iter().filter(|c| c.is_label())
    }

    /// Marks a crate as following `label` (LABELS); it stays the label's after a rename.
    pub fn set_label(&mut self, id: CrateId, label: u64) {
        if let Some(c) = self.index.crates.iter_mut().find(|c| c.id == id)
            && c.label != Some(label)
        {
            c.label = Some(label);
            self.mark_index();
        }
        self.apply_grouped(id);
    }

    /// Marks a crate as a seller's (Top Sellers); it stays theirs after a rename.
    pub fn set_seller(&mut self, id: CrateId, seller: &str) {
        if let Some(c) = self.index.crates.iter_mut().find(|c| c.id == id)
            && c.seller.as_deref() != Some(seller)
        {
            c.seller = Some(seller.to_owned());
            self.mark_index();
        }
        self.apply_grouped(id);
    }

    /// Creates an empty crate (at the end of the list); it isn't shown.
    pub fn create(&mut self, name: &str) -> Result<CrateId, String> {
        let name = self.validate_name(name, None)?;
        let id = self.index.next_id;
        self.index.next_id += 1;
        self.index.crates.push(CrateInfo {
            id,
            name,
            entries: 0,
            created: now_secs(),
            collection: false,
            wantlist: false,
            seller: None,
            label: None,
            bandcamp: None,
            grouped: None,
            records: 0,
        });
        self.loaded.insert(id, Playlist::default());
        self.touch(id);
        self.mark_index();
        Ok(id)
    }

    pub fn rename(&mut self, id: CrateId, name: &str) -> Result<(), String> {
        if id == PLAYLIST {
            return Err("The Playlist crate can't be renamed".into());
        }
        if !self.load(id) {
            return Err("That crate can't be read".into());
        }
        let name = self.validate_name(name, Some(id))?;
        if let Some(c) = self.index.crates.iter_mut().find(|c| c.id == id) {
            c.name = name;
        }
        self.touch(id);
        self.mark_index();
        Ok(())
    }

    /// Deletes a crate and its file. A shown or playing crate hands over to Playlist (the
    /// caller stops playback first when it was playing).
    pub fn delete(&mut self, id: CrateId) -> Result<(), String> {
        if id == PLAYLIST {
            return Err("The Playlist crate can't be deleted".into());
        }
        if self.info(id).is_none() {
            return Err("No such crate".into());
        }
        if let Some(store) = &self.store {
            match std::fs::remove_file(store.dir().join(file_name(id))) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Could not delete the crate: {e}")),
            }
        }
        self.index.crates.retain(|c| c.id != id);
        self.loaded.remove(&id);
        self.dirty.remove(&id);
        self.unreadable.remove(&id);
        if self.index.shown == id {
            self.index.shown = PLAYLIST;
        }
        if self.playing == id {
            self.playing = PLAYLIST;
        }
        // Save the list now, so the deleted crate can't come back.
        self.mark_index();
        self.save_index();
        Ok(())
    }

    /// Eject and command-line files: they replace the Playlist crate, which becomes both
    /// shown and playing. No other crate is ever replaced. Returns the new entries (first one
    /// first) for metadata lookup.
    pub fn replace_playlist(
        &mut self,
        tracks: impl IntoIterator<Item = TrackRef>,
    ) -> Vec<(MetaKey, TrackRef)> {
        self.load_playlist();
        self.show(PLAYLIST);
        self.playing = PLAYLIST;
        let playlist = self.loaded.get_mut(&PLAYLIST).expect("loaded above");
        playlist.clear();
        let added = playlist.add(tracks);
        self.touch(PLAYLIST);
        added.into_iter().map(|(e, t)| ((PLAYLIST, e), t)).collect()
    }

    // ---- send to crate ---------------------------------------------------------------------

    /// Copies entries of `from` (in its order, with fresh ids) to the end of `to`, skipping
    /// those `to` already holds (the same origin clip, or the same file). Returns how many were
    /// added; `from` is unchanged.
    pub fn send(&mut self, from: CrateId, ids: &[EntryId], to: CrateId) -> Result<usize, String> {
        if from == to {
            return Ok(0);
        }
        if !self.load(to) {
            return Err(format!("Crate \"{}\" can't be read", self.name(to)));
        }
        let Some(source) = self.loaded.get(&from) else {
            return Ok(0);
        };
        let copies: Vec<_> = source
            .entries()
            .iter()
            .filter(|e| ids.contains(&e.id))
            .map(|e| (e.duplicate_key(), e.id))
            .collect();
        let target = &self.loaded[&to];
        let mut have: HashSet<_> = target.entries().iter().map(|e| e.duplicate_key()).collect();
        let keep: Vec<EntryId> = copies
            .into_iter()
            .filter(|(key, _)| have.insert(key.clone()))
            .map(|(_, id)| id)
            .collect();
        let saved = self.loaded[&from].saved_entries(&keep);
        let pending = self
            .loaded
            .get_mut(&to)
            .expect("loaded above")
            .add_saved(saved);
        self.pending_meta
            .extend(pending.into_iter().map(|(e, t)| ((to, e), t)));
        if !keep.is_empty() {
            self.touch(to);
        }
        Ok(keep.len())
    }

    // ---- saving ----------------------------------------------------------------------------

    /// Marks a crate as changed; it is saved once `delay` has passed (see [`Crates::save_due`]).
    pub fn touch(&mut self, id: CrateId) {
        self.dirty.entry(id).or_insert_with(Instant::now);
    }

    fn mark_index(&mut self) {
        self.index_dirty.get_or_insert_with(Instant::now);
    }

    /// Something is waiting to be saved.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty() || self.index_dirty.is_some()
    }

    /// Saves every crate changed more than `delay` ago (all of them with `force`, on quit), and
    /// the index with them. Writes are atomic. Returns the errors to show.
    pub fn save_due(&mut self, force: bool, delay: Duration) -> Vec<String> {
        let due = |t: &Instant| force || t.elapsed() >= delay;
        let ids: Vec<CrateId> = self
            .dirty
            .iter()
            .filter(|(_, t)| due(t))
            .map(|(&id, _)| id)
            .collect();
        let mut errors = Vec::new();
        for id in ids {
            self.dirty.remove(&id);
            let (Some(store), Some(playlist)) = (&self.store, self.loaded.get(&id)) else {
                continue;
            };
            let file = CrateFile {
                name: self.name(id).to_owned(),
                playlist: playlist.to_saved(),
            };
            let count = playlist.len();
            let records = count_records(playlist);
            match store.save(&file_name(id), &file) {
                Ok(()) => {
                    if let Some(c) = self.index.crates.iter_mut().find(|c| c.id == id)
                        && (c.entries != count || c.records != records)
                    {
                        c.entries = count;
                        c.records = records;
                        self.mark_index();
                    }
                }
                Err(e) => errors.push(format!("Could not save crate \"{}\": {e}", file.name)),
            }
        }
        if self.index_dirty.as_ref().is_some_and(due) {
            errors.extend(self.save_index());
        }
        errors
    }

    fn save_index(&mut self) -> Option<String> {
        self.index_dirty = None;
        let store = self.store.as_ref()?;
        store
            .save(INDEX_FILE, &self.index)
            .err()
            .map(|e| format!("Could not save the list of crates: {e}"))
    }
}

fn file_name(id: CrateId) -> String {
    format!("{id}.ron")
}

fn unreadable_message(id: CrateId, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("Crate \"{name}\" could not be read ({CRATES_DIR}/{id}.ron)"),
        None => format!("{CRATES_DIR}/{id}.ron could not be read"),
    }
}

/// Ids of the `<id>.ron` files in the crates folder, in order.
fn crate_files(store: &Store) -> Vec<CrateId> {
    let Ok(read) = std::fs::read_dir(store.dir()) else {
        return Vec::new();
    };
    let mut ids: Vec<CrateId> = read
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".ron")?.parse().ok()
        })
        .filter(|&id| id > 0)
        .collect();
    ids.sort_unstable();
    ids
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::Origin;

    fn config(name: &str) -> crate::settings::TestStore {
        crate::settings::TestStore::new(&format!("ui-crates-{name}"))
    }

    fn tracks(names: &[&str]) -> Vec<TrackRef> {
        names
            .iter()
            .map(|n| TrackRef::new(format!("/m/{n}.mp3")))
            .collect()
    }

    fn names(p: &Playlist) -> Vec<String> {
        p.entries()
            .iter()
            .map(|e| e.track.stem().to_owned())
            .collect()
    }

    #[test]
    fn a_fresh_install_has_an_empty_playlist_crate() {
        let cfg = config("fresh");
        let c = Crates::open(&cfg);
        assert_eq!(c.list().len(), 1);
        assert_eq!(c.name(PLAYLIST), "Playlist");
        assert_eq!((c.shown_id(), c.playing_id()), (PLAYLIST, PLAYLIST));
        assert!(c.shown().is_empty());
    }

    #[test]
    fn create_rename_delete_follow_the_name_rules() {
        let cfg = config("names");
        let mut c = Crates::open(&cfg);
        let keepers = c.create("Keepers").unwrap();
        assert_eq!(
            c.create("keepers").unwrap_err(),
            "A crate named \"Keepers\" already exists"
        );
        assert!(c.create("  ").is_err(), "empty");
        assert!(c.create(&"x".repeat(41)).is_err(), "too long");
        let gig = c.create(&"é".repeat(40)).unwrap();
        assert!(c.create("PLAYLIST").is_err(), "clashes with Playlist");
        assert_eq!(c.list().len(), 3);
        assert_eq!(c.shown_id(), PLAYLIST, "a new crate isn't shown by itself");

        assert!(c.rename(PLAYLIST, "Scratch").is_err());
        assert!(c.delete(PLAYLIST).is_err());
        assert!(c.rename(gig, "Keepers").is_err());
        c.rename(keepers, "KEEPERS")
            .expect("a crate can change its own case");
        c.rename(gig, "  Gig 12 Oct ").unwrap();
        assert_eq!(c.name(gig), "Gig 12 Oct");

        c.show(gig);
        c.set_playing(gig);
        c.delete(gig).unwrap();
        assert_eq!((c.shown_id(), c.playing_id()), (PLAYLIST, PLAYLIST));
        let order: Vec<&str> = c.list().iter().map(|i| i.name.as_str()).collect();
        assert_eq!(order, ["Playlist", "KEEPERS"]);
    }

    #[test]
    fn crates_load_lazily_and_restart_where_they_were() {
        let cfg = config("restart");
        {
            let mut c = Crates::open(&cfg);
            c.shown_mut().add(tracks(&["a", "b"]));
            c.touch(PLAYLIST);
            let lt = c.create("Lowtide Tapes").unwrap();
            c.show(lt);
            c.shown_mut()
                .add(tracks(&["1", "2", "3", "4", "5", "6", "7", "8"]));
            let seventh = c.shown().entries()[6].id;
            c.shown_mut().set_current(Some(seventh));
            c.touch(lt);
            assert!(c.save_due(true, Duration::ZERO).is_empty());
        }
        let c = Crates::open(&cfg);
        assert_eq!(c.name(c.shown_id()), "Lowtide Tapes");
        assert_eq!(names(c.shown()), ["1", "2", "3", "4", "5", "6", "7", "8"]);
        assert_eq!(c.shown().current_index(), Some(6));
        assert!(
            !c.is_loaded(PLAYLIST),
            "only the shown crate is read at launch"
        );
        assert_eq!(c.entry_count(PLAYLIST), 2, "counts come from the index");
    }

    #[test]
    fn saving_waits_for_the_delay_and_is_atomic() {
        let cfg = config("delay");
        let mut c = Crates::open(&cfg);
        c.shown_mut().add(tracks(&["a"]));
        c.touch(PLAYLIST);
        let file = cfg.dir().join(CRATES_DIR).join("1.ron");
        assert!(c.save_due(false, Duration::from_secs(60)).is_empty());
        assert!(!file.exists(), "not saved before the delay");
        assert!(c.is_dirty());
        assert!(c.save_due(false, Duration::ZERO).is_empty());
        assert!(file.exists());
        assert!(!c.is_dirty());
        let left: Vec<_> = std::fs::read_dir(cfg.dir().join(CRATES_DIR))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("tmp"))
            .collect();
        assert!(
            left.is_empty(),
            "temp files are renamed into place: {left:?}"
        );
    }

    #[test]
    fn an_unreadable_crate_is_listed_reported_once_and_left_alone() {
        let cfg = config("unreadable");
        let broken = {
            let mut c = Crates::open(&cfg);
            let a = c.create("Keepers").unwrap();
            let b = c.create("Broken").unwrap();
            c.get_mut(a).unwrap().add(tracks(&["k"]));
            c.get_mut(b).unwrap().add(tracks(&["x"]));
            c.touch(a);
            c.touch(b);
            c.show(b);
            c.save_due(true, Duration::ZERO);
            b
        };
        let path = cfg.dir().join(CRATES_DIR).join(format!("{broken}.ron"));
        std::fs::write(&path, "not ron {").unwrap();

        let mut c = Crates::open(&cfg);
        assert_eq!(c.shown_id(), PLAYLIST, "launch falls back to Playlist");
        assert_eq!(c.list().len(), 3, "still listed");
        assert!(c.is_unreadable(broken));
        let msgs = c.take_messages();
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(msgs[0].contains("Broken"), "{msgs:?}");
        assert!(!c.show(broken));
        assert!(c.take_messages().is_empty(), "reported once");
        let keepers = c.list()[1].id;
        assert!(c.show(keepers), "the other crates load");
        assert_eq!(names(c.shown()), ["k"]);
        c.save_due(true, Duration::ZERO);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not ron {");
    }

    #[test]
    fn an_unreadable_playlist_crate_is_kept_aside_and_replaced() {
        let cfg = config("playlist-broken");
        {
            let mut c = Crates::open(&cfg);
            c.shown_mut().add(tracks(&["a"]));
            c.touch(PLAYLIST);
            c.save_due(true, Duration::ZERO);
        }
        let dir = cfg.dir().join(CRATES_DIR);
        std::fs::write(dir.join("1.ron"), "broken").unwrap();
        let mut c = Crates::open(&cfg);
        assert_eq!(c.shown_id(), PLAYLIST);
        assert!(c.shown().is_empty());
        assert_eq!(
            c.take_messages().len(),
            2,
            "unreadable, and where it was kept"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("1.ron.unreadable")).unwrap(),
            "broken"
        );
    }

    #[test]
    fn a_damaged_index_is_rebuilt_from_the_crate_files() {
        let cfg = config("index");
        let lt = {
            let mut c = Crates::open(&cfg);
            c.shown_mut().add(tracks(&["a"]));
            c.touch(PLAYLIST);
            let lt = c.create("Lowtide Tapes").unwrap();
            c.save_due(true, Duration::ZERO);
            lt
        };
        let index = cfg.dir().join(CRATES_DIR).join(INDEX_FILE);
        for damage in ["garbage (", ""] {
            if damage.is_empty() {
                std::fs::remove_file(&index).unwrap();
            } else {
                std::fs::write(&index, damage).unwrap();
            }
            let mut c = Crates::open(&cfg);
            let got: Vec<(CrateId, &str)> =
                c.list().iter().map(|i| (i.id, i.name.as_str())).collect();
            assert_eq!(got, [(PLAYLIST, "Playlist"), (lt, "Lowtide Tapes")]);
            assert_eq!(names(c.shown()), ["a"]);
            assert_eq!(
                c.create("Next").unwrap(),
                lt + 1,
                "new ids follow the existing ones"
            );
            c.save_due(true, Duration::ZERO);
            assert!(c.delete(lt + 1).is_ok());
        }
    }

    fn write_old_playlist(cfg: &Store, n: usize) -> Vec<u8> {
        let mut p = Playlist::default();
        let ids: Vec<EntryId> = p
            .add((0..n).map(|i| TrackRef::new(format!("/music/{i:03}.flac"))))
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        for (i, id) in ids.iter().enumerate() {
            p.set_info(
                *id,
                crate::playlist::tags(format!("Track {i}"), "Artist".into(), Some(i as f64 + 60.0)),
            );
        }
        p.set_current(Some(ids[n * 41 / 100])); // entry 123 of 300
        cfg.save(PLAYLIST_FILE, &p.to_saved()).unwrap();
        std::fs::read(cfg.dir().join(PLAYLIST_FILE)).unwrap()
    }

    #[test]
    fn the_single_playlist_becomes_the_playlist_crate() {
        let cfg = config("migrate");
        let before = write_old_playlist(&cfg, 300);
        let c = Crates::open(&cfg);
        let p = c.shown();
        assert_eq!(c.shown_id(), PLAYLIST);
        assert_eq!(p.len(), 300);
        for (i, e) in p.entries().iter().enumerate() {
            assert_eq!(e.track.0, format!("/music/{i:03}.flac"));
            assert_eq!(e.display_name(), format!("Artist: Track {i}"));
            assert_eq!(e.duration, Some(i as f64 + 60.0));
        }
        assert_eq!(p.current_index(), Some(123));
        assert!(
            cfg.dir().join(CRATES_DIR).join(INDEX_FILE).exists(),
            "written at once"
        );
        assert_eq!(
            std::fs::read(cfg.dir().join(PLAYLIST_FILE)).unwrap(),
            before,
            "the old file is untouched"
        );
        // Only once: later edits to playlist.ron are no longer read.
        drop(c);
        write_old_playlist(&cfg, 5);
        assert_eq!(Crates::open(&cfg).shown().len(), 300);
    }

    #[test]
    fn opening_files_replaces_only_the_playlist_crate() {
        let cfg = config("replace");
        let lt = {
            let mut c = Crates::open(&cfg);
            c.shown_mut().add(tracks(&["old"]));
            c.touch(PLAYLIST);
            let lt = c.create("Lowtide Tapes").unwrap();
            c.show(lt);
            c.shown_mut().add(tracks(&["x", "y"]));
            c.touch(lt);
            c.save_due(true, Duration::ZERO);
            lt
        };
        let mut c = Crates::open(&cfg);
        assert_eq!(c.shown_id(), lt);
        let added = c.replace_playlist(tracks(&["one", "two"]));
        assert_eq!(added.len(), 2);
        assert!(added.iter().all(|((id, _), _)| *id == PLAYLIST));
        assert_eq!((c.shown_id(), c.playing_id()), (PLAYLIST, PLAYLIST));
        assert_eq!(names(c.shown()), ["one", "two"]);
        assert_eq!(names(c.get(lt).unwrap()), ["x", "y"], "unchanged");
        c.save_due(true, Duration::ZERO);
        let mut c = Crates::open(&cfg);
        c.show(lt);
        assert_eq!(names(c.shown()), ["x", "y"]);
    }

    #[test]
    fn send_copies_in_order_without_duplicates() {
        let mut c = Crates::in_memory();
        c.shown_mut().add(tracks(&["a", "b", "c", "d"]));
        let origin = Origin {
            release: Some(123456),
            catno: "LT-012".into(),
            position: "A1".into(),
            clip: Some("abcdefghijk".into()),
            ..Default::default()
        };
        let remote = c.shown_mut().add_waiting(
            "Nightcraft",
            "Glasshouse",
            None,
            Some(origin.clone()),
            "listed",
        );
        let ids: Vec<EntryId> = c.shown().entries().iter().map(|e| e.id).collect();
        let keepers = c.create("Keepers").unwrap();
        c.get_mut(keepers).unwrap().add(tracks(&["c"]));

        let before = c.shown().to_saved();
        // Three selected entries (a, c, d), one already in Keepers.
        assert_eq!(c.send(PLAYLIST, &[ids[3], ids[0], ids[2]], keepers), Ok(2));
        assert_eq!(names(c.get(keepers).unwrap()), ["c", "a", "d"]);
        assert_eq!(c.shown().to_saved(), before, "the source is unchanged");

        // The origin travels; the same clip isn't sent twice even from another entry.
        assert_eq!(c.send(PLAYLIST, &[remote], keepers), Ok(1));
        let copy = c.get(keepers).unwrap().entries().last().unwrap().clone();
        assert_eq!(copy.origin, Some(origin.clone()));
        assert_ne!(copy.id, remote, "fresh id");
        let again = c
            .shown_mut()
            .add_waiting("x", "y", None, Some(origin), "listed");
        assert_eq!(c.send(PLAYLIST, &[again], keepers), Ok(0));
        assert!(c.is_dirty());
    }

    #[test]
    fn an_origin_survives_a_send_and_a_restart() {
        let cfg = config("origin");
        let keepers = {
            let mut c = Crates::open(&cfg);
            let origin = Origin {
                release: Some(123456),
                catno: "LT-012".into(),
                position: "A1".into(),
                album: "Glasshouse EP".into(),
                cover: "https://i.discogs.com/x.jpeg".into(),
                ..Default::default()
            };
            let id =
                c.shown_mut()
                    .add_waiting("Nightcraft", "Glasshouse", None, Some(origin), "listed");
            let keepers = c.create("Keepers").unwrap();
            c.send(PLAYLIST, &[id], keepers).unwrap();
            c.save_due(true, Duration::ZERO);
            keepers
        };
        let mut c = Crates::open(&cfg);
        c.show(keepers);
        let o = c.shown().entries()[0].origin.clone().unwrap();
        assert_eq!(
            (o.release, o.catno.as_str(), o.position.as_str()),
            (Some(123456), "LT-012", "A1")
        );
        assert_eq!(
            (o.album.as_str(), o.cover.as_str()),
            ("Glasshouse EP", "https://i.discogs.com/x.jpeg")
        );
    }

    #[test]
    fn a_seller_crate_stays_the_sellers_and_is_grouped() {
        let cfg = config("seller");
        let id = {
            let mut c = Crates::open(&cfg);
            let id = c.create("Seller: decks.de").unwrap();
            assert_eq!(c.seller_of(id), None);
            c.set_seller(id, "decks.de");
            assert!(c.is_grouped(id), "seller crates are grouped by default");
            c.rename(id, "decks").unwrap();
            c.save_due(true, Duration::ZERO);
            id
        };
        let c = Crates::open(&cfg);
        assert_eq!(
            c.seller_of(id),
            Some("decks.de"),
            "after a rename and a restart"
        );
        assert!(c.is_grouped(id));
        assert_eq!(c.seller_of(PLAYLIST), None);
    }

    #[test]
    fn a_label_crate_stays_the_labels_is_grouped_and_locked() {
        let cfg = config("label");
        let id = {
            let mut c = Crates::open(&cfg);
            let id = c.create("Label: Siesta Records").unwrap();
            assert_eq!((c.label_of(id), c.find_label(77)), (None, None));
            assert!(!c.is_locked(id));
            c.set_label(id, 77);
            assert!(c.is_grouped(id), "label crates are grouped by default");
            assert!(c.is_locked(id), "a label crate takes no hand edits");
            c.rename(id, "Siesta").unwrap();
            c.save_due(true, Duration::ZERO);
            id
        };
        let c = Crates::open(&cfg);
        assert_eq!(c.label_of(id), Some(77), "after a rename and a restart");
        assert_eq!(c.find_label(77), Some(id));
        assert_eq!(c.labels().map(|i| i.id).collect::<Vec<_>>(), [id]);
        assert!(c.is_grouped(id) && c.is_locked(id));
        assert!(!c.is_locked(PLAYLIST));
    }

    #[test]
    fn a_bandcamp_label_crate_alone_or_beside_discogs() {
        let cfg = config("bandcamp-label");
        let (bc, both) = {
            let mut c = Crates::open(&cfg);
            let bc = c.create("Label: Lowtide Tapes").unwrap();
            c.set_bandcamp(bc, "lowtidetapes");
            let both = c.create("Label: Siesta Records").unwrap();
            c.set_label(both, 77);
            c.set_bandcamp(both, "siestarecords");
            c.save_due(true, Duration::ZERO);
            (bc, both)
        };
        let c = Crates::open(&cfg);
        assert!(c.is_label(bc) && c.is_locked(bc) && c.is_grouped(bc));
        assert_eq!(c.label_of(bc), None);
        assert_eq!(c.find_bandcamp("lowtidetapes"), Some(bc));
        assert_eq!(
            (c.label_of(both), c.bandcamp_of(both)),
            (Some(77), Some("siestarecords"))
        );
        assert_eq!(c.labels().count(), 2);
    }

    #[test]
    fn an_index_saved_before_bandcamp_loads_unchanged() {
        let old =
            r#"(id: 4, name: "Label: Siesta Records", entries: 3, created: 1, label: Some(77))"#;
        let info: CrateInfo = ron::from_str(old).unwrap();
        assert_eq!((info.label, info.bandcamp.as_deref()), (Some(77), None));
        assert!(info.is_label());
    }

    #[test]
    fn grouping_is_remembered_and_the_discogs_crates_default_to_it() {
        use crate::playlist::Origin;
        let cfg = config("grouped");
        let o = |r: u64, c: &str| Origin {
            release: Some(r),
            clip: Some(c.into()),
            ..Default::default()
        };
        let (friday, coll) = {
            let mut c = Crates::open(&cfg);
            let friday = c.create("Friday").unwrap();
            let coll = c.create("Collection: digger").unwrap();
            for id in [friday, coll] {
                let p = c.get_mut(id).unwrap();
                for (r, clip) in [(1, "a"), (2, "b"), (1, "c")] {
                    p.add_waiting("A", clip, None, Some(o(r, clip)), "listed");
                }
                c.touch(id);
            }
            assert!(!c.is_grouped(friday), "an ordinary crate is flat");
            c.set_collection(coll);
            assert!(c.is_grouped(coll), "the collection is grouped by default");
            let titles: Vec<String> = c
                .get(coll)
                .unwrap()
                .entries()
                .iter()
                .map(|e| e.title.clone())
                .collect();
            assert_eq!(titles, ["a", "c", "b"], "and gathered");
            c.set_grouped(friday, true);
            c.save_due(true, Duration::ZERO);
            (friday, coll)
        };
        let mut c = Crates::open(&cfg);
        assert!(c.is_grouped(friday) && c.is_grouped(coll), "remembered");
        c.load(friday);
        assert!(c.get(friday).unwrap().is_grouped());
        c.set_grouped(coll, false);
        assert!(!c.is_grouped(coll), "a choice beats the default");
        // An index written before grouping existed loads, every crate by its kind.
        let old: CrateInfo =
            ron::from_str(r#"(id: 5, name: "Old", entries: 0, created: 1)"#).unwrap();
        assert_eq!(old.grouped, None);
        assert!(!old.is_grouped());
    }

    #[test]
    fn one_crate_is_the_wantlist_and_the_flag_is_saved() {
        let cfg = config("wantlist-flag");
        let (a, b) = {
            let mut c = Crates::open(&cfg);
            let a = c.create("Wantlist").unwrap();
            let b = c.create("Wantlist: digger").unwrap();
            c.set_wantlist(a, true);
            c.set_wantlist(b, true);
            assert!(!c.is_wantlist(a) && c.is_wantlist(b), "only one is");
            assert!(c.info(b).unwrap().discogs());
            c.save_due(true, Duration::ZERO);
            (a, b)
        };
        let mut c = Crates::open(&cfg);
        assert!(c.is_wantlist(b) && !c.is_wantlist(a), "saved");
        c.set_wantlist(b, false);
        assert!(!c.is_wantlist(b));
        assert!(!c.info(b).unwrap().discogs());
    }

    #[test]
    fn collection_crates_are_flagged_and_known_by_name_once() {
        let cfg = config("collection-flag");
        let (old, flagged) = {
            let mut c = Crates::open(&cfg);
            // Made before the flag existed: only its name says what it is.
            let old = c.create("Collection: digger").unwrap();
            assert!(!c.is_collection(old));
            let flagged = c.create("Friday").unwrap();
            c.set_collection(flagged);
            c.save_due(true, Duration::ZERO);
            (old, flagged)
        };
        let c = Crates::open(&cfg);
        assert!(
            c.is_collection(old),
            "known by its name when the crates load"
        );
        assert!(c.is_collection(flagged), "the flag is saved");
    }
}
