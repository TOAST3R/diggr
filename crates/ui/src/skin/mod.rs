//! Skins: an atlas image plus a RON map of sprite rectangles, widget layout, font and colors.
//!
//! Every widget is drawn by copying atlas sub-rectangles (nearest filtering, integer scale),
//! as classic skinned players did. Replacing `atlas.png` with a recolored image that keeps the same sprite
//! map reskins the player without code changes.

pub mod generate;

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct R {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl R {
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }
}

/// A fixed-size bitmap font laid out as a grid in the atlas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontDef {
    /// Top-left of the glyph grid in the atlas.
    pub x: u16,
    pub y: u16,
    pub glyph_w: u16,
    pub glyph_h: u16,
    /// Horizontal distance between characters when drawing text.
    pub advance: u16,
    pub per_row: u16,
    /// The characters in grid order. Lower case is drawn with the upper-case glyphs.
    pub chars: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Colors {
    pub pl_bg: [u8; 3],
    pub pl_text: [u8; 3],
    pub pl_current: [u8; 3],
    pub pl_selected_bg: [u8; 3],
    pub vis_bar_low: [u8; 3],
    pub vis_bar_high: [u8; 3],
    pub vis_peak: [u8; 3],
    pub vis_scope: [u8; 3],
    pub eq_curve: [u8; 3],
    /// The crate name on the playlist title bar.
    #[serde(default = "title_gold")]
    pub pl_title: [u8; 3],
    /// The OWNED badge on records already in the user's collection.
    #[serde(default = "owned_amber")]
    pub pl_owned: [u8; 3],
    /// LCD text the app draws (title line, kbps/kHz, the playlist footer's controls). Skins
    /// written before it existed keep their green.
    #[serde(default = "lcd_green")]
    pub lcd: [u8; 3],
}

fn lcd_green() -> [u8; 3] {
    [0, 236, 0]
}

fn owned_amber() -> [u8; 3] {
    [255, 176, 32]
}

fn filter_h() -> u16 {
    16
}

fn title_gold() -> [u8; 3] {
    [236, 204, 90]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkinDef {
    pub name: String,
    pub atlas: String,
    pub atlas_size: (u16, u16),
    pub main_size: (u16, u16),
    pub eq_size: (u16, u16),
    pub pl_width: u16,
    pub pl_top_h: u16,
    pub pl_bottom_h: u16,
    /// The filter bar between the title bar and the list.
    #[serde(default = "filter_h")]
    pub pl_filter_h: u16,
    pub pl_row_h: u16,
    pub sprites: BTreeMap<String, R>,
    /// Widget rectangles, relative to their section's top-left.
    pub layout: BTreeMap<String, R>,
    pub font: FontDef,
    pub colors: Colors,
}

/// Sprites the UI draws. A skin must provide all of them.
pub const REQUIRED_SPRITES: &[&str] = &[
    "main_bg",
    "eq_bg",
    "pl_top_l",
    "pl_top_fill",
    "pl_top_r",
    "pl_bottom_l",
    "pl_bottom_fill",
    "pl_bottom_r",
    "pl_left",
    "pl_right",
    "pl_filter_l",
    "pl_filter_fill",
    "pl_filter_r",
    "pl_field_l",
    "pl_field_fill",
    "pl_field_r",
    "btn_min",
    "btn_min_p",
    "btn_close",
    "btn_close_p",
    "btn_max",
    "btn_max_p",
    "btn_group",
    "btn_group_p",
    "btn_group_on",
    "btn_group_on_p",
    "prev",
    "prev_p",
    "play",
    "play_p",
    "pause",
    "pause_p",
    "stop",
    "stop_p",
    "next",
    "next_p",
    "eject",
    "eject_p",
    "shuffle_off",
    "shuffle_off_p",
    "shuffle_on",
    "shuffle_on_p",
    "repeat_off",
    "repeat_off_p",
    "repeat_all",
    "repeat_all_p",
    "repeat_one",
    "repeat_one_p",
    "tog_eq_off",
    "tog_eq_on",
    "tog_pl_off",
    "tog_pl_on",
    "status_play",
    "status_pause",
    "status_stop",
    "digit_0",
    "digit_1",
    "digit_2",
    "digit_3",
    "digit_4",
    "digit_5",
    "digit_6",
    "digit_7",
    "digit_8",
    "digit_9",
    "digit_minus",
    "digit_blank",
    "digit_colon",
    "mono_off",
    "mono_on",
    "stereo_off",
    "stereo_on",
    "seek_track",
    "seek_thumb",
    "seek_thumb_p",
    "volume_track",
    "volume_fill",
    "volume_thumb",
    "volume_thumb_p",
    "tog_wave_off",
    "tog_wave_on",
    "eq_on_off",
    "eq_on_on",
    "eq_presets",
    "eq_presets_p",
    "eq_track",
    "eq_thumb",
    "eq_thumb_p",
    "pl_plus",
    "pl_plus_p",
    "pl_menu",
    "pl_menu_p",
    "pl_opts",
    "pl_opts_p",
    "pl_scroll_thumb",
    "st_listed",
    "st_queued",
    "st_needs_tool",
    "st_unavailable",
    "st_other",
    "pl_resize",
];

/// Layout rectangles the UI positions widgets with.
pub const REQUIRED_LAYOUT: &[&str] = &[
    "titlebar",
    "btn_min",
    "btn_close",
    "status",
    "time",
    "vis",
    "title_text",
    "kbps",
    "khz",
    "mono",
    "stereo",
    "volume",
    "wave_toggle",
    "eq_toggle",
    "pl_toggle",
    "seek",
    "prev",
    "play",
    "pause",
    "stop",
    "next",
    "eject",
    "shuffle",
    "repeat",
    "eq_titlebar",
    "eq_close",
    "eq_on",
    "eq_presets",
    "eq_graph",
    "eq_preamp",
    "eq_band0",
    "eq_band1",
    "eq_band2",
    "eq_band3",
    "eq_band4",
    "eq_band5",
    "eq_band6",
    "eq_band7",
    "eq_band8",
    "eq_band9",
    "pl_titlebar",
    "pl_close",
    "pl_max",
    "pl_group",
    "pl_list",
    "pl_scroll",
    "pl_plus",
    "pl_menu",
    "pl_opts",
    "pl_info",
    "pl_resize",
];

impl SkinDef {
    /// A sprite rectangle; unknown names give an empty rect (nothing drawn).
    pub fn sprite(&self, name: &str) -> R {
        self.sprites.get(name).copied().unwrap_or_default()
    }

    pub fn at(&self, name: &str) -> R {
        self.layout.get(name).copied().unwrap_or_default()
    }

    /// The atlas rectangle of the glyph for `c` (case- and accent-folded), if the font has it.
    pub fn glyph(&self, c: char) -> Option<R> {
        let c = fold(c);
        let i = self.font.chars.chars().position(|g| g == c)? as u16;
        let f = &self.font;
        Some(R::new(
            f.x + (i % f.per_row) * f.glyph_w,
            f.y + (i / f.per_row) * f.glyph_h,
            f.glyph_w,
            f.glyph_h,
        ))
    }

    /// Checks that everything the UI needs exists and lies inside the atlas.
    pub fn validate(&self, atlas_w: u32, atlas_h: u32) -> Result<(), String> {
        if (atlas_w, atlas_h) != (self.atlas_size.0 as u32, self.atlas_size.1 as u32) {
            return Err(format!(
                "atlas is {atlas_w}×{atlas_h}, skin.ron expects {}×{}",
                self.atlas_size.0, self.atlas_size.1
            ));
        }
        for name in REQUIRED_SPRITES {
            let r = self
                .sprites
                .get(*name)
                .ok_or_else(|| format!("missing sprite `{name}`"))?;
            if r.w == 0 || r.h == 0 || (r.x + r.w) as u32 > atlas_w || (r.y + r.h) as u32 > atlas_h
            {
                return Err(format!("sprite `{name}` {r:?} is outside the atlas"));
            }
        }
        for name in REQUIRED_LAYOUT {
            if !self.layout.contains_key(*name) {
                return Err(format!("missing layout entry `{name}`"));
            }
        }
        let f = &self.font;
        let rows = (f.chars.chars().count() as u16).div_ceil(f.per_row);
        if (f.x + f.per_row * f.glyph_w) as u32 > atlas_w
            || (f.y + rows * f.glyph_h) as u32 > atlas_h
        {
            return Err("font grid is outside the atlas".into());
        }
        Ok(())
    }
}

/// Upper-case, and strip common accents so e.g. "Canción" renders as "CANCION".
pub fn fold(c: char) -> char {
    let c = match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => 'A',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => 'O',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ñ' | 'Ñ' => 'N',
        'ç' | 'Ç' => 'C',
        '’' | '‘' => '\'',
        '“' | '”' => '"',
        '–' | '—' | '−' => '-',
        // Arrows and menu paths ("Options › Discogs…") read as < and >.
        '›' | '»' | '▸' | '→' | '⏵' => '>',
        '‹' | '«' | '◂' | '←' | '⏴' => '<',
        other => other,
    };
    c.to_ascii_uppercase()
}

/// A skin ready to upload: its definition and decoded RGBA pixels.
pub struct LoadedSkin {
    pub def: SkinDef,
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

pub const DEFAULT_ATLAS: &[u8] = include_bytes!("../../../../assets/skin/default/atlas.png");
pub const DEFAULT_DEF: &str = include_str!("../../../../assets/skin/default/skin.ron");

impl LoadedSkin {
    pub fn from_parts(ron_text: &str, png: &[u8]) -> Result<Self, String> {
        let def: SkinDef = ron::from_str(ron_text).map_err(|e| format!("skin.ron: {e}"))?;
        let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .map_err(|e| format!("atlas: {e}"))?
            .to_rgba8();
        def.validate(img.width(), img.height())?;
        Ok(Self {
            size: [img.width() as usize, img.height() as usize],
            rgba: img.into_raw(),
            def,
        })
    }

    /// The bundled original skin.
    pub fn default_skin() -> Self {
        Self::from_parts(DEFAULT_DEF, DEFAULT_ATLAS).expect("bundled skin is valid")
    }

    /// A skin folder containing `skin.ron` and the atlas it names.
    pub fn from_dir(dir: &Path) -> Result<Self, String> {
        let ron_text =
            std::fs::read_to_string(dir.join("skin.ron")).map_err(|e| format!("skin.ron: {e}"))?;
        let def: SkinDef = ron::from_str(&ron_text).map_err(|e| format!("skin.ron: {e}"))?;
        let png = std::fs::read(dir.join(&def.atlas)).map_err(|e| format!("{}: {e}", def.atlas))?;
        Self::from_parts(&ron_text, &png)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_skin_is_valid_and_complete() {
        let skin = LoadedSkin::default_skin();
        assert_eq!(skin.rgba.len(), skin.size[0] * skin.size[1] * 4);
        for c in "ABCXYZ0123456789 .,:-()/'!?&+%*=_[]#".chars() {
            assert!(skin.def.glyph(c).is_some(), "glyph {c:?}");
        }
    }

    #[test]
    fn committed_assets_match_the_generator() {
        let (img, def) = generate::generate();
        assert_eq!(
            ron::from_str::<SkinDef>(DEFAULT_DEF).unwrap(),
            def,
            "run `cargo run -p ui --bin skin-gen`"
        );
        let committed = image::load_from_memory(DEFAULT_ATLAS).unwrap().to_rgba8();
        assert!(
            committed.as_raw() == img.as_raw(),
            "atlas.png is stale: run `cargo run -p ui --bin skin-gen`"
        );
    }

    #[test]
    fn glyph_folding() {
        let def = LoadedSkin::default_skin().def;
        assert_eq!(def.glyph('a'), def.glyph('A'));
        assert_eq!(def.glyph('ñ'), def.glyph('N'));
        assert_eq!(def.glyph('é'), def.glyph('E'));
        assert!(def.glyph('€').is_some(), "for-sale prices");
        assert!(def.glyph('·').is_some());
        assert!(def.glyph('₩').is_none());
    }

    #[test]
    fn a_skin_without_an_lcd_colour_keeps_green() {
        let old: Colors = ron::from_str(
            "(pl_bg: (0, 0, 0), pl_text: (0, 220, 0), pl_current: (255, 255, 255), \
             pl_selected_bg: (0, 0, 150), vis_bar_low: (0, 170, 0), vis_bar_high: (230, 210, 0), \
             vis_peak: (190, 190, 200), vis_scope: (0, 230, 0), eq_curve: (0, 230, 0))",
        )
        .unwrap();
        assert_eq!(old.lcd, [0, 236, 0]);
    }

    #[test]
    fn a_skin_file_without_a_filter_bar_height_gets_the_default() {
        let text = ron::to_string(&LoadedSkin::default_skin().def).unwrap();
        let old = text.replace("pl_filter_h:16,", "");
        assert_ne!(old, text);
        assert_eq!(ron::from_str::<SkinDef>(&old).unwrap().pl_filter_h, 16);
    }

    #[test]
    fn a_recolored_atlas_with_the_same_map_loads() {
        let (img, def) = generate::generate();
        let mut recolored = img.clone();
        for p in recolored.pixels_mut() {
            p.0.swap(0, 2); // swap red and blue
        }
        let mut png = Vec::new();
        recolored
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let skin = LoadedSkin::from_parts(&ron::to_string(&def).unwrap(), &png).unwrap();
        assert_eq!(skin.rgba, recolored.into_raw());
    }

    #[test]
    fn invalid_skins_are_rejected_with_a_reason() {
        let (img, mut def) = generate::generate();
        def.sprites.remove("play");
        let err = def.validate(img.width(), img.height()).unwrap_err();
        assert!(err.contains("play"), "{err}");
        let (img, mut def) = generate::generate();
        def.sprites
            .insert("play".into(), R::new(img.width() as u16 - 2, 0, 23, 18));
        assert!(def.validate(img.width(), img.height()).is_err());
        assert!(def.validate(img.width() + 1, img.height()).is_err());
    }
}
