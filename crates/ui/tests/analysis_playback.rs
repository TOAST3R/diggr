//! Spec (streaming-analyzer, "Playback isolation"): with analysis running on the current and the
//! pre-warmed track, playback has zero underruns.

use std::sync::Arc;
use std::time::{Duration, Instant};

use analysis::cache::ScoreCache;
use analysis::{AnalysisService, synth};
use audio::{Engine, EngineConfig, PlayState};
use platform::native::{NativeFileSource, NativeSpawner};
use platform::testing::ManualSink;
use platform::{CallbackInfo, TrackRef};

fn write(dir: &std::path::Path, name: &str, bpm: f64) -> TrackRef {
    let s = synth::render(&synth::standard_track(bpm), 44_100, bpm as u64);
    let path = dir.join(name);
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for &x in &s.samples {
        let v = (x * 32_000.0) as i16;
        w.write_sample(v).unwrap();
        w.write_sample(v).unwrap();
    }
    w.finalize().unwrap();
    TrackRef::new(path.to_string_lossy())
}

#[test]
fn playback_has_no_underruns_while_two_tracks_are_analyzed() {
    let dir = platform::testing::TestDir::new("ui-analysis-playback");
    let current = write(&dir, "current.wav", 126.0);
    let next = write(&dir, "next.wav", 128.0);

    let sink = ManualSink::new(48_000, 2);
    let mut engine = Engine::new(
        Arc::new(sink.clone()),
        &NativeSpawner,
        Arc::new(NativeFileSource),
        EngineConfig::default(),
    )
    .unwrap();
    engine.set_queue(vec![current.clone(), next.clone()]);
    engine.play_index(0);
    let svc = AnalysisService::new(
        Arc::new(NativeSpawner),
        Arc::new(NativeFileSource),
        Some(ScoreCache::new(dir.join("cache"))),
    );
    svc.prewarm(&next);

    // Play in real time (512 frames every 10.7 ms) for 6 s while both tracks are analyzed.
    let buffer = Duration::from_micros(512 * 1_000_000 / 48_000);
    let (start, mut now_ns) = (Instant::now(), 0u64);
    while start.elapsed() < Duration::from_secs(6) {
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
        svc.playhead(&current, engine.position().seconds());
        std::thread::sleep(buffer);
    }
    assert_eq!(engine.state(), PlayState::Playing);
    assert_eq!(engine.stats().underruns, 0, "analysis disturbed playback");
    // Both tracks were being analyzed during that window (at minimum OS priority, so how far it
    // got depends on the machine; give it time to show real progress on both).
    let covered = |t: &TrackRef| {
        svc.score(t)
            .and_then(|s| s.coverage.covered_until(0.0))
            .unwrap_or(0.0)
    };
    let deadline = Instant::now() + Duration::from_secs(60);
    while covered(&current) < 20.0 || covered(&next) < 20.0 {
        assert!(
            Instant::now() < deadline,
            "analysis stalled: {:.0} s / {:.0} s",
            covered(&current),
            covered(&next)
        );
        svc.playhead(&current, 1.0);
        std::thread::sleep(Duration::from_millis(50));
    }
}
