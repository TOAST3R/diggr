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
    if !catno.trim().is_empty() {
        name += &format!("({}) ", catno.trim());
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

/// Winamp's title line: `N. name (m:ss)`, with the name from [`entry_name`].
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
    /// In the user's collection: "this pressing", or "another pressing (AF014, 2018)".
    pub owned: Option<String>,
}

/// Everything known about an entry, for its tooltip, as (label, value) lines. Nothing unknown
/// is listed. `now` is seconds since the Unix epoch (for the for-sale snapshot's age).
pub fn entry_details(
    e: &crate::playlist::Entry,
    marks: DigMarks,
    now: u64,
) -> Vec<(&'static str, String)> {
    let mut out = vec![("", e.row_name())];
    let mut add = |label: &'static str, value: &str| {
        if !value.trim().is_empty() {
            out.push((label, value.trim().to_owned()));
        }
    };
    add("Album", e.album());
    if let Some(o) = &e.origin {
        add("Label", &o.label);
        add("Cat#", &o.catno);
        add("Side", &o.position);
        add("Year", &o.year.map(|y| y.to_string()).unwrap_or_default());
    }
    add(
        "Tempo",
        &e.bpm.map(|b| format!("{b} BPM")).unwrap_or_default(),
    );
    add("Time", &e.duration.map(clock).unwrap_or_default());
    add("Status", &e.status.note().unwrap_or_default());
    if let Some(why) = &marks.wantlist_failed {
        add("Wantlist", &format!("failed ({why})"));
    } else if marks.wantlist_pending {
        add("Wantlist", "change pending");
    } else if marks.wanted {
        add("Wantlist", "on it");
    }
    if let Some(why) = &marks.collection_failed {
        add("Collection", &format!("add failed ({why})"));
    }
    if marks.passed {
        add("Passed", "yes");
    }
    if let Some(o) = &marks.owned {
        add("Owned", o);
    }
    if let Some(fs) = e.origin.as_ref().and_then(|o| o.for_sale.as_ref()) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
