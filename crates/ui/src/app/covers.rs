//! Record covers in the entry tooltip, UI side: textures for the covers seen lately, and when
//! to ask the cover worker for one. The worker reads and writes the files and talks to the
//! image host; this side only asks, after the pointer has rested on a row, and draws.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use dig::cover::{CoverHandle, CoverResult};
use dig::discogs::model::RecordKey;
use egui::TextureHandle;

use super::CoverSlot;
use crate::playlist::{Entry, EntryId};

/// A cover is asked for once the pointer has rested on its row this long.
pub const REST: Duration = Duration::from_millis(250);
/// Covers kept as textures; older ones are dropped (and read from disk again when needed).
pub const KEEP: usize = 48;

#[derive(Default)]
pub(super) struct CoverCache {
    pub(super) handle: Option<CoverHandle>,
    textures: HashMap<RecordKey, (TextureHandle, u64)>,
    tick: u64,
    /// Asked for and not back yet.
    asked: HashSet<RecordKey>,
    /// No cover this session.
    failed: HashSet<RecordKey>,
    /// Looked up again after its address stopped working (only once).
    refreshed: HashSet<RecordKey>,
    /// Waiting for that lookup: not asked for again meanwhile.
    refreshing: HashSet<RecordKey>,
    /// The row the pointer rests on, and since when.
    hover: Option<(EntryId, Instant)>,
}

/// The record whose cover an entry shows, and the cover's address.
pub(super) fn cover_of(e: &Entry) -> Option<(RecordKey, &str)> {
    let o = e.origin.as_ref().filter(|o| !o.cover.is_empty())?;
    let key = match (o.release, o.master) {
        (Some(r), _) => RecordKey::Release(r),
        (None, Some(m)) => RecordKey::Master(m),
        _ => return None,
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

    /// Results from the worker. Returns the records whose address stopped working and should
    /// be looked up again (once each).
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
                    if self.refreshed.insert(key) {
                        self.refreshing.insert(key);
                        refresh.push(key);
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
        self.textures.contains_key(&key)
    }
}
