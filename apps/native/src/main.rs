//! Native harness for the audio core (a GUI arrives with `classic-ui`).
//!
//! ```text
//! winamp-native FILE...                      interactive player
//! winamp-native --bench [--volume V] FILE... measure start/seek latency and underruns
//! winamp-native --click-test                 clock vs. microphone (needs speaker → mic path)
//! ```

mod bench;
mod click;
mod interactive;

use std::process::ExitCode;
use std::sync::Arc;

use audio::{Engine, EngineConfig};
use platform::TrackRef;
use platform::native::{CpalSink, NativeFileSource, NativeSpawner};

// Debug builds count any allocation made inside the audio callback (shown in the status/bench).
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: audio::rt_guard::GuardAlloc = audio::rt_guard::GuardAlloc;

pub fn open_engine() -> Result<Engine, String> {
    Engine::new(
        Arc::new(CpalSink),
        &NativeSpawner,
        Arc::new(NativeFileSource),
        EngineConfig::default(),
    )
    .map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).peekable();
    let mut mode = "play";
    let mut volume = None;
    let mut files = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bench" => mode = "bench",
            "--click-test" => mode = "click",
            "--volume" => volume = args.next().and_then(|v| v.parse::<f32>().ok()),
            "-h" | "--help" => {
                println!("usage: winamp-native [--bench [--volume 0..1] | --click-test] FILE...");
                return ExitCode::SUCCESS;
            }
            _ => files.push(TrackRef::new(a)),
        }
    }
    let result = match mode {
        "bench" => bench::run(files, volume.unwrap_or(0.0)),
        "click" => click::run(),
        _ if files.is_empty() => Err("no files given (try --help)".into()),
        _ => interactive::run(files, volume.unwrap_or(0.8)),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
