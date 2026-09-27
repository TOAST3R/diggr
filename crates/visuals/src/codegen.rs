//! Turns a scene (manifest + the author's `scene.wgsl`) into one complete WGSL module: engine
//! bindings, the `Music` and generated `Params` structs, the prelude helpers, the author's code,
//! and the entry points. Validated with naga so errors point at the author's file and line.

use crate::manifest::{ParamType, SceneKind, SceneManifest};

/// `Music` fields (all f32, in uniform order), followed by `bands: array<vec4<f32>, 5>` (the 19
/// live spectrum bars, padded to 20). Kept here so WGSL and the CPU packing can't drift apart.
pub const MUSIC_FIELDS: [&str; 32] = [
    "time",   // track seconds
    "beats",  // musical time (beats since track start)
    "motion", // beats × speed macro: use this to drive movement
    "beat",   // phase 0..1
    "bar",    // phase 0..1
    "phrase", // phase 0..1 (8 bars)
    "section_progress",
    "tempo", // BPM
    "energy",
    "bass",
    "mid",
    "treble",
    "brightness",
    "tension",
    "drop_in", // beats to the next drop (9999 when none)
    "beat_confidence",
    "section_kind", // -1 unknown, 0 intro, 1 build, 2 drop, 3 breakdown, 4 groove, 5 outro
    "section_label",
    "kick", // beats since the last kick (large when none)
    "snare",
    "hat",
    "downbeat",
    "section_change",
    "drop",
    "flash",  // director flash 0..1
    "aspect", // width / height
    "res_x",
    "res_y",
    "playing", // 1 or 0
    "seed",    // per-track 0..1
    "frame",
    "hue", // hue macro, turns (-0.5..0.5)
];
pub const MUSIC_BANDS: usize = 5;
pub const MUSIC_FLOATS: usize = MUSIC_FIELDS.len() + MUSIC_BANDS * 4;

pub fn music_index(name: &str) -> usize {
    MUSIC_FIELDS
        .iter()
        .position(|&f| f == name)
        .unwrap_or_else(|| panic!("no music field {name}"))
}

/// Number of vec4 slots in the params uniform (at least one: WGSL has no empty arrays).
pub fn param_slots(m: &SceneManifest) -> usize {
    m.params.len().max(1)
}

/// Where the pieces of an assembled module came from, for mapping error lines.
#[derive(Debug, Clone)]
pub struct Segment {
    pub file: String,
    /// 1-based first line in the assembled source.
    pub start: usize,
    pub lines: usize,
}

#[derive(Debug, Clone)]
pub struct Assembled {
    pub source: String,
    pub segments: Vec<Segment>,
}

impl Assembled {
    /// (file, line within that file) for a 1-based line of the assembled source.
    pub fn locate(&self, line: usize) -> (String, usize) {
        for s in &self.segments {
            if line >= s.start && line < s.start + s.lines {
                return (s.file.clone(), line - s.start + 1);
            }
        }
        ("<generated>".into(), line)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShaderError {
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{l}: {}", self.file, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}

fn bindings(m: &SceneManifest) -> String {
    let mut s = String::from("// ---- engine bindings (generated) ----\nstruct Music {\n");
    for f in MUSIC_FIELDS {
        s += &format!("    {f}: f32,\n");
    }
    s += &format!("    bands: array<vec4<f32>, {MUSIC_BANDS}>,\n}}\n\n");
    s += "struct ParamsRaw {\n";
    s += &format!("    v: array<vec4<f32>, {}>,\n}}\n\n", param_slots(m));
    s += "struct Params {\n";
    for p in &m.params {
        s += &format!("    {}: {},\n", p.name, p.ty.wgsl());
    }
    if m.params.is_empty() {
        s += "    _unused: f32,\n";
    }
    s += "}\n\n";
    s += "@group(0) @binding(0) var<uniform> music_u: Music;\n";
    s += "@group(0) @binding(1) var<uniform> params_u: ParamsRaw;\n";
    s += "@group(0) @binding(2) var prev_frame: texture_2d<f32>;\n";
    s += "@group(0) @binding(3) var prev_sampler: sampler;\n";
    if m.is_compute() {
        s += "@group(0) @binding(4) var<storage, read_write> accum: array<atomic<u32>>;\n";
    }
    s += "\nfn load_music() -> Music {\n    return music_u;\n}\n\n";
    s += "fn load_params() -> Params {\n    var p: Params;\n";
    for (i, p) in m.params.iter().enumerate() {
        let v = format!("params_u.v[{i}]");
        let e = match p.ty {
            ParamType::F32 => format!("{v}.x"),
            ParamType::Int => format!("i32(round({v}.x))"),
            ParamType::Vec2 => format!("{v}.xy"),
            ParamType::Vec3 | ParamType::Color => format!("{v}.xyz"),
        };
        s += &format!("    p.{} = {e};\n", p.name);
    }
    s += "    return p;\n}\n\n";
    s += r#"// Level of a live spectrum bar, 0 (lowest) ..= 18.
fn band(m: Music, i: i32) -> f32 {
    let k = clamp(i, 0, 19);
    return m.bands[k / 4][k % 4];
}

// A trigger as a decaying pulse: 1 on the hit, falling with `rate` per beat.
fn pulse(beats_since: f32, rate: f32) -> f32 {
    return exp(-beats_since * rate);
}

// The previous frame at a scene coordinate (see `scene` for the coordinate system).
fn prev(uv: vec2f) -> vec4f {
    let m = music_u;
    let t = vec2f(uv.x / m.aspect, -uv.y) * 0.5 + 0.5;
    return textureSampleLevel(prev_frame, prev_sampler, t, 0.0);
}
"#;
    if m.is_compute() {
        s += r#"
// Accumulation grid: half the render resolution, 4 channels (r, g, b, density) of 8.8 fixed point.
fn accum_dims() -> vec2<u32> {
    return vec2<u32>(ceil(vec2f(music_u.res_x, music_u.res_y) * 0.5));
}

fn accum_cell(uv: vec2f) -> i32 {
    let d = accum_dims();
    let t = vec2f(uv.x / music_u.aspect, -uv.y) * 0.5 + 0.5;
    if (any(t < vec2f(0.0)) || any(t >= vec2f(1.0))) {
        return -1;
    }
    let c = vec2<u32>(t * vec2f(d));
    return i32((c.y * d.x + c.x) * 4u);
}

// Adds a point of `color` at a scene coordinate (from `simulate`).
fn splat(uv: vec2f, color: vec3f) {
    let i = accum_cell(uv);
    if (i < 0) {
        return;
    }
    let c = vec3<u32>(clamp(color, vec3f(0.0), vec3f(16.0)) * 256.0);
    atomicAdd(&accum[i], c.x);
    atomicAdd(&accum[i + 1], c.y);
    atomicAdd(&accum[i + 2], c.z);
    atomicAdd(&accum[i + 3], 256u);
}

// Accumulated (r, g, b, hits) at a scene coordinate (from `scene`).
fn accum_at(uv: vec2f) -> vec4f {
    let i = accum_cell(uv);
    if (i < 0) {
        return vec4f(0.0);
    }
    return vec4f(
        f32(atomicLoad(&accum[i])),
        f32(atomicLoad(&accum[i + 1])),
        f32(atomicLoad(&accum[i + 2])),
        f32(atomicLoad(&accum[i + 3])),
    ) / 256.0;
}
"#;
    }
    s
}

fn entry_points(m: &SceneManifest) -> String {
    let mut s = String::from(
        r#"
// ---- entry points (generated) ----
struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
    var o: VsOut;
    o.pos = vec4f(p, 0.0, 1.0);
    o.uv = vec2f(p.x * music_u.aspect, p.y);
    return o;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4f {
    return scene(v.uv, load_music(), load_params());
}
"#,
    );
    if m.is_compute() {
        s += r#"
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    simulate(id.x, load_music(), load_params());
}
"#;
    }
    s
}

/// Builds the module: bindings, prelude files (name, text) in the given order, the author's
/// code, then entry points. The author writes
/// `fn scene(uv: vec2f, m: Music, p: Params) -> vec4f` where `uv` is centered with y in −1..1
/// (x scaled by the aspect ratio), and for compute scenes also
/// `fn simulate(i: u32, m: Music, p: Params)`, called once per invocation.
pub fn assemble(
    m: &SceneManifest,
    prelude: &[(String, String)],
    scene_file: &str,
    scene_src: &str,
) -> Assembled {
    let mut source = String::new();
    let mut segments = Vec::new();
    let mut line = 1;
    let mut push = |source: &mut String, text: &str, file: Option<&str>| {
        let text = if text.ends_with('\n') {
            text.to_string()
        } else {
            format!("{text}\n")
        };
        let lines = text.lines().count();
        if let Some(file) = file {
            segments.push(Segment {
                file: file.into(),
                start: line,
                lines,
            });
        }
        line += lines;
        source.push_str(&text);
    };
    push(&mut source, &bindings(m), None);
    for (name, text) in prelude {
        push(&mut source, text, Some(&format!("prelude/{name}")));
    }
    push(&mut source, scene_src, Some(scene_file));
    push(&mut source, &entry_points(m), None);
    Assembled { source, segments }
}

/// Parses and validates with naga; errors point at the file and line they came from.
pub fn validate(a: &Assembled) -> Result<naga::Module, ShaderError> {
    let located = |loc: Option<naga::SourceLocation>, message: String| {
        let (file, line) = match loc {
            Some(l) => {
                let (f, l) = a.locate(l.line_number as usize);
                (f, Some(l))
            }
            None => ("<scene>".into(), None),
        };
        ShaderError {
            file,
            line,
            message,
        }
    };
    let module = naga::front::wgsl::parse_str(&a.source).map_err(|e| {
        let msg = e
            .labels()
            .filter(|(_, l)| !l.is_empty())
            .map(|(_, l)| l.to_string())
            .next();
        let message = match msg {
            Some(l) if l != e.message() => format!("{} ({l})", e.message()),
            _ => e.message().to_string(),
        };
        located(e.location(&a.source), message)
    })?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .map_err(|e| {
        let mut message = e.as_inner().to_string();
        let mut source: Option<&dyn std::error::Error> = std::error::Error::source(e.as_inner());
        while let Some(s) = source {
            message += &format!(": {s}");
            source = s.source();
        }
        located(e.location(&a.source), message)
    })?;
    if !module
        .functions
        .iter()
        .any(|(_, f)| f.name.as_deref() == Some("scene"))
    {
        return Err(ShaderError {
            file: "<scene>".into(),
            line: None,
            message: "missing fn scene".into(),
        });
    }
    Ok(module)
}

/// Packs the params uniform: one vec4 per declared param, in manifest order.
pub fn pack_params(values: &[[f32; 4]], slots: usize) -> Vec<f32> {
    let mut out = vec![0.0; slots * 4];
    for (i, v) in values.iter().take(slots).enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(v);
    }
    out
}

/// Ensures a kind we generate for has the functions it needs (used by tests and the loader).
pub fn required_functions(kind: SceneKind) -> &'static [&'static str] {
    match kind {
        SceneKind::Fragment => &["scene"],
        SceneKind::Compute { .. } => &["scene", "simulate"],
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn bundled_prelude() -> Vec<(String, String)> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/prelude");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        files
            .into_iter()
            .map(|p| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read_to_string(&p).unwrap(),
                )
            })
            .collect()
    }

    fn manifest(kind: &str) -> SceneManifest {
        SceneManifest::parse(&format!(
            r#"(name: "t", kind: {kind}, params: [
                (name: "zoom", type: F32, default: [1.0], min: [0.1], max: [4.0]),
                (name: "c", type: Vec2, default: [0.0, 0.0], min: [-1, -1], max: [1, 1]),
                (name: "tint", type: Color, default: [1, 1, 1], min: [0, 0, 0], max: [1, 1, 1]),
                (name: "iters", type: Int, default: [64], min: [8], max: [256], mutable: false),
            ])"#
        ))
        .unwrap()
    }

    const SCENE: &str = "fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    var z = uv / p.zoom;
    for (var i = 0; i < p.iters; i++) {
        z = csq(z) + p.c;
    }
    let k = pulse(m.kick, 4.0);
    let col = palette(length(z) + m.beat, vec3f(0.5), vec3f(0.5), vec3f(1.0), vec3f(0.0, 0.33, 0.67));
    return vec4f(col * p.tint * (0.5 + k) + prev(uv).rgb * 0.1 + band(m, 3) + fbm(uv, 3), 1.0);
}
";

    #[test]
    fn author_writes_only_the_look() {
        let m = manifest("Fragment");
        let a = assemble(&m, &bundled_prelude(), "scene.wgsl", SCENE);
        let module = validate(&a).unwrap_or_else(|e| panic!("{e}\n{}", a.source));
        let eps: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(eps, ["vs_main", "fs_main"]);
    }

    #[test]
    fn compute_scene_gets_accumulation_helpers() {
        let m = manifest("Compute(invocations: 65536)");
        let src = "fn simulate(i: u32, m: Music, p: Params) {
    let r = hash22(vec2f(f32(i), m.beats));
    splat(r * 2.0 - 1.0, p.tint);
}
fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    let a = accum_at(uv);
    return vec4f(a.rgb / max(a.w, 1.0) * log(1.0 + a.w), 1.0);
}
";
        let a = assemble(&m, &bundled_prelude(), "scene.wgsl", src);
        let module = validate(&a).unwrap_or_else(|e| panic!("{e}"));
        assert!(module.entry_points.iter().any(|e| e.name == "cs_main"));
        assert_eq!(required_functions(m.kind), ["scene", "simulate"]);
    }

    #[test]
    fn syntax_errors_point_at_the_authors_line() {
        let m = manifest("Fragment");
        let broken = "fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {\n    let a = 1.0\n    return vec4f(a);\n}\n";
        let e = validate(&assemble(&m, &bundled_prelude(), "scene.wgsl", broken)).unwrap_err();
        assert_eq!(e.file, "scene.wgsl", "{e}");
        assert!(matches!(e.line, Some(2..=3)), "{e}");
    }

    #[test]
    fn type_errors_point_at_the_authors_line() {
        let m = manifest("Fragment");
        let bad = "fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {\n    let x: f32 = p.c;\n    return vec4f(x);\n}\n";
        let e = validate(&assemble(&m, &bundled_prelude(), "scene.wgsl", bad)).unwrap_err();
        assert_eq!((e.file.as_str(), e.line), ("scene.wgsl", Some(2)), "{e}");
        let unknown =
            "fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {\n    return vec4f(p.nope);\n}\n";
        let e = validate(&assemble(&m, &bundled_prelude(), "scene.wgsl", unknown)).unwrap_err();
        assert_eq!((e.file.as_str(), e.line), ("scene.wgsl", Some(2)), "{e}");
    }

    #[test]
    fn missing_scene_function_is_an_error() {
        let m = manifest("Fragment");
        let e = validate(&assemble(
            &m,
            &bundled_prelude(),
            "scene.wgsl",
            "fn other() {}\n",
        ))
        .unwrap_err();
        assert!(e.message.contains("scene"), "{e}");
    }

    #[test]
    fn scene_without_params_still_compiles() {
        let m = SceneManifest::parse("(name: \"bare\", kind: Fragment)").unwrap();
        let src =
            "fn scene(uv: vec2f, m: Music, p: Params) -> vec4f { return vec4f(uv, m.beat, 1.0); }";
        validate(&assemble(&m, &[], "scene.wgsl", src)).unwrap();
        assert_eq!(param_slots(&m), 1);
    }

    #[test]
    fn music_layout_is_uniform_safe() {
        assert_eq!(
            MUSIC_FIELDS.len() % 4,
            0,
            "scalars fill whole vec4s before the bands array"
        );
        assert_eq!(MUSIC_FLOATS, 52);
        assert_eq!(music_index("kick"), 18);
        let dup = MUSIC_FIELDS
            .iter()
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(dup.len(), MUSIC_FIELDS.len());
        assert_eq!(
            pack_params(&[[1.0, 2.0, 0.0, 0.0]], 2),
            [1.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
    }
}
