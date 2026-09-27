//! Winamp-style player for the desktop.
//!
//! ```text
//! winamp-native [FILE...]                    the player window (files replace the playlist)
//! winamp-native --tui FILE...                terminal player
//! winamp-native --bench [--volume V] [--analysis] FILE...
//!                                            measure start/seek latency and underruns
//!                                            (optionally with music analysis running)
//! winamp-native --click-test                 clock vs. microphone (needs speaker → mic path)
//! winamp-native --startup-time               print time to first frame and quit
//! winamp-native --render-show TRACK -o OUT.mp4 [--size WxH] [--fps N] [--from T] [--to T]
//!               [--overlay] [--look SCENE/VARIANT]
//!                                            render the track's visual show to a video
//! ```

mod bench;
mod click;
mod interactive;
mod render;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use audio::{Engine, EngineConfig};
use platform::TrackRef;
use platform::native::{CpalSink, NativeFileSource, NativeSpawner};
use ui::settings::Store;
use ui::{AppContext, Startup, WinampApp};

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

const USAGE: &str = "usage: winamp-native [FILE...] | --tui FILE... | --bench [--volume 0..1] [--analysis] FILE... | --click-test | --startup-time | --render-show TRACK -o OUT.mp4 [--size WxH] [--fps N] [--from T] [--to T] [--overlay] [--look SCENE/VARIANT]";

fn main() -> ExitCode {
    let process_start = Instant::now();
    let all: Vec<String> = std::env::args().skip(1).collect();
    if all.first().is_some_and(|a| a == "--render-show") {
        return match render::run(&all[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let mut args = all.into_iter().peekable();
    let mut mode = "gui";
    let mut volume = None;
    let mut files = Vec::new();
    let mut startup_time = false;
    let mut with_analysis = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--tui" => mode = "tui",
            "--bench" => mode = "bench",
            "--click-test" => mode = "click",
            "--startup-time" => startup_time = true,
            "--analysis" => with_analysis = true,
            "--volume" => volume = args.next().and_then(|v| v.parse::<f32>().ok()),
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ => files.push(a),
        }
    }
    let tracks = || files.iter().map(TrackRef::new).collect::<Vec<_>>();
    let result = match mode {
        "bench" => bench::run(tracks(), volume.unwrap_or(0.0), with_analysis),
        "click" => click::run(),
        "tui" if files.is_empty() => Err(format!("no files given\n{USAGE}")),
        "tui" => interactive::run(tracks(), volume.unwrap_or(0.8)),
        _ => gui(
            files.into_iter().map(PathBuf::from).collect(),
            process_start,
            startup_time,
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn gui(open: Vec<PathBuf>, process_start: Instant, startup_time: bool) -> Result<(), String> {
    // WINAMP_CONFIG_DIR overrides where settings and the playlist are kept (handy for testing).
    let store = std::env::var_os("WINAMP_CONFIG_DIR")
        .map(Store::new)
        .or_else(Store::platform_default);
    let settings = store.as_ref().map(Store::load_settings).unwrap_or_default();
    let size = WinampApp::window_size(&settings, &ui::skin::LoadedSkin::default_skin());
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Winamp")
        .with_inner_size(size)
        .with_decorations(false)
        .with_resizable(false);
    if let Some((x, y)) = settings.window_pos {
        viewport = viewport.with_position([x, y]);
    }
    // Scores and annotations: ~/Library/Caches/winamp_rust (WINAMP_CACHE_DIR overrides).
    let cache = analysis::cache::ScoreCache::platform_default();
    let options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            surface: ui::app::surface_config(),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "winamp_rust",
        options,
        Box::new(move |cc| {
            let ctx = AppContext {
                engine: Box::new(open_engine),
                spawner: Arc::new(NativeSpawner),
                files: Arc::new(NativeFileSource),
                store,
                open,
                scene: Box::new(visuals::VisualEngine::new()),
                analysis: Some(analysis::AnalysisService::new(
                    Arc::new(NativeSpawner),
                    Arc::new(NativeFileSource),
                    cache.clone(),
                )),
                annotations_dir: cache.as_ref().map(|c| c.dir().join("annotations")),
                overviews: Some(analysis::overview::OverviewService::new(
                    Arc::new(NativeSpawner),
                    Arc::new(NativeFileSource),
                    cache.as_ref().map(|c| c.dir().to_path_buf()),
                )),
                show_renderer: Some(Box::new(visuals::render::BackgroundRenderer::new(
                    Arc::new(NativeFileSource),
                    Arc::new(NativeSpawner),
                ))),
                startup: Startup {
                    process_start,
                    report: startup_time,
                    exit_after_first_frame: startup_time,
                },
            };
            Ok(Box::new(WinampApp::new(cc, ctx)))
        }),
    )
    .map_err(|e| e.to_string())
}
