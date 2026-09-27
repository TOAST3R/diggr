//! GPU side of the compositor: scene programs, per-layer uniforms, the offscreen chain
//! (scene A/B → mix + feedback ping-pong → bloom) and the final pass to the screen.
//!
//! All offscreen work is encoded and submitted by the engine during the host's `paint` call;
//! egui's paint callback then only draws the final full-screen pass into its own render pass.

use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};

use crate::codegen::{Assembled, ShaderError, param_slots};
use crate::manifest::{SceneKind, SceneManifest};

/// Offscreen format: HDR so feedback and bloom don't band or clip.
pub const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const WORKGROUP: u32 = 64;

fn bytes(f: &[f32]) -> Vec<u8> {
    f.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// Runs `make` inside a validation error scope and returns the first error, if any. On native
/// the scope resolves immediately; if it doesn't (web), the object is assumed valid (naga has
/// already validated the source).
fn scoped<T>(device: &wgpu::Device, make: impl FnOnce() -> T) -> Result<T, String> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let out = make();
    let mut fut = std::pin::pin!(scope.pop());
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(Some(e)) => Err(e.to_string()),
        _ => Ok(out),
    }
}

pub struct Rt {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: (u32, u32),
}

impl Rt {
    fn new(
        device: &wgpu::Device,
        label: &str,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self {
            texture,
            view,
            size,
        }
    }
}

/// Parameters for the fixed passes (mix/feedback, bloom, final), one uniform per frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostParams {
    /// 0 = only layer A, 1 = only layer B.
    pub mix_b: f32,
    /// Per-frame feedback decay (already scaled for frame time) and warp per layer.
    pub decay_a: f32,
    pub warp_a: f32,
    pub decay_b: f32,
    pub warp_b: f32,
    pub aspect: f32,
    pub motion: f32,
    pub bloom: f32,
    pub chroma: f32,
    pub vignette: f32,
    pub grain: f32,
    pub flash: f32,
    pub frame: f32,
    /// Global hue rotation in turns (the hue macro).
    pub hue: f32,
}

impl Default for PostParams {
    fn default() -> Self {
        Self {
            mix_b: 0.0,
            decay_a: 0.0,
            warp_a: 0.0,
            decay_b: 0.0,
            warp_b: 0.0,
            aspect: 16.0 / 9.0,
            motion: 0.0,
            bloom: 0.35,
            chroma: 0.0,
            vignette: 0.35,
            grain: 0.02,
            flash: 0.0,
            frame: 0.0,
            hue: 0.0,
        }
    }
}

impl PostParams {
    fn floats(&self, has_b: bool, srgb_out: bool) -> [f32; 16] {
        [
            self.mix_b,
            self.decay_a,
            self.warp_a,
            self.decay_b,
            self.warp_b,
            has_b as u8 as f32,
            self.aspect,
            self.motion,
            self.bloom,
            self.chroma,
            self.vignette,
            self.grain,
            self.flash,
            self.frame,
            srgb_out as u8 as f32,
            self.hue,
        ]
    }
}

const POST: &str = r#"
struct Post {
    mix_b: f32, decay_a: f32, warp_a: f32, decay_b: f32,
    warp_b: f32, has_b: f32, aspect: f32, motion: f32,
    bloom: f32, chroma: f32, vignette: f32, grain: f32,
    flash: f32, frame: f32, srgb_out: f32, hue: f32,
}
@group(0) @binding(0) var<uniform> u: Post;
@group(0) @binding(1) var t0: texture_2d<f32>;
@group(0) @binding(2) var t1: texture_2d<f32>;
@group(0) @binding(3) var t2: texture_2d<f32>;
@group(0) @binding(4) var s: sampler;

struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
    var o: VsOut;
    o.pos = vec4f(p, 0.0, 1.0);
    o.uv = vec2f(p.x, -p.y) * 0.5 + 0.5;
    return o;
}

// The previous frame zoomed in slightly and turned, so trails flow outward.
fn warped_prev(uv: vec2f, strength: f32) -> vec3f {
    var p = (uv - 0.5) * vec2f(u.aspect, 1.0);
    let a = 0.006 * strength * sin(u.motion * 0.125);
    let c = cos(a);
    let sn = sin(a);
    p = mat2x2f(c, sn, -sn, c) * p * (1.0 - 0.015 * strength);
    return textureSampleLevel(t2, s, p / vec2f(u.aspect, 1.0) + 0.5, 0.0).rgb;
}

@fragment
fn fs_composite(v: VsOut) -> @location(0) vec4f {
    let a = textureSampleLevel(t0, s, v.uv, 0.0);
    var col = mix(warped_prev(v.uv, u.warp_a) * u.decay_a, a.rgb, a.a);
    if (u.has_b > 0.5) {
        let b = textureSampleLevel(t1, s, v.uv, 0.0);
        let fb = mix(warped_prev(v.uv, u.warp_b) * u.decay_b, b.rgb, b.a);
        col = mix(col, fb, u.mix_b);
    }
    return vec4f(max(col, vec3f(0.0)), 1.0);
}

@fragment
fn fs_bright(v: VsOut) -> @location(0) vec4f {
    let c = textureSampleLevel(t0, s, v.uv, 0.0).rgb;
    return vec4f(max(c - vec3f(0.8), vec3f(0.0)), 1.0);
}

fn blur(uv: vec2f, dir: vec2f) -> vec4f {
    let px = dir / vec2f(textureDimensions(t0));
    var c = textureSampleLevel(t0, s, uv, 0.0).rgb * 0.227;
    c += (textureSampleLevel(t0, s, uv + px * 1.385, 0.0).rgb + textureSampleLevel(t0, s, uv - px * 1.385, 0.0).rgb) * 0.316;
    c += (textureSampleLevel(t0, s, uv + px * 3.231, 0.0).rgb + textureSampleLevel(t0, s, uv - px * 3.231, 0.0).rgb) * 0.070;
    return vec4f(c, 1.0);
}

@fragment
fn fs_blur_h(v: VsOut) -> @location(0) vec4f {
    return blur(v.uv, vec2f(1.0, 0.0));
}

@fragment
fn fs_blur_v(v: VsOut) -> @location(0) vec4f {
    return blur(v.uv, vec2f(0.0, 1.0));
}

fn knee(x: f32) -> f32 {
    if (x < 0.8) {
        return x;
    }
    return 0.8 + 0.2 * (1.0 - exp(-(x - 0.8) / 0.2));
}

@fragment
fn fs_final(v: VsOut) -> @location(0) vec4f {
    let off = (v.uv - 0.5) * u.chroma * 0.015;
    var col = vec3f(
        textureSampleLevel(t0, s, v.uv + off, 0.0).r,
        textureSampleLevel(t0, s, v.uv, 0.0).g,
        textureSampleLevel(t0, s, v.uv - off, 0.0).b,
    );
    col += textureSampleLevel(t1, s, v.uv, 0.0).rgb * u.bloom;
    if (u.hue != 0.0) {
        // Rotate around the grey axis.
        let k = vec3f(0.57735);
        let a = u.hue * 6.2831853;
        col = col * cos(a) + cross(k, col) * sin(a) + k * dot(k, col) * (1.0 - cos(a));
    }
    col += vec3f(u.flash);
    let d = length((v.uv - 0.5) * vec2f(u.aspect, 1.0));
    col *= 1.0 - u.vignette * smoothstep(0.4, 1.2, d);
    let g = fract(sin(dot(v.pos.xy + u.frame * vec2f(17.0, 59.0), vec2f(12.9898, 78.233))) * 43758.5453);
    col += (g - 0.5) * u.grain;
    col = vec3f(knee(col.r), knee(col.g), knee(col.b));
    col = clamp(col, vec3f(0.0), vec3f(1.0));
    if (u.srgb_out > 0.5) {
        col = pow(col, vec3f(2.2));
    }
    return vec4f(col, 1.0);
}
"#;

/// A compiled scene: its pipelines and bind group layout.
pub struct SceneProgram {
    render: wgpu::RenderPipeline,
    compute: Option<(wgpu::ComputePipeline, u32)>,
    layout: wgpu::BindGroupLayout,
    pub param_slots: usize,
}

impl SceneProgram {
    /// Compiles an assembled (naga-validated) scene. Any wgpu validation error comes back as a
    /// `ShaderError` instead of a panic, so the previous program can keep running.
    pub fn new(
        gpu: &Gpu,
        label: &str,
        a: &Assembled,
        m: &SceneManifest,
    ) -> Result<Self, ShaderError> {
        let device = &gpu.device;
        let compute = matches!(m.kind, SceneKind::Compute { .. });
        let err = |message: String| ShaderError {
            file: label.into(),
            line: None,
            message,
        };
        scoped(device, || {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(a.source.as_str().into()),
            });
            let all = wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE;
            let uniform = |binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: all,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            };
            let mut entries = vec![
                uniform(0),
                uniform(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: all,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: all,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ];
            if compute {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                });
            }
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &entries,
            });
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let render = fullscreen_pipeline(device, label, &module, &pl, "fs_main", HDR);
            let compute = match m.kind {
                SceneKind::Compute { invocations } => Some((
                    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some(label),
                        layout: Some(&pl),
                        module: &module,
                        entry_point: Some("cs_main"),
                        compilation_options: Default::default(),
                        cache: None,
                    }),
                    invocations.max(1),
                )),
                SceneKind::Fragment => None,
            };
            Self {
                render,
                compute,
                layout,
                param_slots: param_slots(m),
            }
        })
        .map_err(err)
    }

    pub fn is_compute(&self) -> bool {
        self.compute.is_some()
    }
}

fn fullscreen_pipeline(
    device: &wgpu::Device,
    label: &str,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    fs: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fs),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// One scene instance being drawn (A or B): its own uniforms and, for compute scenes, its
/// accumulation buffer.
pub struct Layer {
    pub program: Rc<SceneProgram>,
    music: wgpu::Buffer,
    params: wgpu::Buffer,
    accum: Option<(wgpu::Buffer, u64)>,
}

impl Layer {
    pub fn new(gpu: &Gpu, program: Rc<SceneProgram>) -> Self {
        let buf = |label, size| {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Self {
            music: buf("music", (crate::codegen::MUSIC_FLOATS * 4) as u64),
            params: buf("params", (program.param_slots * 16) as u64),
            program,
            accum: None,
        }
    }

    fn encode(
        &mut self,
        gpu: &Gpu,
        enc: &mut wgpu::CommandEncoder,
        target: &Rt,
        prev: &Rt,
        music: &[f32],
        params: &[f32],
    ) {
        gpu.queue.write_buffer(&self.music, 0, &bytes(music));
        let n = self.program.param_slots * 4;
        let mut p = params.to_vec();
        p.resize(n, 0.0);
        gpu.queue.write_buffer(&self.params, 0, &bytes(&p));
        if self.program.is_compute() {
            let (w, h) = (target.size.0.div_ceil(2), target.size.1.div_ceil(2));
            let size = w as u64 * h as u64 * 16;
            if self.accum.as_ref().is_none_or(|(_, s)| *s != size) {
                let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("accum"),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.accum = Some((buffer, size));
            }
        }
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.music.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&prev.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&gpu.sampler),
            },
        ];
        if let Some((buf, _)) = &self.accum {
            entries.push(wgpu::BindGroupEntry {
                binding: 4,
                resource: buf.as_entire_binding(),
            });
        }
        let bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &self.program.layout,
            entries: &entries,
        });
        if let (Some((pipe, invocations)), Some((buf, _))) = (&self.program.compute, &self.accum) {
            enc.clear_buffer(buf, 0, None);
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("simulate"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, &bg, &[]);
            let groups = invocations.div_ceil(WORKGROUP);
            let max = gpu.device.limits().max_compute_workgroups_per_dimension;
            pass.dispatch_workgroups(groups.min(max), groups.div_ceil(max).max(1), 1);
        }
        let mut pass = begin(enc, "scene", &target.view);
        pass.set_pipeline(&self.program.render);
        pass.set_bind_group(0, &bg, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn begin<'e>(
    enc: &'e mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
) -> wgpu::RenderPass<'e> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// Render targets at the current render size.
pub struct Targets {
    pub size: (u32, u32),
    layers: [Rt; 2],
    feedback: [Rt; 2],
    cur: usize,
    bloom: [Rt; 2],
}

impl Targets {
    pub fn new(gpu: &Gpu, size: (u32, u32)) -> Self {
        let size = (size.0.max(1), size.1.max(1));
        let d = &gpu.device;
        let q = (size.0.div_ceil(4), size.1.div_ceil(4));
        Self {
            size,
            layers: [
                Rt::new(d, "layer-a", size, HDR),
                Rt::new(d, "layer-b", size, HDR),
            ],
            feedback: [
                Rt::new(d, "feedback-0", size, HDR),
                Rt::new(d, "feedback-1", size, HDR),
            ],
            cur: 0,
            bloom: [Rt::new(d, "bloom-0", q, HDR), Rt::new(d, "bloom-1", q, HDR)],
        }
    }

    /// The last composited frame (before post FX).
    pub fn frame(&self) -> &Rt {
        &self.feedback[self.cur]
    }
}

/// A scene layer to draw this frame, with its packed uniforms.
pub struct LayerDraw<'a> {
    pub layer: &'a mut Layer,
    pub music: &'a [f32],
    pub params: &'a [f32],
}

/// Device-wide state: the fixed passes and shared objects.
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub out_format: wgpu::TextureFormat,
    sampler: wgpu::Sampler,
    post_layout: wgpu::BindGroupLayout,
    post_buf: wgpu::Buffer,
    dummy: Rt,
    composite: wgpu::RenderPipeline,
    bright: wgpu::RenderPipeline,
    blur_h: wgpu::RenderPipeline,
    blur_v: wgpu::RenderPipeline,
    final_pass: wgpu::RenderPipeline,
    has_b: bool,
    /// Milliseconds from the last submit to the GPU finishing it (f32 bits).
    gpu_ms: Arc<AtomicU32>,
}

impl Gpu {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, out_format: wgpu::TextureFormat) -> Self {
        let d = &device;
        let sampler = d.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("visuals"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let tex = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let post_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                tex(1),
                tex(2),
                tex(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(POST.into()),
        });
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post"),
            bind_group_layouts: &[Some(&post_layout)],
            immediate_size: 0,
        });
        let pipe = |fs, format| fullscreen_pipeline(d, fs, &module, &pl, fs, format);
        let post_buf = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            composite: pipe("fs_composite", HDR),
            bright: pipe("fs_bright", HDR),
            blur_h: pipe("fs_blur_h", HDR),
            blur_v: pipe("fs_blur_v", HDR),
            final_pass: pipe("fs_final", out_format),
            dummy: Rt::new(d, "dummy", (1, 1), HDR),
            sampler,
            post_layout,
            post_buf,
            out_format,
            has_b: false,
            gpu_ms: Arc::new(AtomicU32::new(0)),
            device,
            queue,
        }
    }

    pub fn from_render_state(rs: &egui_wgpu::RenderState) -> Self {
        Self::new(rs.device.clone(), rs.queue.clone(), rs.target_format)
    }

    fn post_bind_group(&self, t0: &Rt, t1: &Rt, t2: &Rt) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post"),
            layout: &self.post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.post_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&t0.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&t1.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&t2.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn fixed_pass(
        &self,
        enc: &mut wgpu::CommandEncoder,
        pipe: &wgpu::RenderPipeline,
        out: &Rt,
        bg: &wgpu::BindGroup,
    ) {
        let mut pass = begin(enc, "post", &out.view);
        pass.set_pipeline(pipe);
        pass.set_bind_group(0, bg, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Renders one frame offscreen: scenes, mix + feedback, bloom. Submits its own commands.
    pub fn render(
        &mut self,
        t: &mut Targets,
        a: LayerDraw,
        b: Option<LayerDraw>,
        post: &PostParams,
    ) {
        self.has_b = b.is_some();
        let srgb = self.out_format.is_srgb();
        self.queue
            .write_buffer(&self.post_buf, 0, &bytes(&post.floats(self.has_b, srgb)));
        let mut enc = self.device.create_command_encoder(&Default::default());
        let prev = t.cur;
        t.cur = 1 - t.cur;
        a.layer.encode(
            self,
            &mut enc,
            &t.layers[0],
            &t.feedback[prev],
            a.music,
            a.params,
        );
        if let Some(b) = b {
            b.layer.encode(
                self,
                &mut enc,
                &t.layers[1],
                &t.feedback[prev],
                b.music,
                b.params,
            );
        }
        let bg = self.post_bind_group(&t.layers[0], &t.layers[1], &t.feedback[prev]);
        self.fixed_pass(&mut enc, &self.composite, &t.feedback[t.cur], &bg);
        let bg = self.post_bind_group(&t.feedback[t.cur], &self.dummy, &self.dummy);
        self.fixed_pass(&mut enc, &self.bright, &t.bloom[0], &bg);
        let bg = self.post_bind_group(&t.bloom[0], &self.dummy, &self.dummy);
        self.fixed_pass(&mut enc, &self.blur_h, &t.bloom[1], &bg);
        let bg = self.post_bind_group(&t.bloom[1], &self.dummy, &self.dummy);
        self.fixed_pass(&mut enc, &self.blur_v, &t.bloom[0], &bg);
        self.queue.submit([enc.finish()]);
        let (sent, slot) = (Instant::now(), self.gpu_ms.clone());
        self.queue.on_submitted_work_done(move || {
            slot.store(
                (sent.elapsed().as_secs_f32() * 1000.0).to_bits(),
                Ordering::Relaxed,
            );
        });
    }

    /// GPU time of the most recently completed frame, in ms: from submit until the GPU was seen
    /// to finish (an upper bound, since completion is noticed when the device is polled).
    pub fn last_gpu_ms(&self) -> f32 {
        let _ = self.device.poll(wgpu::PollType::Poll);
        f32::from_bits(self.gpu_ms.load(Ordering::Relaxed))
    }

    /// The final pass (post FX + upscale) as something egui can draw.
    pub fn final_callback(&self, t: &Targets) -> FinalPass {
        FinalPass {
            pipeline: self.final_pass.clone(),
            bind_group: self.post_bind_group(t.frame(), &t.bloom[0], &self.dummy),
        }
    }
}

/// Draws the composited frame to the screen inside egui's render pass.
pub struct FinalPass {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl FinalPass {
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl CallbackTrait for FinalPass {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        _resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        _resources: &CallbackResources,
    ) {
        self.draw(pass);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::codegen::{MUSIC_FLOATS, assemble, music_index, tests::bundled_prelude, validate};

    /// A headless device, or `None` when the machine has no usable GPU (tests then skip).
    pub fn headless() -> Option<Gpu> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        Some(Gpu::new(device, queue, wgpu::TextureFormat::Rgba8Unorm))
    }

    /// Average RGB (0..1) of an Rgba8Unorm render of the final pass.
    pub fn render_final(gpu: &Gpu, t: &Targets) -> [f32; 3] {
        let (w, h) = t.size;
        let out = Rt::new(&gpu.device, "out", (w, h), wgpu::TextureFormat::Rgba8Unorm);
        let row = (w * 4).div_ceil(256) * 256;
        let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let fp = gpu.final_callback(t);
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = begin(&mut enc, "final", &out.view);
            fp.draw(&mut pass);
        }
        enc.copy_texture_to_buffer(
            out.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([enc.finish()]);
        buf.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        let data = buf.get_mapped_range(..).unwrap();
        let mut sum = [0f64; 3];
        for y in 0..h {
            for x in 0..w {
                let i = (y * row + x * 4) as usize;
                for (c, s) in sum.iter_mut().enumerate() {
                    *s += data[i + c] as f64 / 255.0;
                }
            }
        }
        sum.map(|s| (s / (w * h) as f64) as f32)
    }

    pub fn program(gpu: &Gpu, manifest: &str, src: &str) -> Result<Rc<SceneProgram>, ShaderError> {
        let m = SceneManifest::parse(manifest).unwrap();
        let a = assemble(&m, &bundled_prelude(), "scene.wgsl", src);
        validate(&a)?;
        SceneProgram::new(gpu, "test", &a, &m).map(Rc::new)
    }

    fn music(size: (u32, u32)) -> Vec<f32> {
        let mut m = vec![0.0; MUSIC_FLOATS];
        m[music_index("aspect")] = size.0 as f32 / size.1 as f32;
        m[music_index("res_x")] = size.0 as f32;
        m[music_index("res_y")] = size.1 as f32;
        m
    }

    const FLAT: &str = "(name: \"flat\", kind: Fragment, params: [(name: \"col\", type: Color, default: [1, 0, 0], min: [0, 0, 0], max: [1, 1, 1])])";
    const FLAT_SRC: &str =
        "fn scene(uv: vec2f, m: Music, p: Params) -> vec4f { return vec4f(p.col * 0.5, 1.0); }";

    fn quiet() -> PostParams {
        PostParams {
            bloom: 0.0,
            vignette: 0.0,
            grain: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn fragment_scene_renders_its_params() {
        let Some(mut gpu) = headless() else { return };
        let size = (64, 36);
        let mut t = Targets::new(&gpu, size);
        let mut layer = Layer::new(&gpu, program(&gpu, FLAT, FLAT_SRC).unwrap());
        let m = music(size);
        gpu.render(
            &mut t,
            LayerDraw {
                layer: &mut layer,
                music: &m,
                params: &[0.0, 1.0, 0.0, 0.0],
            },
            None,
            &quiet(),
        );
        let c = render_final(&gpu, &t);
        assert!(
            c[0] < 0.02 && (c[1] - 0.5).abs() < 0.02 && c[2] < 0.02,
            "{c:?}"
        );
    }

    #[test]
    fn crossfade_mixes_two_layers() {
        let Some(mut gpu) = headless() else { return };
        let size = (32, 32);
        let mut t = Targets::new(&gpu, size);
        let prog = program(&gpu, FLAT, FLAT_SRC).unwrap();
        let (mut a, mut b) = (Layer::new(&gpu, prog.clone()), Layer::new(&gpu, prog));
        let m = music(size);
        let post = PostParams {
            mix_b: 0.5,
            ..quiet()
        };
        gpu.render(
            &mut t,
            LayerDraw {
                layer: &mut a,
                music: &m,
                params: &[1.0, 0.0, 0.0, 0.0],
            },
            Some(LayerDraw {
                layer: &mut b,
                music: &m,
                params: &[0.0, 0.0, 1.0, 0.0],
            }),
            &post,
        );
        let c = render_final(&gpu, &t);
        assert!(
            (c[0] - 0.25).abs() < 0.02 && (c[2] - 0.25).abs() < 0.02,
            "{c:?}"
        );
    }

    #[test]
    fn feedback_keeps_decayed_trails() {
        let Some(mut gpu) = headless() else { return };
        let size = (32, 32);
        let mut t = Targets::new(&gpu, size);
        // Transparent scene: only what feedback carries is visible after the first frame.
        let src = "fn scene(uv: vec2f, m: Music, p: Params) -> vec4f { return vec4f(p.col, select(0.0, 1.0, m.frame < 0.5)); }";
        let mut layer = Layer::new(&gpu, program(&gpu, FLAT, src).unwrap());
        let mut m = music(size);
        let post = PostParams {
            decay_a: 0.5,
            ..quiet()
        };
        let white = [0.8, 0.8, 0.8, 0.0];
        gpu.render(
            &mut t,
            LayerDraw {
                layer: &mut layer,
                music: &m,
                params: &white,
            },
            None,
            &post,
        );
        m[music_index("frame")] = 1.0;
        gpu.render(
            &mut t,
            LayerDraw {
                layer: &mut layer,
                music: &m,
                params: &white,
            },
            None,
            &post,
        );
        let c = render_final(&gpu, &t);
        assert!(
            (c[0] - 0.4).abs() < 0.03,
            "half of the previous frame survives: {c:?}"
        );
    }

    #[test]
    fn compute_scene_accumulates_points() {
        let Some(mut gpu) = headless() else { return };
        let size = (64, 64);
        let mut t = Targets::new(&gpu, size);
        let manifest = "(name: \"pts\", kind: Compute(invocations: 4096))";
        let src = "fn simulate(i: u32, m: Music, p: Params) {
    splat(hash22(vec2f(f32(i), 1.0)) * 2.0 - 1.0, vec3f(1.0));
}
fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    let a = accum_at(uv);
    return vec4f(vec3f(min(a.w, 1.0)), 1.0);
}";
        let prog = program(&gpu, manifest, src).unwrap();
        assert!(prog.is_compute());
        let mut layer = Layer::new(&gpu, prog);
        let m = music(size);
        gpu.render(
            &mut t,
            LayerDraw {
                layer: &mut layer,
                music: &m,
                params: &[],
            },
            None,
            &quiet(),
        );
        let c = render_final(&gpu, &t);
        // 4096 points over 32×32 cells: most cells are hit at least once.
        assert!(c[0] > 0.5 && c[0] < 1.0, "{c:?}");
    }

    #[test]
    fn every_bundled_scene_renders() {
        let Some(mut gpu) = headless() else { return };
        let dir = crate::library::tests::temp_dir("render");
        crate::library::install(&dir).unwrap();
        let prelude = crate::library::load_prelude(&dir);
        let size = (96, 54);
        for id in crate::library::scene_ids(&dir) {
            let src = crate::library::load_scene(&dir, &id, &prelude).unwrap();
            let prog = Rc::new(
                SceneProgram::new(&gpu, &id, &src.assembled, &src.manifest)
                    .unwrap_or_else(|e| panic!("{e}")),
            );
            let mut t = Targets::new(&gpu, size);
            let mut layer = Layer::new(&gpu, prog);
            let params = crate::codegen::pack_params(
                &src.manifest.defaults(),
                src.manifest.params.len().max(1),
            );
            let mut m = music(size);
            for (k, v) in [
                ("beats", 5.3),
                ("motion", 5.3),
                ("beat", 0.3),
                ("bar", 0.33),
                ("energy", 0.6),
                ("bass", 0.7),
                ("kick", 0.3),
                ("tempo", 128.0),
            ] {
                m[music_index(k)] = v;
            }
            for f in 0..3 {
                m[music_index("frame")] = f as f32;
                gpu.render(
                    &mut t,
                    LayerDraw {
                        layer: &mut layer,
                        music: &m,
                        params: &params,
                    },
                    None,
                    &quiet(),
                );
            }
            let c = render_final(&gpu, &t);
            let lum = c.iter().sum::<f32>() / 3.0;
            assert!(lum > 0.01 && lum < 0.99, "{id} renders something: {c:?}");
        }
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn hue_rotates_the_output() {
        let Some(mut gpu) = headless() else { return };
        let size = (16, 16);
        let mut t = Targets::new(&gpu, size);
        let mut layer = Layer::new(&gpu, program(&gpu, FLAT, FLAT_SRC).unwrap());
        let m = music(size);
        gpu.render(
            &mut t,
            LayerDraw {
                layer: &mut layer,
                music: &m,
                params: &[1.0, 0.0, 0.0, 0.0],
            },
            None,
            &PostParams {
                hue: 1.0 / 3.0,
                ..quiet()
            },
        );
        let c = render_final(&gpu, &t);
        assert!(c[1] > 0.4 && c[0] < 0.05, "red → green: {c:?}");
    }

    #[test]
    fn broken_pipeline_is_an_error_not_a_panic() {
        let Some(gpu) = headless() else { return };
        let m = SceneManifest::parse(FLAT).unwrap();
        let mut a = assemble(&m, &[], "scene.wgsl", FLAT_SRC);
        // Skip naga pre-validation: wgpu's own validation must be caught by the error scope.
        a.source = a.source.replace("fn fs_main", "fn fs_broken");
        let e = SceneProgram::new(&gpu, "broken", &a, &m)
            .err()
            .expect("an error");
        assert!(!e.message.is_empty());
    }
}
