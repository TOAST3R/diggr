//! Motion in the real app: menus, dialogs and the EQ moving as they open and close, every
//! action still landing in the frame it is made, and the window going idle once it settles.
//! Each `Rig` frame is 1/60 s of egui time.

use super::*;

/// A rig with motion on, the crate sidebar shown (its rows have a context menu).
fn motion_rig(name: &str) -> Rig {
    let (mut rig, _, _, _) = sidebar_rig(name);
    rig.app.settings.reduce_motion = false;
    rig.frame(Vec::new());
    rig
}

/// Where `label` is drawn in `out`.
fn text_at(out: &egui::FullOutput, label: &str) -> Option<Rect> {
    texts(out)
        .into_iter()
        .find(|t| t.text == label)
        .map(|t| t.rect)
}

/// Frames until nothing moves (at most `max`); returns the last output.
fn settle(rig: &mut Rig, max: usize) -> egui::FullOutput {
    let mut out = rig.frame(Vec::new());
    for _ in 0..max {
        if !rig.app.motion.animating() {
            return out;
        }
        out = rig.frame(Vec::new());
    }
    panic!("still moving after {max} frames");
}

/// Right-clicks the sidebar's third row (a crate), opening its menu.
fn open_crate_menu(rig: &mut Rig) {
    let at = side_row(2);
    rig.frame(vec![Event::PointerMoved(at)]);
    rig.press(at, PointerButton::Secondary, true);
    rig.press(at, PointerButton::Secondary, false);
}

#[test]
fn a_context_menu_rises_into_place_and_settles() {
    let mut rig = motion_rig("motion-menu");
    open_crate_menu(&mut rig);
    // Its first frames are drawn lower, rising; 0.12 s later (7–8 frames) it is in place.
    let mut ys = Vec::new();
    for _ in 0..12 {
        let out = rig.frame(Vec::new());
        if let Some(r) = text_at(&out, "Rename crate…") {
            ys.push(r.min.y);
        }
    }
    assert!(ys.len() >= 8, "the menu shows: {ys:?}");
    let last = *ys.last().unwrap();
    assert!(
        ys[0] > last + 0.5 && ys[0] <= last + 4.0 + 1e-3,
        "it rises 4 points: {ys:?}"
    );
    assert!(
        ys.windows(2).all(|w| w[1] <= w[0] + 1e-3),
        "only upwards: {ys:?}"
    );
    let still = ys.iter().position(|&y| (y - last).abs() < 1e-3).unwrap();
    assert!(still <= 8, "in place by 0.12 s: {ys:?}");
    assert!(!rig.app.motion.animating());
}

#[test]
fn a_closed_menu_fades_out_and_is_gone() {
    let mut rig = motion_rig("motion-menu-close");
    open_crate_menu(&mut rig);
    settle(&mut rig, 20);
    rig.key(Key::Escape, Modifiers::NONE);
    assert!(
        !rig.menu_open() && !egui::Popup::is_any_open(&rig.ctx),
        "closed at once"
    );
    // Its look fades out over 0.08 s (5 frames), then nothing is left of it.
    let mut seen = 0;
    for _ in 0..8 {
        let out = rig.frame(Vec::new());
        if text_at(&out, "Rename crate…").is_some() {
            seen += 1;
        }
    }
    assert!((1..=5).contains(&seen), "faded over {seen} frames");
    assert!(!shows(&rig.frame(Vec::new()), "Rename crate…"));
    assert_eq!(rig.app.motion.closing(), 0);
    assert!(!rig.app.motion.animating());
}

#[test]
fn a_menu_item_works_while_the_menu_is_still_moving() {
    let mut rig = motion_rig("motion-menu-click");
    open_crate_menu(&mut rig);
    let out = rig.frame(Vec::new());
    let item = text_at(&out, "Rename crate…")
        .expect("the menu shows")
        .center();
    assert!(rig.app.motion.animating(), "still rising");
    rig.frame(vec![Event::PointerMoved(item)]);
    rig.press(item, PointerButton::Primary, true);
    rig.press(item, PointerButton::Primary, false);
    assert!(rig.app.name_dialog.is_some(), "the item's action ran");
}

#[test]
fn the_help_panel_grows_into_place() {
    let mut rig = motion_rig("motion-help");
    rig.key(Key::H, Modifiers::NONE);
    assert!(rig.app.help);
    let first = rig.frame(Vec::new());
    let title = |out: &egui::FullOutput| text_at(out, crate::help::TITLE);
    let start = title(&first).expect("the help panel shows");
    // 0.18 s: 11 frames (±1).
    let mut frames = 1;
    while rig.app.motion.animating() {
        rig.frame(Vec::new());
        frames += 1;
        assert!(frames <= 13, "still growing after {frames} frames");
    }
    assert!(frames >= 9, "grew over {frames} frames");
    let end = title(&rig.frame(Vec::new())).unwrap();
    // Grown about its centre: the title, at the top, starts a little lower.
    assert!(start.min.y > end.min.y, "{start:?} → {end:?}");
}

#[test]
fn a_closing_dialog_takes_no_clicks() {
    let mut rig = motion_rig("motion-help-close");
    rig.key(Key::H, Modifiers::NONE);
    let out = settle(&mut rig, 20);
    let title = text_at(&out, crate::help::TITLE).unwrap().center();
    rig.key(Key::H, Modifiers::NONE);
    assert!(!rig.app.help, "closed at once");
    assert_eq!(rig.app.motion.closing(), 1, "fading out");
    // What's under the pointer is no longer the help panel, while it still shows.
    let out = rig.frame(vec![Event::PointerMoved(title)]);
    assert!(shows(&out, crate::help::TITLE), "still fading");
    let under = rig.ctx.layer_id_at(title);
    assert!(
        under.is_none_or(|l| l.order != egui::Order::Middle),
        "nothing of the panel takes the pointer: {under:?}"
    );
    settle(&mut rig, 20);
    assert!(!shows(&rig.frame(Vec::new()), crate::help::TITLE));
}

#[test]
fn the_eq_fades_in() {
    let mut rig = motion_rig("motion-eq");
    rig.app.apply(Action::ToggleEq, &rig.ctx.clone());
    // Over 0.18 s (11 frames ±1), then still.
    let mut frames = 0;
    rig.frame(Vec::new());
    while rig.app.motion.animating() {
        rig.frame(Vec::new());
        frames += 1;
        assert!(frames <= 13, "still fading after {frames} frames");
    }
    assert!(frames >= 9, "faded in over {frames} frames");
}

#[test]
fn keys_work_while_something_moves() {
    let mut rig = Rig::new("motion-playback", vec![fixture("tone.flac")], |_| {});
    rig.app.settings.reduce_motion = false;
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    rig.key(Key::H, Modifiers::NONE);
    assert!(rig.app.motion.animating(), "the help panel is opening");
    rig.key(Key::Space, Modifiers::NONE);
    assert_eq!(
        rig.engine().state(),
        PlayState::Paused,
        "paused in that frame"
    );
    assert_eq!(rig.engine().stats().underruns, 0);
}

#[test]
fn motion_stops_asking_for_repaints_once_it_settles() {
    // (The rig's tracks keep it repainting anyway, so this checks what motion asks for; the
    // repaint policy itself has its own unit test.)
    let mut rig = motion_rig("motion-idle");
    open_crate_menu(&mut rig);
    let moving = rig.frame(Vec::new());
    assert!(rig.app.motion.animating());
    assert_eq!(
        repaint_delay(&moving),
        Duration::ZERO,
        "every refresh while moving"
    );
    settle(&mut rig, 20);
    assert!(
        !rig.app.motion.animating(),
        "settled: motion asks for nothing more"
    );
    rig.key(Key::Escape, Modifiers::NONE);
    settle(&mut rig, 20);
    assert!(
        !rig.app.motion.animating(),
        "and after the menu has faded out"
    );
    assert_eq!(rig.app.motion.closing(), 0);
}

/// How soon the main window asked to be repainted.
fn repaint_delay(out: &egui::FullOutput) -> Duration {
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .map_or(Duration::MAX, |v| v.repaint_delay)
}

#[test]
fn a_separate_window_fades_its_content_in() {
    let mut rig = motion_rig("motion-window");
    let ctx = rig.ctx.clone();
    ctx.set_embed_viewports(false);
    let mut fades = Vec::new();
    for i in 1..=14 {
        let input = egui::RawInput {
            time: Some(100.0 + f64::from(i) / 60.0),
            ..Default::default()
        };
        let app = &mut rig.app;
        ctx.run_ui(input, |ui| {
            app.motion.begin_frame(ui.ctx(), false);
            fades.push(app.window_fade(ui.ctx(), "eq", true));
        })
        .textures_delta
        .clear();
    }
    ctx.set_embed_viewports(true);
    // From its background to fully drawn in 0.18 s (11 frames ±1).
    assert!(fades[0] < 0.5, "{fades:?}");
    assert!(fades.windows(2).all(|w| w[1] >= w[0]));
    let full = fades.iter().position(|&f| f == 1.0).expect("fully drawn");
    assert!(
        (9..=12).contains(&full),
        "fully drawn after {full} frames: {fades:?}"
    );
}

#[test]
fn a_filter_panel_slides_into_place() {
    // A bare context with the app's motion and an area under the filters panel's id.
    let ctx = egui::Context::default();
    let mut motion = new_motion();
    let mut ys = Vec::new();
    for i in 0..20 {
        let input = egui::RawInput {
            time: Some(f64::from(i) / 60.0),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            motion.begin_frame(ui.ctx(), false);
            egui::Area::new(filters_popup_id())
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(50.0, 50.0))
                .show(ui.ctx(), |ui| ui.label("STYLES"));
            motion.end_frame(ui.ctx());
        });
        out.textures_delta.clear();
        if let Some(r) = text_at(&out, "STYLES") {
            ys.push(r.min.y);
        }
    }
    let last = *ys.last().unwrap();
    assert!(
        ys[0] > last + 6.0 && ys[0] <= last + 12.0 + 1e-3,
        "slides 12 points: {ys:?}"
    );
    // 0.24 s: in place by the 15th frame (±1).
    let still = ys.iter().position(|&y| (y - last).abs() < 1e-3).unwrap();
    assert!(still <= 15, "in place after {still} frames: {ys:?}");
}

// ---- feedback: presses and lamps ----------------------------------------------------------

/// The top-left of the textured mesh drawn over the deck rect `name`, if any (the sprite).
fn sprite_top(out: &egui::FullOutput, rig: &Rig, name: &str) -> Option<f32> {
    let at = deck_at(rig, name, 0.0, 0.0);
    let w = rig.app.def.at(name).w as f32;
    let mut best = None;
    for s in &out.shapes {
        if let egui::Shape::Mesh(m) = &s.shape {
            let r = Rect::from_points(&m.vertices.iter().map(|v| v.pos).collect::<Vec<_>>());
            if (r.min.x - at.x).abs() < 0.5
                && (r.width() - w).abs() < 0.5
                && (r.min.y - at.y).abs() < 2.0
            {
                best = Some(r.min.y);
            }
        }
    }
    best
}

/// Blurred shapes (glows) drawn around the deck rect `name`.
fn glows_near(out: &egui::FullOutput, rig: &Rig, name: &str) -> usize {
    let c = deck_at(rig, name, 0.5, 0.5);
    out.shapes
        .iter()
        .filter(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.blur_width > 0.0 && r.rect.expand(8.0).contains(c)))
        .count()
}

fn deck_rig(name: &str) -> Rig {
    let mut rig = Rig::new(name, Vec::new(), |_| {});
    rig.screen.x = 720.0;
    rig.app.settings.reduce_motion = false;
    rig.frame(Vec::new());
    rig
}

#[test]
fn a_button_presses_in_and_springs_back() {
    let mut rig = deck_rig("motion-press");
    let at = deck_sync(&rig);
    let rest = sprite_top(&rig.frame(vec![Event::PointerMoved(at)]), &rig, "deck_sync").unwrap();
    rig.press(at, PointerButton::Primary, true);
    let held = sprite_top(&rig.frame(Vec::new()), &rig, "deck_sync").unwrap();
    assert!(
        (held - rest - 1.0).abs() < 1e-3,
        "a pixel lower while held: {rest} → {held}"
    );
    rig.press(at, PointerButton::Primary, false);
    // Back within 0.12 s (8 frames).
    let mut tops = Vec::new();
    for _ in 0..9 {
        tops.push(sprite_top(&rig.frame(Vec::new()), &rig, "deck_sync").unwrap());
    }
    assert!(
        tops[0] > rest && tops[0] <= held,
        "springing back: {tops:?}"
    );
    assert!((tops[8] - rest).abs() < 1e-3, "back in place: {tops:?}");
    assert!(
        rig.app.settings.sync.on,
        "and the click still turned SYNC on"
    );
}

#[test]
fn sync_blooms_as_it_lights_and_settles() {
    let mut rig = deck_rig("motion-bloom");
    assert_eq!(
        glows_near(&rig.frame(Vec::new()), &rig, "deck_sync"),
        0,
        "no glow at rest"
    );
    rig.click(deck_sync(&rig));
    assert!(rig.app.settings.sync.on);
    // click() spent two frames after the press; the glow is still there, then gone by 0.18 s.
    let mut glowing = 0;
    for _ in 0..14 {
        if glows_near(&rig.frame(Vec::new()), &rig, "deck_sync") > 0 {
            glowing += 1;
        }
    }
    assert!(
        (4..=11).contains(&glowing),
        "glowed for {glowing} more frames"
    );
    assert_eq!(
        glows_near(&rig.frame(Vec::new()), &rig, "deck_sync"),
        0,
        "settled"
    );
}

#[test]
fn reduce_motion_leaves_buttons_still() {
    let mut rig = deck_rig("motion-press-reduced");
    rig.app.settings.reduce_motion = true;
    let at = deck_sync(&rig);
    let rest = sprite_top(&rig.frame(vec![Event::PointerMoved(at)]), &rig, "deck_sync").unwrap();
    let out = rig.press(at, PointerButton::Primary, true);
    let _ = out;
    assert_eq!(
        sprite_top(&rig.frame(Vec::new()), &rig, "deck_sync"),
        Some(rest)
    );
    rig.press(at, PointerButton::Primary, false);
    assert_eq!(glows_near(&rig.frame(Vec::new()), &rig, "deck_sync"), 0);
}

// ---- feedback: the pitch fader ------------------------------------------------------------

/// The pitch fader thumb's centre, as drawn.
fn thumb_y(out: &egui::FullOutput, rig: &Rig) -> f32 {
    let cx = deck_at(rig, "deck_pitch", 0.5, 0.0).x;
    let w = rig.app.def.sprite("pitch_thumb").w as f32;
    out.shapes
        .iter()
        .rev()
        .filter_map(|s| match &s.shape {
            egui::Shape::Mesh(m) => {
                let r = Rect::from_points(&m.vertices.iter().map(|v| v.pos).collect::<Vec<_>>());
                ((r.center().x - cx).abs() < 1.0 && (r.width() - w).abs() < 0.5)
                    .then_some(r.center().y)
            }
            _ => None,
        })
        .next()
        .expect("the thumb is drawn")
}

/// A playing 123 BPM record with SYNC on and motion on, the fader at +8% and still.
fn fader_rig(name: &str) -> Rig {
    let mut rig = Rig::new(name, vec![fixture("tone.flac")], |_| {});
    rig.screen.x = 1100.0;
    rig.until(
        |r| {
            r.app.position.state == PlayState::Playing
                && r.app.track_refs.contains_key(&r.app.position.track)
        },
        "the track plays",
    );
    let track = rig.app.track_refs[&rig.app.position.track].clone();
    rig.app.set_track_bpm(&track, 123.0);
    rig.click(deck_sync(&rig));
    rig.click(deck_at(&rig, "deck_pitch", 0.5, 0.0));
    assert_eq!(rig.app.settings.sync.cents(), 13_284);
    rig.app.settings.reduce_motion = false;
    for _ in 0..30 {
        rig.frame(Vec::new());
    }
    rig
}

#[test]
fn reset_glides_the_fader_home_with_a_little_overshoot() {
    let mut rig = fader_rig("motion-fader-reset");
    let top = thumb_y(&rig.frame(Vec::new()), &rig);
    let at = deck_at(&rig, "deck_reset", 0.5, 0.5);
    rig.frame(vec![Event::PointerMoved(at)]);
    rig.press(at, PointerButton::Primary, true);
    rig.press(at, PointerButton::Primary, false);
    assert_eq!(rig.app.settings.sync.cents(), 12_300, "the target at once");
    let mut ys = Vec::new();
    for _ in 0..30 {
        ys.push(thumb_y(&rig.frame(Vec::new()), &rig));
    }
    let home = *ys.last().unwrap();
    let travel = home - top;
    assert!(travel > 10.0, "it moved down to the centre: {top} → {home}");
    assert!(ys[0] < home - 1.0, "gliding, not jumping: {ys:?}");
    let past = ys.iter().map(|y| y - home).fold(0.0f32, f32::max);
    assert!(past <= 0.1 * travel, "overshoot {past} of {travel}");
    // Settled (to the pixel) within 0.3 s: 18 frames.
    let settled = ys
        .iter()
        .rposition(|y| (y - home).abs() > 0.5)
        .map_or(0, |i| i + 1);
    assert!(settled <= 18, "settled after {settled} frames: {ys:?}");
}

#[test]
fn the_fader_stays_under_the_hand() {
    let mut rig = fader_rig("motion-fader-drag");
    let start = deck_at(&rig, "deck_pitch", 0.5, 0.0);
    rig.frame(vec![Event::PointerMoved(start)]);
    rig.press(start, PointerButton::Primary, true);
    for f in [0.2, 0.35, 0.6, 0.8] {
        let p = deck_at(&rig, "deck_pitch", 0.5, f);
        rig.frame(vec![Event::PointerMoved(p)]);
        let y = thumb_y(&rig.frame(Vec::new()), &rig);
        assert!((y - p.y).abs() <= 1.5, "thumb at {y}, hand at {}", p.y);
    }
    rig.press(
        deck_at(&rig, "deck_pitch", 0.5, 0.8),
        PointerButton::Primary,
        false,
    );
}

// ---- feedback: the LCD digits -------------------------------------------------------------

/// Bright fills (an afterglow, not the faint glow) inside the big digit `i`'s segments `mask`.
fn afterglow_in(out: &egui::FullOutput, rig: &Rig, i: usize, mask: u8) -> usize {
    let origin = deck_at(rig, "deck_target", 0.0, 0.0) + vec2(10.0 * i as f32, 0.0);
    let segs: Vec<Rect> = (0..7)
        .filter(|s| mask & (1 << s) != 0)
        .map(|s| {
            let (x, y, w, h) = crate::skin::generate::SEGMENTS[s];
            Rect::from_min_size(origin + vec2(x as f32, y as f32), vec2(w as f32, h as f32))
        })
        .collect();
    out.shapes
        .iter()
        .filter(|s| {
            matches!(&s.shape, egui::Shape::Rect(r)
                if r.fill.a() > 60 && segs.iter().any(|g| g.expand(0.01).contains_rect(r.rect)))
        })
        .count()
}

#[test]
fn a_digit_that_changes_lets_its_old_segments_fade() {
    let mut rig = fader_rig("motion-digits");
    assert_eq!(deck(&rig).1, "132");
    let gone = crate::widgets::segments('2') & !crate::widgets::segments('1');
    assert_eq!(
        afterglow_in(&rig.frame(Vec::new()), &rig, 2, gone),
        0,
        "nothing glowing at rest"
    );
    // (The fader is at the top of its range: step down.)
    rig.key(Key::ArrowDown, Modifiers::SHIFT);
    // The key's frame drew 131 and started the fade; it lasts 0.12 s (7–8 frames).
    let mut lit = Vec::new();
    for _ in 0..10 {
        let out = rig.frame(Vec::new());
        lit.push(afterglow_in(&out, &rig, 2, gone));
    }
    assert_eq!(deck(&rig).1, "131");
    assert!(lit[0] > 0, "the 2's own segments still glow: {lit:?}");
    let last = lit.iter().rposition(|&n| n > 0).unwrap();
    assert!(last <= 7, "faded within 0.12 s: {lit:?}");
}

// ---- feedback: the playlist changing --------------------------------------------------------

/// Where the list row numbered `n` ("3.", its own cell in the columns) is drawn.
fn row_y(out: &egui::FullOutput, n: &str) -> Option<f32> {
    texts(out)
        .into_iter()
        .filter(|t| t.text == n)
        .map(|t| t.rect.center().y)
        .next()
}

#[test]
fn switching_crates_cross_fades_and_the_new_rows_answer_at_once() {
    let (mut rig, _, keepers, _) = sidebar_rig("motion-switch");
    let tracks = ["tone.flac", "tone.wav"].map(|f| TrackRef::new(fixture(f).to_string_lossy()));
    rig.app.crates.get_mut(keepers).unwrap().add(tracks);
    rig.app.settings.reduce_motion = false;
    let rest = row_y(&rig.frame(Vec::new()), "1.").unwrap();
    // To Keepers, below Playlist in the sidebar: its rows come up from 12 points below.
    let at = side_row(1);
    rig.frame(vec![Event::PointerMoved(at)]);
    rig.press(at, PointerButton::Primary, true);
    rig.press(at, PointerButton::Primary, false);
    assert_eq!(rig.app.crates.shown_id(), keepers);
    let out = rig.frame(Vec::new());
    assert!(rig.app.list_motion.switching(), "cross-fading");
    let ys: Vec<f32> = texts(&out)
        .into_iter()
        .filter(|t| t.text == "1.")
        .map(|t| t.rect.center().y)
        .collect();
    assert!(
        ys.len() >= 2,
        "the old list fading out over the new: {ys:?}"
    );
    assert!(
        ys.iter().any(|&y| y > rest + 0.5),
        "the new rows slide up: {ys:?} vs {rest}"
    );
    // Its first row takes a click while it moves.
    let first = rig.ids(keepers)[0];
    let row = side_list_row(0);
    rig.frame(vec![Event::PointerMoved(row)]);
    rig.press(row, PointerButton::Primary, true);
    rig.press(row, PointerButton::Primary, false);
    assert!(
        rig.app.crates.shown().is_selected(first),
        "selected at once"
    );
    // Settled within 0.18 s (11 frames ±1, counting those above).
    let mut frames = 4;
    while rig.app.list_motion.switching() {
        rig.frame(Vec::new());
        frames += 1;
        assert!(frames <= 13, "still switching after {frames} frames");
    }
    assert!((row_y(&rig.frame(Vec::new()), "1.").unwrap() - rest).abs() < 1e-3);
}

#[test]
fn rows_make_room_for_one_that_arrives() {
    let (mut rig, ids, _, _) = sidebar_rig("motion-room");
    rig.app.settings.reduce_motion = false;
    let out = rig.frame(Vec::new());
    let (row2, row3) = (row_y(&out, "2.").unwrap(), row_y(&out, "3.").unwrap());
    let step = row3 - row2;
    // A track arrives after the first: the second and third move down a row.
    rig.app.crates.shown_mut().insert_after(
        ids[0],
        crate::playlist::NewEntry {
            artist: "Arrived".into(),
            title: "New".into(),
            source: None,
            origin: None,
            duration: Some(1.0),
            status: crate::playlist::WaitKind::Queued,
        },
    );
    let mut ys = Vec::new();
    for _ in 0..16 {
        ys.push(row_y(&rig.frame(Vec::new()), "4.").unwrap());
    }
    // The old third row (now the fourth) starts where it was, and eases down.
    assert!(
        (ys[0] - row3).abs() < step * 0.8,
        "from about where it was: {ys:?} ({row3})"
    );
    assert!(
        ys.windows(2).all(|w| w[1] >= w[0] - 1e-3),
        "only down: {ys:?}"
    );
    let home = row3 + step;
    assert!((ys[15] - home).abs() < 1e-3, "in its new place: {ys:?}");
    let settled = ys.iter().position(|y| (y - home).abs() < 0.5).unwrap();
    assert!(settled <= 13, "eased over {settled} frames: {ys:?}");
}
