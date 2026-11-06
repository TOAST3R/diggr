//! Drawing with the skin atlas, and the skinned widgets built on it.
//!
//! Positions are in skin pixels relative to a section's top-left; `scale` converts skin pixels
//! to egui points (1 = classic size, 2 = double size). The atlas uses nearest filtering, so
//! pixels stay crisp at any integer scale, including Retina.

use egui::{Color32, Id, Pos2, Rect, Response, Sense, TextureId, Ui, Vec2, pos2, vec2};

use crate::skin::{R, SkinDef};

pub struct Skinned<'a> {
    pub painter: egui::Painter,
    pub tex: TextureId,
    pub def: &'a SkinDef,
    pub origin: Pos2,
    pub scale: f32,
}

pub fn color(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

impl<'a> Skinned<'a> {
    /// Screen rectangle for a skin-pixel rectangle in this section.
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(
            self.origin + vec2(x, y) * self.scale,
            vec2(w, h) * self.scale,
        )
    }

    pub fn at(&self, name: &str) -> Rect {
        let r = self.def.at(name);
        self.rect(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
    }

    fn uv(&self, s: R) -> Rect {
        let (w, h) = (self.def.atlas_size.0 as f32, self.def.atlas_size.1 as f32);
        Rect::from_min_max(
            pos2(s.x as f32 / w, s.y as f32 / h),
            pos2((s.x + s.w) as f32 / w, (s.y + s.h) as f32 / h),
        )
    }

    /// Draws a sprite at its natural size with its top-left at (x, y).
    pub fn sprite(&self, name: &str, x: f32, y: f32) {
        let s = self.def.sprite(name);
        self.sprite_in(name, self.rect(x, y, s.w as f32, s.h as f32));
    }

    /// Draws a sprite stretched into `dest`.
    pub fn sprite_in(&self, name: &str, dest: Rect) {
        let s = self.def.sprite(name);
        if s.w > 0 {
            self.painter
                .image(self.tex, dest, self.uv(s), Color32::WHITE);
        }
    }

    /// Draws a sprite stretched into `dest`, tinted (white sprites take the tint's colour).
    pub fn sprite_tinted(&self, name: &str, dest: Rect, tint: Color32) {
        let s = self.def.sprite(name);
        if s.w > 0 {
            self.painter.image(self.tex, dest, self.uv(s), tint);
        }
    }

    /// Draws the left `w` skin pixels of a sprite (e.g. a partially filled bar).
    pub fn sprite_left(&self, name: &str, x: f32, y: f32, w: f32) {
        let s = self.def.sprite(name);
        let w = w.clamp(0.0, s.w as f32);
        if w <= 0.0 {
            return;
        }
        let part = R {
            w: w.round() as u16,
            ..s
        };
        let dest = self.rect(x, y, part.w as f32, s.h as f32);
        self.painter
            .image(self.tex, dest, self.uv(part), Color32::WHITE);
    }

    /// Bitmap-font text (upper-case skin font), tinted; returns the width in skin pixels.
    pub fn text(&self, x: f32, y: f32, s: &str, tint: Color32) -> f32 {
        let f = &self.def.font;
        let mut cx = x;
        for c in s.chars() {
            if let Some(g) = self.def.glyph(c) {
                let dest = self.rect(cx, y, f.glyph_w as f32, f.glyph_h as f32);
                self.painter.image(self.tex, dest, self.uv(g), tint);
            }
            cx += f.advance as f32;
        }
        cx - x
    }

    pub fn text_width(&self, s: &str) -> f32 {
        (s.chars().count() as f32 * self.def.font.advance as f32 - 1.0).max(0.0)
    }

    pub fn fill(&self, rect: Rect, c: Color32) {
        self.painter.rect_filled(rect, 0.0, c);
    }
}

/// Momentary button: `sprite` normally, `{sprite}_p` while held.
pub fn button(ui: &mut Ui, sk: &Skinned, id: &str, layout: &str, sprite: &str) -> Response {
    let rect = sk.at(layout);
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    pressable(ui, sk, &resp, rect, sprite);
    resp
}

/// Draws a button's `sprite` for `resp` (`{sprite}_p` while held), pressed in by a skin pixel
/// while held and springing back when let go (see [`crate::motion::press`]).
pub fn pressable(ui: &Ui, sk: &Skinned, resp: &Response, rect: Rect, sprite: &str) {
    let down = resp.is_pointer_button_down_on();
    let name = if down {
        format!("{sprite}_p")
    } else {
        sprite.to_owned()
    };
    let dy = crate::motion::press(ui.ctx(), resp.id, down) * sk.scale;
    sk.sprite_in(&name, rect.translate(vec2(0.0, dy)));
}

/// Two-state toggle drawn with `{base}_on` / `{base}_off`.
pub fn toggle(ui: &mut Ui, sk: &Skinned, id: &str, layout: &str, base: &str, on: bool) -> Response {
    let rect = sk.at(layout);
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    lamp(ui, sk, &resp, rect, base, on, false);
    resp
}

/// How strongly a lit LCD segment glows past its edge.
const SEGMENT_GLOW: f32 = 0.1;

/// The segments `c` lights on a seven-segment digit.
pub fn segments(c: char) -> u8 {
    use crate::skin::generate::{DIGIT_MASKS, MINUS_MASK, QUESTION_MASK};
    match c {
        '0'..='9' => DIGIT_MASKS[c as usize - '0' as usize],
        '-' => MINUS_MASK,
        '?' => QUESTION_MASK,
        _ => 0,
    }
}

/// A seven-segment LCD digit `c` at (x, y) (skin pixels), for the place `slot` on the LCD: the
/// skin's sprite, with a faint glow around its lit segments, and the segments that just went
/// out fading over [`crate::motion::FAST`] instead of vanishing.
pub fn lcd_digit(ui: &Ui, sk: &Skinned, slot: Id, x: f32, y: f32, c: char) {
    let name = match c {
        ' ' => "digit_blank".to_owned(),
        '?' => "digit_q".to_owned(),
        '-' => "digit_minus".to_owned(),
        c => format!("digit_{c}"),
    };
    sk.sprite(&name, x, y);
    let mask = segments(c);
    let lit = color(sk.def.colors.lcd);
    let seg = |i: usize, grow: f32| {
        let (sx, sy, w, h) = crate::skin::generate::SEGMENTS[i];
        sk.rect(x + sx as f32, y + sy as f32, w as f32, h as f32)
            .expand(grow * sk.scale)
    };
    for i in (0..7).filter(|i| mask & (1 << i) != 0) {
        sk.fill(seg(i, 1.0), lit.gamma_multiply(SEGMENT_GLOW));
    }
    let (gone, left) = crate::motion::afterglow(ui.ctx(), slot, mask);
    if left > 0.0 {
        for i in (0..7).filter(|i| gone & (1 << i) != 0) {
            sk.fill(seg(i, 0.0), lit.gamma_multiply(left));
        }
    }
}

/// The glow a lamp gives off as it lights.
pub const GLOW: Color32 = Color32::from_rgb(255, 178, 72);

/// Draws a lamp or lit button for `resp`: `{base}_on` / `{base}_off` (with `_p` while held when
/// `pressable`). As it lights it blooms, a glow up to 3 skin pixels past its edge that settles
/// in [`crate::motion::MEDIUM`]; as it goes out its lit look fades over
/// [`crate::motion::FAST`].
pub fn lamp(
    ui: &Ui,
    sk: &Skinned,
    resp: &Response,
    rect: Rect,
    base: &str,
    on: bool,
    pressable: bool,
) {
    let (bloom, fade) = crate::motion::lamp(ui.ctx(), resp.id, on);
    if bloom > 0.0 {
        let shadow = egui::epaint::Shadow {
            offset: [0, 0],
            blur: (6.0 * sk.scale * bloom).round() as u8,
            spread: (3.0 * sk.scale * bloom).round() as u8,
            color: GLOW.gamma_multiply(0.7 * bloom),
        };
        sk.painter.add(shadow.as_shape(rect, 1.0 * sk.scale));
    }
    let state = if on { "on" } else { "off" };
    let down = pressable && resp.is_pointer_button_down_on();
    let dy = if pressable {
        crate::motion::press(ui.ctx(), resp.id, down) * sk.scale
    } else {
        0.0
    };
    let held = format!("{base}_{state}_p");
    let name = if down && sk.def.sprite(&held).w > 0 {
        held
    } else {
        format!("{base}_{state}")
    };
    let at = rect.translate(vec2(0.0, dy));
    sk.sprite_in(&name, at);
    if fade > 0.0 {
        sk.sprite_tinted(
            &format!("{base}_on"),
            at,
            Color32::WHITE.gamma_multiply(fade),
        );
    }
}

pub struct SliderSprites<'s> {
    pub track: &'s str,
    /// Drawn from the left edge up to the thumb center.
    pub fill: Option<&'s str>,
    pub thumb: &'s str,
}

/// Horizontal slider. Returns the response and the value (0..=1) the user dragged to, if any.
pub fn hslider(
    ui: &mut Ui,
    sk: &Skinned,
    id: &str,
    layout: &str,
    s: SliderSprites,
    value: f32,
) -> (Response, Option<f32>) {
    let r = sk.def.at(layout);
    let thumb = sk.def.sprite(s.thumb);
    let rect = sk.at(layout);
    let resp = ui.interact(rect, Id::new(id), Sense::click_and_drag());
    let travel = (r.w - thumb.w) as f32;
    let mut new = None;
    if let Some(p) = resp.interact_pointer_pos() {
        let px = (p.x - rect.left()) / sk.scale - thumb.w as f32 / 2.0;
        new = Some((px / travel).clamp(0.0, 1.0));
    }
    let v = new.unwrap_or(value).clamp(0.0, 1.0);
    let tx = r.x as f32 + (v * travel).round();
    // A track in three pieces (`{track}_l`, `{track}_m`, `{track}_r`) stretches to the layout's
    // width, with the fill stretched along it and shown up to the thumb; a one-piece track
    // is drawn as it is.
    let cap = sk.def.sprite(&format!("{}_l", s.track));
    if cap.w > 0 {
        let (ln, rn) = (format!("{}_l", s.track), format!("{}_r", s.track));
        let (lw, rw) = (cap.w as f32, sk.def.sprite(&rn).w as f32);
        let h = cap.h as f32;
        let (x0, y0) = (r.x as f32, r.y as f32);
        sk.sprite(&ln, x0, y0);
        sk.sprite_in(
            &format!("{}_m", s.track),
            sk.rect(x0 + lw, y0, r.w as f32 - lw - rw, h),
        );
        sk.sprite(&rn, x0 + r.w as f32 - rw, y0);
        if let Some(fill) = s.fill {
            let shown = tx - x0 + thumb.w as f32 / 2.0;
            let whole = sk.rect(x0, y0, r.w as f32, h);
            let part =
                Rect::from_min_size(whole.min, vec2(shown.max(0.0) * sk.scale, whole.height()));
            let clipped = Skinned {
                painter: sk
                    .painter
                    .with_clip_rect(part.intersect(sk.painter.clip_rect())),
                tex: sk.tex,
                def: sk.def,
                origin: sk.origin,
                scale: sk.scale,
            };
            clipped.sprite_in(fill, whole);
        }
    } else {
        sk.sprite(s.track, r.x as f32, r.y as f32);
        if let Some(fill) = s.fill {
            sk.sprite_left(
                fill,
                r.x as f32,
                r.y as f32,
                tx - r.x as f32 + thumb.w as f32 / 2.0,
            );
        }
    }
    let ty = r.y as f32 + ((r.h - thumb.h) as f32 / 2.0).floor();
    let name = if resp.is_pointer_button_down_on() {
        format!("{}_p", s.thumb)
    } else {
        s.thumb.to_owned()
    };
    sk.sprite(&name, tx, ty);
    (resp, new)
}

/// Vertical slider; value 1 is the top.
pub fn vslider(
    ui: &mut Ui,
    sk: &Skinned,
    id: &str,
    layout: &str,
    track: &str,
    thumb_name: &str,
    value: f32,
) -> (Response, Option<f32>) {
    let r = sk.def.at(layout);
    let thumb = sk.def.sprite(thumb_name);
    let rect = sk.at(layout);
    let resp = ui.interact(rect, Id::new(id), Sense::click_and_drag());
    let travel = (r.h - thumb.h) as f32;
    let mut new = None;
    if let Some(p) = resp.interact_pointer_pos() {
        let py = (p.y - rect.top()) / sk.scale - thumb.h as f32 / 2.0;
        new = Some(1.0 - (py / travel).clamp(0.0, 1.0));
    }
    let v = new.unwrap_or(value).clamp(0.0, 1.0);
    sk.sprite(track, r.x as f32, r.y as f32);
    let ty = r.y as f32 + ((1.0 - v) * travel).round();
    let tx = r.x as f32 + ((r.w - thumb.w) as f32 / 2.0).floor();
    let name = if resp.is_pointer_button_down_on() {
        format!("{thumb_name}_p")
    } else {
        thumb_name.to_owned()
    };
    sk.sprite(&name, tx, ty);
    (resp, new)
}

/// A draggable area; returns the drag delta in skin pixels this frame.
pub fn drag_area(ui: &mut Ui, sk: &Skinned, id: &str, layout_rect: Rect) -> (Response, Vec2) {
    let resp = ui.interact(layout_rect, Id::new(id), Sense::click_and_drag());
    let d = if resp.dragged() {
        resp.drag_delta() / sk.scale
    } else {
        Vec2::ZERO
    };
    (resp, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::LoadedSkin;
    use egui::{Event, PointerButton, RawInput};

    /// Runs one frame with `events`, drawing a button and a slider in the main section.
    fn frame(
        ctx: &egui::Context,
        skin: &LoadedSkin,
        events: Vec<Event>,
        value: f32,
    ) -> (bool, Option<f32>) {
        let mut out = (false, None);
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(550.0, 232.0))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let sk = Skinned {
                painter: ui.painter().clone(),
                tex: TextureId::default(),
                def: &skin.def,
                origin: Pos2::ZERO,
                scale: 2.0,
            };
            out.0 = button(ui, &sk, "close", "eq_close", "btn_close").clicked();
            let s = SliderSprites {
                track: "volume_track",
                fill: Some("volume_fill"),
                thumb: "volume_thumb",
            };
            out.1 = hslider(ui, &sk, "vol", "volume", s, value).1;
        });
        output.textures_delta.clear(); // no GPU in tests
        out
    }

    fn press(p: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }
    }

    #[test]
    fn skinned_button_clicks_at_scaled_layout_position() {
        let ctx = egui::Context::default();
        let skin = LoadedSkin::default_skin();
        let close = skin.def.at("eq_close"); // (262, 3) in skin pixels → ×2 in points
        let p = pos2((close.x as f32 + 4.0) * 2.0, (close.y as f32 + 4.0) * 2.0);
        frame(&ctx, &skin, vec![Event::PointerMoved(p)], 0.5);
        frame(&ctx, &skin, vec![press(p, true)], 0.5);
        let (clicked, _) = frame(&ctx, &skin, vec![press(p, false)], 0.5);
        assert!(clicked, "click inside the scaled button rect");

        let miss = pos2(1.0, 1.0);
        frame(
            &ctx,
            &skin,
            vec![Event::PointerMoved(miss), press(miss, true)],
            0.5,
        );
        let (clicked, _) = frame(&ctx, &skin, vec![press(miss, false)], 0.5);
        assert!(!clicked);
    }

    #[test]
    fn slider_drag_maps_pointer_to_value() {
        let ctx = egui::Context::default();
        let skin = LoadedSkin::default_skin();
        let v = skin.def.at("volume");
        let thumb_w = skin.def.sprite("volume_thumb").w as f32;
        let y = (v.y as f32 + 6.0) * 2.0;
        // Press at the far right of the track: value 1.
        let right = pos2((v.x + v.w) as f32 * 2.0 - 1.0, y);
        frame(&ctx, &skin, vec![Event::PointerMoved(right)], 0.0);
        let (_, value) = frame(&ctx, &skin, vec![press(right, true)], 0.0);
        assert_eq!(value, Some(1.0));
        // Drag to the middle of the travel: value 0.5.
        let mid = pos2(
            (v.x as f32 + thumb_w / 2.0 + (v.w as f32 - thumb_w) / 2.0) * 2.0,
            y,
        );
        let (_, value) = frame(&ctx, &skin, vec![Event::PointerMoved(mid)], 1.0);
        assert!((value.unwrap() - 0.5).abs() < 0.02, "{value:?}");
        frame(&ctx, &skin, vec![press(mid, false)], 0.5);
        let (_, value) = frame(&ctx, &skin, vec![], 0.5);
        assert_eq!(value, None, "no change without interaction");
    }
}
