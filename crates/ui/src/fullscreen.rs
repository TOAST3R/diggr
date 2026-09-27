//! Fullscreen visual mode: the host API the visual engine plugs into, and a placeholder scene
//! (a beat flash driven by the playback clock) that exercises the whole path until the real
//! visual engine lands.

use audio::{PlayState, Position};
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};
use egui::{Color32, Rect, Ui};

use crate::spectrum::BARS;

/// Everything a scene gets each frame.
pub struct SceneFrame<'a> {
    /// The audible position from the playback clock.
    pub position: Position,
    pub bars: &'a [f32; BARS],
    pub artist: &'a str,
    pub title: &'a str,
    /// What the analyzer knows about the audible track (beats, sections, tension, upcoming
    /// drops), when it has analyzed it.
    pub score: Option<&'a analysis::SongScore>,
    /// Length of the audible track, when known.
    pub duration: Option<f64>,
    /// Whether the host's analysis strip is drawn along the bottom (overlays go above it).
    pub strip_visible: bool,
}

/// A fullscreen visual. The host calls, once per displayed frame: `paint` (GPU layer under the
/// egui layer), then `ui` (overlay, fader deck). Keys the host does not use go to `key`.
pub trait VisualScene {
    /// Called once, when fullscreen is first entered, with the app's GPU device and queue.
    fn init(&mut self, render_state: &egui_wgpu::RenderState);
    /// A paint callback covering `rect` (use `egui_wgpu::Callback::new_paint_callback`).
    fn paint(&mut self, rect: Rect, frame: &SceneFrame) -> Option<egui::PaintCallback>;
    fn ui(&mut self, ui: &mut Ui, frame: &SceneFrame);
    /// Returns true if the key was used.
    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> bool;
    /// Fullscreen was just entered (e.g. to show track info again).
    fn entered(&mut self) {}
}

/// Brightness of the placeholder flash: a fixed 120 BPM grid on the *audible* time, decaying
/// quickly after each beat, so audio/visual offset is easy to judge by eye.
/// Brightness of the placeholder flash, decaying quickly after each beat. Uses the analyzed
/// beats when the score covers the audible time (a visible check that the grid is right), and a
/// fixed 120 BPM grid otherwise.
pub fn beat_flash(position: &Position, score: Option<&analysis::SongScore>) -> f32 {
    if position.state != PlayState::Playing {
        return 0.0;
    }
    let secs = position.seconds();
    let beats = score.and_then(|s| s.beat_at(secs)).unwrap_or(secs * 2.0);
    (-(beats.fract() as f32) * 6.0).exp()
}

const SHADER: &str = r#"
struct U { color: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return u.color;
}
"#;

struct FlashResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}

struct FlashCallback {
    color: [f32; 4],
}

impl CallbackTrait for FlashCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(r) = resources.get::<FlashResources>() {
            let bytes: Vec<u8> = self.color.iter().flat_map(|c| c.to_le_bytes()).collect();
            queue.write_buffer(&r.uniform, 0, &bytes);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        if let Some(r) = resources.get::<FlashResources>() {
            pass.set_pipeline(&r.pipeline);
            pass.set_bind_group(0, &r.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

/// Placeholder scene: full-screen color flash on every beat of a 120 BPM grid, tinted by bass.
#[derive(Default)]
pub struct BeatFlash {
    ready: bool,
}

impl VisualScene for BeatFlash {
    fn init(&mut self, rs: &egui_wgpu::RenderState) {
        let device = &rs.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("beat-flash"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("beat-flash-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("beat-flash"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("beat-flash"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("beat-flash"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("beat-flash"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: rs.target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        rs.renderer
            .write()
            .callback_resources
            .insert(FlashResources {
                pipeline,
                bind_group,
                uniform,
            });
        self.ready = true;
    }

    fn paint(&mut self, rect: Rect, frame: &SceneFrame) -> Option<egui::PaintCallback> {
        if !self.ready {
            return None;
        }
        let flash = beat_flash(&frame.position, frame.score);
        let bass = frame.bars[..4].iter().copied().fold(0.0, f32::max);
        let base = [0.02, 0.02, 0.06];
        let hot = [0.2 + 0.6 * bass, 0.9, 0.3];
        let c = |i: usize| base[i] + (hot[i] - base[i]) * flash;
        Some(egui_wgpu::Callback::new_paint_callback(
            rect,
            FlashCallback {
                color: [c(0), c(1), c(2), 1.0],
            },
        ))
    }

    fn ui(&mut self, ui: &mut Ui, frame: &SceneFrame) {
        let rect = ui.max_rect();
        let p = ui.painter();
        let big = egui::FontId::proportional(34.0);
        let small = egui::FontId::proportional(16.0);
        let white = Color32::from_rgba_unmultiplied(255, 255, 255, 230);
        let x = rect.left() + 48.0;
        // Above the host's analysis strip when it is shown.
        let strip = if frame.strip_visible {
            crate::timeline::HEIGHT
        } else {
            0.0
        };
        let y = rect.bottom() - 120.0 - strip - 40.0;
        if !frame.artist.is_empty() {
            p.text(
                egui::pos2(x, y),
                egui::Align2::LEFT_TOP,
                frame.artist,
                small.clone(),
                white,
            );
        }
        p.text(
            egui::pos2(x, y + 22.0),
            egui::Align2::LEFT_TOP,
            frame.title,
            big,
            white,
        );
        let secs = frame.position.seconds();
        p.text(
            egui::pos2(x, y + 66.0),
            egui::Align2::LEFT_TOP,
            format!("{}  ·  placeholder visual: flashes on analyzed beats (120 BPM grid until analyzed) · T analysis strip · A annotate  ·  F / Esc to exit", crate::format::clock(secs)),
            small,
            Color32::from_gray(170),
        );
    }

    fn key(&mut self, _key: egui::Key, _modifiers: egui::Modifiers) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: f64, state: PlayState) -> Position {
        Position {
            track: 1,
            frame: (secs * 48_000.0) as u64,
            sample_rate: 48_000,
            state,
            discontinuity: false,
        }
    }

    #[test]
    fn flash_follows_analyzed_beats_when_available() {
        // A 128 BPM grid starting at 0.1 s: beats at 0.1, 0.56875, …
        let s = analysis::SongScore {
            beats: (0..64).map(|i| 0.1 + i as f64 * 60.0 / 128.0).collect(),
            ..Default::default()
        };
        let on_beat = 0.1 + 10.0 * 60.0 / 128.0 + 0.001; // just after the beat
        assert!(beat_flash(&at(on_beat, PlayState::Playing), Some(&s)) > 0.95);
        assert!(beat_flash(&at(on_beat + 0.2, PlayState::Playing), Some(&s)) < 0.1);
        // Outside the analyzed beats it falls back to the fixed grid.
        assert!(beat_flash(&at(100.0, PlayState::Playing), Some(&s)) > 0.99);
    }

    #[test]
    fn flash_peaks_on_each_beat_and_decays() {
        assert!(
            (beat_flash(&at(1.0, PlayState::Playing), None) - 1.0).abs() < 1e-3,
            "on the beat"
        );
        assert!(
            beat_flash(&at(1.25, PlayState::Playing), None) < 0.1,
            "half a beat later"
        );
        assert!(
            beat_flash(&at(1.5, PlayState::Playing), None) > 0.99,
            "next beat"
        );
        assert_eq!(beat_flash(&at(1.0, PlayState::Paused), None), 0.0);
    }
}
