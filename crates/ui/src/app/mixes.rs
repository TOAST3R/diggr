//! Mixes in the player: a YouTube or SoundCloud mix pasted (read with yt-dlp, see
//! `dig::tracklist`), or a 1001tracklists or Mixcloud tracklist sent by the browser extension,
//! becomes a crate "Mix: ‹title›" in mix order. Each track waits to be searched for, as a
//! Discogs track without a clip does; an unidentified one ("ID") stays, unavailable, so the
//! crate keeps the mix's shape. Each entry keeps the mix's address and its time in it, for
//! Open in mix. The same mix again adds only the tracks the crate lacks.

use dig::bandcamp::fan::DiscogsMatch;
use dig::bridge::Mode;
use dig::discogs::client::ApiError;
use dig::discogs::search::Wanted;
use dig::intake::Command;
use dig::preview::scheduler::PreviewCommand;
use dig::tracklist::{MixError, MixSource, NotMix, Tracklist};
use std::collections::HashSet;

use super::DiggrApp;
use super::bandcamp::match_key;
use crate::crates::CrateId;
use crate::playlist::{Entry, EntryId, Origin};

/// A mix track's Discogs lookup is keyed `mix:‹crate›:‹entry›`.
fn find_key(c: CrateId, id: EntryId) -> String {
    format!("mix:{c}:{id}")
}

fn parse_find(key: &str) -> Option<(CrateId, EntryId)> {
    let (c, id) = key.strip_prefix("mix:")?.split_once(':')?;
    Some((c.parse().ok()?, id.parse().ok()?))
}

/// The reason an unidentified track can't play.
pub(super) const UNIDENTIFIED: &str = "unidentified in the mix";

/// "Mix: ‹title›", cut to a crate name's 40 characters.
fn crate_name(title: &str) -> String {
    let name = format!("Mix: {}", title.trim());
    name.chars()
        .take(40)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// "12:34", or "1:02:03" past an hour.
fn clock(secs: u32) -> String {
    match secs / 3600 {
        0 => format!("{}:{:02}", secs / 60, secs % 60),
        h => format!("{h}:{:02}:{:02}", secs / 60 % 60, secs % 60),
    }
}

impl DiggrApp {
    /// A pasted address that is a mix: read (YouTube, SoundCloud) or refused with where to
    /// send it from (Mixcloud, 1001tracklists). False when it isn't a mix at all.
    pub(super) fn mix_paste(&mut self, text: &str) -> bool {
        match dig::tracklist::parse_url(text) {
            Ok(source) => {
                if let Some(d) = &self.dig {
                    d.preview(PreviewCommand::ReadMix(source));
                }
                self.notify("Reading the mix's tracklist…");
                true
            }
            Err(NotMix::ExtensionOnly(site)) => {
                self.notify(format!(
                    "{site} tracklists are sent from the browser extension"
                ));
                true
            }
            Err(NotMix::Other) => false,
        }
    }

    /// A mix's page sent from the browser: read, then filled as the send's mode says.
    pub(super) fn mix_ask(&mut self, source: MixSource, mode: Mode) {
        let Some(d) = &mut self.dig else { return };
        d.mix_modes.insert(source.clone(), mode);
        d.preview(PreviewCommand::ReadMix(source));
        self.notify("Reading the mix's tracklist…");
    }

    /// A mix was read: its crate fills (shown when pasted, as the mode says when sent from the
    /// browser), or the main window says why not.
    pub(super) fn mix_read(&mut self, source: MixSource, result: Result<Tracklist, MixError>) {
        let sent = self.dig.as_mut().and_then(|d| d.mix_modes.remove(&source));
        match result {
            Ok(list) => match sent {
                Some(mode) => self.mix_send(list, mode),
                None => {
                    if let Some(c) = self.mix_fill(&list, None) {
                        self.show_crate(c);
                    }
                }
            },
            Err(MixError::NoTracklist(title)) => {
                self.notify(format!("No tracklist found in {title}"))
            }
            Err(MixError::Failed(why)) => self.notify(format!("Couldn't read the mix: {why}")),
        }
    }

    /// The crate already holding mix `url`, if one does.
    fn mix_crate(&self, url: &str) -> Option<CrateId> {
        self.crates.loaded_ids().into_iter().find(|&c| {
            self.crates.get(c).is_some_and(|p| {
                p.entries()
                    .iter()
                    .any(|e| e.origin.as_ref().is_some_and(|o| o.page == url))
            })
        })
    }

    /// A tracklist from the browser extension: into its Mix crate, or the crate the send
    /// names; Play shows it and starts its first identified track (one not found hands over
    /// to the next).
    pub(super) fn mix_send(&mut self, list: Tracklist, mode: Mode) {
        let into = match &mode {
            Mode::Crate(name) => self
                .crates
                .loaded_ids()
                .into_iter()
                .find(|&c| self.crates.name(c) == name)
                .or_else(|| self.crates.create(name).ok()),
            _ => None,
        };
        let Some(c) = self.mix_fill(&list, into) else {
            return;
        };
        if mode == Mode::Play {
            self.show_crate(c);
            let first = self.crates.get(c).and_then(|p| {
                p.entries()
                    .iter()
                    .find(|e| {
                        e.status.in_play_order()
                            && e.origin
                                .as_ref()
                                .is_some_and(|o| o.page == list.source.url())
                    })
                    .map(|e| e.id)
            });
            if let Some(id) = first {
                self.play_entry(c, id);
            }
        }
    }

    /// Puts tracklist `list` in crate `into`, else its Mix crate (made, or found holding the
    /// mix): only the tracks it lacks, in mix order. Returns the crate.
    pub(super) fn mix_fill(&mut self, list: &Tracklist, into: Option<CrateId>) -> Option<CrateId> {
        let url = list.source.url();
        let c = match into.or_else(|| self.mix_crate(&url)) {
            Some(c) => c,
            None => match self.crates.create(&crate_name(&list.title)) {
                Ok(c) => c,
                Err(why) => {
                    self.notify(why);
                    return None;
                }
            },
        };
        let p = self.crates.get_mut(c)?;
        let key = |artist: &str, title: &str| match_key(artist, title);
        let mut have: HashSet<String> = p
            .entries()
            .iter()
            .filter(|e| e.origin.as_ref().is_some_and(|o| o.page == url))
            .map(|e| key(&e.artist, &e.title))
            .collect();
        let ids: HashSet<Option<u32>> = p
            .entries()
            .iter()
            .filter(|e| e.status.note().as_deref() == Some(UNIDENTIFIED))
            .map(|e| e.origin.as_ref().and_then(|o| o.mix_time))
            .collect();
        let mut added = 0;
        for t in &list.tracks {
            let title = match t.version.as_str() {
                "" => t.title.clone(),
                v => format!("{} ({v})", t.title),
            };
            if t.is_id {
                if ids.contains(&t.time) {
                    continue;
                }
            } else if !have.insert(key(&t.artist, &title)) {
                continue;
            }
            let origin = Origin {
                page: url.clone(),
                label: t.label.clone(),
                mix_time: t.time,
                search_key: if t.is_id {
                    String::new()
                } else {
                    dig::discogs::matching::track_key(&t.artist, &title)
                },
                ..Origin::default()
            };
            let (artist, shown) = match t.is_id {
                true => ("ID".to_owned(), "ID".to_owned()),
                false => (t.artist.clone(), title),
            };
            let id = p.add_waiting(artist, shown, None, Some(origin), "to search");
            if t.is_id {
                p.set_unavailable(id, UNIDENTIFIED);
            }
            added += 1;
        }
        self.crates.touch(c);
        let name = self.crates.name(c).to_owned();
        self.notify(match added {
            0 => format!("{name}: nothing new"),
            1 => format!("{name}: 1 track"),
            n => format!("{name}: {n} tracks"),
        });
        Some(c)
    }

    /// Whether entry `e` is a mix's track that Find on Discogs can look up: identified, and
    /// not on a Discogs record yet.
    pub(super) fn mix_findable(e: &Entry) -> bool {
        e.origin.as_ref().is_some_and(|o| {
            MixSource::from_url(&o.page).is_some()
                && !o.search_key.is_empty()
                && o.release.is_none()
                && o.master.is_none()
        })
    }

    /// Find on Discogs, for a mix track: one search by artist and track (a token is needed).
    pub(super) fn mix_find(&mut self, c: CrateId, id: EntryId) {
        let Some(e) = self.crates.get(c).and_then(|p| p.get(id)).cloned() else {
            return;
        };
        let Some(d) = &mut self.dig else { return };
        if d.token.is_none() {
            return self
                .notify("Add a Discogs token (Options › Discogs…) to find tracks on Discogs");
        }
        d.send_cmd(Command::FindReleases(vec![(
            find_key(c, id),
            Wanted::track(&e.artist, &e.title),
        )]));
        self.notify(format!("Looking for {} on Discogs…", e.display_name()));
    }

    pub(super) fn is_mix_find(key: &str) -> bool {
        parse_find(key).is_some()
    }

    /// Discogs answered for a mix track: exactly one release links it (it gets the Discogs
    /// verbs); several or none open Discogs' search for it instead.
    pub(super) fn mix_found(&mut self, key: &str, result: Result<DiscogsMatch, ApiError>) {
        let Some((c, id)) = parse_find(key) else {
            return;
        };
        let Some(e) = self.crates.get(c).and_then(|p| p.get(id)).cloned() else {
            return;
        };
        let why = match result {
            Ok(DiscogsMatch::Found(release)) => {
                if let Some(o) = self
                    .crates
                    .get_mut(c)
                    .and_then(|p| p.entries_mut().find(|e| e.id == id))
                    .and_then(|e| e.origin.as_mut())
                {
                    o.release = Some(release);
                }
                self.crates.touch(c);
                return self.notify(format!(
                    "{}: linked to its Discogs release",
                    e.display_name()
                ));
            }
            Ok(DiscogsMatch::Ambiguous(n)) => format!("{n} releases fit"),
            Ok(_) => "no release found".to_owned(),
            Err(err) => return self.notify(format!("Couldn't search Discogs: {err:?}")),
        };
        let url = dig::buy::Shop::Discogs.search_url(&format!("{} {}", e.artist, e.title));
        let opened = self.dig.as_ref().map(|d| d.open_url(&url));
        match opened {
            Some(Ok(())) => self.notify(format!(
                "{}: {why}, opened the Discogs search",
                e.display_name()
            )),
            _ => self.notify(format!("{}: {why}", e.display_name())),
        }
    }

    /// "Open in mix at 12:34" (or "Open mix") for an entry of a mix: the label and the address.
    pub(super) fn mix_link(e: &Entry) -> Option<(String, String)> {
        let o = e.origin.as_ref()?;
        let source = MixSource::from_url(&o.page)?;
        Some(match o.mix_time {
            Some(t) => (format!("Open in mix at {}", clock(t)), source.at(t)),
            None => ("Open mix".to_owned(), source.url()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_times() {
        assert_eq!(
            crate_name("Boiler Room: Kerri Chandler"),
            "Mix: Boiler Room: Kerri Chandler"
        );
        assert_eq!(crate_name(&"x".repeat(60)).chars().count(), 40);
        assert_eq!(
            (clock(754), clock(3723)),
            ("12:34".into(), "1:02:03".into())
        );
    }
}
