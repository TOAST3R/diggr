//! Records with no preview anywhere. A track's sources are tried one after the other and near
//! the playhead (its clip or a YouTube search, the record's album upload, Bandcamp), so a
//! record's outcome arrives over minutes. A record counts as fully tried once none of its
//! tracks waits for anything and no Bandcamp look is pending for one; then, with no playable
//! track it is not found, with some it is partly found.
//!
//! A not-found record is told in one blocking notice, which gathers the records found empty
//! while it is open, each at most once a session unless retried. Only outcomes raise it: a
//! record that was already empty when its crate loaded doesn't. Partly found records only say
//! "2/5 previews" on their row (see `record_row`).

use std::collections::HashSet;

use dig::buy::Shop;

use super::DiggrApp;
use crate::crates::CrateId;
use crate::playlist::{AlbumKey, Entry, EntryId, EntryStatus, UnavailableKind};

/// The not-found part of the dig state.
#[derive(Default)]
pub(super) struct NotFoundState {
    /// Records an outcome touched since the last frame.
    touched: HashSet<(CrateId, AlbumKey)>,
    /// Records told this session (until retried).
    told: HashSet<(CrateId, AlbumKey)>,
    /// The notice's records, while it is open.
    pub(super) notice: Vec<Gone>,
}

/// A record in the notice.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Gone {
    crate_id: CrateId,
    key: AlbumKey,
    /// "Artist – Title (2026)".
    name: String,
    year: Option<u16>,
    /// "YouTube, Bandcamp".
    tried: String,
    /// The other store to search it on by hand (see `cross_search`).
    search: Option<(Shop, String)>,
}

/// Where else to look for an entry's record by hand: Bandcamp for a Discogs record, Discogs
/// for a Bandcamp one, as that store's search for its artist and title.
pub(super) fn cross_search(e: &Entry) -> Option<(Shop, String)> {
    let o = e.origin.as_ref()?;
    let shop = if o.release.is_some() || o.master.is_some() {
        Shop::Bandcamp
    } else if !o.bandcamp.is_empty() {
        Shop::Discogs
    } else {
        return None;
    };
    let title = match e.album() {
        "" => e.title.as_str(),
        a => a,
    };
    let q = dig::buy::query(&o.artist, &e.artist, title);
    Some((shop, shop.search_url(&q)))
}

/// How far a record's tracks got, once every source was tried: (playable, tracks).
pub(super) type Tried = (usize, usize);

/// The year of a Unix time (UTC), by Howard Hinnant's civil-from-days.
fn year_of(secs: u64) -> i64 {
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    yoe + era * 400 + i64::from(mp >= 10)
}

/// What the notice says of a record of `year`, `now` being the current Unix time.
pub(super) fn advice(year: Option<u16>, now: u64) -> &'static str {
    match year {
        Some(y) if i64::from(y) >= year_of(now) => {
            "Probably not released yet: try again after its release"
        }
        _ => "No preview anywhere yet",
    }
}

impl DiggrApp {
    /// Outcomes for these entries arrived: their records are checked at the end of the frame.
    pub(super) fn not_found_touch(&mut self, hits: &[(CrateId, EntryId)]) {
        let keys: Vec<(CrateId, AlbumKey)> = hits
            .iter()
            .filter_map(|&(c, id)| {
                let key = self.crates.get(c)?.get(id)?.album_key()?;
                Some((c, key))
            })
            .collect();
        if let Some(d) = &mut self.dig {
            d.not_found.touched.extend(keys);
        }
    }

    /// The tracks of record `key` in crate `c` that count: not passed, not doubles of another
    /// entry of the crate.
    fn record_tracks(&self, c: CrateId, key: &AlbumKey) -> Vec<&Entry> {
        let Some(p) = self.crates.get(c) else {
            return Vec::new();
        };
        let d = self.dig.as_ref();
        p.entries()
            .iter()
            .filter(|e| e.album_key().as_ref() == Some(key))
            .filter(|e| e.status != EntryStatus::Unavailable(UnavailableKind::AlreadyInCrate))
            .filter(|e| !d.is_some_and(|d| d.marks(e).passed))
            .collect()
    }

    /// (playable, tracks) of record `key` once every source was tried for its tracks; None
    /// while one still waits for its audio, a search or a Bandcamp look.
    pub(super) fn record_tried(&self, c: CrateId, key: &AlbumKey) -> Option<Tried> {
        let tracks = self.record_tracks(c, key);
        let mut playable = 0;
        for e in &tracks {
            match &e.status {
                s if s.is_playable() => playable += 1,
                EntryStatus::Waiting(_) => return None,
                _ if self.bandcamp_pending(c, e.id) => return None,
                _ => {}
            }
        }
        (!tracks.is_empty()).then_some((playable, tracks.len()))
    }

    /// Each frame: the records outcomes touched that are now fully tried with nothing to play
    /// join the notice, once.
    pub(super) fn not_found_check(&mut self) {
        let Some(d) = &mut self.dig else { return };
        if d.not_found.touched.is_empty() {
            return;
        }
        let touched = std::mem::take(&mut d.not_found.touched);
        let mut gone = Vec::new();
        for (c, key) in touched {
            if self
                .dig
                .as_ref()
                .is_some_and(|d| d.not_found.told.contains(&(c, key.clone())))
            {
                continue;
            }
            if let Some((0, _)) = self.record_tried(c, &key)
                && let Some(g) = self.gone(c, key)
            {
                gone.push(g);
            }
        }
        let Some(d) = &mut self.dig else { return };
        for g in gone {
            d.not_found.told.insert((g.crate_id, g.key.clone()));
            d.not_found.notice.push(g);
        }
    }

    fn gone(&self, c: CrateId, key: AlbumKey) -> Option<Gone> {
        let tracks = self.record_tracks(c, &key);
        let first = tracks.first()?;
        let o = first.origin.as_ref();
        let artist = match o.map(|o| o.artist.as_str()) {
            Some(a) if !a.is_empty() => a,
            _ => first.artist.as_str(),
        };
        let title = match first.album() {
            "" => first.title.as_str(),
            a => a,
        };
        let year = o.and_then(|o| o.year);
        let name = match year {
            Some(y) => format!("{artist} – {title} ({y})"),
            None => format!("{artist} – {title}"),
        };
        let mut tried = vec!["YouTube"];
        if tracks.iter().any(|e| self.bandcamp_tried(c, e.id)) {
            tried.push("Bandcamp");
        }
        let store = |e: &&&Entry| {
            e.origin.as_ref().is_some_and(|o| {
                !o.juno.is_empty() || !o.beatport.is_empty() || !o.bleep.is_empty()
            })
        };
        if tracks.iter().any(|e| store(&e)) {
            tried.push("the store's samples");
        }
        Some(Gone {
            crate_id: c,
            key,
            name,
            year,
            tried: tried.join(", "),
            search: cross_search(first),
        })
    }

    /// Retry: every unavailable track of the record goes back to its download or search
    /// (and may be looked for on Bandcamp again); the record may be told again.
    pub(super) fn not_found_retry(&mut self, c: CrateId, key: &AlbumKey) {
        let ids: Vec<EntryId> = self.record_tracks(c, key).iter().map(|e| e.id).collect();
        for id in ids {
            self.dig_retry_entry(c, id);
        }
        if let Some(d) = &mut self.dig {
            d.not_found.told.remove(&(c, key.clone()));
            d.not_found
                .notice
                .retain(|g| !(g.crate_id == c && g.key == *key));
        }
    }

    /// "No preview found": the records found empty, each with Retry; it holds the keyboard
    /// (see `dig_asking`) until Close, Esc or Enter. Playback goes on.
    pub(super) fn not_found_ui(&mut self, ctx: &egui::Context) {
        let Some(notice) = self
            .dig
            .as_ref()
            .map(|d| d.not_found.notice.clone())
            .filter(|n| !n.is_empty())
        else {
            return;
        };
        let now = dig::now_secs();
        let (mut retry, mut retry_all, mut close) = (None, false, false);
        let mut open = None;
        let modal = egui::Modal::new(egui::Id::new("not-found")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.strong("No preview found");
            ui.label("Previews often appear once a record is out.");
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| {
                    for (i, g) in notice.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(&g.name);
                                ui.small(format!("Tried {}. {}", g.tried, advice(g.year, now)));
                            });
                            if ui.button("Retry").clicked() {
                                retry = Some(i);
                            }
                            if let Some((shop, url)) = &g.search
                                && ui
                                    .link(format!("Search on {}", shop.name()))
                                    .on_hover_text(format!("Open {}'s search for it", shop.name()))
                                    .clicked()
                            {
                                open = Some(url.clone());
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                retry_all = ui.button("Retry all").clicked();
                close = ui.button("Close").clicked();
            });
        });
        close |= modal.should_close()
            || ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape));
        let retried: Vec<&Gone> = match (retry, retry_all) {
            (_, true) => notice.iter().collect(),
            (Some(i), _) => notice.get(i).into_iter().collect(),
            _ => Vec::new(),
        };
        for g in retried {
            self.not_found_retry(g.crate_id, &g.key);
        }
        if let Some(url) = open
            && let Some(d) = &self.dig
            && let Err(err) = d.open_url(&url)
        {
            self.notify(format!("Could not open the browser: {err}"));
        }
        if (close || retry_all)
            && let Some(d) = &mut self.dig
        {
            d.not_found.notice.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_is_searched_on_the_other_store() {
        use crate::playlist::{Origin, Playlist};
        let origin = Origin {
            bandcamp: "https://moodymann.bandcamp.com/track/shades-of-jae".into(),
            album: "Silentintroduction".into(),
            ..Default::default()
        };
        let mut p = Playlist::default();
        let id = p.add_waiting("Moodymann", "Shades of Jae", None, Some(origin), "listed");
        let mut e = p.get(id).unwrap().clone();
        assert_eq!(
            cross_search(&e),
            Some((
                Shop::Discogs,
                "https://www.discogs.com/search/?q=Moodymann%20Silentintroduction&type=release"
                    .to_owned()
            ))
        );
        e.origin.as_mut().unwrap().release = Some(1001);
        assert_eq!(cross_search(&e).unwrap().0, Shop::Bandcamp);
        e.origin = None;
        assert_eq!(cross_search(&e), None, "a local file");
    }

    #[test]
    fn the_year_of_a_time() {
        assert_eq!(year_of(0), 1970);
        // 2026-10-14 12:00 UTC, and the last second of 2025.
        assert_eq!(year_of(1_791_979_200), 2026);
        assert_eq!(year_of(1_767_225_599), 2025);
    }

    #[test]
    fn a_record_of_this_year_or_later_is_probably_not_out() {
        let now = 1_791_979_200; // 2026
        let soon = "Probably not released yet: try again after its release";
        assert_eq!(advice(Some(2026), now), soon);
        assert_eq!(advice(Some(2027), now), soon);
        assert_eq!(advice(Some(1998), now), "No preview anywhere yet");
        assert_eq!(advice(None, now), "No preview anywhere yet");
    }
}
