//! Record covers on record rows and in the record card, UI side: textures for the covers seen
//! lately (thumbnails, and the card's large covers apart), and when to ask the cover worker for
//! one. The worker reads and writes the files and talks to the image host; this side only asks
//! (for a record row the pointer rests on, its thumbnail and large cover, and for the record
//! rows in view, their thumbnails) and draws.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use dig::cover::{CoverHandle, CoverKey, CoverResult, allowed, bandcamp_art, bandcamp_large};
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
/// Large covers kept as textures (each up to 600 px square, ~1.4 MB), least recently drawn
/// dropped first.
pub const KEEP_LARGE: usize = 16;

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
    /// Large covers: textures, the one asked for and not back yet, and those with none this
    /// session.
    large: HashMap<CoverKey, (TextureHandle, u64)>,
    large_asked: HashSet<CoverKey>,
    large_failed: HashSet<CoverKey>,
    /// Each cover's colour (the record card's label), from whichever of its covers came first,
    /// so the large cover arriving doesn't change it. Kept when its texture is dropped: three
    /// bytes a record.
    tints: HashMap<CoverKey, [u8; 3]>,
}

/// A record card's two covers: its thumbnail and its large cover (`None`: none to show).
pub(super) type CardSlots = (Option<CoverSlot>, Option<CoverSlot>);

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

/// The record's large cover address: the one its data gave, else (Bandcamp) its art at 700 px.
pub(super) fn large_of(e: &Entry) -> Option<(CoverKey, String)> {
    let (key, thumb) = cover_of(e)?;
    let o = e.origin.as_ref()?;
    let url = match key {
        _ if !o.cover_large.is_empty() => o.cover_large.clone(),
        CoverKey::BandcampArt(_) => bandcamp_large(thumb)?,
        CoverKey::Record(_) => return None,
    };
    allowed(&url).then_some((key, url))
}

impl CoverCache {
    /// The covers for the record card of the hovered record row whose first entry is `e`,
    /// asking the worker for those not loaded yet once the pointer has rested on it: the
    /// thumbnail first, then the large cover.
    pub(super) fn card_slot(&mut self, ctx: &egui::Context, e: &Entry) -> CardSlots {
        let Some((key, url)) = cover_of(e) else {
            return (None, None);
        };
        self.tick += 1;
        let tick = self.tick;
        let thumb = if self.failed.contains(&key) {
            None
        } else if self.refreshing.contains(&key) {
            Some(CoverSlot::Waiting)
        } else if let Some((tex, used)) = self.textures.get_mut(&key) {
            *used = tick;
            Some(CoverSlot::Loaded(tex.id(), tex.size_vec2()))
        } else {
            Some(CoverSlot::Waiting)
        };
        let large_url = large_of(e)
            .map(|(_, u)| u)
            .filter(|_| !self.large_failed.contains(&key));
        let large = match (&large_url, self.large.get_mut(&key)) {
            (_, Some((tex, used))) => {
                *used = tick;
                Some(CoverSlot::Loaded(tex.id(), tex.size_vec2()))
            }
            (Some(_), None) => Some(CoverSlot::Waiting),
            (None, None) => None,
        };
        let want_thumb = thumb == Some(CoverSlot::Waiting)
            && !self.refreshing.contains(&key)
            && !self.asked.contains(&key);
        let want_large = large == Some(CoverSlot::Waiting) && !self.large_asked.contains(&key);
        if !want_thumb && !want_large {
            return (thumb, large);
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
        } else if let Some(h) = &self.handle {
            h.request_record(
                key,
                want_thumb.then_some(url),
                large_url.as_deref().filter(|_| want_large),
            );
            // Only the newest request waits: the ones it replaced may be asked for again.
            self.asked.clear();
            self.large_asked.clear();
            if want_thumb {
                self.asked.insert(key);
            }
            if want_large {
                self.large_asked.insert(key);
            }
        }
        (thumb, large)
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
                    self.keep_tint(key, c.tint);
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
                CoverResult::LargeReady(key, c) => {
                    self.large_asked.remove(&key);
                    self.keep_tint(key, c.tint);
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [c.width as usize, c.height as usize],
                        &c.rgba,
                    );
                    let tex = ctx.load_texture(
                        format!("cover-large-{key:?}"),
                        img,
                        egui::TextureOptions::LINEAR,
                    );
                    self.insert_large(key, tex);
                }
                CoverResult::LargeFailed(key) => {
                    self.large_asked.remove(&key);
                    self.large_failed.insert(key);
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

    /// The colour of the cover of `e`'s record, once one of its covers has been read.
    pub(super) fn tint(&self, e: &Entry) -> Option<[u8; 3]> {
        let (key, _) = cover_of(e)?;
        self.tints.get(&key).copied()
    }

    fn keep_tint(&mut self, key: CoverKey, tint: [u8; 3]) {
        self.tints.entry(key).or_insert(tint);
    }

    fn evict(&mut self) {
        evict(&mut self.textures, KEEP);
    }

    fn insert_large(&mut self, key: CoverKey, tex: TextureHandle) {
        self.tick += 1;
        self.large.insert(key, (tex, self.tick));
        evict(&mut self.large, KEEP_LARGE);
    }

    #[cfg(test)]
    pub(super) fn has_texture(&self, key: RecordKey) -> bool {
        self.textures.contains_key(&CoverKey::Record(key))
    }

    #[cfg(test)]
    pub(super) fn has_large(&self, key: RecordKey) -> bool {
        self.large.contains_key(&CoverKey::Record(key))
    }
}

/// Drops the least recently drawn textures past `keep`.
fn evict(textures: &mut HashMap<CoverKey, (TextureHandle, u64)>, keep: usize) {
    while textures.len() > keep {
        let Some(oldest) = textures
            .iter()
            .min_by_key(|(_, (_, used))| *used)
            .map(|(k, _)| *k)
        else {
            return;
        };
        textures.remove(&oldest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::{Origin, Playlist};

    fn entry(origin: Origin) -> Entry {
        let mut p = Playlist::default();
        let id = p.add_waiting("N", "T", None, Some(origin), "queued");
        p.get(id).unwrap().clone()
    }

    #[test]
    fn a_large_address_comes_from_the_data_or_from_bandcamp_art() {
        let discogs = entry(Origin {
            release: Some(1),
            cover: "https://i.discogs.com/a/150.jpeg".into(),
            cover_large: "https://i.discogs.com/a/full.jpeg".into(),
            ..Default::default()
        });
        assert_eq!(
            large_of(&discogs).map(|(_, u)| u).as_deref(),
            Some("https://i.discogs.com/a/full.jpeg")
        );
        let listing = entry(Origin {
            release: Some(2),
            cover: "https://i.discogs.com/b/150.jpeg".into(),
            ..Default::default()
        });
        assert_eq!(large_of(&listing), None, "a seller listing gives none");
        let bandcamp = entry(Origin {
            cover: "https://f4.bcbits.com/img/a0161524395_7.jpg".into(),
            ..Default::default()
        });
        assert_eq!(
            large_of(&bandcamp).map(|(_, u)| u).as_deref(),
            Some("https://f4.bcbits.com/img/a0161524395_16.jpg")
        );
    }

    #[test]
    fn no_large_cover_is_asked_for_before_the_pointer_rests() {
        let ctx = egui::Context::default();
        let mut c = CoverCache::default();
        let e = entry(Origin {
            release: Some(1),
            cover: "https://i.discogs.com/a/150.jpeg".into(),
            cover_large: "https://i.discogs.com/a/full.jpeg".into(),
            ..Default::default()
        });
        assert_eq!(
            c.card_slot(&ctx, &e),
            (Some(CoverSlot::Waiting), Some(CoverSlot::Waiting))
        );
        assert!(c.large_asked.is_empty() && c.asked.is_empty());
        let none = entry(Origin {
            release: Some(2),
            ..Default::default()
        });
        assert_eq!(c.card_slot(&ctx, &none), (None, None), "no image: the icon");
    }

    #[test]
    fn a_cover_keeps_the_tint_of_whichever_came_first() {
        let mut c = CoverCache::default();
        let e = entry(Origin {
            release: Some(7),
            cover: "https://i.discogs.com/a/150.jpeg".into(),
            ..Default::default()
        });
        assert_eq!(c.tint(&e), None, "nothing read yet");
        let key = CoverKey::Record(RecordKey::Release(7));
        c.keep_tint(key, [200, 40, 40]);
        c.keep_tint(key, [190, 50, 45]);
        assert_eq!(
            c.tint(&e),
            Some([200, 40, 40]),
            "the large cover doesn't retint it"
        );
    }

    #[test]
    fn the_seventeenth_large_cover_drops_the_least_recently_drawn() {
        let ctx = egui::Context::default();
        let mut c = CoverCache::default();
        for id in 0..=KEEP_LARGE as u64 {
            let tex = ctx.load_texture(
                format!("t{id}"),
                egui::ColorImage::filled([1, 1], egui::Color32::WHITE),
                egui::TextureOptions::LINEAR,
            );
            c.insert_large(CoverKey::Record(RecordKey::Release(id)), tex);
        }
        assert_eq!(c.large.len(), KEEP_LARGE);
        assert!(!c.has_large(RecordKey::Release(0)), "the oldest went");
        assert!(c.has_large(RecordKey::Release(KEEP_LARGE as u64)));
    }
}
