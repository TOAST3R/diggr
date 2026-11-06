//! The record card's sleeve in the real app: its vinyl resting and coming out as its record
//! plays, its stickers, and the text kept clear of it. Covers come from fake images (a red
//! JPEG). Each `Rig` frame is 1/60 s of egui time.

use super::*;
use crate::playlist::ForSale;

/// The vinyl's fill (see `card::draw_disc`).
const VINYL: Color32 = Color32::from_rgb(16, 16, 16);

/// How far the vinyl's edge shows past the card's cover, as drawn in `out`.
fn disc_out(rig: &Rig, out: &egui::FullOutput) -> f32 {
    let cover = rig.app.card_cover.expect("a card is open");
    let (c, r, _) = circles(out)
        .into_iter()
        .find(|(c, _, fill)| *fill == VINYL && (c.y - cover.center().y).abs() < 0.5)
        .unwrap_or_else(|| panic!("no vinyl beside {cover:?}"));
    c.x + r - cover.right()
}

fn frames(rig: &mut Rig, n: usize) -> egui::FullOutput {
    let mut out = rig.frame(Vec::new());
    for _ in 1..n {
        out = rig.frame(Vec::new());
    }
    out
}

/// Three records with covers, grouped: "Glasshouse EP" (release 12: A1 and A2, which play,
/// and 6 for sale from €9.00), "Lowlight" (13) and "Undertow EP" (14), each cover loaded.
fn sleeve_rig(name: &str, fakes: &Fakes) -> Rig {
    let mut rig = records_rig(name, fakes);
    for release in [12, 13, 14] {
        rig.until(|r| has_cover(r, release), "the covers in view");
    }
    rig.frame(Vec::new());
    rig
}

/// [`sleeve_rig`]'s records, their covers maybe not loaded yet.
fn records_rig(name: &str, fakes: &Fakes) -> Rig {
    let mut rig = rig(name, fakes, |_| {});
    add_records(&mut rig);
    rig
}

fn add_records(rig: &mut Rig) {
    rig.screen = vec2(480.0, 900.0);
    let tracks = [
        (12, "A1", "Tidepool", "Glasshouse EP", "tone.flac"),
        (12, "A2", "Undertow", "Glasshouse EP", "tone.wav"),
        (13, "A", "Lowlight", "Lowlight", "tone.ogg"),
        (14, "A", "Shoal", "Undertow EP", ""),
    ];
    for (release, side, title, album, file) in tracks {
        let for_sale = (release == 12).then(|| ForSale {
            count: 6,
            lowest_cents: Some(900),
            currency: "EUR".into(),
            fetched_at: 0,
        });
        let id = rig.app.crates.shown_mut().add_waiting(
            "Nightcraft",
            title,
            None,
            Some(Origin {
                release: Some(release),
                album: album.into(),
                artist: "Nightcraft".into(),
                position: side.into(),
                cover: format!("https://i.discogs.com/cover-{release}.jpeg"),
                for_sale,
                ..Default::default()
            }),
            "queued",
        );
        if !file.is_empty() {
            let t = TrackRef::new(fixture(file).to_string_lossy());
            rig.app.set_audio(PLAYLIST, id, t);
        }
    }
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::ToggleGrouped, &ctx);
    rig.frame(Vec::new());
}

/// The ids of release 12's entries.
fn glasshouse(rig: &Rig) -> Vec<EntryId> {
    rig.app
        .crates
        .shown()
        .entries()
        .iter()
        .filter(|e| e.origin.as_ref().and_then(|o| o.release) == Some(12))
        .map(|e| e.id)
        .collect()
}

fn count(out: &egui::FullOutput, text: &str) -> usize {
    texts(out).iter().filter(|t| t.text == text).count()
}

/// The vinyl as its record plays: it rests 6 points out, slides to 16 while a track of it
/// plays, stays still while paused (with no text over it), and slides back on stop. These play
/// real audio, so they're few and each checks several things.
#[test]
fn the_vinyl_comes_out_while_its_record_plays_and_goes_back_on_stop() {
    let mut rig = sleeve_rig("sleeve-vinyl", &Fakes::new());
    rig.app.settings.reduce_motion = false;
    rig.click(rig.row(0));
    // Past the card's opening motion.
    let out = frames(&mut rig, 15);
    let rest = disc_out(&rig, &out);
    assert!((rest - card::DISC_REST).abs() < 0.5, "at rest: {rest}");
    rig.click(card_button(&out, "Tidepool"));
    rig.until(|r| r.app.position.state == PlayState::Playing, "A1 plays");
    let out = frames(&mut rig, 12);
    let played = disc_out(&rig, &out);
    assert!((played - card::DISC_OUT).abs() < 0.5, "out: {played}");
    // Clear of the disc: nothing of the card beside the cover reaches into it.
    let cover = rig.app.card_cover.unwrap();
    let edge = cover.right() + played;
    for t in texts(&out) {
        let beside = t.rect.center().x > cover.right()
            && t.rect.bottom() > cover.top()
            && t.rect.top() < cover.bottom();
        if beside {
            assert!(t.rect.left() >= edge - 0.5, "{:?} at {:?}", t.text, t.rect);
        }
    }
    // Paused counts as playing: out, and still frame after frame. (The rig's tracks keep it
    // repainting anyway, so this checks that the vinyl settles and asks for no motion.)
    let ctx = rig.ctx.clone();
    rig.app.apply(Action::Pause, &ctx);
    rig.until(|r| r.app.position.state == PlayState::Paused, "paused");
    let out = frames(&mut rig, 15);
    let settled = disc_out(&rig, &out);
    assert!((settled - card::DISC_OUT).abs() < 0.01, "paused: {settled}");
    for _ in 0..3 {
        let out = rig.frame(Vec::new());
        assert_eq!(disc_out(&rig, &out), settled, "still");
        assert!(!rig.app.motion.animating());
    }
    rig.app.apply(Action::Stop, &ctx);
    rig.until(|r| r.app.position.state == PlayState::Stopped, "stopped");
    let out = rig.frame(Vec::new());
    let going = disc_out(&rig, &out);
    assert!(going < card::DISC_OUT - 0.1, "sliding back: {going}");
    let out = frames(&mut rig, 12);
    let back = disc_out(&rig, &out);
    assert!((back - card::DISC_REST).abs() < 0.5, "back in: {back}");
}

#[test]
fn with_reduce_motion_nothing_moves_but_the_sleeve_is_all_there() {
    let mut rig = sleeve_rig("sleeve-reduced", &Fakes::new());
    rig.app.settings.reduce_motion = true;
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::Dig(DigAction::Want(glasshouse(&rig))), &ctx);
    rig.frame(Vec::new());
    rig.click_text("Later");
    rig.click(rig.row(0));
    assert!(
        rig.app.card.as_ref().unwrap().from.is_some(),
        "kept, but nothing flies"
    );
    let out = rig.frame(Vec::new());
    // No flight: the cover is in its square at once; the stickers and the vinyl are drawn.
    let cover = rig.app.card_cover.unwrap();
    let covers = card_covers(&rig, &out);
    assert!(
        covers.len() == 1 && (covers[0].center() - cover.center()).length() < 1.0,
        "in place at once: {covers:?}"
    );
    assert!(shows(&out, "€9"), "{:?}", text_list(&out));
    assert_eq!(count(&out, "WANTED"), 2, "pill and sticker");
    assert!(disc_out(&rig, &out) > 0.0);
    // The vinyl is out at once when the record plays.
    rig.click(card_button(&out, "Tidepool"));
    rig.until(|r| r.app.position.state == PlayState::Playing, "A1 plays");
    let out = rig.frame(Vec::new());
    let played = disc_out(&rig, &out);
    assert!(
        (played - card::DISC_OUT).abs() < 0.5,
        "out at once: {played}"
    );
    // A step draws the next cover flat, in place, in the next frame.
    rig.key(Key::ArrowRight, Modifiers::NONE);
    let out = rig.frame(Vec::new());
    assert_eq!(pictures(&out).1, 0, "no flip");
    let covers = card_covers(&rig, &out);
    assert!(
        covers.len() == 1 && (covers[0].center() - cover.center()).length() < 1.0,
        "{covers:?}"
    );
}

#[test]
fn the_price_sticker_reads_the_cheapest_and_takes_no_click() {
    let mut rig = sleeve_rig("sleeve-price", &Fakes::new());
    rig.click(rig.row(0));
    let out = rig.frame(Vec::new());
    let cover = rig.app.card_cover.unwrap();
    let sticker = texts(&out)
        .into_iter()
        .find(|t| t.text == "€9")
        .unwrap_or_else(|| panic!("no price sticker: {:?}", text_list(&out)));
    assert!(
        cover.expand(4.0).contains(sticker.rect.center())
            && sticker.rect.center().x > cover.center().x
            && sticker.rect.center().y < cover.center().y,
        "on the cover's top-right corner: {:?} {cover:?}",
        sticker.rect
    );
    // A click on it does what a click on the cover does: nothing.
    let selected = rig.app.crates.shown().selected_ids().len();
    rig.click(sticker.rect.center());
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(12)), "still open");
    assert_eq!(rig.app.position.state, PlayState::Stopped, "nothing plays");
    assert_eq!(rig.app.crates.shown().selected_ids().len(), selected);
    // The next record has nothing for sale: no sticker.
    rig.key(Key::ArrowRight, Modifiers::NONE);
    let out = rig.frame(Vec::new());
    assert!(!shows(&out, "€9"), "{:?}", text_list(&out));
}

#[test]
fn a_wanted_record_carries_a_wanted_sticker_that_slaps_on() {
    let mut rig = sleeve_rig("sleeve-wanted", &Fakes::new());
    rig.app.settings.reduce_motion = false;
    rig.click(rig.row(0));
    let out = frames(&mut rig, 15);
    assert_eq!(count(&out, "WANTED"), 0, "not wanted yet");
    fn sticker(rig: &Rig, out: &egui::FullOutput) -> Option<Rect> {
        let cover = rig.app.card_cover.unwrap();
        texts(out)
            .into_iter()
            .find(|t| t.text == "WANTED" && cover.contains(t.rect.center()))
            .map(|t| t.rect)
    }
    // Wanted while shown (as Y, + WANT or the menu do): the sticker lands from bigger.
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::Dig(DigAction::Want(glasshouse(&rig))), &ctx);
    let out = rig.frame(Vec::new());
    let first = sticker(&rig, &out).expect("the sticker appears").height();
    rig.click_text("Later");
    let out = frames(&mut rig, 12);
    let landed = sticker(&rig, &out).expect("and stays");
    assert!(
        first > landed.height() * 1.1,
        "it lands from bigger: {first} → {}",
        landed.height()
    );
    let cover = rig.app.card_cover.unwrap();
    assert!(
        landed.center().x < cover.center().x && landed.center().y > cover.center().y,
        "on the bottom-left corner: {landed:?}"
    );
    assert_eq!(count(&out, "WANTED"), 2, "and the pill still says so");
    // The next record isn't wanted; back on this one, it's drawn at full size at once.
    rig.key(Key::ArrowRight, Modifiers::NONE);
    let out = frames(&mut rig, 15);
    assert_eq!(count(&out, "WANTED"), 0, "{:?}", text_list(&out));
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    let out = rig.frame(Vec::new());
    let back = sticker(&rig, &out).expect("still wanted").height();
    assert!((back - landed.height()).abs() < 0.5, "no slap: {back}");
}

#[test]
fn the_vinyls_label_takes_the_covers_colour() {
    // The fake covers are a red JPEG.
    let mut rig = sleeve_rig("sleeve-tint", &Fakes::new());
    rig.click(rig.row(0));
    let out = rig.frame(Vec::new());
    let cover = rig.app.card_cover.unwrap();
    let (disc, r, _) = circles(&out)
        .into_iter()
        .find(|(c, _, f)| *f == VINYL && (c.y - cover.center().y).abs() < 0.5)
        .expect("the vinyl");
    let label = circles(&out)
        .into_iter()
        .find(|(c, lr, _)| *c == disc && (*lr - r * 0.36).abs() < 0.5)
        .map(|(_, _, fill)| fill)
        .expect("its label");
    let [red, green, blue, _] = label.to_array();
    assert!(red > green + 60 && red > blue + 60, "{label:?}");
}

/// An image host that doesn't answer until the test is over.
struct Hanging;

impl ::dig::cover::ImageSource for Hanging {
    fn get(&self, _: &str) -> Result<Vec<u8>, ::dig::cover::ImageError> {
        std::thread::sleep(Duration::from_secs(30));
        Err(::dig::cover::ImageError::Net("too late".into()))
    }
}

#[test]
fn stickers_and_the_vinyl_show_while_the_cover_loads() {
    let fakes = Fakes::new();
    let f = fakes.clone();
    let mut rig = Rig::with_dig(
        "sleeve-loading",
        Vec::new(),
        |_| {},
        move |dir| {
            Some(DigSetup {
                images: Arc::new(Hanging),
                ..f.setup(dir)
            })
        },
    );
    add_records(&mut rig);
    let ctx = rig.ctx.clone();
    rig.app
        .apply(Action::Dig(DigAction::Want(glasshouse(&rig))), &ctx);
    rig.frame(Vec::new());
    rig.click_text("Later");
    rig.click(rig.row(0));
    let out = rig.frame(Vec::new());
    assert!(!has_cover(&rig, 12), "still loading");
    assert!(shows(&out, "€9"), "{:?}", text_list(&out));
    assert_eq!(count(&out, "WANTED"), 2, "{:?}", text_list(&out));
    assert!(disc_out(&rig, &out) > 0.0, "the vinyl's edge");
}

/// The pictures drawn from textures other than egui's own (covers, the skin): (rect, tint) of
/// each flat image (a 4-vertex mesh), and how many tipped sleeves (a 5 × 5 grid).
fn pictures(out: &egui::FullOutput) -> (Vec<(Rect, Color32)>, usize) {
    fn walk(shape: &egui::Shape, acc: &mut (Vec<(Rect, Color32)>, usize)) {
        match shape {
            egui::Shape::Mesh(m) if m.texture_id != egui::TextureId::default() => {
                match m.vertices.len() {
                    4 => {
                        let r = Rect::from_points(
                            &m.vertices.iter().map(|v| v.pos).collect::<Vec<_>>(),
                        );
                        acc.0.push((r, m.vertices[0].color));
                    }
                    25 => acc.1 += 1,
                    _ => {}
                }
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, acc)),
            _ => {}
        }
    }
    let mut acc = (Vec::new(), 0);
    for s in &out.shapes {
        walk(&s.shape, &mut acc);
    }
    acc
}

/// Square pictures drawn: covers (the skin's pictures are bars and buttons).
fn squares(out: &egui::FullOutput) -> Vec<Rect> {
    pictures(out)
        .0
        .into_iter()
        .map(|(r, _)| r)
        .filter(|r| (r.width() - r.height()).abs() < 1.0 && r.width() > 8.0)
        .collect()
}

/// Covers drawn at about the card's cover size (the rows' thumbnails are much smaller).
fn card_covers(rig: &Rig, out: &egui::FullOutput) -> Vec<Rect> {
    let side = rig.app.card_cover.expect("open").width();
    squares(out)
        .into_iter()
        .filter(|r| r.width() > side * 0.8)
        .collect()
}

#[test]
fn opening_a_card_flies_its_cover_from_the_row_and_takes_clicks_at_once() {
    let mut rig = sleeve_rig("sleeve-flight", &Fakes::new());
    rig.app.settings.reduce_motion = false;
    rig.click(rig.row(0));
    let flight = rig.app.card.as_ref().unwrap().from;
    let (row, _) = flight.expect("it flies from the row's cover");
    let early = squares(&rig.frame(Vec::new()));
    // Landed after 180 ms: only in the card's square.
    let out = frames(&mut rig, 12);
    let card = rig.app.card_rect.expect("open");
    let cover = rig.app.card_cover.unwrap();
    let covers = card_covers(&rig, &out);
    assert!(
        covers.len() == 1 && (covers[0].center() - cover.center()).length() < 1.0,
        "landed in the card: {covers:?} {cover:?}"
    );
    assert!(card.contains_rect(covers[0]));
    // On an early frame it was on its way: bigger than the row's, not yet the card's.
    let flying = early.into_iter().find(|r| {
        r.width() > row.width() + 1.0
            && r.width() < cover.width() - 1.0
            && (r.center() - cover.center()).length() > 1.0
    });
    assert!(flying.is_some(), "on its way from {row:?} to {cover:?}");
    // Open again: a click on the card's first frame, mid-flight, lands.
    rig.key(Key::Escape, Modifiers::NONE);
    past_double_click(&mut rig);
    rig.click(rig.row(0));
    assert!(rig.app.card.as_ref().unwrap().from.is_some(), "flying");
    let out = rig.frame(Vec::new());
    rig.click(card_button(&out, "Tidepool"));
    let a1 = glasshouse(&rig)[0];
    rig.until(
        |r| {
            r.app.armed.is_some_and(|(_, id)| id == a1)
                || r.app.crates.shown().current() == Some(a1)
        },
        "A1 asked to play",
    );
}

#[test]
fn a_record_with_no_cover_opens_with_no_flight() {
    let fakes = Fakes::new();
    fakes.images.route(
        "https://i.discogs.com/cover-13.jpeg",
        Err(::dig::cover::ImageError::Net("down".into())),
    );
    let mut rig = records_rig("sleeve-no-flight", &fakes);
    for release in [12, 14] {
        rig.until(|r| has_cover(r, release), "the other covers");
    }
    rig.frame(Vec::new());
    rig.app.settings.reduce_motion = false;
    rig.click(rig.row(2));
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(13)));
    assert_eq!(rig.app.card.as_ref().unwrap().from, None);
}

#[test]
fn stepping_flips_the_sleeve_and_quick_steps_restart_it() {
    let mut rig = sleeve_rig("sleeve-flip", &Fakes::new());
    rig.app.settings.reduce_motion = false;
    rig.click(rig.row(0));
    frames(&mut rig, 15);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    let (_, meshes) = pictures(&rig.frame(Vec::new()));
    assert_eq!(meshes, 1, "the old sleeve tips away over the new one");
    let out = frames(&mut rig, 14);
    let (_, meshes) = pictures(&out);
    assert_eq!(meshes, 0, "flat after 220 ms");
    assert_eq!(card_covers(&rig, &out).len(), 1, "one cover, in place");
    // Steps within 220 ms: the card ends where they lead, flipping again from the cover shown.
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(card_on(&rig), Some(AlbumKey::Release(12)));
    let (_, meshes) = pictures(&rig.frame(Vec::new()));
    assert_eq!(meshes, 1, "flipping again");
}

#[test]
fn following_playback_flips_as_a_step_on() {
    let mut rig = sleeve_rig("sleeve-flip-follow", &Fakes::new());
    rig.app.settings.reduce_motion = false;
    repeat_off(&mut rig);
    rig.click(rig.row(0));
    let out = frames(&mut rig, 15);
    rig.click(card_button(&out, "Undertow"));
    rig.until(
        |r| card_on(r) == Some(AlbumKey::Release(13)),
        "the card follows",
    );
    let moved = rig.app.card.as_ref().unwrap().moved;
    assert!(moved.is_some_and(|(_, d)| d == 1), "{moved:?}");
    assert!(
        rig.app.card.as_ref().unwrap().left.is_some(),
        "the old cover flips away"
    );
}
