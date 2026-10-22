//! Where the sections go: the player column (main, waveform, EQ) on the left at the skin's
//! fixed width, and the playlist to its right, as wide as the user made it and at least as tall
//! as the player column. All sizes are in skin pixels; the app multiplies by the scale.

use crate::settings::Settings;
use crate::skin::SkinDef;

/// The player column's height: main, then the waveform (under its title bar) and the EQ (or
/// the CRATES panel in its place) when they show.
pub fn player_height(settings: &Settings, d: &SkinDef) -> u16 {
    let mut h = d.main_size.1 + deck_h(settings, d);
    if settings.show_waveform {
        h += wave_title_h(d) + crate::waveform::HEIGHT as u16;
    }
    if settings.show_eq || settings.show_crates {
        h += d.eq_size.1;
    }
    h
}

/// The turntable under the main window while SYNC is on; 0 otherwise, or for a skin without
/// one.
pub fn deck_h(settings: &Settings, d: &SkinDef) -> u16 {
    if settings.sync.on {
        d.sprite("deck_bg").h
    } else {
        0
    }
}

/// The waveform's title bar in the player column; 0 for a skin without one.
pub fn wave_title_h(d: &SkinDef) -> u16 {
    d.sprite("wave_title").h
}

/// The playlist's frame above and below its rows: the title bar, the filter bar and the footer.
pub fn playlist_chrome(d: &SkinDef) -> u16 {
    d.pl_top_h + d.pl_filter_h + d.pl_bottom_h
}

/// Rows the playlist shows: the chosen number, or more, so it is never shorter than the player
/// column beside it.
pub fn playlist_rows(settings: &Settings, d: &SkinDef) -> u16 {
    let chrome = playlist_chrome(d);
    let min = player_height(settings, d)
        .saturating_sub(chrome)
        .div_ceil(d.pl_row_h);
    settings.playlist_rows.max(min)
}

pub fn playlist_height(settings: &Settings, d: &SkinDef) -> u16 {
    playlist_chrome(d) + playlist_rows(settings, d) * d.pl_row_h
}

/// The window: the player column, plus the playlist to its right when it shows.
pub fn window_size(settings: &Settings, d: &SkinDef) -> (u16, u16) {
    let (mut w, mut h) = (d.main_size.0, player_height(settings, d));
    if settings.show_playlist {
        w += settings.playlist_width;
        h = h.max(playlist_height(settings, d));
    }
    (w, h)
}

/// Height of the band across the top of a maximized playlist: the mini player, and the
/// waveform beside it. Shown with the waveform off too, so W doesn't move the rows.
pub const BAND_H: u16 = crate::waveform::HEIGHT as u16;

/// The band's height now: taller while SYNC puts the turntable in it.
pub fn band_h(settings: &Settings, d: &SkinDef) -> u16 {
    BAND_H.max(deck_h(settings, d))
}

/// The maximized layout in a window `w` × `h` skin pixels: the playlist's width and rows,
/// under a band `band` tall.
pub fn maximized(d: &SkinDef, band: u16, w: f32, h: f32) -> (u16, u16) {
    let width = w.max(d.pl_width as f32) as u16;
    let free = h - band as f32 - playlist_chrome(d) as f32;
    let rows = (free / d.pl_row_h as f32).floor().max(4.0) as u16;
    (width, rows)
}

/// The widest playlist whose window still fits `screen_w` points at `scale`.
pub fn fit_playlist_width(width: u16, d: &SkinDef, scale: f32, screen_w: f32) -> u16 {
    let room = (screen_w / scale).floor() - d.main_size.0 as f32;
    let room = room.clamp(d.pl_width as f32, u16::MAX as f32) as u16;
    width.min(room)
}

/// The skin's playlist layout stretched to `width`: the list and title bar grow, and what sits
/// on the right (close button, scrollbar, info, OPT, resize grip) moves with the right edge.
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
    for name in [
        "pl_close",
        "pl_max",
        "pl_group",
        "pl_scroll",
        "pl_info",
        "pl_resize",
    ] {
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

    fn settings(eq: bool, waveform: bool, playlist: bool) -> Settings {
        Settings {
            scale: 1,
            show_eq: eq,
            show_waveform: waveform,
            show_playlist: playlist,
            playlist_rows: 10,
            playlist_width: 400,
            ..Default::default()
        }
    }

    #[test]
    fn the_window_is_the_player_column_plus_the_playlist() {
        let d = LoadedSkin::default_skin().def;
        // Player column: 116, +14 title and 58 waveform, +116 EQ.
        assert_eq!(window_size(&settings(false, false, false), &d), (275, 116));
        assert_eq!(window_size(&settings(true, false, false), &d), (275, 232));
        assert_eq!(window_size(&settings(true, true, false), &d), (275, 304));
        // 10 rows are 204 tall with the filter bar: taller than the main window alone.
        assert_eq!(
            window_size(&settings(false, false, true), &d),
            (675, 20 + 16 + 130 + 38)
        );
        // Shorter than main + EQ + waveform (304): the playlist grows whole rows to match.
        let s = settings(true, true, true);
        assert_eq!(playlist_rows(&s, &d), 18);
        assert_eq!(window_size(&s, &d), (675, 20 + 16 + 18 * 13 + 38));
        // More rows than that are kept.
        let tall = Settings {
            playlist_rows: 30,
            ..s
        };
        assert_eq!(window_size(&tall, &d), (675, 20 + 16 + 30 * 13 + 38));
    }

    #[test]
    fn sync_opens_the_turntable_under_the_main_window() {
        let d = LoadedSkin::default_skin().def;
        let mut s = settings(true, false, false);
        s.sync.on = true;
        assert_eq!(window_size(&s, &d), (275, 116 + 104 + 116));
    }

    #[test]
    fn the_filter_bar_adds_height_and_keeps_the_rows() {
        let d = LoadedSkin::default_skin().def;
        let s = settings(false, false, true);
        assert_eq!(playlist_rows(&s, &d), 10, "no row given up for the bar");
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
        for name in ["pl_close", "pl_scroll", "pl_info", "pl_resize"] {
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
    fn a_maximized_playlist_takes_the_window_under_the_band() {
        let d = LoadedSkin::default_skin().def;
        // 1440 × 870 points at 2×: 720 × 435 skin pixels.
        assert_eq!(
            maximized(&d, BAND_H, 720.0, 435.0),
            (720, (435 - 58 - 74) / 13)
        );
        // A tiny window still gets the minimum playlist.
        assert_eq!(maximized(&d, BAND_H, 100.0, 50.0), (275, 4));
        // SYNC's turntable makes the band taller.
        let mut s = Settings::default();
        assert_eq!(band_h(&s, &d), BAND_H);
        s.sync.on = true;
        assert_eq!(band_h(&s, &d), d.sprite("deck_bg").h);
    }

    #[test]
    fn a_restored_width_fits_the_screen() {
        let d = LoadedSkin::default_skin().def;
        // 1440-point screen at 2×: 720 skin pixels, 275 of them the player column.
        assert_eq!(fit_playlist_width(1000, &d, 2.0, 1440.0), 445);
        assert_eq!(fit_playlist_width(300, &d, 2.0, 1440.0), 300);
        assert_eq!(
            fit_playlist_width(1000, &d, 2.0, 300.0),
            275,
            "never below the minimum"
        );
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

    #[test]
    fn the_crates_panel_takes_the_eqs_height() {
        let d = LoadedSkin::default_skin().def;
        let eq = settings(true, true, true);
        let crates = Settings {
            show_eq: false,
            show_crates: true,
            ..eq.clone()
        };
        assert_eq!(player_height(&eq, &d), player_height(&crates, &d));
        assert_eq!(window_size(&eq, &d), window_size(&crates, &d));
    }
}
