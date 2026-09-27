//! `scene.ron`: what a scene declares about itself — parameters, macro mappings, routes,
//! feedback — and the parameter slots the modulation and uniform layout are built from.

use serde::{Deserialize, Serialize};

use crate::modulation::{Macro, MacroMap, Route, Slot, Target, resolve};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamType {
    F32,
    Vec2,
    Vec3,
    /// A vec3 shown as a color on the deck.
    Color,
    /// Stored as f32, exposed to WGSL as `i32` (rounded).
    Int,
}

impl ParamType {
    pub fn components(self) -> usize {
        match self {
            ParamType::F32 | ParamType::Int => 1,
            ParamType::Vec2 => 2,
            ParamType::Vec3 | ParamType::Color => 3,
        }
    }

    pub fn wgsl(self) -> &'static str {
        match self {
            ParamType::F32 => "f32",
            ParamType::Int => "i32",
            ParamType::Vec2 => "vec2<f32>",
            ParamType::Vec3 | ParamType::Color => "vec3<f32>",
        }
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ParamType,
    pub default: Vec<f32>,
    pub min: Vec<f32>,
    pub max: Vec<f32>,
    /// Whether mutation and the deck may move it (iteration caps are usually not).
    #[serde(default = "yes")]
    pub mutable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SceneKind {
    Fragment,
    /// `fn simulate` runs over `invocations` threads each frame, splatting into an
    /// accumulation buffer the scene's `fn scene` then reads.
    Compute {
        invocations: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Feedback {
    /// How much of the previous frame survives per frame at 60 fps (0..1).
    pub decay: f32,
    /// Strength of the default zoom/rotate warp applied to the previous frame.
    pub warp: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneManifest {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub kind: SceneKind,
    #[serde(default)]
    pub params: Vec<ParamDef>,
    #[serde(default)]
    pub macros: Vec<MacroMap>,
    #[serde(default)]
    pub routes: Vec<Route>,
    /// Routes mutation may switch on (big mutate) — not active by default.
    #[serde(default)]
    pub optional_routes: Vec<Route>,
    #[serde(default)]
    pub feedback: Option<Feedback>,
    /// Upper bound for the render scale (heavy raymarchers can cap themselves below 1).
    #[serde(default)]
    pub max_scale: Option<f32>,
}

/// A manifest problem, with a line when it comes from the RON parser.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestError {
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {l}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

fn err(message: String) -> ManifestError {
    ManifestError {
        line: None,
        message,
    }
}

fn valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl SceneManifest {
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let m: Self = ron::from_str(text).map_err(|e| ManifestError {
            line: Some(e.span.start.line),
            message: e.code.to_string(),
        })?;
        m.check()?;
        Ok(m)
    }

    fn check(&self) -> Result<(), ManifestError> {
        let mut seen = std::collections::HashSet::new();
        for p in &self.params {
            if !valid_ident(&p.name) {
                return Err(err(format!(
                    "param name `{}` is not a WGSL identifier",
                    p.name
                )));
            }
            if !seen.insert(&p.name) {
                return Err(err(format!("param `{}` declared twice", p.name)));
            }
            let n = p.ty.components();
            for (what, v) in [("default", &p.default), ("min", &p.min), ("max", &p.max)] {
                if v.len() != n {
                    return Err(err(format!(
                        "param `{}`: {what} needs {n} value(s), has {}",
                        p.name,
                        v.len()
                    )));
                }
            }
            if (0..n).any(|c| p.min[c] > p.max[c]) {
                return Err(err(format!("param `{}`: min above max", p.name)));
            }
        }
        let slots = self.slots();
        for target in self.macros.iter().map(|m| &m.param).chain(
            self.routes
                .iter()
                .chain(&self.optional_routes)
                .map(|r| &r.target),
        ) {
            if resolve(&slots, target).is_none() {
                return Err(err(format!("unknown target `{target}`")));
            }
        }
        Ok(())
    }

    pub fn slots(&self) -> Vec<Slot> {
        self.params
            .iter()
            .map(|p| {
                let mut min = [0.0; 4];
                let mut max = [0.0; 4];
                min[..p.min.len()].copy_from_slice(&p.min);
                max[..p.max.len()].copy_from_slice(&p.max);
                Slot {
                    name: p.name.clone(),
                    components: p.ty.components(),
                    min,
                    max,
                }
            })
            .collect()
    }

    pub fn defaults(&self) -> Vec<[f32; 4]> {
        self.params
            .iter()
            .map(|p| {
                let mut v = [0.0; 4];
                v[..p.default.len()].copy_from_slice(&p.default);
                v
            })
            .collect()
    }

    /// Macro mappings resolved against the params (checked at parse time).
    pub fn macro_targets(&self) -> Vec<(Target, Macro, f32)> {
        let slots = self.slots();
        self.macros
            .iter()
            .filter_map(|m| Some((resolve(&slots, &m.param)?, m.which, m.amount)))
            .collect()
    }

    pub fn param_index(&self, name: &str) -> Option<usize> {
        self.params.iter().position(|p| p.name == name)
    }

    pub fn is_compute(&self) -> bool {
        matches!(self.kind, SceneKind::Compute { .. })
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JULIA: &str = r#"(
        name: "Julia Tunnel",
        tags: ["2d"],
        kind: Fragment,
        params: [
            (name: "zoom", type: F32, default: [1.0], min: [0.2], max: [4.0]),
            (name: "julia_c", type: Vec2, default: [-0.8, 0.156], min: [-1.0, -1.0], max: [1.0, 1.0]),
            (name: "tint", type: Color, default: [0.2, 0.6, 1.0], min: [0, 0, 0], max: [1, 1, 1]),
            (name: "iters", type: Int, default: [96], min: [16], max: [256], mutable: false),
        ],
        macros: [(macro: Chaos, param: "julia_c", amount: 0.1)],
        routes: [(source: Kick, shapers: [Envelope(attack: Ms(5), release: Ms(120)), Range(0, 0.35)], target: "zoom")],
        feedback: Some((decay: 0.8, warp: 0.1)),
    )"#;

    #[test]
    fn parses_a_full_manifest() {
        let m = SceneManifest::parse(JULIA).unwrap();
        assert_eq!(m.params.len(), 4);
        assert!(m.params[0].mutable && !m.params[3].mutable);
        assert_eq!(m.slots()[1].components, 2);
        assert_eq!(m.defaults()[2], [0.2, 0.6, 1.0, 0.0]);
        assert_eq!(m.macro_targets().len(), 1);
        assert_eq!(
            m.feedback,
            Some(Feedback {
                decay: 0.8,
                warp: 0.1
            })
        );
        let c = SceneManifest::parse("(name: \"F\", kind: Compute(invocations: 65536))").unwrap();
        assert!(c.is_compute());
    }

    #[test]
    fn reports_errors_with_lines_and_meaning() {
        let e = SceneManifest::parse("(\n name: \"x\",\n kind: Fragmnt,\n)").unwrap_err();
        assert_eq!(e.line, Some(3), "{e}");
        let bad = |params: &str| {
            SceneManifest::parse(&format!(
                "(name: \"x\", kind: Fragment, params: [{params}])"
            ))
            .unwrap_err()
            .message
        };
        assert!(
            bad("(name: \"z\", type: Vec2, default: [1], min: [0, 0], max: [1, 1])")
                .contains("default needs 2")
        );
        assert!(
            bad("(name: \"2z\", type: F32, default: [1], min: [0], max: [1])")
                .contains("identifier")
        );
        assert!(
            bad("(name: \"z\", type: F32, default: [1], min: [2], max: [1])")
                .contains("min above max")
        );
        let e = SceneManifest::parse(
            "(name: \"x\", kind: Fragment, routes: [(source: Kick, target: \"nope\")])",
        )
        .unwrap_err();
        assert!(e.message.contains("nope"));
    }
}
