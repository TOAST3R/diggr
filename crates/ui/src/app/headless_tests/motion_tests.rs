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
