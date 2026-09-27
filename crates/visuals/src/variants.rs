//! Variants: saved parameter sets ("genomes") for a scene, with lineage, mutation and ratings.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::manifest::{ParamType, SceneManifest};

/// Small deterministic RNG (splitmix64).
#[derive(Debug, Clone)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    /// Standard normal (Box–Muller).
    pub fn gauss(&mut self) -> f32 {
        let u = self.f32().max(1e-7);
        let v = self.f32();
        (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub scene: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub seed: u64,
    /// Values by param name. Unknown names are ignored and missing ones take the default,
    /// so variants survive edits to the scene's manifest.
    #[serde(default)]
    pub params: BTreeMap<String, Vec<f32>>,
    /// Indexes into the scene's `optional_routes` that this variant switches on.
    #[serde(default)]
    pub optional_routes: Vec<usize>,
    /// Indexes into the scene's `routes` that this variant switches off.
    #[serde(default)]
    pub routes_off: Vec<usize>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 0 = unrated, 1..=5.
    #[serde(default)]
    pub rating: u8,
}

impl Variant {
    /// The scene's defaults as a variant.
    pub fn defaults(scene: &str, m: &SceneManifest) -> Self {
        Self {
            scene: scene.into(),
            name: "default".into(),
            parent: None,
            seed: 0,
            params: m
                .params
                .iter()
                .map(|p| (p.name.clone(), p.default.clone()))
                .collect(),
            optional_routes: Vec::new(),
            routes_off: Vec::new(),
            tags: Vec::new(),
            rating: 0,
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("line {}: {}", e.span.start.line, e.code))
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("variants serialize")
    }

    /// Base values for every manifest param (tolerant of added/removed/retyped params).
    pub fn values(&self, m: &SceneManifest) -> Vec<[f32; 4]> {
        m.params
            .iter()
            .map(|p| {
                let src = self
                    .params
                    .get(&p.name)
                    .filter(|v| v.len() == p.ty.components())
                    .unwrap_or(&p.default);
                let mut v = [0.0; 4];
                for (c, x) in src.iter().enumerate() {
                    v[c] = x.clamp(p.min[c], p.max[c]);
                }
                v
            })
            .collect()
    }

    /// Stores `values` (one per manifest param) into this variant.
    pub fn set_values(&mut self, m: &SceneManifest, values: &[[f32; 4]]) {
        for (p, v) in m.params.iter().zip(values) {
            self.params
                .insert(p.name.clone(), v[..p.ty.components()].to_vec());
        }
    }

    /// A mutated child: each mutable param moves by a Gaussian step of `sigma` in its normalized
    /// range. `big` also switches one optional route on or off, or one route off.
    pub fn mutate(&self, m: &SceneManifest, sigma: f32, big: bool, rng: &mut Rng) -> Variant {
        let mut child = self.clone();
        child.parent = Some(self.name.clone());
        child.seed = rng.next_u64();
        child.rating = 0;
        child.name = auto_name(rng);
        let mut values = self.values(m);
        for (p, v) in m.params.iter().zip(values.iter_mut()) {
            if !p.mutable {
                continue;
            }
            for (c, x) in v.iter_mut().enumerate().take(p.ty.components()) {
                let span = p.max[c] - p.min[c];
                if span <= 0.0 {
                    continue;
                }
                let n = ((*x - p.min[c]) / span + sigma * rng.gauss()).clamp(0.0, 1.0);
                *x = p.min[c] + n * span;
                if p.ty == ParamType::Int {
                    *x = x.round();
                }
            }
        }
        child.set_values(m, &values);
        if big {
            let pool = m.optional_routes.len() + m.routes.len();
            if pool > 0 {
                let k = rng.below(pool);
                let (list, i) = if k < m.optional_routes.len() {
                    (&mut child.optional_routes, k)
                } else {
                    (&mut child.routes_off, k - m.optional_routes.len())
                };
                match list.iter().position(|&x| x == i) {
                    Some(at) => {
                        list.remove(at);
                    }
                    None => list.push(i),
                }
            }
        }
        child
    }

    /// The active routes: the scene's routes minus those switched off, plus switched-on optionals.
    pub fn routes<'a>(&self, m: &'a SceneManifest) -> Vec<&'a crate::modulation::Route> {
        let base = m
            .routes
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.routes_off.contains(i))
            .map(|(_, r)| r);
        let extra = self
            .optional_routes
            .iter()
            .filter_map(|&i| m.optional_routes.get(i));
        base.chain(extra).collect()
    }
}

const ADJECTIVES: [&str; 32] = [
    "amber", "bright", "cosmic", "deep", "electric", "feral", "gilded", "hollow", "icy", "jagged",
    "kinetic", "lunar", "molten", "neon", "opal", "primal", "quiet", "radiant", "silken", "tidal",
    "ultra", "velvet", "wild", "xenon", "young", "zesty", "astral", "blazing", "crystal", "dusky",
    "ember", "fractal",
];

/// `<adjective>-<hex4>`.
pub fn auto_name(rng: &mut Rng) -> String {
    let n = rng.next_u64();
    format!(
        "{}-{:04x}",
        ADJECTIVES[(n % ADJECTIVES.len() as u64) as usize],
        (n >> 32) & 0xffff
    )
}

/// Mutation step sizes (normalized parameter space).
pub const SMALL_SIGMA: f32 = 0.08;
pub const BIG_SIGMA: f32 = 0.25;

/// Undo stack of looks visited this session.
#[derive(Debug, Default)]
pub struct History {
    stack: Vec<Variant>,
}

impl History {
    const LIMIT: usize = 64;

    pub fn push(&mut self, v: Variant) {
        self.stack.push(v);
        if self.stack.len() > Self::LIMIT {
            self.stack.remove(0);
        }
    }

    pub fn pop(&mut self) -> Option<Variant> {
        self.stack.pop()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> SceneManifest {
        SceneManifest::parse(
            r#"(name: "t", kind: Fragment,
                params: [
                    (name: "zoom", type: F32, default: [1.0], min: [0.0], max: [4.0]),
                    (name: "c", type: Vec2, default: [0.1, 0.2], min: [-1, -1], max: [1, 1]),
                    (name: "iters", type: Int, default: [64], min: [8], max: [256], mutable: false),
                ],
                routes: [(source: Kick, target: "zoom")],
                optional_routes: [(source: Snare, target: "c.x")],
            )"#,
        )
        .unwrap()
    }

    #[test]
    fn tolerant_of_manifest_changes() {
        let m = manifest();
        let v =
            Variant::parse(r#"(scene: "t", params: {"zoom": [2.5], "gone": [1.0], "c": [9.0]})"#)
                .unwrap();
        // zoom kept; unknown "gone" ignored; "c" has the wrong arity → default; iters missing → default.
        assert_eq!(
            v.values(&m),
            [
                [2.5, 0.0, 0.0, 0.0],
                [0.1, 0.2, 0.0, 0.0],
                [64.0, 0.0, 0.0, 0.0]
            ]
        );
        let clamped = Variant::parse(r#"(scene: "t", params: {"zoom": [99.0]})"#).unwrap();
        assert_eq!(clamped.values(&m)[0][0], 4.0);
    }

    #[test]
    fn save_and_load_round_trip() {
        let m = manifest();
        let mut v = Variant::defaults("t", &m);
        v.rating = 4;
        v.tags.push("calm".into());
        let back = Variant::parse(&v.to_ron()).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn mutation_is_seeded_bounded_and_keeps_lineage() {
        let m = manifest();
        let parent = Variant {
            name: "root".into(),
            rating: 5,
            ..Variant::defaults("t", &m)
        };
        let a = parent.mutate(&m, SMALL_SIGMA, false, &mut Rng(7));
        let b = parent.mutate(&m, SMALL_SIGMA, false, &mut Rng(7));
        assert_eq!(a, b, "same seed, same child");
        assert_eq!(a.parent.as_deref(), Some("root"));
        assert_eq!(a.rating, 0);
        assert!(a.name.contains('-') && a.name.len() > 5);
        let (pv, av) = (parent.values(&m), a.values(&m));
        assert_ne!(pv[0], av[0], "mutable params move");
        assert_eq!(pv[2], av[2], "immutable params stay");
        // Small steps stay small on average; everything stays in range.
        let mut rng = Rng(1);
        let mut total = 0.0;
        for _ in 0..500 {
            let c = parent.mutate(&m, SMALL_SIGMA, false, &mut rng).values(&m);
            assert!((0.0..=4.0).contains(&c[0][0]) && (-1.0..=1.0).contains(&c[1][0]));
            total += (c[0][0] - 1.0).abs() / 4.0;
        }
        let mean = total / 500.0;
        assert!(mean > 0.04 && mean < 0.09, "{mean}");
    }

    #[test]
    fn big_mutation_toggles_a_route() {
        let m = manifest();
        let parent = Variant::defaults("t", &m);
        assert_eq!(parent.routes(&m).len(), 1);
        let mut rng = Rng(3);
        let changed = (0..20)
            .map(|_| parent.mutate(&m, BIG_SIGMA, true, &mut rng))
            .filter(|c| c.routes(&m).len() != 1)
            .count();
        assert_eq!(changed, 20, "each big mutation switches exactly one route");
    }

    #[test]
    fn history_undo() {
        let mut h = History::default();
        let m = manifest();
        for i in 0..70 {
            h.push(Variant {
                name: format!("v{i}"),
                ..Variant::defaults("t", &m)
            });
        }
        assert_eq!(h.len(), 64);
        assert_eq!(h.pop().unwrap().name, "v69");
    }
}
