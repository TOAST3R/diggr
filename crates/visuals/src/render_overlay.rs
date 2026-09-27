//! The artist/title card in offline renders: the live overlay drawn with egui's own renderer
//! into the finished frame, laid out as on a Mac's fullscreen display.

use eframe::egui_wgpu::{self, wgpu};
use ui::fullscreen::SceneFrame;

use crate::gpu::Gpu;
use crate::overlay::{self, Fade};

/// Points of screen height the layout assumes (a 14" MacBook Pro in fullscreen).
const LAYOUT_HEIGHT: f32 = 982.0;

pub struct OverlayPainter {
    ctx: egui::Context,
    renderer: egui_wgpu::Renderer,
    size: (u32, u32),
    fade: Fade,
}

impl OverlayPainter {
    pub fn new(gpu: &Gpu, size: (u32, u32)) -> Self {
        let mut fade = Fade::default();
        fade.poke(0.0);
        Self {
            ctx: egui::Context::default(),
            renderer: egui_wgpu::Renderer::new(&gpu.device, gpu.out_format, Default::default()),
            size,
            fade,
        }
    }

    /// Draws the card for time `now` (seconds since the render started) over `view`.
    pub fn paint(
        &mut self,
        gpu: &Gpu,
        enc: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        frame: &SceneFrame,
        now: f64,
    ) {
        let alpha = self.fade.alpha(now);
        if alpha <= 0.0 {
            return;
        }
        let ppp = self.size.1 as f32 / LAYOUT_HEIGHT;
        let screen = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(self.size.0 as f32, self.size.1 as f32) / ppp,
        );
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        self.ctx.set_pixels_per_point(ppp);
        let mut out = self.ctx.run_ui(input, |ui| {
            let info = overlay::Info {
                artist: frame.artist,
                title: frame.title,
                elapsed: frame.position.seconds(),
                duration: frame.duration,
                score: frame.score,
                bottom_inset: 0.0,
            };
            overlay::draw(ui.painter(), screen, alpha, &info, crate::engine::ACCENT);
        });
        let jobs = self.ctx.tessellate(out.shapes, ppp);
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size.0, self.size.1],
            pixels_per_point: ppp,
        };
        let extra = self
            .renderer
            .update_buffers(&gpu.device, &gpu.queue, enc, &jobs, &desc);
        if !extra.is_empty() {
            gpu.queue.submit(extra);
        }
        {
            let pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.renderer
                .render(&mut pass.forget_lifetime(), &jobs, &desc);
        }
        for id in &out.textures_delta.free {
            self.renderer.free_texture(id);
        }
        out.textures_delta.clear(); // applied above
    }
}
