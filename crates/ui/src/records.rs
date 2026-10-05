//! The rows the playlist draws. Flat, every shown entry is one row. Grouped by record, each
//! album of the crate is one record row (two rows high, with its cover) that opens to show its
//! tracks; an entry of no album stays an ordinary row.
//!
//! Rows are built from the shown entries only (the BPM filter applies), and rebuilt only when
//! the crate, the filter or the open records change, never per frame. Scrolling counts rows
//! (a record row is one step), and how many fit is worked out from their heights.

use std::collections::{HashMap, HashSet};

use crate::playlist::{AlbumKey, Playlist};

/// A record row's height, in list rows.
pub const RECORD_UNITS: usize = 2;

#[derive(Debug, Clone, PartialEq)]
pub enum ListRow {
    /// One entry (crate index). `track`: indented under an open record row.
    Entry {
        idx: usize,
        track: bool,
    },
    Record(RecordRow),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordRow {
    pub key: AlbumKey,
    /// The album's shown entries, in crate order.
    pub members: Vec<usize>,
    /// The album's entries in the crate, shown or hidden.
    pub total: usize,
    /// The album has this one entry: the row acts as that entry and doesn't open.
    pub single: bool,
    pub open: bool,
}

impl ListRow {
    /// Height in list rows.
    pub fn units(&self) -> usize {
        match self {
            ListRow::Entry { .. } => 1,
            ListRow::Record(_) => RECORD_UNITS,
        }
    }

    /// The crate index the row stands for: its entry, or its record's first shown entry.
    pub fn first(&self) -> usize {
        match self {
            ListRow::Entry { idx, .. } => *idx,
            ListRow::Record(r) => r.members[0],
        }
    }

    /// The crate indices it selects and acts on.
    pub fn indices(&self) -> &[usize] {
        match self {
            ListRow::Entry { idx, .. } => std::slice::from_ref(idx),
            ListRow::Record(r) => &r.members,
        }
    }
}

/// The rows for `p`: one per shown entry when flat; grouped, one per album (in the order of
/// its first shown entry) followed by its tracks when it is open.
pub fn build(p: &Playlist, open: &HashSet<AlbumKey>) -> Vec<ListRow> {
    if !p.is_grouped() {
        return p
            .shown_rows()
            .into_iter()
            .map(|idx| ListRow::Entry { idx, track: false })
            .collect();
    }
    let entries = p.entries();
    let shown = p.shown();
    // One pass: each album's shown entries and its size, in the order of its first shown
    // entry; entries of no album in place.
    let mut slot: HashMap<AlbumKey, usize> = HashMap::with_capacity(entries.len());
    let mut groups: Vec<(Option<AlbumKey>, Vec<usize>, usize)> = Vec::new();
    let mut hidden: Vec<(AlbumKey, usize)> = Vec::new();
    for (idx, e) in entries.iter().enumerate() {
        let shows = shown.shows(e);
        match e.album_key() {
            Some(k) => match slot.get(&k) {
                Some(&g) => {
                    let group = &mut groups[g];
                    group.2 += 1;
                    if shows {
                        group.1.push(idx);
                    }
                }
                None if shows => {
                    slot.insert(k.clone(), groups.len());
                    groups.push((Some(k), vec![idx], 1));
                }
                // Counted once its album shows (a hidden first entry).
                None => hidden.push((k, idx)),
            },
            None if shows => groups.push((None, vec![idx], 1)),
            None => {}
        }
    }
    for (k, _) in hidden {
        if let Some(&g) = slot.get(&k) {
            groups[g].2 += 1;
        }
    }
    let mut rows = Vec::with_capacity(groups.len());
    for (key, members, total) in groups {
        let Some(key) = key else {
            rows.push(ListRow::Entry {
                idx: members[0],
                track: false,
            });
            continue;
        };
        let single = total == 1;
        let is_open = !single && open.contains(&key);
        let tracks: Vec<usize> = if is_open { members.clone() } else { Vec::new() };
        rows.push(ListRow::Record(RecordRow {
            key,
            members,
            total,
            single,
            open: is_open,
        }));
        rows.extend(
            tracks
                .into_iter()
                .map(|idx| ListRow::Entry { idx, track: true }),
        );
    }
    rows
}

/// The row showing crate index `idx`: its own row, the record row of a closed record holding
/// it, or (hidden by the filter) the first row after it.
pub fn row_of(rows: &[ListRow], idx: usize) -> usize {
    let mut record = None;
    for (r, row) in rows.iter().enumerate() {
        match row {
            ListRow::Entry { idx: i, .. } if *i == idx => return r,
            ListRow::Record(rec) if rec.members.contains(&idx) && !rec.open => return r,
            ListRow::Record(rec) if rec.members.contains(&idx) => record = Some(r),
            _ => {}
        }
    }
    record.unwrap_or_else(|| {
        rows.iter()
            .position(|row| row.first() > idx)
            .unwrap_or(rows.len().saturating_sub(1))
    })
}

/// How many whole rows fit in `units` list rows from row `start`.
pub fn fit(rows: &[ListRow], start: usize, units: usize) -> usize {
    let mut used = 0;
    let mut n = 0;
    for row in rows.iter().skip(start) {
        if used + row.units() > units {
            break;
        }
        used += row.units();
        n += 1;
    }
    n.max(1).min(rows.len().saturating_sub(start))
}

/// The furthest the list scrolls: the first row from which every remaining row fits.
pub fn max_start(rows: &[ListRow], units: usize) -> usize {
    let mut used = 0;
    let mut start = rows.len();
    while start > 0 && used + rows[start - 1].units() <= units {
        used += rows[start - 1].units();
        start -= 1;
    }
    start.min(rows.len().saturating_sub(1))
}

/// The scroll that shows row `row` with the least change from `start`.
pub fn scroll_to(rows: &[ListRow], start: usize, row: usize, units: usize) -> usize {
    if row < start {
        return row;
    }
    if row < start + fit(rows, start, units) {
        return start;
    }
    // The first start from which `row` is the last row to fit.
    let mut used = 0;
    let mut s = row + 1;
    while s > 0 && used + rows[s - 1].units() <= units {
        used += rows[s - 1].units();
        s -= 1;
    }
    s.min(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::Origin;
    use platform::TrackRef;

    /// A crate of these releases (`None`: a local file with no album), one entry each.
    fn crate_of(releases: &[Option<u64>], grouped: bool) -> Playlist {
        let mut p = Playlist::default();
        for (i, r) in releases.iter().enumerate() {
            match r {
                Some(r) => {
                    let origin = Origin {
                        release: Some(*r),
                        clip: Some(format!("c{i}")),
                        ..Default::default()
                    };
                    p.add_waiting("A", format!("t{i}"), None, Some(origin), "listed");
                }
                None => {
                    p.add([TrackRef::new(format!("/m/{i}.mp3"))]);
                }
            }
        }
        p.set_grouped(grouped);
        p
    }

    fn shape(rows: &[ListRow]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                ListRow::Entry { idx, track: false } => format!("e{idx}"),
                ListRow::Entry { idx, track: true } => format!("  t{idx}"),
                ListRow::Record(r) if r.single => format!("s{}", r.members[0]),
                ListRow::Record(r) => format!("r{:?}/{}", r.members, r.total),
            })
            .collect()
    }

    #[test]
    fn flat_is_one_row_per_shown_entry() {
        let p = crate_of(&[Some(1), Some(1), None], false);
        assert_eq!(shape(&build(&p, &HashSet::new())), ["e0", "e1", "e2"]);
    }

    #[test]
    fn grouped_is_one_row_per_record_and_open_records_show_their_tracks() {
        // After gathering: t0 t1 (release 1), t2 (release 2, alone), the local file.
        let p = crate_of(&[Some(1), Some(1), Some(2), None], true);
        let closed = build(&p, &HashSet::new());
        assert_eq!(shape(&closed), ["r[0, 1]/2", "s2", "e3"]);
        let open = HashSet::from([AlbumKey::Release(1), AlbumKey::Release(2)]);
        let rows = build(&p, &open);
        assert_eq!(shape(&rows), ["r[0, 1]/2", "  t0", "  t1", "s2", "e3"]);
        // A single record never opens.
        assert!(matches!(&rows[3], ListRow::Record(r) if !r.open));
        assert_eq!(
            rows.iter().map(ListRow::units).sum::<usize>(),
            2 + 1 + 1 + 2 + 1
        );
    }

    #[test]
    fn under_a_filter_a_record_counts_what_it_shows() {
        let mut p = crate_of(&[Some(1), Some(1), Some(1), Some(1), Some(2)], true);
        for (e, bpm) in p.entries_mut().zip([134, 124, 134, 124, 124]) {
            e.bpm = Some(bpm);
        }
        p.set_bpm_filter(Some((130, 140)));
        // Release 1 shows 2 of its 4; release 2 shows none, so it has no row.
        assert_eq!(shape(&build(&p, &HashSet::new())), ["r[0, 2]/4"]);
    }

    #[test]
    fn under_a_style_filter_records_without_the_style_have_no_row() {
        let mut p = crate_of(&[Some(1), Some(1), Some(2), Some(3)], true);
        let styles = ["Deep House", "Deep House", "Electro", "Minimal, Deep House"];
        for (e, st) in p.entries_mut().zip(styles) {
            e.origin.as_mut().unwrap().styles = st.into();
            e.bpm = Some(134);
        }
        p.set_style("Deep House", true);
        assert_eq!(shape(&build(&p, &HashSet::new())), ["r[0, 1]/2", "s3"]);
        // With the BPM filter too, a record shows only what passes both.
        p.entries_mut().next().unwrap().bpm = Some(124);
        p.set_bpm_filter(Some((130, 140)));
        assert_eq!(shape(&build(&p, &HashSet::new())), ["r[1]/2", "s3"]);
    }

    #[test]
    fn the_row_of_an_entry_follows_open_and_closed_records() {
        let p = crate_of(&[Some(1), Some(1), Some(2)], true);
        let closed = build(&p, &HashSet::new());
        assert_eq!((row_of(&closed, 1), row_of(&closed, 2)), (0, 1));
        let open = build(&p, &HashSet::from([AlbumKey::Release(1)]));
        assert_eq!((row_of(&open, 1), row_of(&open, 2)), (2, 3));
    }

    #[test]
    fn scrolling_counts_rows_and_fits_their_heights() {
        // Rows of heights 2, 2, 1, 2, 1 (units), in a 4-unit list.
        let p = crate_of(
            &[
                Some(1),
                Some(1),
                Some(2),
                Some(2),
                None,
                Some(3),
                Some(3),
                None,
            ],
            true,
        );
        let rows = build(&p, &HashSet::new());
        assert_eq!(
            shape(&rows),
            ["r[0, 1]/2", "r[2, 3]/2", "e4", "r[5, 6]/2", "e7"]
        );
        assert_eq!(fit(&rows, 0, 4), 2, "two record rows fill it");
        assert_eq!(fit(&rows, 2, 4), 3);
        assert_eq!(max_start(&rows, 4), 2, "rows 2.. take 1 + 2 + 1");
        assert_eq!(scroll_to(&rows, 0, 1, 4), 0, "already in view");
        assert_eq!(
            scroll_to(&rows, 0, 3, 4),
            2,
            "the least move that shows row 3"
        );
        assert_eq!(scroll_to(&rows, 3, 1, 4), 1, "up: to the row itself");
        assert_eq!(
            fit(&rows, 0, 1),
            1,
            "a row taller than the list still counts"
        );
    }

    #[test]
    fn building_rows_for_five_thousand_entries_is_quick() {
        let releases: Vec<Option<u64>> = (0..5000u64).map(|i| Some(i / 3)).collect();
        let p = crate_of(&releases, true);
        // The best of a few runs: a busy machine shouldn't decide.
        let mut took = std::time::Duration::MAX;
        let mut rows = Vec::new();
        for _ in 0..5 {
            let t = std::time::Instant::now();
            rows = build(&p, &HashSet::new());
            took = took.min(t.elapsed());
        }
        assert_eq!(rows.len(), 1667);
        // The target is for the app (an optimized build); an unoptimized test build, often
        // sharing the machine with other tests, gets room, enough to catch quadratic work.
        let budget = if cfg!(debug_assertions) { 25 } else { 5 };
        assert!(took < std::time::Duration::from_millis(budget), "{took:?}");
    }
}
