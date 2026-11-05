//! The record card: what a click on a record row opens in a grouped crate. Its cover beside the
//! record's names, pressing, styles and tracklist, with its marks and what's for sale along the
//! bottom, painted in the skin's playlist colours. Each track plays and pauses from its line,
//! and ‹ › step to the previous and next record.
//!
//! The text comes ready from [`crate::format::record_details`] and the pills from the app; this
//! lays them out in an area centred over the window and says what was clicked. The app keeps
//! which record's card is open, and what each pill does.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

use super::CoverSlot;
use crate::format::{CardTrackState, RecordCard};
use crate::playlist::EntryId;
use crate::skin::Colors;
use crate::widgets::color;

/// The cover square's side at 1×, in points.
pub const COVER: f32 = 112.0;
/// The card's widest, at 1× (narrower in a narrower window).
pub const MAX_W: f32 = 420.0;
/// How long the slide to another record takes, in seconds.
pub const SLIDE: f32 = 0.22;
/// How far the content starts from its place when it slides in, at 1×.
const SLIDE_FROM: f32 = 28.0;
/// How dark the layer over the window is, out of 255.
const SCRIM: u8 = 150;
/// Tracklist lines shown before it scrolls.
pub const LINES: usize = 8;
const PAD: f32 = 8.0;
const GAP: f32 = 10.0;
/// A tracklist line's height at 1×.
const LINE: f32 = 14.0;

/// What the cover square shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardImage {
    /// A cover (large or thumbnail), at its own size: fitted into the square.
    Image(egui::TextureId, Vec2),
    /// Its space, empty until a cover arrives.
    Waiting,
    /// No image: a record.
    Icon,
}

/// The large cover when it's loaded, else the thumbnail (scaled up), else an empty square while
/// either is on its way, else the record icon.
pub fn pick(thumb: Option<CoverSlot>, large: Option<CoverSlot>) -> CardImage {
    match (large, thumb) {
        (Some(CoverSlot::Loaded(t, s)), _) | (_, Some(CoverSlot::Loaded(t, s))) => {
            CardImage::Image(t, s)
        }
        (Some(CoverSlot::Waiting), _) | (_, Some(CoverSlot::Waiting)) => CardImage::Waiting,
        (None, None) => CardImage::Icon,
    }
}

/// What a click in the card asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardAction {
    /// Play (or arm) this track from its start.
    Play(EntryId),
    /// Resume the paused track.
    Resume,
    Pause,
    /// The previous (-1) or next (1) record.
    Step(i8),
    /// The pill at this index of [`Card::pills`].
    Pill(usize),
    /// Seek the playing track to this many seconds.
    Seek(f64),
    Close,
}

/// One of the record's pills (the record row's actions), as the app describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct CardPill {
    pub label: String,
    /// While the pointer is over it (IN CART reads REMOVE), when it says something else.
    pub hover_label: Option<String>,
    pub tip: String,
    /// Lit: on the wantlist, in a cart.
    pub lit: bool,
    /// A badge (OWNED, SOLD) only says; a button acts.
    pub button: bool,
}

/// The card's colours, from the skin.
struct Ink {
    bg: Color32,
    text: Color32,
    dim: Color32,
    faint: Color32,
    hl: Color32,
    pill: Color32,
    owned: Color32,
}

impl Ink {
    fn of(c: &Colors) -> Self {
        let mix = |a: [u8; 3], b: [u8; 3], t: f32| {
            let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
            Color32::from_rgb(l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2]))
        };
        Self {
            bg: color(c.pl_bg),
            text: color(c.pl_text),
            dim: mix(c.pl_text, c.pl_bg, 0.45),
            faint: mix(c.pl_text, c.pl_bg, 0.7),
            hl: color(c.pl_current),
            pill: color(c.pl_selected_bg),
            owned: color(c.pl_owned),
        }
    }
}

/// The open card, for [`show`].
pub struct Card<'a> {
    pub card: &'a RecordCard,
    pub image: CardImage,
    /// There's a record before / after it.
    pub prev: bool,
    pub next: bool,
    pub pills: Vec<CardPill>,
    /// The record's playing (or paused) track: elapsed and total seconds.
    pub progress: Option<(f64, f64)>,
    /// Seconds since the card moved to this record, and which way (-1 back, 1 on); `None`
    /// once it has settled.
    pub slide: Option<(f32, i8)>,
}

/// The slide's state `elapsed` seconds in: how far the content still is from its place (1 at
/// the start, 0 settled) and its opacity. Eased out, so it arrives softly.
pub fn ease(elapsed: f32) -> (f32, f32) {
    let t = (elapsed / SLIDE).clamp(0.0, 1.0);
    let e = 1.0 - (1.0 - t).powi(3);
    (1.0 - e, 0.25 + 0.75 * e)
}

/// Shows the card centred over the window at the skin's `scale`, over a dark layer that takes
/// every click, drag and hover outside it: only the card answers the pointer while it's open.
/// Returns the card's rect and what was clicked in it.
pub fn show(
    ctx: &egui::Context,
    card: &Card,
    colors: &Colors,
    scale: f32,
) -> (Rect, Vec<CardAction>) {
    let ink = Ink::of(colors);
    let mut actions = Vec::new();
    let width = width(scale, ctx.content_rect().width());
    let screen = ctx.content_rect();
    egui::Area::new(Id::new("record_card_scrim"))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .constrain(false)
        .show(ctx, |ui| {
            let (r, _) = ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
            ui.painter()
                .rect_filled(r, 0.0, Color32::from_black_alpha(SCRIM));
        });
    let resp = egui::Area::new(Id::new("record_card"))
        .order(egui::Order::Foreground)
        .fixed_pos(ctx.content_rect().center())
        .pivot(Align2::CENTER_CENTER)
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(ink.bg)
                .stroke(Stroke::new(1.0, ink.faint))
                .inner_margin(PAD * scale)
                .corner_radius(2.0 * scale)
                .show(ui, |ui| {
                    // A record change slides its content in from that side, fading in; the frame
                    // stays put.
                    let (off, opacity) = card.slide.map_or((0.0, 1.0), |(t, _)| ease(t));
                    let dir = card.slide.map_or(0.0, |(_, d)| f32::from(d));
                    let shift = egui::emath::TSTransform::from_translation(vec2(
                        dir * off * SLIDE_FROM * scale,
                        0.0,
                    ));
                    ui.with_visual_transform(shift, |ui| {
                        ui.set_opacity(opacity);
                        draw(ui, card, width, &ink, scale, &mut actions);
                    });
                });
        });
    if card.slide.is_some() {
        ctx.request_repaint();
    }
    (resp.response.rect, actions)
}

/// The card's width inside its frame: [`MAX_W`] at the skin's scale, or the window's width
/// when that's narrower.
pub fn width(scale: f32, window: f32) -> f32 {
    (MAX_W * scale).min(window - 2.0) - 2.0 * PAD * scale
}

fn draw(ui: &mut Ui, view: &Card, width: f32, ink: &Ink, s: f32, actions: &mut Vec<CardAction>) {
    let card = view.card;
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let side = COVER * s;
    let text_w = (width - side - GAP * s).max(60.0 * s);
    let title = FontId::proportional(15.0 * s);
    let sub = FontId::proportional(12.0 * s);
    let body = FontId::proportional(9.5 * s);
    let small = FontId::proportional(8.5 * s);
    ui.horizontal_top(|ui| {
        let (square, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
        draw_cover(ui.painter(), square, view.image, ink, s);
        ui.add_space(GAP * s);
        ui.vertical(|ui| {
            ui.set_width(text_w);
            header(ui, view, text_w, title, ink, s, actions);
            // The album, the rule, the pressing.
            let (r, _) = ui.allocate_exact_size(vec2(text_w, 17.0 * s), Sense::hover());
            ui.painter().with_clip_rect(r).text(
                r.left_top(),
                Align2::LEFT_TOP,
                &card.album,
                sub,
                ink.hl,
            );
            let (r, _) = ui.allocate_exact_size(vec2(text_w, 8.0 * s), Sense::hover());
            ui.painter().line_segment(
                [r.left_center(), r.right_center()],
                Stroke::new(s.max(1.0), ink.faint),
            );
            if !card.pressing.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(text_w, 14.0 * s), Sense::hover());
                ui.painter().with_clip_rect(r).text(
                    r.left_top(),
                    Align2::LEFT_TOP,
                    &card.pressing,
                    body.clone(),
                    ink.dim,
                );
            }
            // The styles, as pills.
            let pills = pill_rows(ui.painter(), &card.styles, &small, text_w, s);
            if let Some(h) = pills.last().map(|p| p.1.bottom() + 6.0 * s) {
                let (r, _) = ui.allocate_exact_size(vec2(text_w, h), Sense::hover());
                for (galley, at) in pills {
                    let p = at.translate(r.min.to_vec2());
                    ui.painter().rect_filled(p, p.height() / 2.0, ink.pill);
                    ui.painter()
                        .galley(p.min + vec2(5.0 * s, 1.5 * s), galley, ink.text);
                }
            }
            // The tracklist: always LINES high, so every record's card is the same size;
            // scrolling past that.
            let list_h = LINES as f32 * LINE * s;
            egui::ScrollArea::vertical()
                .id_salt(("card_tracks", card.album.as_str()))
                .max_height(list_h)
                .min_scrolled_height(list_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for t in &card.tracks {
                        track_line(ui, t, text_w, &body, ink, s, actions);
                    }
                });
            seek_bar(ui, view.progress, text_w, &small, ink, s, actions);
        });
    });
    if card.has_strip() || !view.pills.is_empty() {
        strip(ui, view, &body, &small, ink, s, actions);
    }
}

/// A thin position bar for the record's playing track, with its elapsed and total time: a click
/// or a drag (on release) seeks it. Empty, and deaf, when none of its tracks plays.
fn seek_bar(
    ui: &mut Ui,
    progress: Option<(f64, f64)>,
    width: f32,
    font: &FontId,
    ink: &Ink,
    s: f32,
    actions: &mut Vec<CardAction>,
) {
    let (r, _) = ui.allocate_exact_size(vec2(width, 16.0 * s), Sense::hover());
    let times_w = 62.0 * s;
    let bar = Rect::from_min_max(
        pos2(r.left() + 2.0 * s, r.center().y - 4.0 * s),
        pos2(r.right() - times_w, r.center().y + 4.0 * s),
    );
    let line = Rect::from_center_size(bar.center(), vec2(bar.width(), 2.0 * s));
    ui.painter().rect_filled(line, s, ink.faint);
    let Some((at, total)) = progress.filter(|(_, t)| *t > 0.0) else {
        return;
    };
    let resp = ui
        .interact(bar, Id::new("card_seek"), Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let frac_at = |x: f32| ((x - bar.left()) / bar.width()).clamp(0.0, 1.0);
    // While dragging, the knob follows the pointer; the seek happens on release.
    let shown = match resp.interact_pointer_pos().filter(|_| resp.dragged()) {
        Some(p) => frac_at(p.x),
        None => (at / total).clamp(0.0, 1.0) as f32,
    };
    if (resp.clicked() || resp.drag_stopped())
        && let Some(p) = resp.interact_pointer_pos()
    {
        actions.push(CardAction::Seek(f64::from(frac_at(p.x)) * total));
    }
    let x = bar.left() + shown * bar.width();
    let played = Rect::from_min_max(line.min, pos2(x, line.max.y));
    ui.painter().rect_filled(played, s, ink.hl);
    let knob = if resp.hovered() || resp.dragged() {
        4.0
    } else {
        3.0
    };
    ui.painter()
        .circle_filled(pos2(x, line.center().y), knob * s, ink.hl);
    let time = format!(
        "{} / {}",
        crate::format::clock(f64::from(shown) * total),
        crate::format::clock(total)
    );
    ui.painter().text(
        pos2(r.right(), r.center().y),
        Align2::RIGHT_CENTER,
        time,
        font.clone(),
        ink.dim,
    );
}

/// The artist, with ‹ › at the right.
fn header(
    ui: &mut Ui,
    view: &Card,
    width: f32,
    font: FontId,
    ink: &Ink,
    s: f32,
    actions: &mut Vec<CardAction>,
) {
    let (r, _) = ui.allocate_exact_size(vec2(width, 19.0 * s), Sense::hover());
    // ‹ › then × at the right.
    let arrows_w = 51.0 * s;
    let artist = match view.card.artist.as_str() {
        "" => "Unknown artist",
        a => a,
    };
    ui.painter()
        .with_clip_rect(Rect::from_min_max(
            r.min,
            pos2(r.right() - arrows_w, r.bottom()),
        ))
        .text(r.left_top(), Align2::LEFT_TOP, artist, font, ink.text);
    let close = Rect::from_min_size(
        pos2(r.right() - arrows_w / 3.0, r.top()),
        vec2(arrows_w / 3.0, r.height()),
    );
    let resp = ui.interact(close, Id::new("card_close"), Sense::click());
    let c = if resp.hovered() { ink.hl } else { ink.dim };
    ui.painter().text(
        close.center(),
        Align2::CENTER_CENTER,
        "×",
        FontId::proportional(14.0 * s),
        c,
    );
    if resp.on_hover_text("Close (Esc)").clicked() {
        actions.push(CardAction::Close);
    }
    for (i, (mark, step, on)) in [("‹", -1, view.prev), ("›", 1, view.next)]
        .into_iter()
        .enumerate()
    {
        let hit = Rect::from_min_size(
            pos2(r.right() - arrows_w + i as f32 * arrows_w / 3.0, r.top()),
            vec2(arrows_w / 3.0, r.height()),
        );
        let resp = ui.interact(hit, Id::new(("card_step", step)), Sense::click());
        let c = match (on, resp.hovered()) {
            (false, _) => ink.faint,
            (true, true) => ink.hl,
            (true, false) => ink.text,
        };
        ui.painter().text(
            hit.center(),
            Align2::CENTER_CENTER,
            mark,
            FontId::proportional(16.0 * s),
            c,
        );
        let tip = if step < 0 {
            "Previous record (left arrow)"
        } else {
            "Next record (right arrow)"
        };
        if on && resp.on_hover_text(tip).clicked() {
            actions.push(CardAction::Step(step));
        }
    }
}

fn track_line(
    ui: &mut Ui,
    t: &crate::format::CardTrack,
    width: f32,
    body: &FontId,
    ink: &Ink,
    s: f32,
    actions: &mut Vec<CardAction>,
) {
    let (r, _) = ui.allocate_exact_size(vec2(width, LINE * s), Sense::hover());
    let mid = r.center().y;
    let btn_w = 16.0 * s;
    let side_w = 26.0 * s;
    let bpm_w = 40.0 * s;
    let painter = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
    let current = matches!(t.state, CardTrackState::Playing | CardTrackState::Paused);
    let c = match t.state {
        CardTrackState::Playing | CardTrackState::Paused => ink.hl,
        CardTrackState::Dim | CardTrackState::Retry | CardTrackState::Unavailable => ink.faint,
        CardTrackState::Normal => ink.text,
    };
    if current {
        painter.rect_filled(r, 0.0, ink.pill.gamma_multiply(0.5));
    }
    // ⏵ / ⏸, but nothing to play on an unavailable track.
    if t.state != CardTrackState::Unavailable {
        let hit = Rect::from_min_size(r.min, vec2(btn_w, r.height()));
        let resp = ui.interact(hit, Id::new(("card_play", t.id)), Sense::click());
        let mark = if t.state == CardTrackState::Playing {
            "⏸"
        } else {
            "⏵"
        };
        // Waiting tracks play too (fetched first): their ⏵ looks like any other.
        let bc = if resp.hovered() || current {
            ink.hl
        } else {
            ink.text
        };
        painter.text(hit.center(), Align2::CENTER_CENTER, mark, body.clone(), bc);
        let tip = match t.state {
            CardTrackState::Playing => "Pause",
            CardTrackState::Paused => "Resume",
            CardTrackState::Dim => "Play: its preview is fetched first",
            CardTrackState::Retry => "Try it again and play it",
            _ => "Play",
        };
        if resp.on_hover_text(tip).clicked() {
            actions.push(match t.state {
                CardTrackState::Playing => CardAction::Pause,
                CardTrackState::Paused => CardAction::Resume,
                _ => CardAction::Play(t.id),
            });
        }
    }
    let x = r.left() + btn_w;
    let side_c = if current { ink.hl } else { ink.dim };
    painter.text(
        pos2(x, mid),
        Align2::LEFT_CENTER,
        &t.side,
        body.clone(),
        side_c,
    );
    let title_r = Rect::from_min_max(
        pos2(x + side_w, r.top()),
        pos2(r.right() - bpm_w, r.bottom()),
    );
    painter
        .with_clip_rect(title_r.intersect(painter.clip_rect()))
        .text(
            pos2(title_r.left(), mid),
            Align2::LEFT_CENTER,
            &t.title,
            body.clone(),
            c,
        );
    // Its tempo, or how its audio is coming.
    let end = t.note.clone().or_else(|| t.bpm.map(|b| b.to_string()));
    if let Some(end) = end {
        let ec = if t.note.as_deref() == Some("next") {
            ink.hl
        } else {
            side_c
        };
        painter.text(
            pos2(r.right() - 2.0 * s, mid),
            Align2::RIGHT_CENTER,
            end,
            body.clone(),
            ec,
        );
    }
}

/// The record's pills, what's for sale and the counts, under the cover and the column.
fn strip(
    ui: &mut Ui,
    view: &Card,
    body: &FontId,
    small: &FontId,
    ink: &Ink,
    s: f32,
    actions: &mut Vec<CardAction>,
) {
    let card = view.card;
    let w = ui.min_rect().width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0 * s), Sense::hover());
    let painter = ui.painter().with_clip_rect(r);
    let top = r.top() + 5.0 * s;
    painter.line_segment(
        [
            pos2(r.left(), top - 1.0 * s),
            pos2(r.right(), top - 1.0 * s),
        ],
        Stroke::new(s.max(1.0), ink.faint),
    );
    let mid = top + 10.0 * s;
    let mut at = r.left();
    for (i, p) in view.pills.iter().enumerate() {
        let measure = |t: &str| painter.layout_no_wrap(t.to_owned(), small.clone(), ink.text);
        let g = measure(&p.label);
        let wide = p
            .hover_label
            .as_deref()
            .map_or(g.size().x, |h| g.size().x.max(measure(h).size().x));
        let b = Rect::from_min_size(
            pos2(at, mid - g.size().y / 2.0 - 2.0 * s),
            vec2(wide + 10.0 * s, g.size().y + 4.0 * s),
        );
        let sense = if p.button {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut resp = ui
            .interact(b, Id::new(("card_pill", i)), sense)
            .on_hover_text(&p.tip);
        if p.button {
            resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                actions.push(CardAction::Pill(i));
            }
        }
        let hovered = p.button && resp.hovered();
        let label = match (&p.hover_label, hovered) {
            (Some(h), true) => h.as_str(),
            _ => p.label.as_str(),
        };
        let c = match (p.button, p.lit) {
            (false, _) if p.label == "OWNED" => ink.owned,
            (false, _) => ink.dim,
            (true, true) => ink.bg,
            (true, false) => ink.hl,
        };
        if p.button && p.lit {
            painter.rect_filled(b, b.height() / 2.0, ink.hl);
        } else {
            painter.rect_stroke(
                b,
                b.height() / 2.0,
                Stroke::new(s.max(1.0), c),
                egui::StrokeKind::Inside,
            );
        }
        painter.text(b.center(), Align2::CENTER_CENTER, label, small.clone(), c);
        at = b.right() + 5.0 * s;
    }
    at += 3.0 * s;
    let words = [card.sale.as_deref(), card.counts.as_deref()];
    for (i, w) in words.into_iter().flatten().enumerate() {
        let c = if i == 0 && card.sale.is_some() {
            ink.text
        } else {
            ink.dim
        };
        let t = painter.text(pos2(at, mid), Align2::LEFT_CENTER, w, body.clone(), c);
        at = t.right() + 10.0 * s;
    }
}

/// The style pills, wrapped to `width`: each galley with its rect relative to the pills' top
/// left.
fn pill_rows(
    painter: &egui::Painter,
    styles: &[String],
    font: &FontId,
    width: f32,
    s: f32,
) -> Vec<(std::sync::Arc<egui::Galley>, Rect)> {
    let mut out = Vec::new();
    let mut at = Pos2::ZERO;
    let h = font.size + 3.0 * s;
    for st in styles {
        let g = painter.layout_no_wrap(st.clone(), font.clone(), Color32::PLACEHOLDER);
        let w = g.size().x + 10.0 * s;
        if at.x > 0.0 && at.x + w > width {
            at = pos2(0.0, at.y + h + 4.0 * s);
        }
        out.push((g, Rect::from_min_size(at, vec2(w.min(width), h))));
        at.x += w + 4.0 * s;
    }
    out
}

fn draw_cover(painter: &egui::Painter, square: Rect, image: CardImage, ink: &Ink, s: f32) {
    match image {
        CardImage::Image(tex, size) => {
            // Fitted into the square, centred, its shape kept: the square never changes size.
            let fit = size * (square.width() / size.x.max(size.y).max(1.0));
            painter.image(
                tex,
                Rect::from_center_size(square.center(), fit),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        CardImage::Waiting => {
            painter.rect_stroke(
                square,
                2.0 * s,
                Stroke::new(s.max(1.0), ink.faint),
                egui::StrokeKind::Inside,
            );
        }
        CardImage::Icon => {
            let c = square.center();
            let r = square.width() * 0.42;
            painter.circle_filled(c, r, ink.faint.gamma_multiply(0.35));
            for k in [0.92, 0.78, 0.64] {
                painter.circle_stroke(c, r * k, Stroke::new(s.max(1.0), ink.faint));
            }
            painter.circle_filled(c, r * 0.3, ink.pill);
            painter.circle_filled(c, r * 0.04, ink.bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_large_cover_wins_then_the_thumbnail_then_waiting_then_the_icon() {
        let tex = |n| egui::TextureId::Managed(n);
        let big = CoverSlot::Loaded(tex(1), vec2(600.0, 600.0));
        let thumb = CoverSlot::Loaded(tex(2), vec2(150.0, 150.0));
        assert_eq!(
            pick(Some(thumb), Some(big)),
            CardImage::Image(tex(1), vec2(600.0, 600.0))
        );
        assert_eq!(
            pick(Some(thumb), Some(CoverSlot::Waiting)),
            CardImage::Image(tex(2), vec2(150.0, 150.0)),
            "the thumbnail, scaled up, while the large one comes"
        );
        assert_eq!(
            pick(Some(thumb), None),
            CardImage::Image(tex(2), vec2(150.0, 150.0))
        );
        assert_eq!(pick(Some(CoverSlot::Waiting), None), CardImage::Waiting);
        assert_eq!(pick(None, None), CardImage::Icon);
    }

    #[test]
    fn a_cover_fills_the_same_square_whatever_its_size() {
        let fit = |size: Vec2, side: f32| size * (side / size.x.max(size.y).max(1.0));
        assert_eq!(fit(vec2(150.0, 150.0), COVER), vec2(112.0, 112.0));
        assert_eq!(fit(vec2(600.0, 600.0), COVER), vec2(112.0, 112.0));
        assert_eq!(fit(vec2(600.0, 300.0), COVER), vec2(112.0, 56.0));
        assert_eq!(
            fit(vec2(600.0, 600.0), COVER * 2.0),
            vec2(224.0, 224.0),
            "2×"
        );
    }

    #[test]
    fn a_slide_starts_off_to_the_side_and_settles_softly() {
        let (off, op) = ease(0.0);
        assert_eq!((off, op), (1.0, 0.25), "off to the side, faint");
        let (off, op) = ease(SLIDE / 2.0);
        assert!(
            off < 0.5 && op > 0.6,
            "most of the way after half the time: {off} {op}"
        );
        assert_eq!(ease(SLIDE), (0.0, 1.0), "in place");
        assert_eq!(ease(5.0), (0.0, 1.0));
    }

    #[test]
    fn the_card_is_420_wide_or_the_window_when_narrower() {
        assert_eq!(width(1.0, 1200.0), MAX_W - 2.0 * PAD);
        assert_eq!(width(2.0, 1200.0), 2.0 * MAX_W - 4.0 * PAD, "2×");
        assert_eq!(
            width(1.0, 275.0),
            273.0 - 2.0 * PAD,
            "a classic-size window"
        );
    }
}
