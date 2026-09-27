//! Minimal eframe app: fullscreen, black, repaint every frame; prints painted fps per second.
//! A baseline for what the platform can present, independent of the player.
//!
//! cargo run -p ui --example fullscreen_probe --release -- [seconds]

use std::time::{Duration, Instant};

struct Probe {
    start: Instant,
    window: Instant,
    frames: u32,
    max_ms: f64,
    last: Instant,
    secs: u64,
}

impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, egui::Color32::BLACK);
        let now = Instant::now();
        self.max_ms = self.max_ms.max((now - self.last).as_secs_f64() * 1e3);
        self.last = now;
        self.frames += 1;
        if self.window.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "probe fps {:3} | max frame {:5.1} ms",
                self.frames, self.max_ms
            );
            self.window = now;
            self.frames = 0;
            self.max_ms = 0.0;
        }
        if self.start.elapsed() >= Duration::from_secs(self.secs) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let secs = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);
    // PROBE_PRESENT=auto|fifo|mailbox|immediate, PROBE_LATENCY=n, PROBE_WINDOWED=1.
    use eframe::egui_wgpu::wgpu::PresentMode;
    let present_mode = match std::env::var("PROBE_PRESENT").as_deref() {
        Ok("fifo") => PresentMode::Fifo,
        Ok("mailbox") => PresentMode::Mailbox,
        Ok("immediate") => PresentMode::Immediate,
        Ok("novsync") => PresentMode::AutoNoVsync,
        _ => PresentMode::AutoVsync,
    };
    let latency = std::env::var("PROBE_LATENCY")
        .ok()
        .and_then(|v| v.parse().ok());
    let windowed = std::env::var_os("PROBE_WINDOWED").is_some();
    let wgpu_options = eframe::egui_wgpu::WgpuConfiguration {
        surface: eframe::egui_wgpu::SurfaceConfig {
            present_mode,
            desired_maximum_frame_latency: latency,
        },
        ..Default::default()
    };
    eprintln!(
        "probe: {present_mode:?}, latency {latency:?}, {}",
        if windowed {
            "maximized window"
        } else {
            "fullscreen"
        }
    );
    let viewport = if windowed {
        egui::ViewportBuilder::default().with_maximized(true)
    } else {
        egui::ViewportBuilder::default().with_fullscreen(true)
    };
    let options = eframe::NativeOptions {
        viewport,
        wgpu_options,
        ..Default::default()
    };
    eframe::run_native(
        "fullscreen-probe",
        options,
        Box::new(move |_| {
            let now = Instant::now();
            Ok(Box::new(Probe {
                start: now,
                window: now,
                frames: 0,
                max_ms: 0.0,
                last: now,
                secs,
            }))
        }),
    )
}
