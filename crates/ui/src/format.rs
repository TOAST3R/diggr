//! Text formatting for time displays and scrolling titles.

/// `m:ss` (or `h:mm:ss` from one hour on).
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0).floor() as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Parses `90`, `1:30` or `1:02:03.5` into seconds.
pub fn parse_clock(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    s.split(':').try_fold(0.0, |acc, part| {
        part.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| *v >= 0.0 && v.is_finite())
            .map(|v| acc * 60.0 + v)
    })
}

/// The main LCD: minutes (at least two digits) and seconds, e.g. `("01", "51")`.
/// Minutes run past 99 rather than wrapping.
pub fn lcd(secs: f64) -> (String, String) {
    let s = secs.max(0.0).floor() as u64;
    (format!("{:02}", s / 60), format!("{:02}", s % 60))
}

/// Lowest and highest tempo a DJ reads: a track outside is shown at half or double time.
pub const DJ_BPM: (f64, f64) = (88.0, 176.0);

/// An analysed tempo folded into [`DJ_BPM`] by halving or doubling, as a whole number, so
/// half-time and double-time readings of the same groove agree (87 → 174, 280 → 140).
pub fn dj_bpm(raw: f64) -> Option<u16> {
    if !raw.is_finite() || raw <= 0.0 {
        return None;
    }
    let mut bpm = raw;
    while bpm < DJ_BPM.0 {
        bpm *= 2.0;
    }
    while bpm > DJ_BPM.1 {
        bpm /= 2.0;
    }
    Some(bpm.round() as u16)
}

/// An entry's name: `(catno) Artist: Title · Album (N BPM)`. The catalog number, artist,
/// album and tempo are left out when they aren't known, so a local file reads `Artist: Title`;
/// so is an album named like the title (a single). The album follows the title, so a row cut
/// short on the right loses it before any of the title.
pub fn entry_name(catno: &str, artist: &str, title: &str, album: &str, bpm: Option<u16>) -> String {
    let mut name = String::new();
    // Discogs writes "none" for a release without a catalogue number.
    let catno = catno.trim();
    if !catno.is_empty() && !catno.eq_ignore_ascii_case("none") {
        name += &format!("({catno}) ");
    }
    if !artist.is_empty() {
        name += &format!("{artist}: ");
    }
    name += title;
    let album = album.trim();
    if !album.is_empty() && !album.eq_ignore_ascii_case(title.trim()) {
        name += &format!(" · {album}");
    }
    if let Some(b) = bpm {
        name += &format!(" ({b} BPM)");
    }
    name
}

/// The LP knob's tooltip: "LP 2.4 kHz", "LP 340 Hz", or "LP off".
pub fn lp_label(knob: f32) -> String {
    match audio::filter::cutoff_hz(knob) {
        None => "LP off".into(),
        Some(hz) if hz >= 1000.0 => format!("LP {:.1} kHz", hz / 1000.0),
        Some(hz) => format!("LP {} Hz", hz.round() as u32),
    }
}

/// The classic title line: `N. name (m:ss)`, with the name from [`entry_name`].
pub fn title_line(number: usize, name: &str, duration: Option<f64>) -> String {
    match duration {
        Some(d) => format!("{number}. {name} ({})", clock(d)),
        None => format!("{number}. {name}"),
    }
}

/// What the title line adds for an entry from a catalogue page: its side, year and a for-sale
/// summary, e.g. ` · A1 · 1994 · 6 for sale from €9.00` (the catalog number already leads the
/// name). Parts that aren't known are left out; a count of 0 reads "none for sale".
pub fn origin_details(o: &crate::playlist::Origin) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !o.position.trim().is_empty() {
        parts.push(o.position.trim().to_owned());
    }
    if let Some(y) = o.year {
        parts.push(y.to_string());
    }
    match &o.for_sale {
        Some(fs) if fs.count == 0 => parts.push("none for sale".into()),
        Some(fs) => parts.push(match fs.lowest_cents {
            Some(c) => format!("{} for sale from {}", fs.count, price(c, &fs.currency)),
            None => format!("{} for sale", fs.count),
        }),
        None => {}
    }
    parts.iter().map(|p| format!(" · {p}")).collect()
}

/// The player's second line for an entry from a catalogue page: its label, side, year and
/// for-sale summary, e.g. `Lowtide Tapes · A1 · 1994 · 6 for sale from €9.00`. Parts that
/// aren't known are left out.
pub fn origin_line(o: &crate::playlist::Origin) -> String {
    let rest = origin_details(o);
    let rest = rest.strip_prefix(" · ").unwrap_or(&rest);
    match (o.label.trim(), rest) {
        ("", r) => r.to_owned(),
        (l, "") => l.to_owned(),
        (l, r) => format!("{l} · {r}"),
    }
}

/// A record row's second line: catalog number, year, tracks ("3 tracks", or "2 of 4 tracks"
/// under a filter; "tracks coming…" while the record is only listed, its tracklist not read
/// yet) and the for-sale summary, e.g. `LT-012 · 1994 · 3 tracks · 6 for sale from €9.00`.
/// Parts that aren't known are left out.
pub fn record_line(
    o: Option<&crate::playlist::Origin>,
    shown: usize,
    total: usize,
    listed: bool,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(o) = o {
        if !o.catno.trim().is_empty() {
            parts.push(o.catno.trim().to_owned());
        }
        if let Some(y) = o.year {
            parts.push(y.to_string());
        }
    }
    let tracks = if total == 1 { "track" } else { "tracks" };
    parts.push(if listed {
        "tracks coming…".to_owned()
    } else if shown < total {
        format!("{shown} of {total} {tracks}")
    } else {
        format!("{total} {tracks}")
    });
    match o.and_then(|o| o.for_sale.as_ref()) {
        Some(fs) if fs.count == 0 => parts.push("none for sale".into()),
        Some(fs) => parts.push(match fs.lowest_cents {
            Some(c) => format!("{} for sale from {}", fs.count, price(c, &fs.currency)),
            None => format!("{} for sale", fs.count),
        }),
        None => {}
    }
    parts.join(" · ")
}

/// A seller crate's record line: catalog number, year, tracks, then its unsold copies and
/// their price range ("3 copies €9.00–€18.00"), or "sold" when none is left.
pub fn seller_record_line(
    o: Option<&crate::playlist::Origin>,
    (shown, total, listed): (usize, usize, bool),
    copies: &[&crate::playlist::SaleCopy],
) -> String {
    let mut line = record_line(
        o.map(|o| crate::playlist::Origin {
            for_sale: None,
            ..o.clone()
        })
        .as_ref(),
        shown,
        total,
        listed,
    );
    let Some(part) = copies_part(copies) else {
        return line;
    };
    line.push_str(" · ");
    line.push_str(&part);
    line
}

/// A seller's copies of a record: "3 copies €9.00–€18.00", "1 copy €9.00", or "sold" when none
/// is left; `None` with no copies.
fn copies_part(copies: &[&crate::playlist::SaleCopy]) -> Option<String> {
    let unsold: Vec<&&crate::playlist::SaleCopy> = copies.iter().filter(|c| !c.sold).collect();
    Some(match unsold.as_slice() {
        [] if copies.is_empty() => return None,
        [] => "sold".to_owned(),
        [c] => format!("1 copy {}", price(c.cents, &c.currency)),
        cs => {
            let lo = cs.iter().map(|c| c.cents).min().unwrap_or(0);
            let hi = cs.iter().map(|c| c.cents).max().unwrap_or(0);
            let cur = &cs[0].currency;
            if lo == hi {
                format!("{} copies {}", cs.len(), price(lo, cur))
            } else {
                format!("{} copies {}–{}", cs.len(), price(lo, cur), price(hi, cur))
            }
        }
    })
}

/// The warning before an owned record's in-cart tooltip, from which pressing is owned
/// ("this pressing", "another pressing (AF001R, 2019)", "" when unknown).
pub fn owned_warning(pressing: &str) -> String {
    match pressing {
        "" => "You already own this. ".to_owned(),
        p => format!("You already own {p}. "),
    }
}

/// A copy row: "€9.00 · VG+ / VG · Germany".
pub fn copy_line(c: &crate::playlist::SaleCopy) -> String {
    [
        price(c.cents, &c.currency),
        c.grades.clone(),
        c.ships_from.clone(),
    ]
    .into_iter()
    .filter(|t| !t.is_empty())
    .collect::<Vec<_>>()
    .join(" · ")
}

/// How long ago, roughly: "just now", "12 min ago", "3 h ago", "2 d ago".
pub fn ago(secs: u64) -> String {
    match secs {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", secs / 60),
        3600..86_400 => format!("{} h ago", secs / 3600),
        _ => format!("{} d ago", secs / 86_400),
    }
}

/// What the dig side knows about an entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DigMarks {
    /// On the wantlist (its record).
    pub wanted: bool,
    pub passed: bool,
    pub wantlist_pending: bool,
    /// A wantlist change gave up, with why.
    pub wantlist_failed: Option<String>,
    /// Adding the record to the collection failed, with why.
    pub collection_failed: Option<String>,
    /// A copy is waiting to be taken out of the collection.
    pub discard_pending: bool,
    /// Taking a copy out of the collection failed, with why.
    pub discard_failed: Option<String>,
    /// In the user's collection: "this pressing", or "another pressing (AF014, 2018)".
    pub owned: Option<String>,
    /// A copy of the record is in the user's Discogs cart: "from decks.de, €9.00".
    pub cart: Option<String>,
    /// In a seller crate, the record's copies: "€9.00 VG+ / VG, €12.00 VG+ / VG+".
    pub copies: Option<String>,
    /// What each store's page last said of the record: ("Juno", "in stock, €14.78 (seen 14
    /// Oct)").
    pub stores: Vec<(&'static str, String)>,
    /// Its preview is being searched for on YouTube right now.
    pub searching: bool,
}

/// For a Discogs record with tracks searched for: how many entries it has, how many have a
/// clip from Discogs and how many are still to search ("11 tracks · 1 clip · 10 to search");
/// counts of zero are left out. `None` for a record with no track searched for.
pub fn record_counts<'a>(
    entries: impl IntoIterator<Item = &'a crate::playlist::Entry>,
) -> Option<String> {
    use crate::playlist::{EntryStatus, WaitKind};
    let (mut tracks, mut clips, mut to_search, mut searched) = (0, 0, 0, false);
    for e in entries {
        tracks += 1;
        let Some(o) = &e.origin else { continue };
        searched |= !o.search_key.is_empty();
        if o.clip.is_some() && o.search_key.is_empty() {
            clips += 1;
        }
        if e.status == EntryStatus::Waiting(WaitKind::Search) {
            to_search += 1;
        }
    }
    if !searched {
        return None;
    }
    let count = |n: usize, one: &str, many: &str| match n {
        0 => None,
        1 => Some(format!("1 {one}")),
        n => Some(format!("{n} {many}")),
    };
    let parts: Vec<String> = [
        count(tracks, "track", "tracks"),
        count(clips, "clip", "clips"),
        count(to_search, "to search", "to search"),
    ]
    .into_iter()
    .flatten()
    .collect();
    Some(parts.join(" · "))
}

/// A record card's text (see [`record_details`]): built for the open card's record, drawn by
/// the app.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecordCard {
    pub artist: String,
    pub album: String,
    /// Label · catalog number · year · formats, what's known of them.
    pub pressing: String,
    /// Its styles (else its genres), one pill each.
    pub styles: Vec<String>,
    /// Every shown entry (the card scrolls past 8).
    pub tracks: Vec<CardTrack>,
    /// WANTED, OWNED, IN CART, SOLD, as they apply.
    pub marks: Vec<&'static str>,
    /// The for-sale snapshot and its age, or a seller's copies.
    pub sale: Option<String>,
    /// "11 tracks · 1 clip · 10 to search" (see [`record_counts`]).
    pub counts: Option<String>,
}

impl RecordCard {
    /// The status strip has something to show.
    pub fn has_strip(&self) -> bool {
        !self.marks.is_empty() || self.sale.is_some() || self.counts.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardTrack {
    pub id: crate::playlist::EntryId,
    pub side: String,
    /// The title, after the track's artist when that isn't the record's.
    pub title: String,
    pub bpm: Option<u16>,
    pub state: CardTrackState,
    /// How its audio is coming, where the tempo goes: "queued", "40%", "search", "next" once
    /// armed; "failed", "not found" for one to try again.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardTrackState {
    Normal,
    /// Waiting for its audio: its play button fetches it first and plays it on arrival.
    Dim,
    /// Its clip failed or its search found nothing: its play button tries again.
    Retry,
    /// Nothing to try (no clip, already in the crate): no play button.
    Unavailable,
    Playing,
    Paused,
}

/// A record's card from its shown entries (crate order, at least one), the first entry's dig
/// marks, a seller's copies of it (empty outside a seller crate), the entry playing in it
/// (paused or not), the armed entry, and `now` in Unix seconds.
pub fn record_details(
    members: &[&crate::playlist::Entry],
    marks: &DigMarks,
    copies: &[&crate::playlist::SaleCopy],
    playing: Option<(crate::playlist::EntryId, bool)>,
    armed: Option<crate::playlist::EntryId>,
    now: u64,
) -> RecordCard {
    use crate::playlist::{EntryStatus, UnavailableKind, WaitKind};
    let Some(first) = members.first() else {
        return RecordCard::default();
    };
    let o = first.origin.as_ref();
    // The record's own credit ("Various" for a compilation), else its first track's.
    let artist = match o.map(|o| o.artist.trim()) {
        Some(a) if !a.is_empty() => a,
        _ => first.artist.trim(),
    };
    let album = match first.album().trim() {
        "" => first.title.trim(),
        a => a,
    };
    let pressing = o.map_or_else(String::new, |o| {
        let catno = o.catno.trim();
        let catno = if catno.eq_ignore_ascii_case("none") {
            ""
        } else {
            catno
        };
        let year = o.year.map(|y| y.to_string()).unwrap_or_default();
        [o.label.trim(), catno, year.as_str(), o.formats.trim()]
            .into_iter()
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    });
    let styles = o.map_or_else(Vec::new, |o| {
        o.styles
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect()
    });
    let tracks = members
        .iter()
        .map(|e| {
            let own = e.artist.trim();
            let title = if own.is_empty() || own.eq_ignore_ascii_case(artist) {
                e.title.trim().to_owned()
            } else {
                format!("{own}: {}", e.title.trim())
            };
            let state = match playing {
                Some((id, paused)) if id == e.id => {
                    if paused {
                        CardTrackState::Paused
                    } else {
                        CardTrackState::Playing
                    }
                }
                _ => match &e.status {
                    EntryStatus::Unavailable(
                        UnavailableKind::ClipFailed | UnavailableKind::NotFound,
                    ) => CardTrackState::Retry,
                    EntryStatus::Unavailable(_) => CardTrackState::Unavailable,
                    EntryStatus::Waiting(_) => CardTrackState::Dim,
                    _ => CardTrackState::Normal,
                },
            };
            let note = match &e.status {
                _ if armed == Some(e.id) => Some("next".to_owned()),
                EntryStatus::Waiting(w) => match w {
                    WaitKind::Listed => Some("listed".to_owned()),
                    WaitKind::Queued => Some("queued".to_owned()),
                    WaitKind::Downloading(p) => Some(format!("{p}%")),
                    WaitKind::NeedsYtDlp | WaitKind::PreparingYtDlp => Some("yt-dlp".to_owned()),
                    WaitKind::Search => Some("search".to_owned()),
                    WaitKind::Other(_) => None,
                },
                EntryStatus::Unavailable(UnavailableKind::ClipFailed) => Some("failed".to_owned()),
                EntryStatus::Unavailable(UnavailableKind::NotFound) => Some("not found".to_owned()),
                _ => None,
            };
            CardTrack {
                id: e.id,
                side: e
                    .origin
                    .as_ref()
                    .map(|o| o.position.trim().to_owned())
                    .unwrap_or_default(),
                title,
                bpm: e.bpm,
                state,
                note,
            }
        })
        .collect();
    let all_sold = !copies.is_empty() && copies.iter().all(|c| c.sold);
    let marks_on = [
        (marks.wanted, "WANTED"),
        (marks.owned.is_some(), "OWNED"),
        (marks.cart.is_some(), "IN CART"),
        (all_sold, "SOLD"),
    ];
    let sale = copies_part(copies).or_else(|| {
        let fs = o?.for_sale.as_ref()?;
        let what = match (fs.count, fs.lowest_cents) {
            (0, _) => "none for sale".to_owned(),
            (n, Some(c)) => format!("{n} for sale from {}", price(c, &fs.currency)),
            (n, None) => format!("{n} for sale"),
        };
        Some(format!(
            "{what} · fetched {}",
            ago(now.saturating_sub(fs.fetched_at))
        ))
    });
    RecordCard {
        artist: artist.to_owned(),
        album: album.to_owned(),
        pressing,
        styles,
        tracks,
        marks: marks_on
            .into_iter()
            .filter_map(|(on, m)| on.then_some(m))
            .collect(),
        sale,
        counts: record_counts(members.iter().copied()),
    }
}

/// Everything known about an entry, for its tooltip, as (label, value) lines (with the record's
/// facts: see [`track_details`]).
pub fn entry_details(
    e: &crate::playlist::Entry,
    marks: DigMarks,
    now: u64,
) -> Vec<(&'static str, String)> {
    track_details(e, marks, now, true)
}

/// A track's tooltip, as (label, value) lines. Nothing unknown is listed. `with_record`: also
/// the record's facts (album, style, label, catalog number, year, for sale), for a row with no
/// record row above it to show them; a local file always keeps its album. `now` is seconds
/// since the Unix epoch (for the for-sale snapshot's age).
pub fn track_details(
    e: &crate::playlist::Entry,
    marks: DigMarks,
    now: u64,
    with_record: bool,
) -> Vec<(&'static str, String)> {
    let mut out = vec![("", e.row_name())];
    let mut add = |label: &'static str, value: &str| {
        if !value.trim().is_empty() {
            out.push((label, value.trim().to_owned()));
        }
    };
    if with_record || e.origin.is_none() {
        add("Album", e.album());
    }
    if with_record {
        add("Style", e.origin.as_ref().map_or("", |o| o.styles.as_str()));
    }
    if let Some(o) = &e.origin {
        if with_record {
            add("Label", &o.label);
            add("Cat#", &o.catno);
        }
        add("Side", &o.position);
        if with_record {
            add("Year", &o.year.map(|y| y.to_string()).unwrap_or_default());
        }
    }
    // A store's tempo says so until the analyser has its own.
    let store = e.origin.as_ref().and_then(|o| {
        let name = match () {
            _ if !o.beatport.is_empty() => "Beatport",
            _ if !o.traxsource.is_empty() => "Traxsource",
            _ => return None,
        };
        o.store_bpm.then_some(name)
    });
    add(
        "Tempo",
        &e.bpm
            .map(|b| match store {
                Some(s) => format!("{b} BPM (from {s})"),
                None => format!("{b} BPM"),
            })
            .unwrap_or_default(),
    );
    if let Some(o) = &e.origin
        && let Some(name) = crate_key_name(&o.key)
    {
        add("Key", &format!("{} ({name})", o.key));
    }
    add("Time", &e.duration.map(clock).unwrap_or_default());
    add("Status", &e.status.note().unwrap_or_default());
    // Where the audio comes from, with the Bandcamp album it is on.
    if let Some(o) = &e.origin
        && let Some(source) = o.source()
    {
        use crate::playlist::ClipSource;
        let text = match source {
            ClipSource::Bandcamp if !o.album.is_empty() => {
                format!("{} · {}", source.name(), o.album)
            }
            ClipSource::Juno => "Juno sample (2 min)".to_owned(),
            ClipSource::Beatport => "Beatport sample (2 min)".to_owned(),
            ClipSource::YouTube if !o.traxsource.is_empty() => {
                "YouTube (found for Traxsource)".to_owned()
            }
            ClipSource::YouTube if !o.bleep.is_empty() => "YouTube (found for Bleep)".to_owned(),
            _ => source.name().to_owned(),
        };
        add("Source", &text);
    }
    // A track of a record with no clip: how its preview was found.
    if let Some(o) = e.origin.as_ref().filter(|o| !o.search_key.is_empty()) {
        use crate::playlist::{EntryStatus, UnavailableKind};
        let preview = if !o.found.is_empty() {
            format!("found by search ({})", o.found)
        } else if e.status == EntryStatus::Unavailable(UnavailableKind::AlreadyInCrate) {
            "already in crate".to_owned()
        } else if matches!(&e.status, EntryStatus::Unavailable(_)) {
            "not found by search".to_owned()
        } else if marks.searching {
            "searching YouTube…".to_owned()
        } else {
            "to search".to_owned()
        };
        add("Preview", &preview);
    }
    if let Some(why) = &marks.wantlist_failed {
        add("Wantlist", &format!("failed ({why})"));
    } else if marks.wantlist_pending {
        add("Wantlist", "change pending");
    } else if marks.wanted {
        add("Wantlist", "on it");
    }
    if let Some(why) = &marks.collection_failed {
        add("Collection", &format!("add failed ({why})"));
    } else if let Some(why) = &marks.discard_failed {
        add("Collection", &format!("removal failed ({why})"));
    } else if marks.discard_pending {
        add("Collection", "removal pending");
    }
    if marks.passed {
        add("Passed", "yes");
    }
    if let Some(o) = &marks.owned {
        add("Owned", o);
    }
    if let Some(c) = &marks.cart {
        add("In your cart", c);
    }
    if let Some(c) = &marks.copies {
        add("Copies", c);
    }
    for (store, note) in &marks.stores {
        add(store, note);
    }
    if let Some(fs) = e
        .origin
        .as_ref()
        .and_then(|o| o.for_sale.as_ref())
        .filter(|_| with_record)
    {
        let what = match (fs.count, fs.lowest_cents) {
            (0, _) => "none".to_owned(),
            (n, Some(c)) => format!("{n} from {}", price(c, &fs.currency)),
            (n, None) => n.to_string(),
        };
        add(
            "For sale",
            &format!(
                "{what} (fetched {})",
                ago(now.saturating_sub(fs.fetched_at))
            ),
        );
    }
    if e.origin.is_none() && e.source.is_none() {
        add("File", &e.track.0);
    }
    out
}

/// `€9.00`, `£12.50`, `$7.00`, `¥1500`; other currencies by their ISO code (`CHF 12.00`).
pub fn price(cents: u64, currency: &str) -> String {
    let units = format!("{}.{:02}", cents / 100, cents % 100);
    match currency {
        "EUR" => format!("€{units}"),
        "GBP" => format!("£{units}"),
        "USD" => format!("${units}"),
        "JPY" => format!("¥{}", (cents + 50) / 100),
        other => format!("{other} {units}"),
    }
}

/// A window of `width` characters into `text`, scrolled by `offset` characters. Text that fits
/// is returned unchanged; longer text loops with a `  ***  ` separator.
pub fn scroll(text: &str, width: usize, offset: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_owned();
    }
    let looped: Vec<char> = chars.iter().copied().chain("  ***  ".chars()).collect();
    (0..width)
        .map(|i| looped[(offset + i) % looped.len()])
        .collect()
}

/// "A minor" for the Camelot code "8A".
fn crate_key_name(code: &str) -> Option<&'static str> {
    #[cfg(not(target_arch = "wasm32"))]
    return dig::stores::key_name(code);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = code;
        None
    }
}

/// "40,728".
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_warning_names_the_pressing() {
        assert_eq!(owned_warning(""), "You already own this. ");
        assert_eq!(
            owned_warning("another pressing (AF001R, 2019)"),
            "You already own another pressing (AF001R, 2019). "
        );
    }

    #[test]
    fn seller_records_show_their_copies() {
        use crate::playlist::{Origin, SaleCopy};
        let o = Origin {
            catno: "LT-012".into(),
            year: Some(1994),
            for_sale: Some(crate::playlist::ForSale {
                count: 6,
                lowest_cents: Some(900),
                currency: "EUR".into(),
                fetched_at: 0,
            }),
            ..Origin::default()
        };
        let c = |cents, sold| SaleCopy {
            cents,
            sold,
            currency: "EUR".into(),
            grades: "VG+ / VG".into(),
            ships_from: "Germany".into(),
            ..SaleCopy::default()
        };
        let (a, b, x) = (c(900, false), c(1800, false), c(1200, true));
        assert_eq!(
            seller_record_line(Some(&o), (3, 3, false), &[&a, &b, &x]),
            "LT-012 · 1994 · 3 tracks · 2 copies €9.00–€18.00"
        );
        assert_eq!(
            seller_record_line(Some(&o), (3, 3, false), &[&a]),
            "LT-012 · 1994 · 3 tracks · 1 copy €9.00"
        );
        assert_eq!(
            seller_record_line(Some(&o), (3, 3, false), &[&x]),
            "LT-012 · 1994 · 3 tracks · sold"
        );
        assert_eq!(copy_line(&a), "€9.00 · VG+ / VG · Germany");
    }

    #[test]
    fn clock_formats() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(111.9), "1:51");
        assert_eq!(clock(3_725.0), "1:02:05");
        assert_eq!(clock(-3.0), "0:00");
    }

    #[test]
    fn parses_clock_times() {
        assert_eq!(parse_clock("90"), Some(90.0));
        assert_eq!(parse_clock(" 1:30 "), Some(90.0));
        assert_eq!(parse_clock("1:02:03.5"), Some(3723.5));
        assert_eq!(parse_clock("x"), None);
        assert_eq!(parse_clock(""), None);
        assert_eq!(parse_clock("-5"), None);
    }

    #[test]
    fn lcd_digits() {
        assert_eq!(lcd(111.0), ("01".into(), "51".into()));
        assert_eq!(lcd(6_000.0), ("100".into(), "00".into()));
    }

    #[test]
    fn tempo_folds_into_the_dj_range() {
        assert_eq!(dj_bpm(124.3), Some(124));
        assert_eq!(dj_bpm(87.0), Some(174));
        assert_eq!(dj_bpm(280.0), Some(140));
        assert_eq!(dj_bpm(176.0), Some(176));
        assert_eq!(dj_bpm(88.0), Some(88));
        assert_eq!(dj_bpm(43.0), Some(172));
        assert_eq!(dj_bpm(0.0), None);
        assert_eq!(dj_bpm(f64::NAN), None);
    }

    #[test]
    fn entry_names_leave_out_what_isnt_known() {
        assert_eq!(
            entry_name("LT-012", "Nightcraft", "Glasshouse", "", Some(124)),
            "(LT-012) Nightcraft: Glasshouse (124 BPM)"
        );
        assert_eq!(
            entry_name("", "Mira Sol", "Coastline", "", None),
            "Mira Sol: Coastline"
        );
        assert_eq!(
            entry_name(" ", "", "untitled", "", Some(128)),
            "untitled (128 BPM)"
        );
        for none in ["none", " NONE ", "None"] {
            assert_eq!(
                entry_name(none, "Ben Gibson", "Vanished Between", "", None),
                "Ben Gibson: Vanished Between"
            );
        }
    }

    #[test]
    fn the_album_follows_the_title_unless_it_is_the_title() {
        assert_eq!(
            entry_name(
                "LT-012",
                "Nightcraft",
                "Glasshouse",
                "Glasshouse EP",
                Some(124)
            ),
            "(LT-012) Nightcraft: Glasshouse · Glasshouse EP (124 BPM)"
        );
        assert_eq!(
            entry_name("TRS-07", "Ohm Field", "Static", " static ", Some(130)),
            "(TRS-07) Ohm Field: Static (130 BPM)",
            "a single: the album is the title"
        );
        assert_eq!(entry_name("", "", "Fold", "  ", None), "Fold");
        // Cut on the right, the album goes before any of the title.
        let name = entry_name("", "A", "Title", "Album", None);
        assert!(name.find("Album").unwrap() > name.find("Title").unwrap());
    }

    #[test]
    fn lp_labels() {
        assert_eq!(lp_label(1.0), "LP off");
        assert_eq!(lp_label(0.0), "LP 60 Hz");
        assert_eq!(lp_label(0.5), "LP 1.1 kHz");
    }

    #[test]
    fn title_lines() {
        assert_eq!(
            title_line(
                4,
                &entry_name("", "Crusher-P", "Echo", "", None),
                Some(230.0)
            ),
            "4. Crusher-P: Echo (3:50)"
        );
        assert_eq!(title_line(1, "untitled", None), "1. untitled");
    }

    #[test]
    fn the_players_second_line_leads_with_the_label() {
        use crate::playlist::{ForSale, Origin};
        let o = Origin {
            label: "Lowtide Tapes".into(),
            position: "A1".into(),
            year: Some(1994),
            for_sale: Some(ForSale {
                count: 6,
                lowest_cents: Some(900),
                currency: "EUR".into(),
                fetched_at: 0,
            }),
            ..Default::default()
        };
        assert_eq!(
            origin_line(&o),
            "Lowtide Tapes · A1 · 1994 · 6 for sale from €9.00"
        );
        let bare = Origin {
            position: "B2".into(),
            ..Default::default()
        };
        assert_eq!(origin_line(&bare), "B2", "no label: no leading separator");
        let only = Origin {
            label: "Lowtide Tapes".into(),
            ..Default::default()
        };
        assert_eq!(origin_line(&only), "Lowtide Tapes");
        assert_eq!(origin_line(&Origin::default()), "");
    }

    #[test]
    fn discogs_details_follow_the_title() {
        use crate::playlist::{ForSale, Origin};
        let mut o = Origin {
            position: "A1".into(),
            catno: "LT-012".into(),
            year: Some(1994),
            for_sale: Some(ForSale {
                count: 6,
                lowest_cents: Some(900),
                currency: "EUR".into(),
                fetched_at: 0,
            }),
            ..Default::default()
        };
        assert_eq!(
            title_line(
                3,
                &entry_name(&o.catno, "Nightcraft", "Glasshouse", "", Some(124)),
                Some(372.0)
            ) + &origin_details(&o),
            "3. (LT-012) Nightcraft: Glasshouse (124 BPM) (6:12) · A1 · 1994 · 6 for sale from €9.00"
        );
        o.for_sale.as_mut().unwrap().count = 0;
        assert_eq!(origin_details(&o), " · A1 · 1994 · none for sale");
        o.for_sale = None;
        o.year = None;
        assert_eq!(origin_details(&o), " · A1", "unknown numbers are left out");
        o.position.clear();
        assert_eq!(origin_details(&o), "", "the catalog number isn't repeated");
        assert_eq!(origin_details(&Origin::default()), "");
    }

    #[test]
    fn ages_read_roughly() {
        assert_eq!(ago(5), "just now");
        assert_eq!(ago(720), "12 min ago");
        assert_eq!(ago(3 * 3600 + 100), "3 h ago");
        assert_eq!(ago(2 * 86_400), "2 d ago");
    }

    #[test]
    fn a_searched_track_says_how_its_preview_was_found() {
        use crate::playlist::{Origin, Playlist};
        let mut p = Playlist::default();
        let mut add = |found: &str, status: &str| {
            let o = Origin {
                search_key: "release/38583846/A1".into(),
                found: found.into(),
                ..Default::default()
            };
            let id = p.add_waiting("The 89th Passenger", "Paper Wings", None, Some(o), status);
            p.get(id).unwrap().clone()
        };
        let preview = |e| {
            entry_details(&e, DigMarks::default(), 0)
                .into_iter()
                .find(|(l, _)| *l == "Preview")
                .map(|(_, v)| v)
        };
        assert_eq!(preview(add("", "to search")).as_deref(), Some("to search"));
        let found = add("The 89th Passenger – Paper Wings [AF069]", "queued");
        assert_eq!(
            preview(found).as_deref(),
            Some("found by search (The 89th Passenger – Paper Wings [AF069])")
        );
        let mut gone = add("", "to search");
        gone.status =
            crate::playlist::EntryStatus::Unavailable(crate::playlist::UnavailableKind::NotFound);
        assert_eq!(preview(gone).as_deref(), Some("not found by search"));
        let mut twice = add("", "to search");
        twice.status = crate::playlist::EntryStatus::Unavailable(
            crate::playlist::UnavailableKind::AlreadyInCrate,
        );
        assert_eq!(preview(twice).as_deref(), Some("already in crate"));
    }

    #[test]
    fn a_record_counts_its_tracks_clips_and_tracks_to_search() {
        use crate::playlist::{Origin, Playlist};
        let mut p = Playlist::default();
        let teardrop = Origin {
            clip: Some("u7K72X4eo_s".into()),
            ..Default::default()
        };
        p.add_waiting(
            "Massive Attack",
            "Teardrop",
            None,
            Some(teardrop.clone()),
            "queued",
        );
        for t in ["Angel", "Risingson"] {
            let o = Origin {
                search_key: format!("track/massive attack/{t}"),
                ..Default::default()
            };
            p.add_waiting("Massive Attack", t, None, Some(o), "to search");
        }
        assert_eq!(
            record_counts(p.entries()).as_deref(),
            Some("3 tracks · 1 clip · 2 to search")
        );
        // Found by search: not a Discogs clip, and no longer to search.
        let found = Origin {
            clip: Some("angel000001".into()),
            search_key: "track/massive attack/angel".into(),
            ..Default::default()
        };
        p.entries_mut().nth(1).unwrap().origin = Some(found);
        p.set_status(p.entries()[1].id, "queued");
        assert_eq!(
            record_counts(p.entries()).as_deref(),
            Some("3 tracks · 1 clip · 1 to search")
        );
        // Every track with its clip from Discogs: nothing to say.
        let mut all = Playlist::default();
        all.add_waiting("N", "Glasshouse", None, Some(teardrop), "queued");
        assert_eq!(record_counts(all.entries()), None);
    }

    #[test]
    fn details_list_what_is_known_about_an_entry() {
        use crate::playlist::{ForSale, Origin, Playlist};
        use platform::TrackRef;
        let now = 1_800_000_000;
        let mut p = Playlist::default();
        let dig = p.add_waiting(
            "Nightcraft",
            "Glasshouse",
            None,
            Some(Origin {
                album: "Glasshouse EP".into(),
                label: "Lowtide Tapes".into(),
                catno: "LT-012".into(),
                position: "A1".into(),
                year: Some(1994),
                for_sale: Some(ForSale {
                    count: 6,
                    lowest_cents: Some(900),
                    currency: "EUR".into(),
                    fetched_at: now - 3 * 3600,
                }),
                ..Default::default()
            }),
            "downloading 40%",
        );
        let local = p.add([TrackRef::new("/music/Mira Sol - Coastline.flac")])[0].0;
        let e = p.get(dig).unwrap().clone();
        let marks = DigMarks {
            wanted: true,
            wantlist_pending: true,
            owned: Some("another pressing (AF014, 2018)".into()),
            ..Default::default()
        };
        assert_eq!(
            entry_details(&e, marks, now),
            [
                (
                    "",
                    "(LT-012) Nightcraft: Glasshouse · Glasshouse EP".to_owned()
                ),
                ("Album", "Glasshouse EP".into()),
                ("Label", "Lowtide Tapes".into()),
                ("Cat#", "LT-012".into()),
                ("Side", "A1".into()),
                ("Year", "1994".into()),
                ("Status", "downloading 40%".into()),
                ("Wantlist", "change pending".into()),
                ("Owned", "another pressing (AF014, 2018)".into()),
                ("For sale", "6 from €9.00 (fetched 3 h ago)".into()),
            ]
        );
        let mut l = p.get(local).unwrap().clone();
        l.bpm = Some(128);
        l.duration = Some(372.0);
        assert_eq!(
            entry_details(&l, DigMarks::default(), now),
            [
                ("", "Mira Sol - Coastline (128 BPM)".to_owned()),
                ("Tempo", "128 BPM".into()),
                ("Time", "6:12".into()),
                ("File", "/music/Mira Sol - Coastline.flac".into()),
            ],
            "a local file: its path, and no Discogs fields"
        );
        l.album = "Geogaddi".into();
        let d = entry_details(&l, DigMarks::default(), now);
        assert_eq!(d[1], ("Album", "Geogaddi".to_owned()), "and its album tag");
        let d = track_details(&l, DigMarks::default(), now, false);
        assert_eq!(
            d[1],
            ("Album", "Geogaddi".to_owned()),
            "kept under an open record too"
        );
    }

    #[test]
    fn under_an_open_record_a_track_leaves_the_record_facts_to_its_card() {
        use crate::playlist::{ForSale, Origin, Playlist};
        let mut p = Playlist::default();
        let id = p.add_waiting(
            "Nightcraft",
            "Glasshouse",
            None,
            Some(Origin {
                album: "Glasshouse EP".into(),
                label: "Lowtide Tapes".into(),
                catno: "LT-012".into(),
                position: "A1".into(),
                year: Some(1994),
                styles: "Deep House".into(),
                for_sale: Some(ForSale {
                    count: 6,
                    lowest_cents: Some(900),
                    currency: "EUR".into(),
                    fetched_at: 0,
                }),
                ..Default::default()
            }),
            "queued",
        );
        let mut e = p.get(id).unwrap().clone();
        e.bpm = Some(124);
        let labels: Vec<&str> = track_details(&e, DigMarks::default(), 0, false)
            .into_iter()
            .map(|(l, _)| l)
            .collect();
        assert_eq!(labels, ["", "Side", "Tempo", "Status"]);
    }

    fn glasshouse(p: &mut crate::playlist::Playlist, tracks: &[(&str, &str, Option<u16>)]) {
        use crate::playlist::Origin;
        for &(side, title, bpm) in tracks {
            let id = p.add_waiting(
                "Nightcraft",
                title,
                None,
                Some(Origin {
                    album: "Glasshouse EP".into(),
                    artist: "Nightcraft".into(),
                    label: "Lowtide Tapes".into(),
                    catno: "LT-012".into(),
                    year: Some(1994),
                    formats: "Vinyl".into(),
                    styles: "Deep House, Minimal".into(),
                    position: side.into(),
                    ..Default::default()
                }),
                "queued",
            );
            let e = p.entries_mut().find(|e| e.id == id).unwrap();
            e.bpm = bpm;
            e.status = crate::playlist::EntryStatus::Ready;
        }
    }

    #[test]
    fn a_record_card_names_the_record_its_pressing_and_its_tracks() {
        let mut p = crate::playlist::Playlist::default();
        glasshouse(
            &mut p,
            &[
                ("A1", "Tidepool", Some(124)),
                ("A2", "Lowlight", None),
                ("B1", "Undertow", Some(122)),
            ],
        );
        let members: Vec<_> = p.entries().iter().collect();
        let playing = Some((members[1].id, false));
        let c = record_details(&members, &DigMarks::default(), &[], playing, None, 0);
        assert_eq!(c.artist, "Nightcraft");
        assert_eq!(c.album, "Glasshouse EP");
        assert_eq!(c.pressing, "Lowtide Tapes · LT-012 · 1994 · Vinyl");
        assert_eq!(c.styles, ["Deep House", "Minimal"]);
        let lines: Vec<_> = c
            .tracks
            .iter()
            .map(|t| (t.side.as_str(), t.title.as_str(), t.bpm, t.state))
            .collect();
        assert_eq!(
            lines,
            [
                ("A1", "Tidepool", Some(124), CardTrackState::Normal),
                ("A2", "Lowlight", None, CardTrackState::Playing),
                ("B1", "Undertow", Some(122), CardTrackState::Normal),
            ]
        );
        assert!(!c.has_strip(), "nothing known to buy or mark");
    }

    #[test]
    fn a_long_record_lists_every_track() {
        let mut p = crate::playlist::Playlist::default();
        let titles: Vec<String> = (1..=14).map(|i| format!("T{i}")).collect();
        let tracks: Vec<_> = titles.iter().map(|t| ("A", t.as_str(), None)).collect();
        glasshouse(&mut p, &tracks);
        p.entries_mut().nth(2).unwrap().status =
            crate::playlist::EntryStatus::Waiting(crate::playlist::WaitKind::Search);
        let members: Vec<_> = p.entries().iter().collect();
        let c = record_details(&members, &DigMarks::default(), &[], None, None, 0);
        assert_eq!(c.tracks.len(), 14, "the card scrolls past 8");
        assert_eq!(c.tracks[2].state, CardTrackState::Dim, "still to search");
        p.entries_mut().nth(3).unwrap().status =
            crate::playlist::EntryStatus::Unavailable(crate::playlist::UnavailableKind::NotFound);
        let members: Vec<_> = p.entries().iter().collect();
        let c = record_details(&members, &DigMarks::default(), &[], None, None, 0);
        assert_eq!(
            c.tracks[3].state,
            CardTrackState::Retry,
            "not found: try again"
        );
        assert_eq!(c.tracks[3].note.as_deref(), Some("not found"));
        assert_eq!(c.tracks[2].note.as_deref(), Some("search"));
        assert_eq!(c.tracks[0].note, None, "ready: its tempo goes there");
        {
            let mut it = p.entries_mut();
            it.nth(4).unwrap().status =
                crate::playlist::EntryStatus::Unavailable(crate::playlist::UnavailableKind::NoClip);
            it.next().unwrap().status =
                crate::playlist::EntryStatus::Waiting(crate::playlist::WaitKind::Downloading(40));
            it.next().unwrap().status =
                crate::playlist::EntryStatus::Waiting(crate::playlist::WaitKind::Queued);
        }
        let members: Vec<_> = p.entries().iter().collect();
        let armed = Some(members[6].id);
        let c = record_details(&members, &DigMarks::default(), &[], None, armed, 0);
        assert_eq!(
            c.tracks[4].state,
            CardTrackState::Unavailable,
            "no clip: nothing to try"
        );
        assert_eq!(c.tracks[5].note.as_deref(), Some("40%"));
        assert_eq!(c.tracks[6].note.as_deref(), Some("next"), "armed");
        assert_eq!(c.tracks[6].state, CardTrackState::Dim);
    }

    #[test]
    fn a_compilation_card_credits_various_and_names_each_track_artist() {
        use crate::playlist::{Origin, Playlist};
        let mut p = Playlist::default();
        let o = Origin {
            album: "Night Moves".into(),
            artist: "Various".into(),
            ..Default::default()
        };
        p.add_waiting("Lumen", "Halo", None, Some(o), "queued");
        let members: Vec<_> = p.entries().iter().collect();
        let c = record_details(&members, &DigMarks::default(), &[], None, None, 0);
        assert_eq!(c.artist, "Various");
        assert_eq!(c.tracks[0].title, "Lumen: Halo");
        assert_eq!(
            c.tracks[0].state,
            CardTrackState::Dim,
            "waiting for its audio"
        );
    }

    #[test]
    fn a_record_card_strip_shows_marks_sale_and_counts() {
        use crate::playlist::{ForSale, SaleCopy};
        let now = 1_800_000_000;
        let mut p = crate::playlist::Playlist::default();
        glasshouse(&mut p, &[("A1", "Tidepool", None)]);
        p.entries_mut()
            .next()
            .unwrap()
            .origin
            .as_mut()
            .unwrap()
            .for_sale = Some(ForSale {
            count: 6,
            lowest_cents: Some(900),
            currency: "EUR".into(),
            fetched_at: now - 3 * 3600,
        });
        let members: Vec<_> = p.entries().iter().collect();
        let marks = DigMarks {
            wanted: true,
            ..Default::default()
        };
        let c = record_details(&members, &marks, &[], None, None, now);
        assert_eq!(c.marks, ["WANTED"]);
        assert_eq!(
            c.sale.as_deref(),
            Some("6 for sale from €9.00 · fetched 3 h ago")
        );
        assert_eq!(c.counts, None, "no track searched for");
        let copy = |cents, sold| SaleCopy {
            cents,
            currency: "EUR".into(),
            sold,
            ..Default::default()
        };
        let copies = [copy(900, false), copy(1200, false), copy(1800, false)];
        let refs: Vec<_> = copies.iter().collect();
        let c = record_details(&members, &marks, &refs, None, None, now);
        assert_eq!(c.sale.as_deref(), Some("3 copies €9.00–€18.00"));
        let gone = [copy(900, true)];
        let refs: Vec<_> = gone.iter().collect();
        let c = record_details(&members, &DigMarks::default(), &refs, None, None, now);
        assert_eq!(c.marks, ["SOLD"]);
        assert_eq!(c.sale.as_deref(), Some("sold"));
    }

    #[test]
    fn a_local_album_card_has_no_pressing_pills_or_strip() {
        use platform::TrackRef;
        let mut p = crate::playlist::Playlist::default();
        let id = p.add([TrackRef::new("/music/boc/01.flac")])[0].0;
        let e = p.entries_mut().find(|e| e.id == id).unwrap();
        e.artist = "Boards of Canada".into();
        e.title = "Music Is Math".into();
        e.album = "Geogaddi".into();
        let members: Vec<_> = p.entries().iter().collect();
        let c = record_details(&members, &DigMarks::default(), &[], None, None, 0);
        assert_eq!(
            (c.artist.as_str(), c.album.as_str()),
            ("Boards of Canada", "Geogaddi")
        );
        assert_eq!(c.pressing, "");
        assert!(c.styles.is_empty());
        assert!(!c.has_strip());
        assert_eq!(c.tracks[0].title, "Music Is Math");
    }

    #[test]
    fn prices_use_a_symbol_or_the_iso_code() {
        assert_eq!(price(900, "EUR"), "€9.00");
        assert_eq!(price(1250, "GBP"), "£12.50");
        assert_eq!(price(705, "USD"), "$7.05");
        assert_eq!(price(150_000, "JPY"), "¥1500");
        assert_eq!(price(1200, "CHF"), "CHF 12.00");
    }

    #[test]
    fn scrolling() {
        assert_eq!(scroll("short", 10, 7), "short");
        assert_eq!(scroll("ABCDEFGH", 4, 0), "ABCD");
        assert_eq!(scroll("ABCDEFGH", 4, 6), "GH  ");
        // Loops back to the start after text + separator (8 + 7 chars).
        assert_eq!(scroll("ABCDEFGH", 4, 15), "ABCD");
    }

    #[test]
    fn a_record_line_names_the_pressing_tracks_and_what_is_for_sale() {
        let o = crate::playlist::Origin {
            catno: "LT-012".into(),
            year: Some(1994),
            for_sale: Some(crate::playlist::ForSale {
                count: 6,
                lowest_cents: Some(900),
                currency: "EUR".into(),
                fetched_at: 0,
            }),
            ..Default::default()
        };
        assert_eq!(
            record_line(Some(&o), 3, 3, false),
            "LT-012 · 1994 · 3 tracks · 6 for sale from €9.00"
        );
        assert_eq!(
            record_line(Some(&o), 2, 4, false).split(" · ").nth(2),
            Some("2 of 4 tracks")
        );
        assert_eq!(record_line(None, 2, 2, false), "2 tracks");
        assert_eq!(
            record_line(None, 1, 1, true),
            "tracks coming…",
            "a record only listed doesn't claim one track"
        );
    }
}
