//! End-to-end engine behavior with a manually driven sink (no audio hardware needed).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio::{Engine, EngineConfig, EngineEvent, PlayState};
use platform::native::{NativeFileSource, NativeSpawner};
use platform::testing::ManualSink;
use platform::{CallbackInfo, SinkError, TrackRef};

const BUF: usize = 512;

/// A deterministic, non-periodic stereo signal so any misplaced sample is detectable.
fn signal(frame: u64) -> (f32, f32) {
    let t = frame as f32;
    (((t * 0.0123).sin() * 0.4), ((t * 0.0071).cos() * 0.3))
}

fn temp_dir(name: &str) -> platform::testing::TestDir {
    platform::testing::TestDir::new(&format!("audio-engine-{name}"))
}

/// Writes frames `[from, to)` of `signal` as a 32-bit float WAV.
fn write_wav(path: &PathBuf, rate: u32, from: u64, to: u64) -> TrackRef {
    write_wav_with(path, rate, from..to, signal)
}

fn write_wav_with(
    path: &PathBuf,
    rate: u32,
    frames: std::ops::Range<u64>,
    f: impl Fn(u64) -> (f32, f32),
) -> TrackRef {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in frames {
        let (l, r) = f(i);
        w.write_sample(l).unwrap();
        w.write_sample(r).unwrap();
    }
    w.finalize().unwrap();
    TrackRef::new(path.to_string_lossy())
}

struct Rig {
    engine: Engine,
    sink: ManualSink,
    now: u64,
    latency_ns: u64,
}

impl Rig {
    fn new(rate: u32, config: EngineConfig) -> Self {
        let sink = ManualSink::new(rate, 2);
        let engine = Engine::new(
            Arc::new(sink.clone()),
            &NativeSpawner,
            Arc::new(NativeFileSource),
            config,
        )
        .unwrap();
        Rig {
            engine,
            sink,
            now: 0,
            latency_ns: 0,
        }
    }

    /// One device callback, advancing fake time by one buffer.
    fn pull(&mut self) -> Vec<f32> {
        let rate = self.engine.stats().format.sample_rate as u64;
        self.now += BUF as u64 * 1_000_000_000 / rate;
        self.sink.set_now_ns(self.now);
        let info = CallbackInfo {
            host_ns: self.now,
            output_latency_ns: self.latency_ns,
        };
        self.sink.pull(BUF, info).unwrap()
    }

    /// Pulls until audio arrives (the decode thread runs concurrently), then returns everything
    /// from the first non-silent buffer on, `buffers` buffers in total.
    fn pull_audio(&mut self, buffers: usize) -> Vec<f32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let first = loop {
            let b = self.pull();
            if b.iter().any(|&s| s != 0.0) {
                break b;
            }
            assert!(Instant::now() < deadline, "no audio arrived");
            std::thread::sleep(Duration::from_millis(1));
        };
        let mut out = first;
        for _ in 1..buffers {
            out.extend(self.pull_waiting());
        }
        out
    }

    /// Pulls one buffer after half of its real-time duration: 2× real time, which the decode
    /// thread must keep up with even while other tests load the CPU (tests assert zero
    /// underruns).
    fn pull_waiting(&mut self) -> Vec<f32> {
        let rate = self.engine.stats().format.sample_rate as u64;
        std::thread::sleep(Duration::from_micros(BUF as u64 * 1_000_000 / rate / 2));
        self.pull()
    }

    fn wait_for<T>(&mut self, mut f: impl FnMut(&mut Engine) -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(v) = f(&mut self.engine) {
                return v;
            }
            assert!(Instant::now() < deadline, "condition not reached");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn expected(from: u64, frames: usize) -> Vec<f32> {
    (from..from + frames as u64)
        .flat_map(|f| {
            let (l, r) = signal(f);
            [l, r]
        })
        .collect()
}

fn assert_close(actual: &[f32], expected: &[f32], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: length");
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a - e).abs() < 1e-6,
            "{what}: sample {i} is {a}, expected {e}"
        );
    }
}

#[test]
fn device_is_open_and_silent_before_anything_plays() {
    let mut rig = Rig::new(48_000, EngineConfig::default());
    assert!(rig.sink.is_running(), "stream starts with the engine");
    assert!(rig.pull().iter().all(|&s| s == 0.0));
    assert_eq!(rig.engine.state(), PlayState::Stopped);
}

#[test]
fn plays_file_bit_exact_and_clock_follows() {
    let dir = temp_dir("play");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 48_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    let out = rig.pull_audio(20);
    assert_close(&out, &expected(0, 20 * BUF), "playback");
    let pos = rig.engine.position();
    assert_eq!(pos.state, PlayState::Playing);
    assert_eq!(rig.engine.current_index(), Some(0));
    assert!(
        pos.frame >= 19 * BUF as u64 && pos.frame <= 21 * BUF as u64,
        "clock at {}",
        pos.frame
    );
}

#[test]
fn pause_resumes_at_the_exact_frame() {
    let dir = temp_dir("pause");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 48_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    let before = rig.pull_audio(4);
    rig.engine.pause();
    assert!(rig.pull().iter().all(|&s| s == 0.0));
    let frozen = rig.engine.position().frame;
    assert_eq!(
        frozen,
        before.len() as u64 / 2,
        "frozen where playback stopped"
    );
    for _ in 0..5 {
        assert!(
            rig.pull().iter().all(|&s| s == 0.0),
            "paused output is silent"
        );
    }
    assert_eq!(
        rig.engine.position().frame,
        frozen,
        "clock frozen while paused"
    );
    rig.engine.resume();
    let after = rig.pull_waiting();
    assert_close(&after, &expected(before.len() as u64 / 2, BUF), "resume");
}

#[test]
fn seek_jumps_without_old_audio() {
    let dir = temp_dir("seek");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 96_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    rig.pull_audio(3);
    rig.engine.seek(1.5);
    // Everything after the seek is either silence (while it loads) or post-seek audio.
    let deadline = Instant::now() + Duration::from_secs(5);
    let first = loop {
        let b = rig.pull();
        if b.iter().any(|&s| s != 0.0) {
            break b;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_close(&first, &expected(72_000, BUF), "post-seek audio");
    let pos = rig.engine.position();
    assert!(pos.discontinuity || pos.frame >= 72_000);
    assert!(
        (72_000..=72_000 + 2 * BUF as u64).contains(&pos.frame),
        "clock at {}",
        pos.frame
    );
}

#[test]
fn gapless_across_tracks_is_bit_exact_with_prewarm_and_clock_switch() {
    let dir = temp_dir("gapless");
    // One continuous signal split at an odd frame.
    let split = 30_001;
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, split);
    let b = write_wav(&dir.join("b.wav"), 48_000, split, 60_000);
    let mut rig = Rig::new(
        48_000,
        EngineConfig {
            prewarm_secs: 30.0,
            ..Default::default()
        },
    );
    rig.engine.set_queue(vec![a, b.clone()]);
    rig.engine.play_index(0);
    let buffers = 60_000 / BUF;
    let out = rig.pull_audio(buffers);
    assert_close(&out, &expected(0, buffers * BUF), "gapless output");
    assert_eq!(rig.engine.stats().underruns, 0);

    let events = rig.engine.poll_events();
    assert!(
        events.contains(&EngineEvent::PreWarm { index: 1, track: b }),
        "pre-warm event for the next track: {events:?}"
    );
    let pos = rig.engine.position();
    assert_eq!(
        rig.engine.current_index(),
        Some(1),
        "clock reports the second track"
    );
    let expected_frame = (buffers * BUF) as u64 - split;
    assert!(
        pos.frame.abs_diff(expected_frame) <= BUF as u64,
        "{} vs {expected_frame}",
        pos.frame
    );
}

#[test]
fn gapless_with_resampling_is_smooth() {
    let dir = temp_dir("gapless-rs");
    let split = 22_050;
    let a = write_wav(&dir.join("a.wav"), 44_100, 0, split);
    let b = write_wav(&dir.join("b.wav"), 44_100, split, 44_100);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a, b]);
    rig.engine.play_index(0);
    let out = rig.pull_audio(100);
    assert_eq!(
        rig.engine.stats().underruns,
        0,
        "decode kept up with playback"
    );
    let left: Vec<f32> = out.chunks(2).map(|f| f[0]).collect();
    // Around the boundary (~24 000 output frames) the signal must stay continuous: the source
    // changes by < 0.005 per input sample, so any gap or click would show as a large step.
    let max_step = left[23_000..25_000]
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0, f32::max);
    assert!(
        max_step < 0.02,
        "discontinuity at the boundary: step {max_step}"
    );
    // Exactly 1 s of 44.1 kHz audio → 48 000 frames at 48 kHz (resampler delay trimmed, tail
    // flushed), give or take the last near-zero samples.
    let last_audible = left.iter().rposition(|s| s.abs() > 1e-4).unwrap();
    assert!(
        (47_990..48_000).contains(&last_audible),
        "track end at frame {last_audible}"
    );
}

#[test]
fn unplayable_track_is_skipped() {
    let dir = temp_dir("skip");
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, 4_800);
    let bad = dir.join("bad.mp3");
    std::fs::write(&bad, vec![0x55u8; 5000]).unwrap();
    let bad = TrackRef::new(bad.to_string_lossy());
    let c = write_wav(&dir.join("c.wav"), 48_000, 4_800, 9_600);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a, bad.clone(), c]);
    rig.engine.play_index(0);
    let out = rig.pull_audio(9_600 / BUF);
    assert_close(
        &out,
        &expected(0, (9_600 / BUF) * BUF),
        "skips the bad track seamlessly",
    );
    let failed = rig.engine.poll_events().into_iter().any(
        |e| matches!(e, EngineEvent::TrackFailed { index: 1, ref track, .. } if *track == bad),
    );
    assert!(failed, "failure reported");
}

#[test]
fn end_of_queue_stops() {
    let dir = temp_dir("end");
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, 2_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a]);
    rig.engine.play_index(0);
    rig.pull_audio(1);
    for _ in 0..10 {
        rig.pull_waiting();
    }
    assert_eq!(rig.engine.state(), PlayState::Stopped);
    assert_eq!(rig.engine.stats().queue_ended, 1);
    assert_eq!(rig.engine.stats().underruns, 0);
}

#[test]
fn next_previous_and_queue_edits() {
    let dir = temp_dir("nav");
    let tracks: Vec<_> = (0..3)
        .map(|i| {
            write_wav(
                &dir.join(format!("{i}.wav")),
                48_000,
                i * 10_000,
                (i + 1) * 10_000,
            )
        })
        .collect();
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(tracks.clone());
    rig.engine.play_index(0);
    rig.pull_audio(1);
    rig.engine.next();
    let out = rig.pull_audio(1);
    assert_close(&out, &expected(10_000, BUF), "next");
    rig.engine.previous();
    let out = rig.pull_audio(1);
    assert_close(&out, &expected(0, BUF), "previous");

    // Reordering keeps the playing track and tracks its new index.
    rig.engine.set_queue(vec![
        tracks[2].clone(),
        tracks[1].clone(),
        tracks[0].clone(),
    ]);
    assert_eq!(rig.engine.current_index(), Some(2));
}

#[test]
fn metadata_arrives_as_events() {
    let dir = temp_dir("meta");
    let t = write_wav(&dir.join("My Song.wav"), 48_000, 0, 4_800);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    rig.pull_audio(1);
    let info = rig.wait_for(|e| {
        e.poll_events().into_iter().find_map(|ev| match ev {
            EngineEvent::TrackInfo { info, .. } => Some(info),
            _ => None,
        })
    });
    assert_eq!(info.title, "My Song");
    assert_eq!(info.sample_rate, 48_000);
    assert!((info.duration_secs.unwrap() - 0.1).abs() < 1e-6);
}

#[test]
fn volume_balance_and_eq_are_applied_immediately() {
    let dir = temp_dir("levels");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 96_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    let start = rig.pull_audio(1).len() as u64 / 2;
    rig.engine.set_volume(0.0);
    rig.pull_waiting(); // ramp buffer
    assert!(
        rig.pull_waiting().iter().all(|&s| s == 0.0),
        "muted within one buffer"
    );

    rig.engine.set_volume(1.0);
    rig.engine.set_balance(-1.0);
    rig.pull_waiting();
    let b = rig.pull_waiting();
    assert!(
        b.chunks(2).all(|f| f[1] == 0.0) && b.chunks(2).any(|f| f[0] != 0.0),
        "hard left"
    );

    rig.engine.set_balance(0.0);
    let mut eq = rig.engine.eq();
    eq.enabled = true;
    eq.preamp_db = -12.0;
    rig.engine.set_eq(eq);
    rig.pull_waiting();
    let b = rig.pull_waiting();
    let frame = start + 4 * BUF as u64;
    let quiet = expected(frame, BUF);
    let ratio = b.iter().map(|x| x.abs()).sum::<f32>() / quiet.iter().map(|x| x.abs()).sum::<f32>();
    assert!(
        (ratio - 0.251).abs() < 0.02,
        "-12 dB preamp → ×0.25, got {ratio}"
    );
    assert_eq!(rig.engine.eq().preamp_db, -12.0);
}

#[test]
fn tap_carries_stamped_samples() {
    let dir = temp_dir("tap");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 48_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    let mut tap = rig.engine.take_tap().unwrap();
    assert!(rig.engine.take_tap().is_none(), "single reader");
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    rig.engine.set_volume(0.0);
    let deadline = Instant::now() + Duration::from_secs(5);
    let chunk = loop {
        let silent = rig.pull();
        assert!(
            silent.iter().all(|&s| s == 0.0),
            "volume 0 is silent on the device"
        );
        if let Some(c) = tap.pop() {
            break c;
        }
        assert!(Instant::now() < deadline, "no tap data");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(chunk.start_frame, 0);
    assert_close(
        chunk.data(),
        &expected(0, chunk.frames),
        "tap ignores volume",
    );
}

#[test]
fn device_loss_rebuilds_stream_and_continues_at_new_rate() {
    let dir = temp_dir("device");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 96_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    rig.pull_audio(40);
    let before = rig.engine.position().seconds();

    rig.sink.set_format(44_100, 2);
    rig.sink.fail(SinkError::DeviceLost);
    let sink = rig.sink.clone();
    rig.wait_for(|_| (sink.starts() == 2).then_some(()));
    rig.wait_for(|e| (e.stats().format.sample_rate == 44_100).then_some(()));

    let out = rig.pull_audio(10);
    assert!(
        out.iter().any(|&s| s != 0.0),
        "audio continues after the device change"
    );
    let after = rig.engine.position();
    assert_eq!(after.sample_rate, 44_100);
    assert!(
        (after.seconds() - before).abs() < 0.2,
        "resumed near {before:.3}s, at {:.3}s",
        after.seconds()
    );
    assert_eq!(rig.engine.state(), PlayState::Playing);
}

/// The clock must say "frame F" at the moment frame F reaches the speaker, i.e. one output
/// latency after the callback that wrote it (spec: within ±2 ms).
#[test]
fn clock_reports_a_click_when_it_is_heard() {
    let dir = temp_dir("click");
    let click = 30_000u64;
    let t = write_wav_with(&dir.join("click.wav"), 48_000, 0..48_000, |f| {
        if f == click { (1.0, 1.0) } else { (0.0, 0.001) }
    });
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.latency_ns = 12_000_000;
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);

    let deadline = Instant::now() + Duration::from_secs(5);
    let (host_ns, offset) = loop {
        let b = rig.pull_waiting();
        if let Some(i) = b.chunks(2).position(|f| f[0] == 1.0) {
            break (rig.now, i as u64);
        }
        assert!(Instant::now() < deadline, "click never played");
    };
    let heard_at = host_ns + rig.latency_ns + offset * 1_000_000_000 / 48_000;
    let mut reader = rig.engine.clock_reader();
    let pos = reader.position(heard_at);
    let error_ms = (pos.frame as f64 - click as f64) / 48.0;
    assert!(
        error_ms.abs() <= 2.0,
        "clock off by {error_ms:.3} ms at the click"
    );
    // 10 ms later the clock has moved on by 10 ms.
    let later = reader.position(heard_at + 10_000_000);
    assert_eq!(later.frame - pos.frame, 480);
}

#[test]
fn clock_switches_track_at_the_audible_gapless_boundary_when_resampling() {
    let dir = temp_dir("boundary-rs");
    let a = write_wav(&dir.join("a.wav"), 44_100, 0, 22_050);
    let b = write_wav(&dir.join("b.wav"), 44_100, 22_050, 44_100);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a, b]);
    rig.engine.play_index(0);
    rig.pull_audio(1);
    let mut reader = rig.engine.clock_reader();
    // Pull until the clock has crossed into the second track; the first frame reported for it
    // must be (close to) frame 0, at output frame 24 000 of the stream.
    let mut switched = None;
    for _ in 0..80 {
        rig.pull_waiting();
        let p = reader.position(rig.now);
        if rig.engine.current_index() == Some(1) {
            switched = Some(p);
            break;
        }
    }
    let p = switched.expect("never reached the second track");
    assert!(
        p.frame < 2 * BUF as u64,
        "second track starts at its frame 0, got {}",
        p.frame
    );
    assert_eq!(rig.engine.stats().underruns, 0);
}

#[test]
fn repeat_one_loops_gaplessly() {
    let dir = temp_dir("repeat-one");
    let len = 5_000u64;
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, len);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_repeat(audio::RepeatMode::One);
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    let out = rig.pull_audio(30); // 15 360 frames ≈ 3 loops
    let expected: Vec<f32> = (0..30 * BUF as u64)
        .flat_map(|i| {
            let (l, r) = signal(i % len);
            [l, r]
        })
        .collect();
    assert_close(&out, &expected, "repeat one");
    assert_eq!(rig.engine.stats().underruns, 0);
    assert_eq!(rig.engine.state(), PlayState::Playing);
}

#[test]
fn repeat_all_wraps_to_the_first_track() {
    let dir = temp_dir("repeat-all");
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, 3_000);
    let b = write_wav(&dir.join("b.wav"), 48_000, 3_000, 6_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_repeat(audio::RepeatMode::All);
    rig.engine.set_queue(vec![a, b]);
    rig.engine.play_index(0);
    let out = rig.pull_audio(20); // 10 240 frames: a, b, a, b(part)
    let expected: Vec<f32> = (0..20 * BUF as u64)
        .flat_map(|i| {
            let (l, r) = signal(i % 6_000);
            [l, r]
        })
        .collect();
    assert_close(&out, &expected, "repeat all");
    assert_eq!(rig.engine.stats().queue_ended, 0);
    // `next` from the last track wraps as well.
    rig.engine.play_index(1);
    rig.pull_audio(1);
    rig.engine.next();
    let out = rig.pull_audio(1);
    assert_close(&out, &expected[..BUF * 2], "next wraps");
}

/// Pulls until `frames` frames of audio (from the first audible buffer) are collected,
/// recording the clock after every buffer.
fn collect(rig: &mut Rig, first: Vec<f32>, frames: usize) -> (Vec<f32>, Vec<audio::Position>) {
    let mut out = first;
    let mut clock = Vec::new();
    while out.len() < frames * 2 {
        out.extend(rig.pull_waiting());
        clock.push(rig.engine.position());
    }
    out.truncate(frames * 2);
    (out, clock)
}

fn wait_event(rig: &mut Rig, f: impl Fn(&EngineEvent) -> bool) -> EngineEvent {
    rig.wait_for(|e| e.poll_events().into_iter().find(|ev| f(ev)))
}

/// Largest jump between consecutive samples of one channel: a click shows up as a big step.
fn max_step(samples: &[f32]) -> f32 {
    samples
        .as_chunks::<2>()
        .0
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).abs().max((w[1][1] - w[0][1]).abs()))
        .fold(0.0, f32::max)
}

#[test]
fn scheduled_jump_splices_at_the_exact_frame() {
    let dir = temp_dir("seek-at");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 480_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    let first = rig.pull_audio(1);
    let track = rig.engine.position().track;
    // Candidates: one already sent (0.005 s), then 1.0 s. Jump to 5.0 s.
    rig.engine.seek_at(track, vec![0.005, 1.0], 5.0);
    let ev = wait_event(&mut rig, |e| {
        matches!(
            e,
            EngineEvent::JumpScheduled { .. } | EngineEvent::JumpMissed { .. }
        )
    });
    assert_eq!(
        ev,
        EngineEvent::JumpScheduled {
            track,
            at_secs: 1.0
        }
    );
    let (out, clock) = collect(&mut rig, first, 48_000 + 8_192);
    let xf = 96;
    assert_close(
        &out[..48_000 * 2],
        &expected(0, 48_000),
        "before the splice",
    );
    assert_close(
        &out[(48_000 + xf) * 2..],
        &expected(240_000 + xf as u64, 8_192 - xf),
        "after the splice",
    );
    assert!(
        max_step(&out[(48_000 - 4) * 2..(48_000 + xf + 4) * 2]) < 0.03,
        "no click at the splice"
    );
    assert_eq!(rig.engine.stats().underruns, 0);
    // The clock follows the jump (no clamp at the old position) and flags it.
    assert!(
        clock.iter().any(|p| p.discontinuity),
        "discontinuity reported"
    );
    let last = clock.last().unwrap();
    assert!(
        (240_000..=240_000 + 8_192 + 2 * BUF as u64).contains(&last.frame),
        "clock at {}",
        last.frame
    );
}

#[test]
fn a_jump_too_late_for_every_point_is_reported() {
    let dir = temp_dir("seek-at-late");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 96_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    rig.pull_audio(2);
    let track = rig.engine.position().track;
    rig.engine.seek_at(track, vec![0.001, 0.002], 1.5);
    let ev = wait_event(&mut rig, |e| {
        matches!(
            e,
            EngineEvent::JumpScheduled { .. } | EngineEvent::JumpMissed { .. }
        )
    });
    assert_eq!(ev, EngineEvent::JumpMissed { track });
}

#[test]
fn loop_wraps_gaplessly_and_the_clock_goes_back() {
    let dir = temp_dir("loop");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 480_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    let first = rig.pull_audio(1);
    let track = rig.engine.position().track;
    rig.engine.set_loop(track, Some((1.0, 1.5)));
    let ev = wait_event(&mut rig, |e| {
        matches!(
            e,
            EngineEvent::LoopChanged { .. } | EngineEvent::LoopRejected { .. }
        )
    });
    assert_eq!(
        ev,
        EngineEvent::LoopChanged {
            track,
            region: Some((1.0, 1.5))
        }
    );
    // 1.5 s, then the 0.5 s loop twice, then a bit.
    let len = 24_000;
    let (out, clock) = collect(&mut rig, first, 72_000 + 2 * len + 4_096);
    let xf = 96;
    assert_close(
        &out[..72_000 * 2],
        &expected(0, 72_000),
        "up to the loop end",
    );
    for k in 0..2 {
        let at = 72_000 + k * len;
        assert_close(
            &out[(at + xf) * 2..(at + len) * 2],
            &expected(48_000 + xf as u64, len - xf),
            "loop body",
        );
        assert!(
            max_step(&out[(at - 4) * 2..(at + xf + 4) * 2]) < 0.03,
            "no click at wrap {k}"
        );
    }
    assert_eq!(rig.engine.stats().underruns, 0);
    let last = clock.last().unwrap();
    assert!(
        (48_000..72_000).contains(&last.frame),
        "clock inside the loop: {}",
        last.frame
    );
}

#[test]
fn a_seek_ends_the_loop() {
    let dir = temp_dir("loop-seek");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 480_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    rig.pull_audio(1);
    let track = rig.engine.position().track;
    rig.engine.set_loop(track, Some((2.0, 3.0)));
    wait_event(&mut rig, |e| {
        matches!(
            e,
            EngineEvent::LoopChanged {
                region: Some(_),
                ..
            }
        )
    });
    rig.engine.seek(5.0);
    let ev = wait_event(&mut rig, |e| matches!(e, EngineEvent::LoopChanged { .. }));
    assert_eq!(
        ev,
        EngineEvent::LoopChanged {
            track,
            region: None
        }
    );
}

#[test]
fn a_track_change_ends_the_loop() {
    let dir = temp_dir("loop-next");
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, 480_000);
    let b = write_wav(&dir.join("b.wav"), 48_000, 0, 96_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a, b]);
    rig.engine.play_index(0);
    rig.pull_audio(1);
    let track = rig.engine.position().track;
    rig.engine.set_loop(track, Some((2.0, 3.0)));
    wait_event(&mut rig, |e| {
        matches!(
            e,
            EngineEvent::LoopChanged {
                region: Some(_),
                ..
            }
        )
    });
    rig.engine.next();
    let ev = wait_event(&mut rig, |e| matches!(e, EngineEvent::LoopChanged { .. }));
    assert_eq!(
        ev,
        EngineEvent::LoopChanged {
            track,
            region: None
        }
    );
}

/// Zero crossings (rising, left channel) per second of output at 48 kHz.
fn crossings_per_sec(samples: &[f32]) -> f64 {
    let left: Vec<f32> = samples.chunks(2).map(|f| f[0]).collect();
    let n = left
        .windows(2)
        .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
        .count();
    n as f64 * 48_000.0 / left.len() as f64
}

fn sine_wav(path: &PathBuf, hz: f64, frames: std::ops::Range<u64>) -> TrackRef {
    write_wav_with(path, 48_000, frames, |i| {
        let v = (2.0 * std::f64::consts::PI * hz * i as f64 / 48_000.0).sin() as f32 * 0.5;
        (v, v)
    })
}

#[test]
fn a_speed_change_is_heard_within_50_ms_without_click_or_gap() {
    let dir = temp_dir("speed-change");
    let t = sine_wav(&dir.join("a.wav"), 1_000.0, 0..480_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.latency_ns = 5_000_000;
    rig.engine.set_queue(vec![t]);
    rig.engine.play_index(0);
    let mut out = rig.pull_audio(40);
    let before = rig.engine.position();
    assert_eq!(before.speed, 1.0);

    rig.engine.set_speed(0, 1.04);
    assert_eq!(rig.engine.speed(0), 1.04);
    let changed_at = out.len() / 2;
    let mut heard = None;
    let mut clock = vec![before];
    for i in 0..60 {
        out.extend(rig.pull_waiting());
        let p = rig.engine.position();
        if heard.is_none() && p.speed == 1.04 {
            heard = Some((i + 1) * BUF);
        }
        clock.push(p);
    }
    let heard = heard.expect("the new speed reached the clock");
    // A buffer for the device's audio, the 5 ms latency, and the worker's restart.
    assert!(heard * 1000 / 48_000 <= 50, "heard after {heard} frames");
    // A sine at 0.5 changes by at most 0.068 per sample at 1.04 × 1 kHz.
    assert!(
        max_step(&out[changed_at * 2..]) < 0.08,
        "click at the change"
    );
    let tail = &out[(out.len() / 2 - 20 * BUF) * 2..];
    assert!(tail.iter().all(|s| s.abs() <= 0.5001), "no overshoot");
    let hz = crossings_per_sec(tail);
    assert!((1_035.0..1_045.0).contains(&hz), "pitch follows: {hz} Hz");
    assert_eq!(rig.engine.stats().underruns, 0);
    assert!(
        clock.windows(2).all(|w| w[1].frame >= w[0].frame),
        "the clock never goes back"
    );
}

#[test]
fn the_clock_runs_in_track_time_under_varispeed() {
    let dir = temp_dir("speed-clock");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 480_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.set_speed(0, 1.06);
    rig.engine.play_index(0);
    let first = rig.pull_audio(1);
    let (_, clock) = collect(&mut rig, first, 100 * BUF);
    let (a, b) = (clock[10], *clock.last().unwrap());
    assert_eq!(b.speed, 1.06);
    let per_frame = (b.frame - a.frame) as f64 / ((clock.len() - 11) * BUF) as f64;
    assert!(
        (per_frame - 1.06).abs() < 1e-3,
        "{per_frame} track frames/frame"
    );
    // Track seconds: 100 buffers of output at 1.06 is 1.13 s of the track (± two buffers:
    // the clock is read after each pull, and the first buffer may be partly silent).
    let expected = 100.0 * BUF as f64 * 1.06 / 48_000.0;
    assert!((b.seconds() - expected).abs() < 0.023, "{}", b.seconds());

    // Down to 0.96 mid-track: never backwards, then 0.96 track seconds per second.
    rig.engine.set_speed(0, 0.96);
    let mut after = Vec::new();
    for _ in 0..60 {
        rig.pull_waiting();
        after.push(rig.engine.position());
    }
    assert!(after.iter().all(|p| p.frame >= b.frame));
    assert!(after.windows(2).all(|w| w[1].frame >= w[0].frame));
    let (c, d) = (after[20], after[59]);
    assert_eq!(d.speed, 0.96);
    let rate = (d.frame - c.frame) as f64 / (39 * BUF) as f64;
    assert!((rate - 0.96).abs() < 1e-3, "{rate}");
    assert_eq!(rig.engine.stats().underruns, 0);
}

#[test]
fn gapless_across_tracks_at_different_speeds() {
    let dir = temp_dir("speed-gapless");
    let split = 24_000;
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, split);
    let b = write_wav(&dir.join("b.wav"), 48_000, split, 96_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a, b]);
    rig.engine.set_speed(0, 1.03);
    rig.engine.set_speed(1, 0.97);
    rig.engine.play_index(0);
    let out = rig.pull_audio(100);
    assert_eq!(rig.engine.stats().underruns, 0);
    // Track a lasts 24 000 / 1.03 = 23 301 output frames; the signal is continuous across
    // the files, so a gap or click shows as a step (the source moves < 0.005 per frame).
    let boundary = (split as f64 / 1.03).round() as usize;
    let around = &out[(boundary - 1_000) * 2..(boundary + 1_000) * 2];
    assert!(max_step(around) < 0.02, "step {}", max_step(around));
    let pos = rig.engine.position();
    assert_eq!((rig.engine.current_index(), pos.speed), (Some(1), 0.97));
    let expected = ((100 * BUF - boundary) as f64 * 0.97) as u64;
    assert!(
        pos.frame.abs_diff(expected) <= BUF as u64,
        "{} vs {expected}",
        pos.frame
    );
}

#[test]
fn jumps_and_loops_land_in_track_time_at_speed_1_04() {
    let dir = temp_dir("speed-jump");
    let t = write_wav(&dir.join("a.wav"), 48_000, 0, 480_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![t]);
    rig.engine.set_speed(0, 1.04);
    rig.engine.play_index(0);
    let first = rig.pull_audio(1);
    let track = rig.engine.position().track;
    rig.engine.seek_at(track, vec![1.0], 5.0);
    let ev = wait_event(&mut rig, |e| {
        matches!(
            e,
            EngineEvent::JumpScheduled { .. } | EngineEvent::JumpMissed { .. }
        )
    });
    assert_eq!(
        ev,
        EngineEvent::JumpScheduled {
            track,
            at_secs: 1.0
        }
    );
    // 1 s of the track is 1 / 1.04 s of output; then the track from 5 s, at 1.04.
    let at = (48_000.0f64 / 1.04).round() as usize;
    let (out, clock) = collect(&mut rig, first, at + 8_192);
    let landed = &out[(at + 96) * 2..(at + 96 + 16) * 2];
    let expected_first = expected(240_000 + 100, 1)[0];
    assert!(
        (landed[0] - expected_first).abs() < 0.01,
        "lands at 5 s: {} vs {expected_first}",
        landed[0]
    );
    assert!(
        max_step(&out[(at - 4) * 2..(at + 100) * 2]) < 0.03,
        "no click"
    );
    let last = clock.last().unwrap();
    assert!((5.0..5.25).contains(&last.seconds()), "{}", last.seconds());

    // A loop of half a second keeps the clock in it, in track seconds.
    rig.engine.set_loop(track, Some((6.0, 6.5)));
    wait_event(&mut rig, |e| matches!(e, EngineEvent::LoopChanged { .. }));
    let mut inside = Vec::new();
    for _ in 0..200 {
        rig.pull_waiting();
        let p = rig.engine.position();
        if p.seconds() >= 6.0 {
            inside.push(p.seconds());
        }
    }
    assert!(inside.len() > 100, "reached the loop");
    assert!(inside.iter().all(|&s| s < 6.5 + 0.011), "stays in it");
    assert!(inside.windows(2).any(|w| w[1] < w[0]), "and wraps");
    assert_eq!(rig.engine.stats().underruns, 0);
}

#[test]
fn a_speed_change_in_a_tracks_last_moments_leaves_the_queue_playing() {
    let dir = temp_dir("speed-end");
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, 48_000);
    let b = write_wav(&dir.join("b.wav"), 48_000, 48_000, 96_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a, b]);
    rig.engine.play_index(0);
    rig.pull_audio(1);
    while rig.engine.position().seconds() < 0.99 {
        rig.pull_waiting();
    }
    // Restarting 20-odd ms ahead would be past the end: it finishes at its own speed.
    rig.engine.set_speed(0, 1.08);
    for _ in 0..40 {
        rig.pull_waiting();
    }
    let events = rig.engine.poll_events();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EngineEvent::TrackFailed { .. })),
        "{events:?}"
    );
    assert_eq!(rig.engine.current_index(), Some(1));
    assert_eq!(rig.engine.stats().underruns, 0);
}

#[test]
fn a_speed_change_at_a_gapless_boundary_applies_to_the_audible_track() {
    let dir = temp_dir("speed-boundary");
    let a = write_wav(&dir.join("a.wav"), 48_000, 0, 48_000);
    let b = write_wav(&dir.join("b.wav"), 48_000, 48_000, 72_000);
    let mut rig = Rig::new(48_000, EngineConfig::default());
    rig.engine.set_queue(vec![a, b]);
    rig.engine.play_index(0);
    rig.pull_audio(1);
    // 0.8 s in, the decoder is already in b (the ring holds 0.5 s): the change is for a.
    while rig.engine.position().seconds() < 0.8 {
        rig.pull_waiting();
    }
    rig.engine.set_speed(0, 1.08);
    let mut speeds = Vec::new();
    for _ in 0..60 {
        rig.pull_waiting();
        let p = rig.engine.position();
        speeds.push((rig.engine.current_index(), p.speed));
    }
    let events = rig.engine.poll_events();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EngineEvent::TrackFailed { .. })),
        "{events:?}"
    );
    assert!(speeds.contains(&(Some(0), 1.08)), "a sped up: {speeds:?}");
    assert!(
        speeds.contains(&(Some(1), 1.0)),
        "then b at its own: {speeds:?}"
    );
    assert_eq!(rig.engine.stats().underruns, 0);
}
