//! Spec scenario "Large add during playback": 2,000 files added while music plays → playback
//! has no underruns and entries update progressively.

use std::sync::Arc;
use std::time::{Duration, Instant};

use audio::{Engine, EngineConfig, PlayState, RepeatMode};
use platform::native::{NativeFileSource, NativeSpawner};
use platform::testing::ManualSink;
use platform::{CallbackInfo, TrackRef};
use ui::metadata::{MetaResult, MetaWorker};

fn fixture(name: &str) -> TrackRef {
    TrackRef::new(format!(
        "{}/../audio/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

#[test]
fn adding_2000_files_during_playback_causes_no_underruns() {
    let sink = ManualSink::new(48_000, 2);
    let mut engine = Engine::new(
        Arc::new(sink.clone()),
        &NativeSpawner,
        Arc::new(NativeFileSource),
        EngineConfig::default(),
    )
    .unwrap();
    engine.set_repeat(RepeatMode::One); // the 2 s tone loops for as long as the test runs
    engine.set_queue(vec![fixture("tone.flac")]);
    engine.play_index(0);

    let worker =
        MetaWorker::start(&NativeSpawner, Arc::new(NativeFileSource), None, || {}).unwrap();
    let names = ["tone.mp3", "tone.flac", "tone.ogg", "tone.wav", "tone.m4a"];
    worker.request(
        (0..2000)
            .map(|i| (i as u64, fixture(names[i % names.len()])))
            .collect(),
    );

    // Play in real time: one 512-frame buffer every 10.67 ms.
    let buffer = Duration::from_micros(512 * 1_000_000 / 48_000);
    let start = Instant::now();
    let mut now_ns = 0u64;
    let mut results = Vec::new();
    let mut progressive = Vec::new();
    while results.len() < 2000 {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "metadata reading stalled"
        );
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
        let batch = worker.poll();
        if !batch.is_empty() {
            progressive.push(batch.len());
        }
        results.extend(batch);
        std::thread::sleep(buffer);
    }

    assert_eq!(engine.state(), PlayState::Playing);
    assert_eq!(
        engine.stats().underruns,
        0,
        "playback was disturbed by the metadata worker"
    );
    assert!(
        results
            .iter()
            .all(|r| matches!(r, MetaResult::Info(_, i) if i.artist == "M83"))
    );
    assert!(
        progressive.len() > 1,
        "results arrive progressively, not all at the end"
    );
    eprintln!(
        "2000 entries read in {:.2} s while playing",
        start.elapsed().as_secs_f64()
    );
}
