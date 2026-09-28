//! Headless GPU benchmark of the bundled scenes: renders each scene offscreen (scene → feedback
//! → bloom → final) at a given resolution and render scale, waiting for the GPU every frame.
//!
//! cargo run -p visuals --example visual_bench --release -- [width height] [frames]

use std::rc::Rc;
use std::time::Instant;

use eframe::egui_wgpu::wgpu;
use visuals::automaton::AutoFrame;
use visuals::codegen::{MUSIC_FLOATS, music_index, pack_params};
use visuals::gpu::{Gpu, Layer, LayerDraw, PostParams, SceneProgram, Targets};
use visuals::library;

fn main() {
    let args: Vec<u32> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let (w, h) = (
        args.first().copied().unwrap_or(3024),
        args.get(1).copied().unwrap_or(1964),
    );
    let frames = args.get(2).copied().unwrap_or(120);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("a GPU adapter");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("a device");
    println!(
        "{} ({:?}), {w}×{h}, {frames} frames per scene\n",
        adapter.get_info().name,
        adapter.get_info().backend
    );
    let mut gpu = Gpu::new(device, queue, wgpu::TextureFormat::Bgra8Unorm);

    let dir = std::env::temp_dir().join(format!("visual-bench-{}", std::process::id()));
    library::install(&dir).expect("install assets");
    let prelude = library::load_prelude(&dir);
    println!(
        "{:<18} {:>6} {:>9} {:>9}   {:>9} {:>9}",
        "scene", "scale", "mean ms", "p95 ms", "@0.75 ms", "@0.5 ms"
    );
    for id in library::scene_ids(&dir) {
        let src = library::load_scene(&dir, &id, &prelude).expect("scene loads");
        let prog = Rc::new(
            SceneProgram::new(&gpu, &id, &src.assembled, &src.manifest).expect("scene compiles"),
        );
        let params = pack_params(&src.manifest.defaults(), src.manifest.params.len().max(1));
        let cap = src.manifest.max_scale.unwrap_or(1.0);
        let mut row = format!("{id:<18} {cap:>6.2}");
        for (k, scale) in [cap, 0.75f32.min(cap), 0.5].into_iter().enumerate() {
            let size = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
            let mut targets = Targets::new(&gpu, size);
            let mut layer = Layer::new(&gpu, prog.clone());
            let mut times = Vec::new();
            for f in 0..frames + 10 {
                let mut m = vec![0.0; MUSIC_FLOATS];
                let beats = f as f32 / 60.0 * 2.2;
                for (key, v) in [
                    ("beats", beats),
                    ("motion", beats),
                    ("beat", beats.fract()),
                    ("bar", (beats / 4.0).fract()),
                    ("tempo", 132.0),
                    ("energy", 0.7),
                    ("bass", 0.6),
                    ("mid", 0.5),
                    ("treble", 0.4),
                    ("kick", beats.fract()),
                    ("aspect", size.0 as f32 / size.1 as f32),
                    ("res_x", size.0 as f32),
                    ("res_y", size.1 as f32),
                    ("playing", 1.0),
                    ("frame", f as f32),
                ] {
                    m[music_index(key)] = v;
                }
                // Automata: a step every frame (4 steps per beat at 132 BPM is ~9 per second,
                // so this is the heavier case).
                layer.advance(AutoFrame {
                    first: f as i64,
                    steps: 1,
                    ..Default::default()
                });
                let t0 = Instant::now();
                gpu.render(
                    &mut targets,
                    LayerDraw {
                        layer: &mut layer,
                        music: &m,
                        params: &params,
                    },
                    None,
                    &PostParams::default(),
                );
                gpu.device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("gpu finishes");
                if f >= 10 {
                    times.push(t0.elapsed().as_secs_f64() * 1000.0);
                }
            }
            times.sort_by(f64::total_cmp);
            let mean = times.iter().sum::<f64>() / times.len() as f64;
            let p95 = times[times.len() * 95 / 100];
            row += &match k {
                0 => format!(" {mean:>9.2} {p95:>9.2}  "),
                _ => format!(" {mean:>9.2}"),
            };
        }
        println!("{row}");
    }
    let _ = std::fs::remove_dir_all(dir);
    println!(
        "\n60 fps needs < 16.7 ms per frame (120 Hz: < 8.3 ms). The final upscale pass is not included."
    );
}
