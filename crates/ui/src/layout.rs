//! Where the sections go: a header across the top, a band under it (the mini player, the
//! turntable and the waveform, a third each) and the playlist under that, filling the window. All sizes are in
//! skin pixels; the app multiplies by the scale.

use crate::skin::SkinDef;

/// The playlist's frame: the title bar and the filter bar above its rows, and an edge under
/// them like the one at its left. There is no footer.
pub fn playlist_chrome(d: &SkinDef) -> u16 {
    d.pl_top_h + d.pl_filter_h + d.sprite("pl_bottom_edge").h
}

/// Height of the window header across the top.
pub const HEADER_H: u16 = 15;

/// Height of the band under the header. It never changes: the turntable is always there, so
/// SYNC doesn't move the rows.
pub const BAND_H: u16 = crate::waveform::HEIGHT as u16;

/// The header and the band together: what the playlist sits under.
pub const TOP_H: u16 = HEADER_H + BAND_H;

/// The narrowest the mini player and the turntable go, and the widest (skin pixels).
pub const PANEL_MIN: u16 = 227;
pub const PANEL_MAX: u16 = 360;

/// The band's three panels along a window `win_w` wide: each an x and a width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panels {
    pub player: (u16, u16),
    pub deck: (u16, u16),
    pub wave: (u16, u16),
}

/// A third of the window (rounded down) for the player and for the turntable, kept within
/// [`PANEL_MIN`] and [`PANEL_MAX`]; the waveform has the rest, which never goes below nothing.
pub fn band_panels(win_w: f32) -> Panels {
    let third = (win_w / 3.0).floor().max(0.0) as u16;
    let w = third.clamp(PANEL_MIN, PANEL_MAX);
    let used = 2 * w;
    let wave = win_w.max(0.0) as u16;
    Panels {
        player: (0, w),
        deck: (w, w),
        wave: (used, wave.saturating_sub(used)),
    }
}

/// The playlist in a window `w` × `h` skin pixels, under a band `band` tall: its width and rows.
pub fn playlist_fit(d: &SkinDef, band: u16, w: f32, h: f32) -> (u16, u16) {
    let width = w.max(d.pl_width as f32) as u16;
    let free = h - band as f32 - playlist_chrome(d) as f32;
    let rows = (free / d.pl_row_h as f32).floor().max(4.0) as u16;
    (width, rows)
}

/// The skin's playlist layout stretched to `width`: the list and title bar grow, and what sits
/// on the right (the scrollbar) moves with the right edge.
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
    if let Some(r) = out.layout.get_mut("pl_scroll") {
        r.x += extra;
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
        assert_eq!(w.at("pl_scroll").x, d.at("pl_scroll").x + 125);
        assert_eq!(
            playlist_def(&d, 100).pl_width,
            275,
            "never narrower than the skin"
        );
    }

    #[test]
    fn the_playlist_takes_the_window_under_the_band() {
        let d = LoadedSkin::default_skin().def;
        // 1440 × 870 points at 2×: 720 × 435 skin pixels; the 15-pixel header and the 76-pixel
        // band, then 22 rows.
        assert_eq!((HEADER_H, BAND_H, TOP_H), (15, 76, 91));
        assert_eq!(
            d.sprite("hdr_fill").h,
            HEADER_H,
            "the skin's header is as tall"
        );
        assert_eq!(playlist_chrome(&d), 20 + 16 + 12);
        assert_eq!(playlist_fit(&d, TOP_H, 720.0, 435.0), (720, 22));
        // A tiny window still gets the minimum playlist.
        assert_eq!(playlist_fit(&d, TOP_H, 100.0, 50.0), (275, 4));
    }

    #[test]
    fn the_band_is_three_panels_in_thirds_within_limits() {
        let p = band_panels;
        // The default window: a third each.
        assert_eq!(
            p(720.0),
            Panels {
                player: (0, 240),
                deck: (240, 240),
                wave: (480, 240)
            }
        );
        // Up to 1,080 pixels the thirds are equal; beyond, the two fixed panels stop at 360
        // and the waveform takes the rest.
        assert_eq!(
            p(1080.0),
            Panels {
                player: (0, 360),
                deck: (360, 360),
                wave: (720, 360)
            }
        );
        assert_eq!(
            p(1100.0),
            Panels {
                player: (0, 360),
                deck: (360, 360),
                wave: (720, 380)
            }
        );
        assert_eq!(
            p(1500.0),
            Panels {
                player: (0, 360),
                deck: (360, 360),
                wave: (720, 780)
            }
        );
        // Narrow: the two keep 227 and the waveform keeps what is left, then nothing.
        assert_eq!(
            p(600.0),
            Panels {
                player: (0, 227),
                deck: (227, 227),
                wave: (454, 146)
            }
        );
        assert_eq!(p(300.0).wave.1, 0);
        for w in (200..=2000).step_by(7) {
            let q = p(w as f32);
            assert_eq!(q.deck.0, q.player.1);
            assert_eq!(q.wave.0, q.deck.0 + q.deck.1);
            assert!(
                q.wave.0 as f32 + q.wave.1 as f32 <= (w as f32).max(454.0),
                "{w}"
            );
        }
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
