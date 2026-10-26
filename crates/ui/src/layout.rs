//! Where the sections go: a band across the top (the mini player, SYNC's turntable while on,
//! the waveform) and the playlist under it, filling the window. All sizes are in skin pixels;
//! the app multiplies by the scale.

use crate::settings::Settings;
use crate::skin::SkinDef;

/// The turntable in the band while SYNC is on; 0 otherwise, or for a skin without one.
pub fn deck_h(settings: &Settings, d: &SkinDef) -> u16 {
    if settings.sync.on {
        d.sprite("deck_bg").h
    } else {
        0
    }
}

/// The playlist's frame above and below its rows: the title bar, the filter bar and the footer.
pub fn playlist_chrome(d: &SkinDef) -> u16 {
    d.pl_top_h + d.pl_filter_h + d.pl_bottom_h
}

/// Height of the band across the top: the mini player, and the waveform beside it. Shown with
/// the waveform off too (a seek bar takes its place), so W doesn't move the rows.
pub const BAND_H: u16 = crate::waveform::HEIGHT as u16;

/// The band's height now: taller while SYNC puts the turntable in it.
pub fn band_h(settings: &Settings, d: &SkinDef) -> u16 {
    BAND_H.max(deck_h(settings, d))
}

/// The playlist in a window `w` × `h` skin pixels, under a band `band` tall: its width and rows.
pub fn playlist_fit(d: &SkinDef, band: u16, w: f32, h: f32) -> (u16, u16) {
    let width = w.max(d.pl_width as f32) as u16;
    let free = h - band as f32 - playlist_chrome(d) as f32;
    let rows = (free / d.pl_row_h as f32).floor().max(4.0) as u16;
    (width, rows)
}

/// The skin's playlist layout stretched to `width`: the list and title bar grow, and what sits
/// on the right (▤, scrollbar, info) moves with the right edge.
/// The list and scrollbar start under the filter bar.
pub fn playlist_def(d: &SkinDef, width: u16) -> SkinDef {
    let width = width.max(d.pl_width);
    let extra = width - d.pl_width;
    let mut out = d.clone();
    out.pl_width = width;
    for name in ["pl_titlebar", "pl_list"] {
        if let Some(r) = out.layout.get_mut(name) {
            r.w += extra;
        }
    }
    for name in ["pl_list", "pl_scroll"] {
        if let Some(r) = out.layout.get_mut(name) {
            r.y += d.pl_filter_h;
        }
    }
    for name in ["pl_group", "pl_scroll", "pl_info"] {
        if let Some(r) = out.layout.get_mut(name) {
            r.x += extra;
        }
    }
    out
}

/// How the crate sidebar's rows go to its two scrolling lists: the user's crates (`crates`
/// long) and the open Discogs group (`group` long), around `fixed` rows that always show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideLayout {
    pub crates_rows: usize,
    pub group_rows: usize,
    /// Too short to give each open list a row: the whole sidebar scrolls as one list.
    pub single: bool,
}

/// Each list gets what it needs when both fit; otherwise each gets up to half of the free rows
/// (the crates one more when odd) and the rows one leaves go to the other.
pub fn side_layout(rows: usize, fixed: usize, crates: usize, group: usize) -> SideLayout {
    let free = rows.saturating_sub(fixed);
    if crates + group <= free {
        return SideLayout {
            crates_rows: crates,
            group_rows: group,
            single: false,
        };
    }
    if free < usize::from(crates > 0) + usize::from(group > 0) {
        return SideLayout {
            crates_rows: 0,
            group_rows: 0,
            single: true,
        };
    }
    let group_rows = group.min(free - crates.min(free.div_ceil(2)));
    SideLayout {
        crates_rows: crates.min(free - group_rows),
        group_rows,
        single: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::LoadedSkin;

    #[test]
    fn the_filter_bar_adds_height_and_keeps_the_rows() {
        let d = LoadedSkin::default_skin().def;
        // 400 skin pixels high: the band, the frame with the bar, then whole rows.
        let (_, rows) = playlist_fit(&d, BAND_H, 700.0, 400.0);
        assert_eq!(rows, (400 - BAND_H - playlist_chrome(&d)) / d.pl_row_h);
        let w = playlist_def(&d, 275);
        assert_eq!(w.at("pl_list").y, d.pl_top_h + d.pl_filter_h);
        assert_eq!(w.at("pl_scroll").y, d.pl_top_h + d.pl_filter_h);
    }

    #[test]
    fn a_wide_playlist_moves_its_right_side_along() {
        let d = LoadedSkin::default_skin().def;
        let w = playlist_def(&d, 400);
        assert_eq!(w.pl_width, 400);
        assert_eq!(w.at("pl_list").w, d.at("pl_list").w + 125);
        assert_eq!(w.at("pl_titlebar").w, 400);
        for name in ["pl_group", "pl_scroll", "pl_info"] {
            assert_eq!(w.at(name).x, d.at(name).x + 125, "{name}");
        }
        for name in ["pl_plus", "pl_opts"] {
            assert_eq!(w.at(name), d.at(name), "{name} stays on the left");
        }
        assert_eq!(
            playlist_def(&d, 100).pl_width,
            275,
            "never narrower than the skin"
        );
    }

    #[test]
    fn the_playlist_takes_the_window_under_the_band() {
        let d = LoadedSkin::default_skin().def;
        // 1440 × 870 points at 2×: 720 × 435 skin pixels.
        assert_eq!(
            playlist_fit(&d, BAND_H, 720.0, 435.0),
            (720, (435 - 58 - 74) / 13)
        );
        // A tiny window still gets the minimum playlist.
        assert_eq!(playlist_fit(&d, BAND_H, 100.0, 50.0), (275, 4));
        // SYNC's turntable makes the band taller.
        let mut s = Settings::default();
        assert_eq!(band_h(&s, &d), BAND_H);
        s.sync.on = true;
        assert_eq!(band_h(&s, &d), d.sprite("deck_bg").h);
    }

    #[test]
    fn the_sidebar_lists_share_the_free_rows() {
        let l = |c, g| side_layout(20, 9, c, g);
        assert_eq!(
            (l(3, 5).crates_rows, l(3, 5).group_rows),
            (3, 5),
            "both fit"
        );
        assert_eq!(
            (l(3, 31).crates_rows, l(3, 31).group_rows),
            (3, 8),
            "the leftover"
        );
        assert_eq!((l(40, 2).crates_rows, l(40, 2).group_rows), (9, 2));
        assert_eq!(
            (l(40, 31).crates_rows, l(40, 31).group_rows),
            (6, 5),
            "half each"
        );
        assert_eq!((l(40, 0).crates_rows, l(0, 31).group_rows), (11, 11));
        assert!(
            side_layout(10, 9, 40, 31).single,
            "one free row for two lists"
        );
        assert!(!side_layout(10, 9, 40, 0).single);
        for rows in 8..=40 {
            for c in (0..=100).step_by(7) {
                for g in (0..=100).step_by(9) {
                    let s = side_layout(rows, 8, c, g);
                    let free = rows.saturating_sub(8);
                    if s.single {
                        assert!(free < usize::from(c > 0) + usize::from(g > 0));
                        continue;
                    }
                    assert!(s.crates_rows <= c && s.group_rows <= g);
                    assert_eq!(
                        s.crates_rows + s.group_rows,
                        (c + g).min(free),
                        "no row wasted"
                    );
                    assert!(c == 0 || s.crates_rows > 0, "an open list shows a row");
                    assert!(g == 0 || s.group_rows > 0, "an open list shows a row");
                }
            }
        }
    }
}
