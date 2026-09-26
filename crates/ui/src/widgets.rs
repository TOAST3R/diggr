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
    let name = if resp.is_pointer_button_down_on() {
        format!("{sprite}_p")
    } else {
        sprite.to_owned()
    };
    sk.sprite_in(&name, rect);
    resp
}

/// Two-state toggle drawn with `{base}_on` / `{base}_off`.
pub fn toggle(ui: &mut Ui, sk: &Skinned, id: &str, layout: &str, base: &str, on: bool) -> Response {
    let rect = sk.at(layout);
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    sk.sprite_in(&format!("{base}_{}", if on { "on" } else { "off" }), rect);
    resp
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
    sk.sprite(s.track, r.x as f32, r.y as f32);
    if let Some(fill) = s.fill {
        sk.sprite_left(
            fill,
            r.x as f32,
            r.y as f32,
            tx - r.x as f32 + thumb.w as f32 / 2.0,
        );
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
            out.0 = button(ui, &sk, "play", "play", "play").clicked();
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
        let play = skin.def.at("play"); // (39, 88) in skin pixels → ×2 in points
        let p = pos2((play.x as f32 + 5.0) * 2.0, (play.y as f32 + 5.0) * 2.0);
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
