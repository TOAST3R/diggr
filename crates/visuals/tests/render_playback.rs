//! Rendering a show must not disturb playback: real-time playback with zero underruns while a
//! render runs on the GPU and CPU.

use std::sync::Arc;
use std::time::{Duration, Instant};

use analysis::cache::ScoreCache;
use audio::{Engine, EngineConfig, PlayState, RepeatMode};
use platform::native::{NativeFileSource, NativeSpawner};
use platform::testing::ManualSink;
use platform::{CallbackInfo, TrackRef};
use visuals::render::{Options, headless_gpu, render_frames};

fn fixture(name: &str) -> TrackRef {
    TrackRef::new(format!(
        "{}/../audio/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

#[test]
fn playback_has_no_underruns_while_a_show_renders() {
    let Ok(gpu) = headless_gpu() else { return };
    let dir = std::env::temp_dir().join(format!("render-playback-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let sink = ManualSink::new(48_000, 2);
    let mut engine = Engine::new(
        Arc::new(sink.clone()),
        &NativeSpawner,
        Arc::new(NativeFileSource),
        EngineConfig::default(),
    )
    .unwrap();
    engine.set_repeat(RepeatMode::One);
    engine.set_queue(vec![fixture("tone.flac")]);
    engine.play_index(0);

    let opts = Options {
        size: (1280, 720),
        fps: 60,
        from: 0.0,
        to: Some(1.9),
        visuals_dir: dir.join("visuals"),
        cache: Some(ScoreCache::new(dir.join("cache"))),
        ..Options::new(fixture("tone.flac"), dir.join("unused.mp4"))
    };
    let render = std::thread::spawn(move || {
        render_frames(&NativeFileSource, &opts, gpu, |_| Ok(()), |_| true)
    });

    // Play in real time until the render is done (at least 2 s).
    let buffer = Duration::from_micros(512 * 1_000_000 / 48_000);
    let start = Instant::now();
    let mut now_ns = 0u64;
    while !render.is_finished() || start.elapsed() < Duration::from_secs(2) {
        assert!(start.elapsed() < Duration::from_secs(120), "render stalled");
        now_ns += buffer.as_nanos() as u64;
        sink.set_now_ns(now_ns);
        sink.pull(
            512,
            CallbackInfo {
                host_ns: now_ns,
                output_latency_ns: 0,
            },
        )
        .unwrap();
        std::thread::sleep(buffer);
    }
    let frames = render.join().unwrap().unwrap();
    assert_eq!(frames, 114);
    assert_eq!(engine.state(), PlayState::Playing);
    assert_eq!(
        engine.stats().underruns,
        0,
        "playback was disturbed by the render"
    );
    std::fs::remove_dir_all(dir).ok();
}
