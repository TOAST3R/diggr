//! How fast offline show rendering is without encoding (frames discarded), to separate GPU
//! and readback cost from ffmpeg's.
//!
//! cargo run -p visuals --example render_speed --release -- TRACK [WxH] [SECONDS]

use std::time::Instant;

use platform::TrackRef;
use platform::native::NativeFileSource;
use visuals::render::{Options, headless_gpu, render_frames};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let track = args
        .first()
        .expect("usage: render_speed TRACK [WxH] [SECONDS]");
    let size = args
        .get(1)
        .and_then(|s| s.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or((1920, 1080));
    let secs: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20.0);
    let opts = Options {
        size,
        from: 30.0,
        to: Some(30.0 + secs),
        ..Options::new(TrackRef::new(track), "unused.mp4".into())
    };
    let gpu = headless_gpu().expect("a GPU");
    let started = Instant::now();
    let mut bytes = 0usize;
    let frames = render_frames(
        &NativeFileSource,
        &opts,
        gpu,
        |f| {
            bytes += f.len();
            Ok(())
        },
        |_| true,
    )
    .expect("render");
    let t = started.elapsed().as_secs_f64();
    println!(
        "{frames} frames at {}x{} in {t:.2} s: {:.0} fps ({:.1}× real time at 60 fps), {:.0} MB read back",
        size.0,
        size.1,
        frames as f64 / t,
        frames as f64 / t / 60.0,
        bytes as f64 / 1e6
    );
}
