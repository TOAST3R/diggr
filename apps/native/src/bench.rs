//! Non-interactive measurements on the real output device (task 9.2).
//!
//! For each file: time from play request to first audio at the device, then seeks at 25/50/75%.
//! Finally the whole queue plays through gaplessly while underruns are counted.
//!
//! With `--analysis`, the music analyzer runs throughout (the playing track and, earlier than in
//! real use, the next one) to show it does not slow playback down.

use std::time::{Duration, Instant};

use std::sync::Arc;

use analysis::AnalysisService;
use audio::{Engine, EngineEvent, PlayState};
use platform::TrackRef;
use platform::native::{NativeFileSource, NativeSpawner};

/// Keeps the analyzer busy the way the player does: playhead every tick, next track pre-warmed.
struct Load {
    svc: Option<AnalysisService>,
    files: Vec<TrackRef>,
}

impl Load {
    fn tick(&self, engine: &mut Engine) {
        let Some(svc) = &self.svc else { return };
        if let Some(i) = engine.current_index() {
            svc.playhead(&self.files[i], engine.position().seconds());
            if let Some(next) = self.files.get(i + 1) {
                svc.prewarm(next);
            }
        }
    }

    /// Sleeps `d` while ticking every 5 ms.
    fn sleep(&self, engine: &mut Engine, d: Duration) {
        let end = Instant::now() + d;
        while Instant::now() < end {
            self.tick(engine);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

const START_TARGET_MS: f64 = 30.0;
const SEEK_TARGET_MS: f64 = 50.0;

pub fn run(files: Vec<TrackRef>, volume: f32, with_analysis: bool) -> Result<(), String> {
    if files.is_empty() {
        return Err("--bench needs audio files".into());
    }
    let opened = Instant::now();
    let mut engine = crate::open_engine()?;
    let fmt = engine.stats().format;
    println!(
        "device: {} Hz, {} ch, buffer {:?} frames (opened in {:.1} ms), volume {:.0}%",
        fmt.sample_rate,
        fmt.channels,
        fmt.buffer_frames,
        opened.elapsed().as_secs_f64() * 1e3,
        volume * 100.0
    );
    engine.set_volume(volume);
    engine.set_queue(files.clone());
    // A fresh cache, so everything really gets analyzed during the run; deleted afterwards.
    let cache = with_analysis.then(|| platform::testing::TestDir::new("diggr-bench-cache"));
    let load = Load {
        svc: cache.as_ref().map(|dir| {
            AnalysisService::new(
                Arc::new(NativeSpawner),
                Arc::new(NativeFileSource),
                Some(analysis::cache::ScoreCache::new(dir.path())),
            )
        }),
        files: files.clone(),
    };
    println!(
        "analysis {}",
        if with_analysis {
            "running (current + next track)"
        } else {
            "off"
        }
    );
    std::thread::sleep(Duration::from_millis(300)); // device fully running, as in a warm app
    let underruns_before = engine.stats().underruns;

    let mut worst_start = 0f64;
    let mut worst_seek = 0f64;
    println!(
        "\n{:<32} {:>9} {:>9} {:>9} {:>9}",
        "file", "start ms", "seek25", "seek50", "seek75"
    );
    for (i, f) in files.iter().enumerate() {
        engine.play_index(i);
        let Some(duration) = wait_started(&mut engine, i, &load)? else {
            println!("{:<32} failed to play", f.file_name());
            continue;
        };
        let start = engine.stats().last_start_latency_ms;
        worst_start = worst_start.max(start);
        let mut seeks = Vec::new();
        for frac in [0.25, 0.5, 0.75] {
            engine.seek(duration * frac);
            load.sleep(&mut engine, Duration::from_millis(250));
            let ms = engine.stats().last_start_latency_ms;
            worst_seek = worst_seek.max(ms);
            seeks.push(ms);
        }
        println!(
            "{:<32} {:>9.2} {:>9.2} {:>9.2} {:>9.2}",
            truncate(f.file_name(), 32),
            start,
            seeks[0],
            seeks[1],
            seeks[2]
        );
    }

    println!("\ngapless run through the whole queue…");
    let ended_before = engine.stats().queue_ended;
    engine.play_index(0);
    let began = Instant::now();
    while engine.stats().queue_ended == ended_before {
        load.sleep(&mut engine, Duration::from_millis(50));
        engine.poll_events();
        if began.elapsed() > Duration::from_secs(3600) {
            break;
        }
    }
    let s = engine.stats();
    let underruns = s.underruns - underruns_before;
    println!(
        "played {:.1} s, state {:?}, underruns this session {underruns}",
        began.elapsed().as_secs_f64(),
        engine.state()
    );

    println!("\nworst start {worst_start:.2} ms (target < {START_TARGET_MS})");
    println!("worst seek  {worst_seek:.2} ms (target < {SEEK_TARGET_MS})");
    println!("underruns   {underruns} (target 0)");
    if cfg!(debug_assertions) {
        println!("rt allocs   {} (target 0)", s.rt_alloc_violations);
    } else {
        println!("rt allocs   not measured in release builds");
    }
    if let Some(svc) = &load.svc {
        let analyzed: f64 = files
            .iter()
            .filter_map(|f| {
                svc.score(f).and_then(|s| {
                    s.coverage
                        .spans()
                        .iter()
                        .map(|(a, b)| b - a)
                        .reduce(|x, y| x + y)
                })
            })
            .sum();
        println!("analyzed    {analyzed:.0} s of audio during the run");
    }
    let ok = worst_start < START_TARGET_MS
        && worst_seek < SEEK_TARGET_MS
        && underruns == 0
        && s.rt_alloc_violations == 0;
    if ok {
        Ok(())
    } else {
        Err("targets not met".into())
    }
}

/// Waits until track `index` is audible; returns its duration in seconds.
fn wait_started(engine: &mut Engine, index: usize, load: &Load) -> Result<Option<f64>, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut duration = None;
    loop {
        for e in engine.poll_events() {
            match e {
                EngineEvent::TrackInfo { index: i, info, .. } if i == index => {
                    duration = info.duration_secs;
                }
                EngineEvent::TrackFailed { index: i, .. } if i == index => return Ok(None),
                _ => {}
            }
        }
        let p = engine.position();
        if duration.is_some()
            && p.state == PlayState::Playing
            && p.frame > 0
            && engine.current_index() == Some(index)
        {
            return Ok(duration);
        }
        if Instant::now() > deadline {
            return Err(format!("track {index} did not start within 5 s"));
        }
        load.tick(engine);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_owned()
    } else {
        s.chars().take(n - 1).chain(['…']).collect()
    }
}
