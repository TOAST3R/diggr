//! The level meter in the mini player, fed by the real engine through a fake sink.

use super::*;

/// A stereo sine, left at `left_db` and right at `right_db` (dBFS), `secs` long, at 48 kHz.
fn tone(path: &std::path::Path, left_db: f32, right_db: f32, secs: f32) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    let (l, r) = (10f32.powf(left_db / 20.0), 10f32.powf(right_db / 20.0));
    for i in 0..(48_000.0 * secs) as usize {
        let s = (std::f32::consts::TAU * 440.0 * i as f32 / 48_000.0).sin();
        w.write_sample(s * l).unwrap();
        w.write_sample(s * r).unwrap();
    }
    w.finalize().unwrap();
}

/// A rig playing a tone at `left_db` / `right_db`, the meter up to its level.
fn playing(name: &str, left_db: f32, right_db: f32) -> Rig {
    let mut rig = Rig::new(name, Vec::new(), |_| {});
    let path = rig.dir.join("tone.wav");
    tone(&path, left_db, right_db, 4.0);
    rig.app.add_paths(vec![path], Open::AddAndPlay);
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    for _ in 0..10 {
        rig.pump();
    }
    rig
}

#[test]
fn a_loud_left_channel_lights_into_the_red() {
    let mut rig = playing("meter-hit", -1.0, -12.0);
    let (lit, _) = rig.app.meter_drawn;
    // -1 dBFS fills all 24 segments, the red ones included; -12 dBFS stops at the amber.
    assert_eq!(lit[0], crate::meter::SEGMENTS, "left: {lit:?}");
    assert_eq!(crate::meter::Led::of(lit[0] - 1), crate::meter::Led::Red);
    assert_eq!(lit[1], 18, "right, its own level: {lit:?}");
    assert_eq!(crate::meter::Led::of(lit[1] - 1), crate::meter::Led::Green);
    rig.pump();
}

#[test]
fn the_meter_moves_with_the_volume_at_zero() {
    let mut rig = playing("meter-volume", -6.0, -6.0);
    let full = rig.app.meter_drawn.0;
    rig.engine().set_volume(0.0);
    for _ in 0..10 {
        rig.pump();
    }
    assert_eq!(rig.app.meter_drawn.0, full, "as at full volume");
    assert!(full[0] > 0);
}

#[test]
fn a_paused_meter_falls_empty_and_then_asks_for_nothing() {
    let mut rig = playing("meter-pause", -3.0, -3.0);
    rig.app.settings.reduce_motion = false;
    rig.key(Key::Space, Modifiers::NONE);
    assert_eq!(rig.engine().state(), PlayState::Paused);
    // The bars within 3.6 s (216 frames), the dots after their second's hold within 4.6 s.
    let mut bars_empty = None;
    for frame in 1..=290 {
        rig.frame(Vec::new());
        let (lit, dots) = rig.app.meter_drawn;
        if bars_empty.is_none() && lit == [0, 0] {
            bars_empty = Some(frame);
        }
        if lit == [0, 0] && dots == [None, None] {
            assert!(
                bars_empty.unwrap() <= 216,
                "bars empty after {bars_empty:?} frames"
            );
            assert!(frame <= 276, "dots gone after {frame} frames");
            assert!(
                rig.app.meter.iter().all(|b| b.settled()),
                "nothing left to move"
            );
            return;
        }
    }
    panic!("the meter never emptied: {:?}", rig.app.meter_drawn);
}

#[test]
fn reduce_motion_shows_the_level_without_a_dot() {
    let mut rig = playing("meter-reduced", -6.0, -6.0);
    assert!(rig.app.settings.reduce_motion);
    assert!(rig.app.meter_drawn.0[0] > 0);
    assert_eq!(rig.app.meter_drawn.1, [None, None], "no peak dot");
    rig.key(Key::Space, Modifiers::NONE);
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert_eq!(rig.app.meter_drawn.0, [0, 0], "empty at once when paused");
}

// ---- the turntable's beat lamp --------------------------------------------------------------

/// A 120 BPM beat grid (a beat every 0.5 s, a downbeat every fourth) over the 2 s fixture.
fn grid(beats: bool) -> Arc<analysis::SongScore> {
    let b: Vec<f64> = if beats {
        (0..4).map(|i| 0.25 + 0.5 * f64::from(i)).collect()
    } else {
        Vec::new()
    };
    Arc::new(analysis::SongScore {
        downbeats: if beats { vec![0] } else { Vec::new() },
        beats: b,
        ..Default::default()
    })
}

/// Plays the fixture with `score`, then counts the lamp's lightings over `buffers` device
/// buffers: (lightings, downbeat lightings, device seconds).
fn lightings(rig: &mut Rig, buffers: usize) -> (usize, usize, f64) {
    let (mut n, mut down, mut last) = (0, 0, None);
    for _ in 0..buffers {
        rig.pump();
        if let Some((since, strength)) = rig.app.beat_lit
            && last != Some(since)
        {
            last = Some(since);
            n += 1;
            if strength == 1.0 {
                down += 1;
            }
        }
    }
    (n, down, buffers as f64 * BUF as f64 / 48_000.0)
}

fn beat_rig(name: &str, beats: bool) -> Rig {
    let mut rig = Rig::new(name, vec![fixture("tone.flac")], |_| {});
    rig.screen.x = 720.0;
    rig.app.score_override = Some(grid(beats));
    rig.until(|r| r.app.position.state == PlayState::Playing, "playing");
    rig
}

#[test]
fn the_beat_lamp_lights_on_each_beat_and_brighter_on_the_downbeat() {
    let mut rig = beat_rig("beat-lamp", true);
    let (n, down, secs) = lightings(&mut rig, (48_000 * 4) / BUF);
    // Two beats a second (the 2 s fixture repeats, its grid with it), a downbeat in four.
    let expected = 2.0 * secs;
    assert!(
        (n as f64 - expected).abs() <= 1.5,
        "{n} lightings in {secs:.2} s"
    );
    assert!(down >= 1 && down * 4 <= n + 3, "{down} downbeats of {n}");
}

#[test]
fn under_sync_the_lamp_follows_the_beats_as_heard() {
    let mut rig = beat_rig("beat-lamp-sync", true);
    let index = rig.engine().current_index().unwrap();
    rig.engine().set_speed(index, 1.05); // 120 BPM fitted to 126
    for _ in 0..20 {
        rig.pump();
    }
    let (n, _, secs) = lightings(&mut rig, (48_000 * 4) / BUF);
    let expected = 2.1 * secs;
    assert!(
        (n as f64 - expected).abs() <= 1.5,
        "{n} lightings in {secs:.2} s at 1.05"
    );
}

#[test]
fn the_lamp_stays_dark_without_beats_or_with_the_record_held() {
    let mut rig = beat_rig("beat-lamp-none", false);
    let (n, _, _) = lightings(&mut rig, 200);
    assert_eq!(n, 0, "no beat grid yet: dark");
    assert_eq!(rig.app.beat_drawn, None);

    let mut rig = beat_rig("beat-lamp-held", true);
    rig.engine().hold(true);
    // Through the brake (0.4 s), confirming the hold.
    for _ in 0..(48_000 / 2) / BUF {
        rig.engine().hold_keepalive();
        rig.pump();
    }
    let mut n = 0;
    let mut last = rig.app.beat_lit.map(|(t, _)| t);
    for _ in 0..(48_000 * 2) / BUF {
        rig.engine().hold_keepalive();
        rig.pump();
        let now = rig.app.beat_lit.map(|(t, _)| t);
        if now.is_some() && now != last {
            n += 1;
        }
        last = now;
    }
    assert_eq!(n, 0, "a held record passes no beats");
}
