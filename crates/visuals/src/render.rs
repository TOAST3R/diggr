//! Offline show rendering: a track's visual show drawn frame by frame on a supplied timeline
//! (frame n shows the music at exactly `from + n/fps`), read back from the GPU, and encoded
//! with ffmpeg together with the track's own audio. It is the live engine with a different
//! clock: same seed, same looks, same director — so the file matches the hands-off live show.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use analysis::SongScore;
use analysis::cache::ScoreCache;
use audio::decode::TrackDecoder;
use audio::tap::TAP_CHUNK_FRAMES;
use audio::{PlayState, Position, TapChunk};
use eframe::egui_wgpu::wgpu;
use platform::{FileSource, TrackRef};
use ui::fullscreen::SceneFrame;
use ui::render_job::{JobStatus, RenderJob, RenderRequest, ShowRenderer};
use ui::spectrum::Analyzer;

use crate::engine::{Tick, VisualEngine};
use crate::gpu::{Gpu, Rt, Targets};
use crate::render_overlay::OverlayPainter;

/// Readback buffers in flight: the GPU renders frame n while frame n−2 is copied out.
const RING: usize = 3;
/// Audio decoded before the start, so the spectrum bars have history on the first frame.
const PRE_ROLL_SECS: f64 = 1.0;

/// What to render.
#[derive(Debug, Clone)]
pub struct Options {
    pub track: TrackRef,
    pub out: PathBuf,
    pub size: (u32, u32),
    pub fps: u32,
    /// Seconds of the track.
    pub from: f64,
    pub to: Option<f64>,
    /// Draw the artist/title card for the first seconds.
    pub overlay: bool,
    /// Pin one look (`scene`, `variant`) with the director off.
    pub look: Option<(String, String)>,
    /// Looks, prelude and director rules: the live `visuals/` folder.
    pub visuals_dir: PathBuf,
    pub cache: Option<ScoreCache>,
}

impl Options {
    pub fn new(track: TrackRef, out: PathBuf) -> Self {
        Self {
            track,
            out,
            size: (1920, 1080),
            fps: 60,
            from: 0.0,
            to: None,
            overlay: false,
            look: None,
            visuals_dir: crate::library::default_dir(),
            cache: ScoreCache::platform_default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    pub frame: u64,
    pub frames: u64,
    pub elapsed_secs: f64,
}

impl Progress {
    pub fn fraction(&self) -> f64 {
        self.frame as f64 / self.frames.max(1) as f64
    }

    pub fn eta_secs(&self) -> Option<f64> {
        (self.frame > 0)
            .then(|| self.elapsed_secs / self.frame as f64 * (self.frames - self.frame) as f64)
    }

    /// Frames rendered per second of wall time.
    pub fn speed(&self) -> f64 {
        self.frame as f64 / self.elapsed_secs.max(1e-6)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    NoFfmpeg,
    Cancelled,
    Failed(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RenderError::NoFfmpeg => {
                f.write_str("ffmpeg not found — install it with `brew install ffmpeg`")
            }
            RenderError::Cancelled => f.write_str("cancelled"),
            RenderError::Failed(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for RenderError {}

fn failed(m: impl Into<String>) -> RenderError {
    RenderError::Failed(m.into())
}

/// The H.264 encoder ffmpeg offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoder {
    VideoToolbox,
    X264,
}

/// Checks ffmpeg is installed and picks an H.264 encoder (hardware when available).
pub fn check_ffmpeg() -> Result<Encoder, RenderError> {
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .stdin(Stdio::null())
        .output()
        .map_err(|_| RenderError::NoFfmpeg)?;
    let list = String::from_utf8_lossy(&out.stdout);
    if list.contains("h264_videotoolbox") {
        Ok(Encoder::VideoToolbox)
    } else if list.contains("libx264") {
        Ok(Encoder::X264)
    } else {
        Err(failed(
            "this ffmpeg has no H.264 encoder (h264_videotoolbox or libx264)",
        ))
    }
}

/// A headless GPU for rendering, or an error when the machine has none.
pub fn headless_gpu() -> Result<Gpu, RenderError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .map_err(|e| failed(format!("no GPU for rendering: {e}")))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| failed(format!("could not open the GPU: {e}")))?;
    Ok(Gpu::new(device, queue, wgpu::TextureFormat::Rgba8Unorm))
}

/// The track's complete analysis: cached, or computed now (and cached).
pub fn complete_score(
    files: &dyn FileSource,
    track: &TrackRef,
    cache: Option<&ScoreCache>,
) -> Option<SongScore> {
    let hash = analysis::cache::content_hash(files, track);
    if let (Some(c), Some(h)) = (cache, hash)
        && let Some(s) = c.load(h).filter(|s| s.complete)
    {
        return Some(s);
    }
    let score = analysis::eval::analyze_file(files, track)?;
    if let Some(c) = cache {
        let _ = c.save(&score);
    }
    Some(score)
}

const YUV_SHADER: &str = r#"
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;

// BT.709, limited ("TV") range, from display-referred RGB.
fn luma(c: vec3f) -> f32 {
    return 16.0 + 219.0 * dot(c, vec3f(0.2126, 0.7152, 0.0722));
}

fn chroma(c: vec3f) -> vec2f {
    let y = dot(c, vec3f(0.2126, 0.7152, 0.0722));
    return vec2f(128.0 + 224.0 * (c.b - y) / 1.8556, 128.0 + 224.0 * (c.r - y) / 1.5748);
}

fn px(x: u32, y: u32) -> vec3f {
    return clamp(textureLoad(src, vec2<u32>(x, y), 0).rgb, vec3f(0.0), vec3f(1.0));
}

fn pack(a: f32, b: f32, c: f32, d: f32) -> u32 {
    let v = vec4<u32>(round(clamp(vec4f(a, b, c, d), vec4f(0.0), vec4f(255.0))));
    return v.x | (v.y << 8u) | (v.z << 16u) | (v.w << 24u);
}

// One invocation packs 4 luma samples of a row.
@compute @workgroup_size(64)
fn cs_y(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(src);
    let quads = dims.x / 4u;
    if (id.x >= quads * dims.y) { return; }
    let x = (id.x % quads) * 4u;
    let y = id.x / quads;
    out[id.x] = pack(luma(px(x, y)), luma(px(x + 1u, y)), luma(px(x + 2u, y)), luma(px(x + 3u, y)));
}

// One invocation packs 4 samples each of U and V: an 8×2 block of pixels.
@compute @workgroup_size(64)
fn cs_uv(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(src);
    let quads = dims.x / 8u;
    let rows = dims.y / 2u;
    if (id.x >= quads * rows) { return; }
    let x0 = (id.x % quads) * 8u;
    let y0 = (id.x / quads) * 2u;
    var u: array<f32, 4>;
    var v: array<f32, 4>;
    for (var k = 0u; k < 4u; k++) {
        let x = x0 + k * 2u;
        let c = (px(x, y0) + px(x + 1u, y0) + px(x, y0 + 1u) + px(x + 1u, y0 + 1u)) * 0.25;
        let uv = chroma(c);
        u[k] = uv.x;
        v[k] = uv.y;
    }
    let luma_words = dims.x * dims.y / 4u;
    let plane_words = luma_words / 4u;
    out[luma_words + id.x] = pack(u[0], u[1], u[2], u[3]);
    out[luma_words + plane_words + id.x] = pack(v[0], v[1], v[2], v[3]);
}
"#;

pub use ui::render_job::size_ok;

/// Converts an Rgba8 texture to planar YUV 4:2:0 (BT.709, limited range) on the GPU, so only
/// 1.5 bytes per pixel leave the GPU and ffmpeg needs no conversion.
struct Yuv {
    y: wgpu::ComputePipeline,
    uv: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    out: wgpu::Buffer,
    size: (u32, u32),
}

impl Yuv {
    fn new(gpu: &Gpu, size: (u32, u32)) -> Self {
        let d = &gpu.device;
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("yuv"),
            source: wgpu::ShaderSource::Wgsl(YUV_SHADER.into()),
        });
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("yuv"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("yuv"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipe = |entry| {
            d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Self {
            y: pipe("cs_y"),
            uv: pipe("cs_uv"),
            out: d.create_buffer(&wgpu::BufferDescriptor {
                label: Some("yuv"),
                size: Self::bytes(size) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            layout,
            size,
        }
    }

    fn bytes(size: (u32, u32)) -> usize {
        (size.0 * size.1 * 3 / 2) as usize
    }

    /// Records the conversion of `src` into the output buffer.
    fn encode(&self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, src: &wgpu::TextureView) {
        let bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("yuv"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.out.as_entire_binding(),
                },
            ],
        });
        let (w, h) = self.size;
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("yuv"),
            timestamp_writes: None,
        });
        pass.set_bind_group(0, &bg, &[]);
        pass.set_pipeline(&self.y);
        pass.dispatch_workgroups((w / 4 * h).div_ceil(64), 1, 1);
        pass.set_pipeline(&self.uv);
        pass.dispatch_workgroups((w / 8 * h / 2).div_ceil(64), 1, 1);
    }
}

/// Final frames converted to YUV 4:2:0 and read back from the GPU, three in flight.
struct Capture {
    tex: Rt,
    yuv: Yuv,
    bufs: Vec<(wgpu::Buffer, Arc<AtomicBool>)>,
    free: VecDeque<usize>,
    inflight: VecDeque<(usize, wgpu::SubmissionIndex)>,
    out: Vec<u8>,
}

impl Capture {
    fn new(gpu: &Gpu, size: (u32, u32)) -> Self {
        let bytes = Yuv::bytes(size);
        let bufs = (0..RING)
            .map(|_| {
                let b = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("render-readback"),
                    size: bytes as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                (b, Arc::new(AtomicBool::new(false)))
            })
            .collect();
        Self {
            tex: Rt::new(
                &gpu.device,
                "render-out",
                size,
                wgpu::TextureFormat::Rgba8Unorm,
            ),
            yuv: Yuv::new(gpu, size),
            bufs,
            free: (0..RING).collect(),
            inflight: VecDeque::new(),
            out: vec![0; bytes],
        }
    }

    fn has_room(&self) -> bool {
        !self.free.is_empty()
    }

    /// Draws the final pass of the last rendered frame (and the overlay card), converts it to
    /// YUV, and starts copying it back.
    fn push(
        &mut self,
        gpu: &Gpu,
        t: &Targets,
        overlay: Option<(&mut OverlayPainter, &SceneFrame, f64)>,
    ) {
        let i = self.free.pop_front().expect("room checked by the caller");
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        gpu.encode_final(&mut enc, t, &self.tex.view);
        if let Some((painter, frame, now)) = overlay {
            painter.paint(gpu, &mut enc, &self.tex.view, frame, now);
        }
        self.yuv.encode(gpu, &mut enc, &self.tex.view);
        enc.copy_buffer_to_buffer(&self.yuv.out, 0, &self.bufs[i].0, 0, self.out.len() as u64);
        let idx = gpu.queue.submit([enc.finish()]);
        let flag = self.bufs[i].1.clone();
        flag.store(false, Ordering::Release);
        self.bufs[i].0.map_async(wgpu::MapMode::Read, .., move |r| {
            if r.is_ok() {
                flag.store(true, Ordering::Release);
            }
        });
        self.inflight.push_back((i, idx));
    }

    /// The oldest frame in flight (planar YUV 4:2:0), if it is ready (or `block`ing for it).
    fn pop(&mut self, gpu: &Gpu, block: bool) -> Option<&[u8]> {
        let (i, idx) = self.inflight.front().cloned()?;
        let flag = &self.bufs[i].1;
        if !flag.load(Ordering::Acquire) {
            let poll = if block {
                wgpu::PollType::Wait {
                    submission_index: Some(idx),
                    timeout: None,
                }
            } else {
                wgpu::PollType::Poll
            };
            let _ = gpu.device.poll(poll);
            if !flag.load(Ordering::Acquire) {
                return None;
            }
        }
        self.inflight.pop_front();
        let buf = &self.bufs[i].0;
        self.out
            .copy_from_slice(&buf.get_mapped_range(..).expect("mapped"));
        buf.unmap();
        self.free.push_back(i);
        Some(&self.out)
    }
}

/// Feeds the 19-band spectrum analyzer with decoded audio up to each frame's time, as the live
/// tap would.
struct Bars {
    dec: TrackDecoder,
    analyzer: Analyzer,
    raw: Vec<f32>,
    /// Decoded but not yet fed (interleaved stereo), starting at track frame `next`.
    pending: Vec<f32>,
    next: u64,
    done: bool,
}

impl Bars {
    fn new(files: &dyn FileSource, track: &TrackRef, from: f64) -> Result<Self, RenderError> {
        let mut dec = TrackDecoder::open(files, track).map_err(|e| failed(e.to_string()))?;
        let rate = dec.src_rate();
        let start = (from - PRE_ROLL_SECS).max(0.0);
        if start > 0.0 {
            dec.seek(start).map_err(|e| failed(e.to_string()))?;
        }
        Ok(Self {
            dec,
            analyzer: Analyzer::new(rate),
            raw: Vec::new(),
            pending: Vec::new(),
            next: (start * rate as f64).round() as u64,
            done: false,
        })
    }

    fn rate(&self) -> u32 {
        self.dec.src_rate()
    }

    /// Feeds audio up to track frame `upto` and updates the bars for `pos`.
    fn update(&mut self, pos: &Position, dt: f32) -> &[f32; ui::spectrum::BARS] {
        while self.next + ((self.pending.len() / 2) as u64) < pos.frame && !self.done {
            self.raw.clear();
            self.done = !self.dec.decode_next(&mut self.raw).unwrap_or(false);
            self.pending.extend_from_slice(&self.raw);
        }
        while self.pending.len() >= TAP_CHUNK_FRAMES * 2
            && self.next + TAP_CHUNK_FRAMES as u64 <= pos.frame
        {
            let mut c = TapChunk {
                track: pos.track,
                start_frame: self.next,
                frames: TAP_CHUNK_FRAMES,
                samples: [0.0; TAP_CHUNK_FRAMES * 2],
            };
            c.samples
                .copy_from_slice(&self.pending[..TAP_CHUNK_FRAMES * 2]);
            self.pending.drain(..TAP_CHUNK_FRAMES * 2);
            self.next += TAP_CHUNK_FRAMES as u64;
            self.analyzer.push(&c);
        }
        self.analyzer.update(pos, dt);
        self.analyzer.bars()
    }
}

/// Renders the show frame by frame and hands each finished frame to `sink` as planar YUV 4:2:0
/// (BT.709, limited range): the Y plane (`w × h` bytes), then U and V (`w/2 × h/2` each). `progress` is called after every frame; returning false cancels.
/// Returns the number of frames.
pub fn render_frames(
    files: &dyn FileSource,
    opts: &Options,
    gpu: Gpu,
    mut sink: impl FnMut(&[u8]) -> Result<(), String>,
    mut progress: impl FnMut(&Progress) -> bool,
) -> Result<u64, RenderError> {
    if !size_ok(opts.size) {
        return Err(failed(format!(
            "frame size {}x{} must have a width divisible by 8 and an even height",
            opts.size.0, opts.size.1
        )));
    }
    let score = complete_score(files, &opts.track, opts.cache.as_ref())
        .ok_or_else(|| failed("could not analyze the track"))?;
    let mut bars = Bars::new(files, &opts.track, opts.from)?;
    let rate = bars.rate();
    let info = TrackDecoder::open(files, &opts.track)
        .map(|mut d| d.complete_info(files, &opts.track).clone())
        .map_err(|e| failed(e.to_string()))?;
    let duration = info
        .duration_secs
        .or_else(|| score.beats.last().copied())
        .ok_or_else(|| failed("unknown track length"))?;
    let to = opts.to.unwrap_or(duration).min(duration);
    if to <= opts.from {
        return Err(failed("the range is empty"));
    }
    let frames = ((to - opts.from) * opts.fps as f64).floor() as u64;
    let dt = 1.0 / opts.fps as f32;

    let mut engine = VisualEngine::new_offline(&opts.visuals_dir, gpu);
    if let Some((scene, variant)) = &opts.look {
        engine.pin_look(scene, variant).map_err(failed)?;
    }
    let mut capture = Capture::new(engine.gpu().expect("attached"), opts.size);
    let mut overlay = opts
        .overlay
        .then(|| OverlayPainter::new(engine.gpu().expect("attached"), opts.size));
    let started = Instant::now();
    for n in 0..frames {
        let t = opts.from + n as f64 / opts.fps as f64;
        let position = Position {
            track: 1,
            frame: (t * rate as f64).round() as u64,
            sample_rate: rate,
            state: PlayState::Playing,
            discontinuity: n == 0,
        };
        let bars = *bars.update(&position, dt);
        let frame = SceneFrame {
            position,
            bars: &bars,
            artist: &info.artist,
            title: &info.title,
            score: Some(&score),
            duration: Some(duration),
            strip_visible: false,
        };
        let tick = Tick {
            now: t - opts.from,
            dt,
            lead_secs: 0.0,
        };
        if !engine.render_offline(&frame, tick, opts.size) {
            let why = engine.errors().join("; ");
            return Err(failed(format!(
                "nothing could be drawn{}",
                if why.is_empty() {
                    String::new()
                } else {
                    format!(": {why}")
                }
            )));
        }
        let gpu = engine.gpu().expect("attached");
        let targets = engine.targets().expect("drawn");
        while !capture.has_room() {
            let bytes = capture.pop(gpu, true).expect("frames in flight");
            sink(bytes).map_err(failed)?;
        }
        capture.push(
            gpu,
            targets,
            overlay.as_mut().map(|o| (o, &frame, t - opts.from)),
        );
        while let Some(bytes) = capture.pop(gpu, false) {
            sink(bytes).map_err(failed)?;
        }
        let p = Progress {
            frame: n + 1,
            frames,
            elapsed_secs: started.elapsed().as_secs_f64(),
        };
        if !progress(&p) {
            return Err(RenderError::Cancelled);
        }
    }
    let gpu = engine.gpu().expect("attached");
    while let Some(bytes) = capture.pop(gpu, true) {
        sink(bytes).map_err(failed)?;
    }
    Ok(frames)
}

fn ffmpeg_args(opts: &Options, enc: Encoder, dur: Option<f64>, part: &Path) -> Vec<String> {
    let (w, h) = opts.size;
    const BT709: [&str; 8] = [
        "-color_range",
        "tv",
        "-colorspace",
        "bt709",
        "-color_primaries",
        "bt709",
        "-color_trc",
        "bt709",
    ];
    let mut a: Vec<String> = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "yuv420p",
    ]
    .map(String::from)
    .to_vec();
    a.extend(BT709.map(String::from));
    a.extend([
        "-s".into(),
        format!("{w}x{h}"),
        "-framerate".into(),
        opts.fps.to_string(),
        "-i".into(),
        "-".into(),
    ]);
    a.extend(["-ss".into(), format!("{:.3}", opts.from)]);
    if let Some(d) = dur {
        a.extend(["-t".into(), format!("{d:.3}")]);
    }
    a.extend([
        "-i".into(),
        opts.track.0.clone(),
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "1:a:0?".into(),
    ]);
    match enc {
        Encoder::VideoToolbox => {
            // About 0.16 bits per pixel per frame: 20 Mbit/s at 1080p60.
            let bitrate = (w as u64 * h as u64 * opts.fps as u64 * 16 / 100).max(2_000_000);
            a.extend([
                "-c:v".into(),
                "h264_videotoolbox".into(),
                "-b:v".into(),
                bitrate.to_string(),
            ]);
        }
        Encoder::X264 => {
            a.extend(["-c:v", "libx264", "-preset", "medium", "-crf", "18"].map(String::from))
        }
    }
    a.extend(BT709.map(String::from));
    a.extend(
        [
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "320k",
            "-shortest",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ]
        .map(String::from),
    );
    a.push(part.to_string_lossy().into_owned());
    a
}

/// A render that finished.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub frames: u64,
    pub seconds: f64,
}

/// Renders the show to an MP4 at `opts.out`. The file appears only when the render succeeds;
/// on failure or cancel nothing is left behind.
pub fn render_show(
    files: &dyn FileSource,
    opts: &Options,
    progress: impl FnMut(&Progress) -> bool,
) -> Result<Summary, RenderError> {
    let enc = check_ffmpeg()?;
    let gpu = headless_gpu()?;
    let name = opts
        .out
        .file_name()
        .map_or_else(|| "show.mp4".into(), |n| n.to_string_lossy().into_owned());
    let part = opts.out.with_file_name(format!(".{name}.part"));
    let dur = opts.to.map(|t| t - opts.from);
    let mut child = Command::new("ffmpeg")
        .args(ffmpeg_args(opts, enc, dur, &part))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| RenderError::NoFfmpeg)?;
    let mut stdin = child.stdin.take().expect("piped");
    let started = Instant::now();
    let result = render_frames(
        files,
        opts,
        gpu,
        |bytes| stdin.write_all(bytes).map_err(|e| e.to_string()),
        progress,
    );
    drop(stdin);
    let cleanup = |child: &mut std::process::Child| {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&part);
    };
    let frames = match result {
        Ok(f) => f,
        Err(e) => {
            // A broken pipe usually means ffmpeg failed: prefer its message.
            let mut msg = String::new();
            if let Some(mut err) = child.stderr.take() {
                let _ = err.read_to_string(&mut msg);
            }
            cleanup(&mut child);
            return Err(match e {
                RenderError::Failed(m) if !msg.trim().is_empty() => {
                    failed(format!("ffmpeg: {} ({m})", msg.trim()))
                }
                e => e,
            });
        }
    };
    let status = child.wait().map_err(|e| failed(e.to_string()))?;
    if !status.success() {
        let mut msg = String::new();
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_string(&mut msg);
        }
        let _ = std::fs::remove_file(&part);
        return Err(failed(format!("ffmpeg failed: {}", msg.trim())));
    }
    std::fs::rename(&part, &opts.out).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        failed(format!("could not write {}: {e}", opts.out.display()))
    })?;
    Ok(Summary {
        frames,
        seconds: started.elapsed().as_secs_f64(),
    })
}

/// Renders shows on a low-priority background thread for the player's "Render show…".
pub struct BackgroundRenderer {
    files: Arc<dyn FileSource>,
    spawner: Arc<dyn platform::Spawner>,
    visuals_dir: PathBuf,
    cache: Option<ScoreCache>,
}

impl BackgroundRenderer {
    /// Renders with the live looks (`<config>/winamp_rust/visuals`) and the score cache.
    pub fn new(files: Arc<dyn FileSource>, spawner: Arc<dyn platform::Spawner>) -> Self {
        Self {
            files,
            spawner,
            visuals_dir: crate::library::default_dir(),
            cache: ScoreCache::platform_default(),
        }
    }

    /// Other folders for looks and scores.
    pub fn with_dirs(mut self, visuals_dir: PathBuf, cache: Option<ScoreCache>) -> Self {
        self.visuals_dir = visuals_dir;
        self.cache = cache;
        self
    }
}

struct Job {
    status: std::sync::Mutex<JobStatus>,
    cancel: AtomicBool,
    paused: AtomicBool,
}

impl RenderJob for Job {
    fn status(&self) -> JobStatus {
        self.status.lock().expect("not poisoned").clone()
    }

    fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }
}

impl ShowRenderer for BackgroundRenderer {
    fn looks(&self) -> Vec<(String, String)> {
        let _ = crate::library::install(&self.visuals_dir);
        let mut out = Vec::new();
        for id in crate::library::scene_ids(&self.visuals_dir) {
            let (variants, _) = crate::library::load_variants(&self.visuals_dir, &id);
            if variants.is_empty() {
                out.push((id.clone(), "default".to_string()));
            }
            out.extend(variants.into_iter().map(|v| (id.clone(), v.name)));
        }
        out
    }

    fn start(&self, r: RenderRequest) -> Arc<dyn RenderJob> {
        let (track, out) = (r.track.clone(), r.out.clone());
        let job = Arc::new(Job {
            status: std::sync::Mutex::new(JobStatus::Running {
                fraction: 0.0,
                eta_secs: None,
            }),
            cancel: AtomicBool::new(false),
            paused: AtomicBool::new(false),
        });
        let opts = Options {
            size: r.size,
            fps: r.fps,
            from: r.from,
            to: r.to,
            overlay: r.overlay,
            look: r.look,
            visuals_dir: self.visuals_dir.clone(),
            cache: self.cache.clone(),
            ..Options::new(track, out.clone())
        };
        let (j, files) = (job.clone(), self.files.clone());
        let spawned = self.spawner.spawn(
            "show-render",
            platform::Priority::Low,
            Box::new(move || {
                let set = |s: JobStatus| *j.status.lock().expect("not poisoned") = s;
                let result = render_show(&*files, &opts, |p| {
                    while j.paused.load(Ordering::Relaxed) && !j.cancel.load(Ordering::Relaxed) {
                        set(JobStatus::Paused {
                            fraction: p.fraction(),
                        });
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    set(JobStatus::Running {
                        fraction: p.fraction(),
                        eta_secs: p.eta_secs(),
                    });
                    !j.cancel.load(Ordering::Relaxed)
                });
                set(match result {
                    Ok(_) => JobStatus::Done { out },
                    Err(RenderError::Cancelled) => JobStatus::Cancelled,
                    Err(e) => JobStatus::Failed(e.to_string()),
                });
            }),
        );
        if let Err(e) = spawned {
            *job.status.lock().expect("not poisoned") = JobStatus::Failed(e.to_string());
        }
        job
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analysis::synth::{self, Pattern};
    use platform::native::NativeFileSource;

    fn temp_dir(name: &str) -> platform::testing::TestDir {
        platform::testing::TestDir::new(&format!("show-render-{name}"))
    }

    /// Eight bars of groove into eight of drop at 128 BPM (30 s), as a stereo WAV.
    fn track(dir: &Path) -> TrackRef {
        let s = synth::render(
            &[
                synth::part(Pattern::Groove, 8, 128.0),
                synth::part(Pattern::Drop, 8, 128.0),
            ],
            44_100,
            7,
        );
        let path = dir.join("track.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for x in &s.samples {
            let v = (x.clamp(-1.0, 1.0) * 32000.0) as i16;
            w.write_sample(v).unwrap();
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
        TrackRef::new(path.to_string_lossy())
    }

    fn options(dir: &Path, t: TrackRef) -> Options {
        Options {
            size: (160, 90),
            fps: 30,
            from: 14.0,
            to: Some(16.0),
            visuals_dir: dir.join("visuals"),
            cache: Some(ScoreCache::new(dir.join("cache"))),
            ..Options::new(t, dir.join("show.mp4"))
        }
    }

    fn frames(opts: &Options) -> Option<Vec<Vec<u8>>> {
        let gpu = headless_gpu().ok()?;
        let mut out = Vec::new();
        render_frames(
            &NativeFileSource,
            opts,
            gpu,
            |f| {
                out.push(f.to_vec());
                Ok(())
            },
            |_| true,
        )
        .unwrap();
        Some(out)
    }

    #[test]
    fn renders_are_deterministic_and_complete() {
        let dir = temp_dir("determinism");
        let opts = options(&dir, track(&dir));
        let Some(a) = frames(&opts) else { return };
        assert_eq!(a.len(), 60, "2 s at 30 fps");
        assert!(a.iter().all(|f| f.len() == 160 * 90 * 3 / 2), "YUV 4:2:0");
        let lit = |f: &[u8]| f[..160 * 90].iter().filter(|&&y| y > 40).count();
        assert!(a.iter().any(|f| lit(f) > 160 * 9), "the show is visible");
        let b = frames(&opts).unwrap();
        assert!(a == b, "same track, same looks: identical frames");
        // The score was analyzed once and cached.
        assert!(dir.join("cache").join("scores").read_dir().unwrap().count() == 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn overlay_card_and_pinned_look() {
        let dir = temp_dir("overlay");
        let t = track(&dir);
        let base = Options {
            to: Some(14.5),
            look: Some(("flame".into(), "silk".into())),
            ..options(&dir, t)
        };
        let Some(plain) = frames(&base) else { return };
        let with = frames(&Options {
            overlay: true,
            ..base.clone()
        })
        .unwrap();
        let diff: usize = plain[0]
            .iter()
            .zip(&with[0])
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            diff > 500,
            "the card changes the first frame ({diff} bytes)"
        );
        let bad = Options {
            look: Some(("flame".into(), "nope".into())),
            ..base
        };
        let err = render_frames(
            &NativeFileSource,
            &bad,
            headless_gpu().unwrap(),
            |_| Ok(()),
            |_| true,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("flame/silk"),
            "lists the available looks: {err}"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn yuv_conversion_is_bt709_limited_range() {
        let Ok(gpu) = headless_gpu() else { return };
        let size = (16, 16);
        let yuv = Yuv::new(&gpu, size);
        let tex = Rt::new(&gpu.device, "src", size, wgpu::TextureFormat::Rgba8Unorm);
        let check = |rgb: [u8; 3]| -> (u8, u8, u8) {
            let px: Vec<u8> = (0..16 * 16)
                .flat_map(|_| [rgb[0], rgb[1], rgb[2], 255])
                .collect();
            gpu.queue.write_texture(
                tex.texture.as_image_copy(),
                &px,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(64),
                    rows_per_image: Some(16),
                },
                wgpu::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
            );
            let read = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: Yuv::bytes(size) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut enc = gpu.device.create_command_encoder(&Default::default());
            yuv.encode(&gpu, &mut enc, &tex.view);
            enc.copy_buffer_to_buffer(&yuv.out, 0, &read, 0, Yuv::bytes(size) as u64);
            gpu.queue.submit([enc.finish()]);
            read.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            let d = read.get_mapped_range(..).unwrap();
            (d[0], d[256], d[256 + 64])
        };
        assert_eq!(check([255, 255, 255]), (235, 128, 128), "white");
        assert_eq!(check([0, 0, 0]), (16, 128, 128), "black");
        assert_eq!(check([255, 0, 0]), (63, 102, 240), "red");
    }

    #[test]
    fn ffmpeg_arguments() {
        let opts = Options {
            size: (1920, 1080),
            fps: 60,
            from: 60.0,
            ..Options::new(TrackRef::new("/m/a b.flac"), "/o/s.mp4".into())
        };
        let a = ffmpeg_args(
            &opts,
            Encoder::VideoToolbox,
            Some(30.0),
            Path::new("/o/.s.mp4.part"),
        )
        .join(" ");
        assert!(
            a.contains("-pix_fmt yuv420p -color_range tv -colorspace bt709"),
            "{a}"
        );
        assert!(a.contains("-s 1920x1080 -framerate 60 -i -"), "{a}");
        assert!(a.contains("-ss 60.000 -t 30.000 -i /m/a b.flac"), "{a}");
        assert!(
            a.contains("h264_videotoolbox -b:v 19906560"),
            "≈20 Mbit/s at 1080p60: {a}"
        );
        assert!(a.ends_with("-f mp4 /o/.s.mp4.part"), "{a}");
        let x = ffmpeg_args(&opts, Encoder::X264, None, Path::new("p")).join(" ");
        assert!(x.contains("libx264") && !x.contains(" -t "), "{x}");
        assert_eq!(
            RenderError::NoFfmpeg.to_string(),
            "ffmpeg not found — install it with `brew install ffmpeg`"
        );
    }

    #[test]
    fn mp4_end_to_end_and_cancel_leaves_nothing() {
        if check_ffmpeg().is_err() || headless_gpu().is_err() {
            return;
        }
        let dir = temp_dir("mp4");
        let opts = Options {
            size: (320, 180),
            ..options(&dir, track(&dir))
        };
        let mut last = None;
        let summary = render_show(&NativeFileSource, &opts, |p| {
            last = Some(*p);
            true
        })
        .unwrap();
        assert_eq!(summary.frames, 60);
        assert_eq!(last.unwrap().frame, 60);
        let probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type,width,height",
                "-show_entries",
                "format=duration",
                "-of",
                "compact",
            ])
            .arg(&opts.out)
            .output()
            .unwrap();
        let info = String::from_utf8_lossy(&probe.stdout);
        assert!(
            info.contains("codec_type=video|width=320|height=180"),
            "{info}"
        );
        assert!(info.contains("codec_type=audio"), "{info}");
        let dur: f64 = info
            .lines()
            .find_map(|l| l.strip_prefix("format|duration="))
            .unwrap()
            .parse()
            .unwrap();
        assert!((dur - 2.0).abs() < 0.1, "{dur}");

        let cancelled = Options {
            out: dir.join("cancelled.mp4"),
            ..opts
        };
        let err = render_show(&NativeFileSource, &cancelled, |p| p.frame < 10).unwrap_err();
        assert_eq!(err, RenderError::Cancelled);
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("cancelled"))
            .collect();
        assert!(left.is_empty(), "no partial file: {left:?}");
        std::fs::remove_dir_all(dir).ok();
    }

    struct Threads;
    impl platform::Spawner for Threads {
        fn spawn(
            &self,
            _: &str,
            _: platform::Priority,
            f: Box<dyn FnOnce() + Send + 'static>,
        ) -> Result<(), platform::PlatformError> {
            std::thread::spawn(f);
            Ok(())
        }
    }

    fn wait_finished(job: &dyn RenderJob) -> JobStatus {
        let t = Instant::now();
        loop {
            let s = job.status();
            if s.finished() {
                return s;
            }
            assert!(
                t.elapsed().as_secs() < 120,
                "render finished in time: {s:?}"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn background_jobs_finish_pause_and_cancel() {
        if check_ffmpeg().is_err() || headless_gpu().is_err() {
            return;
        }
        let dir = temp_dir("job");
        let t = track(&dir);
        let r = BackgroundRenderer::new(Arc::new(NativeFileSource), Arc::new(Threads)).with_dirs(
            dir.join("visuals"),
            Some(ScoreCache::new(dir.join("cache"))),
        );
        // The whole 30 s track at 1080p60 would be slow for a test; the job API renders the
        // whole track, so check it on this short one.
        let out = dir.join("job.mp4");
        let req = |out: PathBuf| RenderRequest {
            track: t.clone(),
            out,
            size: (320, 180),
            fps: 30,
            from: 14.0,
            to: Some(16.0),
            overlay: false,
            look: None,
        };
        assert!(
            r.looks()
                .contains(&("flame".to_string(), "silk".to_string()))
        );
        let job = r.start(req(out.clone()));
        job.set_paused(true);
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(
            matches!(
                job.status(),
                JobStatus::Paused { .. } | JobStatus::Running { fraction: 0.0, .. }
            ),
            "{:?}",
            job.status()
        );
        job.set_paused(false);
        assert_eq!(wait_finished(&*job), JobStatus::Done { out: out.clone() });
        assert!(out.exists());
        let cancelled = dir.join("cancelled.mp4");
        let job = r.start(RenderRequest {
            to: None,
            ..req(cancelled.clone())
        });
        std::thread::sleep(std::time::Duration::from_millis(300));
        job.cancel();
        assert_eq!(wait_finished(&*job), JobStatus::Cancelled);
        assert!(!cancelled.exists());
        std::fs::remove_dir_all(dir).ok();
    }
}
