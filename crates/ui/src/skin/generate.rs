//! Draws the bundled default skin: original pixel art in the spirit of the classic layout
//! (bluish metal panels, green LCD, gold sliders). No Winamp artwork is used.
//!
//! `cargo run -p ui --bin skin-gen` writes `assets/skin/default/{atlas.png,skin.ron}`; a test
//! keeps the committed files in sync with this code.

use std::collections::BTreeMap;

use image::{Rgba, RgbaImage};

use super::{Colors, FontDef, R, SkinDef};

type C = [u8; 4];

const fn rgb(r: u8, g: u8, b: u8) -> C {
    [r, g, b, 255]
}

const PANEL_TOP: C = rgb(62, 66, 96);
const PANEL_BOT: C = rgb(34, 36, 56);
const HI: C = rgb(120, 126, 166);
const LO: C = rgb(14, 14, 24);
const TITLE_BG: C = rgb(26, 27, 42);
const GOLD: C = rgb(236, 204, 90);
const GOLD_DIM: C = rgb(150, 126, 52);
const LCD_BG: C = rgb(0, 0, 0);
const LCD_ON: C = rgb(0, 236, 0);
const LCD_DIM: C = rgb(0, 44, 0);
const BTN_TOP: C = rgb(168, 172, 196);
const BTN_BOT: C = rgb(102, 106, 132);
const BTN_HI: C = rgb(218, 222, 240);
const BTN_LO: C = rgb(40, 42, 58);
const ICON: C = rgb(26, 26, 38);
const LABEL: C = rgb(170, 176, 204);
const GROOVE: C = rgb(18, 18, 28);

const ATLAS_W: u16 = 512;
const FONT_CHARS: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,:;-_()[]/'!?&+#%*\"=<>@·€£$¥";
const GLYPH_W: u16 = 5;
const GLYPH_H: u16 = 7;

/// 5×7 glyphs, `#` = ink.
fn glyph_rows(c: char) -> [&'static str; 7] {
    match c {
        'A' => [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'B' => [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
        'C' => [
            ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
        ],
        'D' => [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'E' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'F' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
        'G' => [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####",
        ],
        'H' => [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'I' => [
            ".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        'J' => [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
        'K' => [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
        'L' => [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'M' => [
            "#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#",
        ],
        'N' => [
            "#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#",
        ],
        'O' => [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'P' => [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Q' => [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
        'R' => [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
        'S' => [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
        'T' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'U' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'V' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
        'W' => [
            "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#.",
        ],
        'X' => [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
        'Y' => [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        '0' => [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
        '1' => [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        '2' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
        '3' => [
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
        '4' => [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
        '5' => [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
        '6' => [
            "..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###.",
        ],
        '7' => [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
        '8' => [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
        '9' => [
            ".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##..",
        ],
        '.' => [
            ".....", ".....", ".....", ".....", ".....", ".##..", ".##..",
        ],
        ',' => [
            ".....", ".....", ".....", ".....", ".##..", "..#..", ".#...",
        ],
        ':' => [
            ".....", ".##..", ".##..", ".....", ".##..", ".##..", ".....",
        ],
        ';' => [
            ".....", ".##..", ".##..", ".....", ".##..", "..#..", ".#...",
        ],
        '-' => [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
        '_' => [
            ".....", ".....", ".....", ".....", ".....", ".....", "#####",
        ],
        '(' => [
            "...#.", "..#..", ".#...", ".#...", ".#...", "..#..", "...#.",
        ],
        ')' => [
            ".#...", "..#..", "...#.", "...#.", "...#.", "..#..", ".#...",
        ],
        '[' => [
            ".###.", ".#...", ".#...", ".#...", ".#...", ".#...", ".###.",
        ],
        ']' => [
            ".###.", "...#.", "...#.", "...#.", "...#.", "...#.", ".###.",
        ],
        '/' => [
            ".....", "....#", "...#.", "..#..", ".#...", "#....", ".....",
        ],
        '\'' => [
            "..#..", "..#..", ".#...", ".....", ".....", ".....", ".....",
        ],
        '!' => [
            "..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#..",
        ],
        '?' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#..",
        ],
        '&' => [
            ".##..", "#..#.", "#.#..", ".#...", "#.#.#", "#..#.", ".##.#",
        ],
        '+' => [
            ".....", "..#..", "..#..", "#####", "..#..", "..#..", ".....",
        ],
        '#' => [
            ".#.#.", ".#.#.", "#####", ".#.#.", "#####", ".#.#.", ".#.#.",
        ],
        '%' => [
            "##...", "##..#", "...#.", "..#..", ".#...", "#..##", "...##",
        ],
        '*' => [
            ".....", "..#..", "#.#.#", ".###.", "#.#.#", "..#..", ".....",
        ],
        '"' => [
            ".#.#.", ".#.#.", ".#.#.", ".....", ".....", ".....", ".....",
        ],
        '=' => [
            ".....", ".....", "#####", ".....", "#####", ".....", ".....",
        ],
        '<' => [
            "...#.", "..#..", ".#...", "#....", ".#...", "..#..", "...#.",
        ],
        '>' => [
            ".#...", "..#..", "...#.", "....#", "...#.", "..#..", ".#...",
        ],
        '@' => [
            ".###.", "#...#", "#.###", "#.#.#", "#.###", "#....", ".###.",
        ],
        // The title line's Discogs details: "· A1 · LT-012 · 6 for sale from €9.00".
        '·' => [
            ".....", ".....", ".....", "..#..", ".....", ".....", ".....",
        ],
        '€' => [
            "..###", ".#...", "####.", ".#...", "####.", ".#...", "..###",
        ],
        '£' => [
            "..##.", ".#..#", ".#...", "###..", ".#...", ".#...", "#####",
        ],
        '$' => [
            "..#..", ".####", "#.#..", ".###.", "..#.#", "####.", "..#..",
        ],
        '¥' => [
            "#...#", ".#.#.", "..#..", "#####", "..#..", "#####", "..#..",
        ],
        _ => ["....."; 7],
    }
}

/// 3×5 glyphs for the small labels baked into backgrounds (EQ scale and band names).
fn tiny_rows(c: char) -> [&'static str; 5] {
    match c {
        '0' => ["###", "#.#", "#.#", "#.#", "###"],
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["###", "..#", "###", "#..", "###"],
        '3' => ["###", "..#", ".##", "..#", "###"],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' => ["###", "#..", "###", "..#", "###"],
        '6' => ["###", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", "..#", ".#.", ".#."],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "###"],
        'K' => ["#.#", "#.#", "##.", "#.#", "#.#"],
        '+' => ["...", ".#.", "###", ".#.", "..."],
        '-' => ["...", "...", "###", "...", "..."],
        'D' => ["##.", "#.#", "#.#", "#.#", "##."],
        'B' => ["##.", "#.#", "##.", "#.#", "##."],
        'P' => ["##.", "#.#", "##.", "#..", "#.."],
        'R' => ["##.", "#.#", "##.", "#.#", "#.#"],
        'E' => ["###", "#..", "##.", "#..", "###"],
        'A' => [".#.", "#.#", "###", "#.#", "#.#"],
        'M' => ["#.#", "###", "###", "#.#", "#.#"],
        _ => ["..."; 5],
    }
}

fn tiny_w(s: &str) -> i32 {
    s.chars().count() as i32 * 4 - 1
}

struct Canvas {
    img: RgbaImage,
}

impl Canvas {
    fn px(&mut self, x: i32, y: i32, c: C) {
        if x >= 0 && y >= 0 && (x as u32) < self.img.width() && (y as u32) < self.img.height() {
            self.img.put_pixel(x as u32, y as u32, Rgba(c));
        }
    }

    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: C) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.px(xx, yy, c);
            }
        }
    }

    fn grad_v(&mut self, x: i32, y: i32, w: i32, h: i32, top: C, bot: C) {
        for i in 0..h {
            let t = if h > 1 {
                i as f32 / (h - 1) as f32
            } else {
                0.0
            };
            self.fill(x, y + i, w, 1, mix(top, bot, t));
        }
    }

    fn grad_h(&mut self, x: i32, y: i32, w: i32, h: i32, stops: &[C]) {
        for i in 0..w {
            let t = i as f32 / (w - 1).max(1) as f32 * (stops.len() - 1) as f32;
            let k = (t.floor() as usize).min(stops.len() - 2);
            self.fill(x + i, y, 1, h, mix(stops[k], stops[k + 1], t - k as f32));
        }
    }

    /// Raised edge: light top/left, dark bottom/right.
    fn bevel(&mut self, x: i32, y: i32, w: i32, h: i32, hi: C, lo: C) {
        self.fill(x, y, w, 1, hi);
        self.fill(x, y, 1, h, hi);
        self.fill(x, y + h - 1, w, 1, lo);
        self.fill(x + w - 1, y, 1, h, lo);
    }

    /// Recessed box.
    fn inset(&mut self, x: i32, y: i32, w: i32, h: i32, bg: C) {
        self.fill(x, y, w, h, bg);
        self.bevel(x, y, w, h, LO, HI);
    }

    fn text(&mut self, x: i32, y: i32, s: &str, c: C) {
        for (i, ch) in s.chars().enumerate() {
            for (row, bits) in glyph_rows(ch).iter().enumerate() {
                for (col, b) in bits.bytes().enumerate() {
                    if b == b'#' {
                        self.px(x + i as i32 * 6 + col as i32, y + row as i32, c);
                    }
                }
            }
        }
    }

    fn text_centered(&mut self, cx: i32, y: i32, s: &str, c: C) {
        self.text(cx - text_w(s) / 2, y, s, c);
    }

    fn tiny(&mut self, x: i32, y: i32, s: &str, c: C) {
        for (i, ch) in s.chars().enumerate() {
            for (row, bits) in tiny_rows(ch).iter().enumerate() {
                for (col, b) in bits.bytes().enumerate() {
                    if b == b'#' {
                        self.px(x + i as i32 * 4 + col as i32, y + row as i32, c);
                    }
                }
            }
        }
    }

    fn tiny_centered(&mut self, cx: i32, y: i32, s: &str, c: C) {
        self.tiny(cx - tiny_w(s) / 2, y, s, c);
    }

    /// Fills pixels whose centers lie inside the triangle.
    fn tri(&mut self, ox: i32, oy: i32, p: [(f32, f32); 3], c: C) {
        let edge = |a: (f32, f32), b: (f32, f32), x: f32, y: f32| {
            (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0)
        };
        let area = edge(p[0], p[1], p[2].0, p[2].1);
        for y in 0..32 {
            for x in 0..32 {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let w = [
                    edge(p[1], p[2], fx, fy),
                    edge(p[2], p[0], fx, fy),
                    edge(p[0], p[1], fx, fy),
                ];
                if w.iter().all(|&v| v * area >= 0.0) {
                    self.px(ox + x, oy + y, c);
                }
            }
        }
    }
}

fn text_w(s: &str) -> i32 {
    s.chars().count() as i32 * 6 - 1
}

fn mix(a: C, b: C, t: f32) -> C {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2]), 255]
}

fn darken(c: C, f: f32) -> C {
    mix(c, rgb(0, 0, 0), f)
}

/// Shelf packer: sprites go left to right in rows.
struct Packer {
    x: u16,
    y: u16,
    row_h: u16,
}

impl Packer {
    fn place(&mut self, w: u16, h: u16) -> R {
        if self.x + w > ATLAS_W {
            self.x = 0;
            self.y += self.row_h + 1;
            self.row_h = 0;
        }
        let r = R::new(self.x, self.y, w, h);
        self.x += w + 1;
        self.row_h = self.row_h.max(h);
        r
    }
}

struct Builder {
    c: Canvas,
    pack: Packer,
    sprites: BTreeMap<String, R>,
}

impl Builder {
    /// Allocates a sprite and draws it with coordinates relative to its top-left.
    fn sprite(&mut self, name: &str, w: u16, h: u16, draw: impl FnOnce(&mut Canvas, i32, i32)) {
        let r = self.pack.place(w, h);
        draw(&mut self.c, r.x as i32, r.y as i32);
        self.sprites.insert(name.to_owned(), r);
    }
}

fn panel(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    c.grad_v(x, y, w, h, PANEL_TOP, PANEL_BOT);
    c.bevel(x, y, w, h, HI, LO);
}

/// Title strip with gold stripes either side of the caption (buttons sit on the right). With
/// no caption the stripes run across the whole bar, and the app draws its own text on a
/// `pl_title_fill` strip.
fn titlebar(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, caption: &str) {
    c.grad_v(x, y, w, h, TITLE_BG, TITLE_BG);
    c.bevel(x, y, w, h, HI, LO);
    let ty = y + (h - 7) / 2;
    if caption.is_empty() {
        for sy in (ty..ty + 7).step_by(2) {
            c.fill(x + 6, sy, w - 40, 1, GOLD_DIM);
        }
        return;
    }
    let tw = text_w(caption);
    let tx = x + (w - tw) / 2;
    for sy in (ty..ty + 7).step_by(2) {
        c.fill(x + 6, sy, tx - 5 - (x + 6), 1, GOLD_DIM);
        c.fill(tx + tw + 5, sy, x + w - 34 - (tx + tw + 5), 1, GOLD_DIM);
    }
    c.text(tx, ty, caption, GOLD);
}

/// Part of a raised bar: a vertical gradient with the top and bottom bevel, and the left
/// and/or right bevel when it is an end piece.
#[allow(clippy::too_many_arguments)]
fn bar_piece(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, top: C, bot: C, ends: (bool, bool)) {
    c.grad_v(x, y, w, h, top, bot);
    c.fill(x, y, w, 1, HI);
    c.fill(x, y + h - 1, w, 1, LO);
    if ends.0 {
        c.fill(x, y, 1, h, HI);
    }
    if ends.1 {
        c.fill(x + w - 1, y, 1, h, LO);
    }
}

/// Widths of the playlist bars' end pieces (at the skin's 275-pixel minimum, the tiled middle
/// fills the rest: 235 pixels of title stripes, and nothing of the bottom bar).
pub const PL_TOP_L: u16 = 6;
pub const PL_TOP_R: u16 = 34;
pub const PL_BOTTOM_L: u16 = 125;
pub const PL_BOTTOM_R: u16 = 150;

/// One column of a plain title bar (its gradient and top/bottom bevel), to stretch behind text.
fn title_fill(c: &mut Canvas, x: i32, y: i32, h: i32) {
    c.grad_v(x, y, 1, h, TITLE_BG, TITLE_BG);
    c.px(x, y, HI);
    c.px(x, y + h - 1, LO);
}

fn button_face(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, pressed: bool) {
    if pressed {
        c.grad_v(x, y, w, h, BTN_BOT, darken(BTN_BOT, 0.2));
        c.bevel(x, y, w, h, BTN_LO, BTN_HI);
    } else {
        c.grad_v(x, y, w, h, BTN_TOP, BTN_BOT);
        c.bevel(x, y, w, h, BTN_HI, BTN_LO);
    }
}

/// Seven-segment digit, 9×13: segments lit per `mask` (bit 0 = a … bit 6 = g).
fn seven_seg(c: &mut Canvas, x: i32, y: i32, mask: u8) {
    c.fill(x, y, 9, 13, LCD_BG);
    let segs: [(i32, i32, i32, i32); 7] = [
        (2, 0, 5, 2),  // a
        (7, 2, 2, 4),  // b
        (7, 7, 2, 4),  // c
        (2, 11, 5, 2), // d
        (0, 7, 2, 4),  // e
        (0, 2, 2, 4),  // f
        (2, 6, 5, 1),  // g
    ];
    for (i, (sx, sy, w, h)) in segs.iter().enumerate() {
        let col = if mask & (1 << i) != 0 {
            LCD_ON
        } else {
            LCD_DIM
        };
        c.fill(x + sx, y + sy, *w, *h, col);
    }
}

const DIGIT_MASKS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];

pub fn generate() -> (RgbaImage, SkinDef) {
    let mut b = Builder {
        c: Canvas {
            img: RgbaImage::from_pixel(ATLAS_W as u32, 1024, Rgba([0, 0, 0, 0])),
        },
        pack: Packer {
            x: 0,
            y: 0,
            row_h: 0,
        },
        sprites: BTreeMap::new(),
    };

    // ---- section backgrounds --------------------------------------------------------------
    b.sprite("main_bg", 275, 116, |c, x, y| {
        panel(c, x, y, 275, 116);
        titlebar(c, x, y, 275, 14, "WINAMP");
        c.inset(x + 9, y + 15, 93, 46, LCD_BG);
        c.inset(x + 109, y + 22, 157, 13, LCD_BG);
        c.inset(x + 109, y + 38, 20, 11, LCD_BG);
        c.text(x + 131, y + 40, "KBPS", LABEL);
        c.inset(x + 157, y + 38, 14, 11, LCD_BG);
        c.text(x + 173, y + 40, "KHZ", LABEL);
    });
    b.sprite("eq_bg", 275, 116, |c, x, y| {
        panel(c, x, y, 275, 116);
        titlebar(c, x, y, 275, 14, "WINAMP EQUALIZER");
        c.inset(x + 86, y + 17, 113, 19, LCD_BG);
        c.fill(x + 87, y + 26, 111, 1, LCD_DIM);
        // Scale marks line up with the slider thumb centers at +12, 0 and −12 dB.
        for (label, ty) in [("+12DB", 41), ("+0DB", 67), ("-12DB", 93)] {
            c.tiny(x + 74 - tiny_w(label), y + ty, label, LABEL);
        }
        c.tiny_centered(x + 28, y + 105, "PREAMP", LABEL);
        for (i, l) in [
            "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K",
        ]
        .iter()
        .enumerate()
        {
            c.tiny_centered(x + 78 + 18 * i as i32 + 7, y + 105, l, LABEL);
        }
    });
    // The playlist stretches sideways: its title and bottom bars are a left cap, a column
    // the app tiles to the width, and a right cap. The title bar names the shown crate: the
    // app draws it on `pl_title_fill`.
    b.sprite("pl_top_l", PL_TOP_L, 20, |c, x, y| {
        bar_piece(
            c,
            x,
            y,
            PL_TOP_L as i32,
            20,
            TITLE_BG,
            TITLE_BG,
            (true, false),
        );
    });
    b.sprite("pl_top_fill", 1, 20, |c, x, y| {
        bar_piece(c, x, y, 1, 20, TITLE_BG, TITLE_BG, (false, false));
        for sy in (6..13).step_by(2) {
            c.px(x, y + sy, GOLD_DIM);
        }
    });
    b.sprite("pl_top_r", PL_TOP_R, 20, |c, x, y| {
        bar_piece(
            c,
            x,
            y,
            PL_TOP_R as i32,
            20,
            TITLE_BG,
            TITLE_BG,
            (false, true),
        );
    });
    b.sprite("pl_bottom_l", PL_BOTTOM_L, 38, |c, x, y| {
        bar_piece(
            c,
            x,
            y,
            PL_BOTTOM_L as i32,
            38,
            PANEL_TOP,
            PANEL_BOT,
            (true, false),
        );
    });
    b.sprite("pl_bottom_fill", 1, 38, |c, x, y| {
        bar_piece(c, x, y, 1, 38, PANEL_TOP, PANEL_BOT, (false, false));
    });
    b.sprite("pl_bottom_r", PL_BOTTOM_R, 38, |c, x, y| {
        bar_piece(
            c,
            x,
            y,
            PL_BOTTOM_R as i32,
            38,
            PANEL_TOP,
            PANEL_BOT,
            (false, true),
        );
        c.inset(x + 1, y + 12, 104, 11, LCD_BG);
    });
    b.sprite("pl_left", 12, 1, |c, x, y| {
        c.grad_h(x, y, 12, 1, &[HI, PANEL_TOP, PANEL_BOT]);
        c.px(x + 11, y, LO);
    });
    b.sprite("pl_right", 20, 1, |c, x, y| {
        c.grad_h(x, y, 20, 1, &[PANEL_BOT, PANEL_TOP, PANEL_BOT]);
        c.px(x, y, HI);
        c.fill(x + 5, y, 10, 1, GROOVE);
        c.px(x + 19, y, LO);
    });

    // ---- title bar buttons ----------------------------------------------------------------
    for (name, icon) in [("btn_min", 0), ("btn_close", 1), ("btn_max", 2)] {
        for pressed in [false, true] {
            let n = if pressed {
                format!("{name}_p")
            } else {
                name.to_owned()
            };
            b.sprite(&n, 9, 9, |c, x, y| {
                button_face(c, x, y, 9, 9, pressed);
                let o = pressed as i32;
                if icon == 0 {
                    c.fill(x + 2 + o, y + 6 + o, 5, 1, ICON);
                } else if icon == 2 {
                    // ⇔: a bar with a head at each end.
                    c.fill(x + 2 + o, y + 4 + o, 5, 1, ICON);
                    for (hx, dir) in [(2, 1), (6, -1)] {
                        c.px(x + hx + dir + o, y + 3 + o, ICON);
                        c.px(x + hx + dir + o, y + 5 + o, ICON);
                    }
                } else {
                    for i in 0..5 {
                        c.px(x + 2 + i + o, y + 2 + i + o, ICON);
                        c.px(x + 6 - i + o, y + 2 + i + o, ICON);
                    }
                }
            });
        }
    }

    // ---- transport ------------------------------------------------------------------------
    type Icon = fn(&mut Canvas, i32, i32);
    let icons: [(&str, u16, u16, Icon); 6] = [
        ("prev", 23, 18, |c, x, y| {
            c.fill(x + 5, y + 4, 2, 10, ICON);
            c.tri(x, y, [(12.0, 4.0), (12.0, 14.0), (7.0, 9.0)], ICON);
            c.tri(x, y, [(17.0, 4.0), (17.0, 14.0), (12.0, 9.0)], ICON);
        }),
        ("play", 23, 18, |c, x, y| {
            c.tri(x, y, [(8.0, 4.0), (8.0, 14.0), (16.0, 9.0)], ICON)
        }),
        ("pause", 23, 18, |c, x, y| {
            c.fill(x + 8, y + 4, 3, 10, ICON);
            c.fill(x + 13, y + 4, 3, 10, ICON);
        }),
        ("stop", 23, 18, |c, x, y| c.fill(x + 7, y + 5, 9, 8, ICON)),
        ("next", 23, 18, |c, x, y| {
            c.tri(x, y, [(6.0, 4.0), (6.0, 14.0), (11.0, 9.0)], ICON);
            c.tri(x, y, [(11.0, 4.0), (11.0, 14.0), (16.0, 9.0)], ICON);
            c.fill(x + 16, y + 4, 2, 10, ICON);
        }),
        ("eject", 22, 16, |c, x, y| {
            c.tri(x, y, [(11.0, 3.0), (5.0, 9.0), (17.0, 9.0)], ICON);
            c.fill(x + 5, y + 11, 12, 2, ICON);
        }),
    ];
    for (name, w, h, icon) in icons {
        for pressed in [false, true] {
            let n = if pressed {
                format!("{name}_p")
            } else {
                name.to_owned()
            };
            b.sprite(&n, w, h, |c, x, y| {
                button_face(c, x, y, w as i32, h as i32, pressed);
                let o = pressed as i32;
                icon(c, x + o, y + o);
            });
        }
    }

    // ---- labelled toggles -----------------------------------------------------------------
    let text_button =
        |b: &mut Builder, name: &str, w: u16, h: u16, label: &str, lit: bool, pressed: bool| {
            b.sprite(name, w, h, |c, x, y| {
                button_face(c, x, y, w as i32, h as i32, pressed);
                let col = if lit { rgb(0, 150, 0) } else { ICON };
                let o = pressed as i32;
                c.text_centered(x + w as i32 / 2 + o, y + (h as i32 - 7) / 2 + o, label, col);
            });
        };
    for (state, lit) in [("off", false), ("on", true)] {
        text_button(
            &mut b,
            &format!("shuffle_{state}"),
            47,
            15,
            "SHUFFLE",
            lit,
            false,
        );
        text_button(
            &mut b,
            &format!("shuffle_{state}_p"),
            47,
            15,
            "SHUFFLE",
            lit,
            true,
        );
    }
    for (state, label, lit) in [
        ("off", "REP", false),
        ("all", "REP", true),
        ("one", "REP1", true),
    ] {
        text_button(
            &mut b,
            &format!("repeat_{state}"),
            28,
            15,
            label,
            lit,
            false,
        );
        text_button(
            &mut b,
            &format!("repeat_{state}_p"),
            28,
            15,
            label,
            lit,
            true,
        );
    }
    for (name, w, label) in [
        ("tog_eq", 23, "EQ"),
        ("tog_pl", 23, "PL"),
        ("tog_wave", 38, "WAVE"),
        ("eq_on", 26, "ON"),
    ] {
        for (state, lit) in [("off", false), ("on", true)] {
            b.sprite(&format!("{name}_{state}"), w, 12, |c, x, y| {
                button_face(c, x, y, w as i32, 12, lit);
                c.fill(
                    x + 3,
                    y + 4,
                    3,
                    3,
                    if lit { rgb(0, 255, 0) } else { rgb(0, 60, 0) },
                );
                c.text(
                    x + 8,
                    y + 3,
                    label,
                    if lit { rgb(230, 240, 230) } else { ICON },
                );
            });
        }
    }
    text_button(&mut b, "eq_presets", 44, 12, "PRESETS", false, false);
    text_button(&mut b, "eq_presets_p", 44, 12, "PRESETS", false, true);
    for (name, label) in [
        ("pl_add", "ADD"),
        ("pl_rem", "REM"),
        ("pl_sel", "SEL"),
        ("pl_misc", "MISC"),
        ("pl_opts", "OPT"),
    ] {
        text_button(&mut b, name, 25, 18, label, false, false);
        text_button(&mut b, &format!("{name}_p"), 25, 18, label, false, true);
    }

    // ---- LCD ------------------------------------------------------------------------------
    for (i, mask) in DIGIT_MASKS.iter().enumerate() {
        b.sprite(&format!("digit_{i}"), 9, 13, |c, x, y| {
            seven_seg(c, x, y, *mask)
        });
    }
    b.sprite("digit_minus", 9, 13, |c, x, y| seven_seg(c, x, y, 0x40));
    b.sprite("digit_blank", 9, 13, |c, x, y| c.fill(x, y, 9, 13, LCD_BG));
    b.sprite("digit_colon", 3, 13, |c, x, y| {
        c.fill(x, y, 3, 13, LCD_BG);
        c.fill(x, y + 3, 2, 2, LCD_ON);
        c.fill(x, y + 8, 2, 2, LCD_ON);
    });
    b.sprite("status_play", 9, 9, |c, x, y| {
        c.fill(x, y, 9, 9, LCD_BG);
        c.tri(x, y, [(2.0, 1.0), (2.0, 8.0), (7.5, 4.5)], LCD_ON);
    });
    b.sprite("status_pause", 9, 9, |c, x, y| {
        c.fill(x, y, 9, 9, LCD_BG);
        c.fill(x + 2, y + 1, 2, 7, LCD_ON);
        c.fill(x + 5, y + 1, 2, 7, LCD_ON);
    });
    b.sprite("status_stop", 9, 9, |c, x, y| {
        c.fill(x, y, 9, 9, LCD_BG);
        c.fill(x + 2, y + 2, 5, 5, LCD_ON);
    });
    for (name, w, label) in [("mono", 25, "MONO"), ("stereo", 36, "STEREO")] {
        for (state, col) in [("off", rgb(40, 60, 40)), ("on", LCD_ON)] {
            b.sprite(&format!("{name}_{state}"), w, 12, |c, x, y| {
                c.inset(x, y, w as i32, 12, LCD_BG);
                c.text_centered(x + w as i32 / 2, y + 3, label, col);
            });
        }
    }

    // ---- sliders --------------------------------------------------------------------------
    b.sprite("seek_track", 248, 10, |c, x, y| {
        c.inset(x, y, 248, 10, GROOVE)
    });
    for (name, pressed) in [("seek_thumb", false), ("seek_thumb_p", true)] {
        b.sprite(name, 29, 10, |c, x, y| {
            let (top, bot) = if pressed {
                (rgb(255, 232, 140), GOLD)
            } else {
                (GOLD, GOLD_DIM)
            };
            c.grad_v(x, y, 29, 10, top, bot);
            c.bevel(x, y, 29, 10, rgb(255, 244, 200), rgb(90, 70, 20));
            for gx in [12, 14, 16] {
                c.fill(x + gx, y + 3, 1, 4, rgb(110, 86, 24));
            }
        });
    }
    b.sprite("volume_track", 68, 13, |c, x, y| {
        c.inset(x, y + 3, 68, 7, GROOVE)
    });
    b.sprite("volume_fill", 68, 13, |c, x, y| {
        c.grad_h(
            x + 1,
            y + 4,
            66,
            5,
            &[rgb(0, 190, 0), rgb(220, 220, 0), rgb(230, 70, 0)],
        );
    });
    for name in ["volume_thumb"] {
        for pressed in [false, true] {
            let n = if pressed {
                format!("{name}_p")
            } else {
                name.to_owned()
            };
            b.sprite(&n, 14, 11, |c, x, y| {
                button_face(c, x, y, 14, 11, pressed);
                c.fill(x + 6, y + 3, 2, 5, ICON);
            });
        }
    }
    b.sprite("eq_track", 14, 63, |c, x, y| {
        c.inset(x + 3, y, 8, 63, GROOVE);
        c.grad_v(x + 5, y + 1, 4, 61, rgb(255, 236, 120), GOLD_DIM);
    });
    for (name, pressed) in [("eq_thumb", false), ("eq_thumb_p", true)] {
        b.sprite(name, 11, 11, |c, x, y| {
            button_face(c, x, y, 11, 11, pressed);
            c.fill(x + 2, y + 5, 7, 1, ICON);
        });
    }
    b.sprite("pl_scroll_thumb", 10, 18, |c, x, y| {
        button_face(c, x, y, 10, 18, false);
        for gy in [7, 9, 11] {
            c.fill(x + 3, y + gy, 4, 1, ICON);
        }
    });
    b.sprite("pl_resize", 11, 11, |c, x, y| {
        for i in 0..3 {
            for k in 0..=(2 * i + 2) {
                let d = 4 * i + 2;
                let (px, py) = (x + 10 - (d - k.min(d)), y + 10 - k.min(d));
                if k <= d {
                    c.px(px, py, if k % 2 == 0 { HI } else { LO });
                }
            }
        }
    });

    b.sprite("pl_title_fill", 1, 20, |c, x, y| title_fill(c, x, y, 20));

    // Playlist status icons, white: the app tints them with the row's colour.
    const W: C = rgb(255, 255, 255);
    let ring = |c: &mut Canvas, x: i32, y: i32| {
        for (dx, dy) in [
            (3, 0),
            (4, 0),
            (5, 0),
            (2, 1),
            (6, 1),
            (1, 2),
            (7, 2),
            (0, 3),
            (8, 3),
            (0, 4),
            (8, 4),
            (0, 5),
            (8, 5),
            (1, 6),
            (7, 6),
            (2, 7),
            (6, 7),
            (3, 8),
            (4, 8),
            (5, 8),
        ] {
            c.px(x + dx, y + dy, W);
        }
    };
    // Listed: a hollow dot.
    b.sprite("st_listed", 9, 9, |c, x, y| ring(c, x, y));
    // Queued: a clock.
    b.sprite("st_queued", 9, 9, |c, x, y| {
        ring(c, x, y);
        c.fill(x + 4, y + 2, 1, 3, W);
        c.fill(x + 5, y + 4, 2, 1, W);
    });
    // Needs yt-dlp: a warning sign.
    b.sprite("st_needs_tool", 9, 9, |c, x, y| {
        for row in 0..9 {
            let half = row / 2;
            c.px(x + 4 - half, y + row, W);
            c.px(x + 4 + half, y + row, W);
        }
        c.fill(x, y + 8, 9, 1, W);
        c.fill(x + 4, y + 4, 1, 2, W);
        c.px(x + 4, y + 7, W);
    });
    // Unavailable: a barred circle.
    b.sprite("st_unavailable", 9, 9, |c, x, y| {
        ring(c, x, y);
        for i in 2..7 {
            c.px(x + i, y + 8 - i, W);
        }
    });
    // Anything else: three dots.
    b.sprite("st_other", 9, 9, |c, x, y| {
        for dx in [1, 4, 7] {
            c.px(x + dx, y + 4, W);
        }
    });

    // ---- font -----------------------------------------------------------------------------
    let per_row: u16 = 32;
    let rows = (FONT_CHARS.chars().count() as u16).div_ceil(per_row);
    let font_r = b.pack.place(per_row * GLYPH_W, rows * GLYPH_H);
    for (i, ch) in FONT_CHARS.chars().enumerate() {
        let gx = font_r.x as i32 + (i as i32 % per_row as i32) * GLYPH_W as i32;
        let gy = font_r.y as i32 + (i as i32 / per_row as i32) * GLYPH_H as i32;
        for (row, bits) in glyph_rows(ch).iter().enumerate() {
            for (col, bit) in bits.bytes().enumerate() {
                if bit == b'#' {
                    b.c.px(gx + col as i32, gy + row as i32, rgb(255, 255, 255));
                }
            }
        }
    }

    // Crop to the used height.
    let used = (b.pack.y + b.pack.row_h).div_ceil(8) * 8;
    let img = image::imageops::crop_imm(&b.c.img, 0, 0, ATLAS_W as u32, used as u32).to_image();

    let def = SkinDef {
        name: "Default (original)".into(),
        atlas: "atlas.png".into(),
        atlas_size: (ATLAS_W, used),
        main_size: (275, 116),
        eq_size: (275, 116),
        pl_width: 275,
        pl_top_h: 20,
        pl_bottom_h: 38,
        pl_row_h: 13,
        sprites: b.sprites,
        layout: layout(),
        font: FontDef {
            x: font_r.x,
            y: font_r.y,
            glyph_w: GLYPH_W,
            glyph_h: GLYPH_H,
            advance: 6,
            per_row,
            chars: FONT_CHARS.into(),
        },
        colors: Colors {
            pl_bg: [0, 0, 0],
            pl_text: [0, 220, 0],
            pl_current: [255, 255, 255],
            pl_selected_bg: [0, 0, 150],
            vis_bar_low: [0, 170, 0],
            vis_bar_high: [230, 210, 0],
            vis_peak: [190, 190, 200],
            vis_scope: [0, 230, 0],
            eq_curve: [0, 230, 0],
            pl_title: [GOLD[0], GOLD[1], GOLD[2]],
        },
    };
    (img, def)
}

fn layout() -> BTreeMap<String, R> {
    let mut l: Vec<(&str, R)> = vec![
        // main (relative to the main section)
        ("titlebar", R::new(0, 0, 275, 14)),
        ("btn_min", R::new(244, 3, 9, 9)),
        ("btn_close", R::new(262, 3, 9, 9)),
        ("status", R::new(14, 28, 9, 9)),
        ("time", R::new(26, 26, 64, 13)),
        ("vis", R::new(24, 43, 76, 16)),
        ("title_text", R::new(112, 25, 152, 7)),
        ("kbps", R::new(111, 40, 17, 7)),
        ("khz", R::new(158, 40, 12, 7)),
        ("mono", R::new(206, 38, 25, 12)),
        ("stereo", R::new(233, 38, 36, 12)),
        ("volume", R::new(107, 57, 68, 13)),
        ("wave_toggle", R::new(177, 58, 38, 12)),
        ("eq_toggle", R::new(219, 58, 23, 12)),
        ("pl_toggle", R::new(243, 58, 23, 12)),
        ("seek", R::new(16, 72, 248, 10)),
        ("prev", R::new(16, 88, 23, 18)),
        ("play", R::new(39, 88, 23, 18)),
        ("pause", R::new(62, 88, 23, 18)),
        ("stop", R::new(85, 88, 23, 18)),
        ("next", R::new(108, 88, 23, 18)),
        ("eject", R::new(136, 89, 22, 16)),
        ("shuffle", R::new(164, 89, 47, 15)),
        ("repeat", R::new(212, 89, 28, 15)),
        // equalizer (relative to the EQ section)
        ("eq_titlebar", R::new(0, 0, 275, 14)),
        ("eq_close", R::new(262, 3, 9, 9)),
        ("eq_on", R::new(14, 18, 26, 12)),
        ("eq_presets", R::new(217, 18, 44, 12)),
        ("eq_graph", R::new(86, 17, 113, 19)),
        ("eq_preamp", R::new(21, 38, 14, 63)),
        // playlist: titlebar/list/scroll relative to the playlist top; buttons, info and
        // resize relative to the bottom bar. A height of 0 means "stretch".
        ("pl_titlebar", R::new(0, 0, 275, 20)),
        ("pl_close", R::new(262, 6, 9, 9)),
        ("pl_max", R::new(250, 6, 9, 9)),
        ("pl_list", R::new(12, 20, 243, 0)),
        ("pl_scroll", R::new(260, 20, 10, 0)),
        ("pl_add", R::new(11, 12, 25, 18)),
        ("pl_rem", R::new(40, 12, 25, 18)),
        ("pl_sel", R::new(69, 12, 25, 18)),
        ("pl_misc", R::new(98, 12, 25, 18)),
        ("pl_opts", R::new(236, 12, 25, 18)),
        ("pl_info", R::new(128, 14, 100, 7)),
        ("pl_resize", R::new(263, 26, 11, 11)),
    ];
    let bands: Vec<(String, R)> = (0..10)
        .map(|i| (format!("eq_band{i}"), R::new(78 + 18 * i, 38, 14, 63)))
        .collect();
    let mut map: BTreeMap<String, R> = l.drain(..).map(|(k, v)| (k.to_owned(), v)).collect();
    map.extend(bands);
    map
}
