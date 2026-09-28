//! Spec (playlist, "Skip what is not ready"): track 3 ends while track 4 is still downloading
//! and track 5 is playable → track 5 follows track 3 gaplessly, and track 4 is not marked as
//! failed.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio::{Engine, EngineConfig, EngineEvent};
use platform::native::{NativeFileSource, NativeSpawner};
use platform::testing::ManualSink;
use platform::{CallbackInfo, TrackRef};
use ui::playlist::{EntryStatus, Playlist};

const BUF: usize = 512;
const RATE: u32 = 48_000;

/// A deterministic, non-periodic stereo signal, so a missing or repeated sample shows.
fn signal(frame: u64) -> (f32, f32) {
    let t = frame as f32;
    ((t * 0.0123).sin() * 0.4, (t * 0.0071).cos() * 0.3)
}

/// Frames `[from, to)` of `signal` as a float WAV.
fn write_wav(path: &Path, from: u64, to: u64) -> TrackRef {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in from..to {
        let (l, r) = signal(i);
        w.write_sample(l).unwrap();
        w.write_sample(r).unwrap();
    }
    w.finalize().unwrap();
    TrackRef::new(path.to_string_lossy())
}

#[test]
fn a_waiting_entry_is_skipped_gaplessly_and_not_marked_failed() {
    let dir = platform::testing::TestDir::new("ui-waiting-skip");
    // Tracks 3 and 5 are one continuous signal split at an odd frame.
    let split = 30_001;
    let mut playlist = Playlist::default();
    playlist.add([
        write_wav(&dir.join("1.wav"), 0, 4_800),
        write_wav(&dir.join("2.wav"), 0, 4_800),
        write_wav(&dir.join("3.wav"), 0, split),
    ]);
    let four = playlist.add_waiting(
        "Nightcraft",
        "Glasshouse",
        Some("https://www.youtube.com/watch?v=abcdefghijk".into()),
        None,
        "downloading 40%",
    );
    playlist.add([write_wav(&dir.join("5.wav"), split, 60_000)]);
    let ids: Vec<_> = playlist.entries().iter().map(|e| e.id).collect();

    // The app builds the engine queue from the playable entries in play order.
    let (queue, tracks): (Vec<_>, Vec<_>) = playlist.queue(false, None, 1).into_iter().unzip();
    assert_eq!(queue, [ids[0], ids[1], ids[2], ids[4]]);

    let sink = ManualSink::new(RATE, 2);
    let mut engine = Engine::new(
        Arc::new(sink.clone()),
        &NativeSpawner,
        Arc::new(NativeFileSource),
        EngineConfig {
            prewarm_secs: 30.0,
            ..Default::default()
        },
    )
    .unwrap();
    engine.set_queue(tracks);
    engine.play_index(2); // track 3

    let mut now_ns = 0u64;
    let mut pull = |wait: bool| {
        if wait {
            // 2× real time, which the decoder keeps up with even on a loaded machine.
            std::thread::sleep(Duration::from_micros(
                BUF as u64 * 1_000_000 / RATE as u64 / 2,
            ));
        }
        now_ns += BUF as u64 * 1_000_000_000 / RATE as u64;
        sink.set_now_ns(now_ns);
        sink.pull(
            BUF,
            CallbackInfo {
                host_ns: now_ns,
                output_latency_ns: 0,
            },
        )
        .unwrap()
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut out = loop {
        let b = pull(false);
        if b.iter().any(|&s| s != 0.0) {
            break b;
        }
        assert!(Instant::now() < deadline, "no audio arrived");
        std::thread::sleep(Duration::from_millis(1));
    };
    let buffers = 60_000 / BUF;
    for _ in 1..buffers {
        out.extend(pull(true));
    }

    // Track 3 then track 5, sample for sample: no gap, nothing in between.
    for (i, pair) in out.chunks(2).enumerate() {
        let (l, r) = signal(i as u64);
        assert!(
            (pair[0] - l).abs() < 1e-6 && (pair[1] - r).abs() < 1e-6,
            "frame {i}: {pair:?} vs ({l}, {r})"
        );
    }
    assert_eq!(engine.stats().underruns, 0);
    assert_eq!(engine.current_index(), Some(3), "the clock is on track 5");

    // The app maps engine events to entries through the queue, as it does while playing.
    for ev in engine.poll_events() {
        if let EngineEvent::TrackFailed { index, .. } = ev {
            playlist.set_failed(queue[index]);
        }
    }
    let status = &playlist.get(four).unwrap().status;
    assert_eq!(status, &EntryStatus::Waiting("downloading 40%".into()));
}
