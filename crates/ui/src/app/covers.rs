//! Record covers in the entry tooltip and on record rows, UI side: textures for the covers
//! seen lately, and when to ask the cover worker for one. The worker reads and writes the files
//! and talks to the image host; this side only asks (for a row the pointer rests on, and for
//! the record rows in view) and draws.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use dig::cover::{CoverHandle, CoverKey, CoverResult, bandcamp_art};
use dig::discogs::model::RecordKey;
use egui::TextureHandle;

use super::CoverSlot;
use crate::playlist::{Entry, EntryId};

/// A cover is asked for once the pointer has rested on its row this long.
pub const REST: Duration = Duration::from_millis(250);
/// Covers kept as textures; the least recently drawn are dropped (and read from disk again
/// when needed). A screenful of record rows and some scrolling back fit.
pub const KEEP: usize = 128;
/// Record rows in view asked for at a time.
pub const IN_VIEW: usize = 40;

#[derive(Default)]
pub(super) struct CoverCache {
    pub(super) handle: Option<CoverHandle>,
    textures: HashMap<CoverKey, (TextureHandle, u64)>,
    tick: u64,
    /// Asked for and not back yet.
    asked: HashSet<CoverKey>,
    /// No cover this session.
    failed: HashSet<CoverKey>,
    /// Looked up again after its address stopped working (only once).
    refreshed: HashSet<CoverKey>,
    /// Waiting for that lookup: not asked for again meanwhile.
    refreshing: HashSet<CoverKey>,
    /// The row the pointer rests on, and since when.
    hover: Option<(EntryId, Instant)>,
    /// The covers of the record rows in view last asked for.
    in_view: Vec<CoverKey>,
}

/// The record whose cover an entry shows, and the cover's address: its Discogs release or
/// master, else its Bandcamp album's art.
pub(super) fn cover_of(e: &Entry) -> Option<(CoverKey, &str)> {
    let o = e.origin.as_ref().filter(|o| !o.cover.is_empty())?;
    let key = match (o.release, o.master) {
        (Some(r), _) => CoverKey::Record(RecordKey::Release(r)),
        (None, Some(m)) => CoverKey::Record(RecordKey::Master(m)),
        _ => CoverKey::BandcampArt(bandcamp_art(&o.cover)?),
    };
    Some((key, o.cover.as_str()))
}

impl CoverCache {
    /// The cover slot for the hovered entry `e`, asking the worker for its cover once the
    /// pointer has rested on it. `None`: the entry shows no cover.
    pub(super) fn slot(&mut self, ctx: &egui::Context, e: &Entry) -> Option<CoverSlot> {
        let (key, url) = cover_of(e)?;
        if self.failed.contains(&key) {
            return None;
        }
        if self.refreshing.contains(&key) {
            return Some(CoverSlot::Waiting);
        }
        self.tick += 1;
        if let Some((tex, used)) = self.textures.get_mut(&key) {
            *used = self.tick;
            return Some(CoverSlot::Loaded(tex.id(), tex.size_vec2()));
        }
        let now = Instant::now();
        let since = match self.hover {
            Some((id, t)) if id == e.id => t,
            _ => {
                self.hover = Some((e.id, now));
                now
            }
        };
        let rested = now - since;
        if rested < REST {
            ctx.request_repaint_after(REST - rested);
        } else if !self.asked.contains(&key)
            && let Some(h) = &self.handle
        {
            h.request(key, url);
            // Only the newest request waits: the ones it replaced may be asked for again.
            self.asked.clear();
            self.asked.insert(key);
        }
        Some(CoverSlot::Waiting)
    }

    /// The cover for a record row (no waiting for the pointer to rest): drawn, waiting, or
    /// `None` when the record has none.
    pub(super) fn row_slot(&mut self, e: &Entry) -> Option<CoverSlot> {
        let (key, _) = cover_of(e)?;
        if self.failed.contains(&key) {
            return None;
        }
        self.tick += 1;
        match self.textures.get_mut(&key) {
            Some((tex, used)) => {
                *used = self.tick;
                Some(CoverSlot::Loaded(tex.id(), tex.size_vec2()))
            }
            None => Some(CoverSlot::Waiting),
        }
    }

    /// The record rows in view, top first: their covers not loaded yet are asked for (at
    /// most [`IN_VIEW`]), replacing what was still waiting, when the rows in view change.
    pub(super) fn want_in_view(&mut self, entries: &[&Entry]) {
        let list: Vec<(CoverKey, String)> = entries
            .iter()
            .filter_map(|e| cover_of(e))
            .filter(|(k, _)| {
                !self.textures.contains_key(k)
                    && !self.failed.contains(k)
                    && !self.refreshing.contains(k)
            })
            .take(IN_VIEW)
            .map(|(k, url)| (k, url.to_owned()))
            .collect();
        let keys: Vec<CoverKey> = list.iter().map(|(k, _)| *k).collect();
        if keys == self.in_view {
            return;
        }
        self.in_view = keys;
        if let Some(h) = &self.handle {
            self.asked.extend(self.in_view.iter().copied());
            h.want(list);
        }
    }

    /// Results from the worker. Returns the records whose address stopped working and should
    /// be looked up again (once each); a Bandcamp cover has no lookup, so it has none.
    pub(super) fn poll(&mut self, ctx: &egui::Context) -> Vec<RecordKey> {
        let Some(h) = &self.handle else {
            return Vec::new();
        };
        let mut refresh = Vec::new();
        for r in h.poll() {
            match r {
                CoverResult::Ready(key, c) => {
                    self.asked.remove(&key);
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [c.width as usize, c.height as usize],
                        &c.rgba,
                    );
                    let tex = ctx.load_texture(
                        format!("cover-{key:?}"),
                        img,
                        egui::TextureOptions::LINEAR,
                    );
                    self.tick += 1;
                    self.textures.insert(key, (tex, self.tick));
                    self.evict();
                }
                CoverResult::Failed(key) => {
                    self.asked.remove(&key);
                    self.failed.insert(key);
                }
                CoverResult::Stale(key) => {
                    self.asked.remove(&key);
                    if let CoverKey::Record(record) = key
                        && self.refreshed.insert(key)
                    {
                        self.refreshing.insert(key);
                        refresh.push(record);
                    } else {
                        self.failed.insert(key);
                    }
                }
            }
        }
        refresh
    }

    /// A record's address after looking it up again: an unchanged or missing one means no
    /// cover; a new one is asked for the next time its row is hovered.
    pub(super) fn refreshed(&mut self, key: RecordKey, changed: bool) {
        let key = CoverKey::Record(key);
        self.refreshing.remove(&key);
        if !changed {
            self.failed.insert(key);
        }
    }

    fn evict(&mut self) {
        while self.textures.len() > KEEP {
            let Some(oldest) = self
                .textures
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(k, _)| *k)
            else {
                return;
            };
            self.textures.remove(&oldest);
        }
    }

    #[cfg(test)]
    pub(super) fn has_texture(&self, key: RecordKey) -> bool {
        self.textures.contains_key(&CoverKey::Record(key))
    }
}
