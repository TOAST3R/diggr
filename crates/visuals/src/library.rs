//! Asset folder `visuals/{prelude,scenes,variants,director.ron}`: bundled copies installed into
//! the config dir for editing, loading with errors that name the file and line, and a notify
//! watcher that reports which parts changed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::codegen::{Assembled, assemble, required_functions, validate};
use crate::director::Rules;
use crate::manifest::SceneManifest;
use crate::variants::Variant;

macro_rules! bundled {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../assets/", $path)))),*]
    };
}

/// Files shipped inside the binary, as (relative path, contents).
pub const BUNDLED: &[(&str, &str)] = bundled![
    "director.ron",
    "prelude/color.wgsl",
    "prelude/complex.wgsl",
    "prelude/noise.wgsl",
    "prelude/space.wgsl",
    "prelude/tunnel.wgsl",
    "scenes/julia_tunnel/scene.ron",
    "scenes/julia_tunnel/scene.wgsl",
    "scenes/liquid_feedback/scene.ron",
    "scenes/liquid_feedback/scene.wgsl",
    "scenes/kifs_cathedral/scene.ron",
    "scenes/kifs_cathedral/scene.wgsl",
    "scenes/flame/scene.ron",
    "scenes/flame/scene.wgsl",
    "scenes/polar_life/scene.ron",
    "scenes/polar_life/scene.wgsl",
    "scenes/coral_tunnel/scene.ron",
    "scenes/coral_tunnel/scene.wgsl",
    "variants/julia_tunnel/deep-dive.ron",
    "variants/julia_tunnel/solar.ron",
    "variants/liquid_feedback/ink.ron",
    "variants/liquid_feedback/plasma.ron",
    "variants/kifs_cathedral/nave.ron",
    "variants/kifs_cathedral/crypt.ron",
    "variants/flame/silk.ron",
    "variants/flame/nebula.ron",
    "variants/polar_life/drift.ron",
    "variants/polar_life/bloom.ron",
    "variants/polar_life/hyperdrive.ron",
    "variants/coral_tunnel/reef.ron",
    "variants/coral_tunnel/mitosis.ron",
];

/// Something that failed to load, with where.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadError {
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{l}: {}", self.file, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}

/// `<config>/winamp_rust/visuals`, or a temp folder when there is no config dir.
pub fn default_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("winamp_rust")
        .join("visuals")
}

/// Writes bundled files that are missing; never overwrites the user's edits.
pub fn install(dir: &Path) -> std::io::Result<usize> {
    let mut written = 0;
    for (rel, text) in BUNDLED {
        let path = dir.join(rel);
        if !path.exists() {
            std::fs::create_dir_all(path.parent().expect("has parent"))?;
            std::fs::write(&path, text)?;
            written += 1;
        }
    }
    Ok(written)
}

fn read(path: &Path, rel: &str) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|e| LoadError {
        file: rel.into(),
        line: None,
        message: e.to_string(),
    })
}

/// The prelude files, sorted by name.
pub fn load_prelude(dir: &Path) -> Vec<(String, String)> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("prelude"))
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "wgsl"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            Some((
                p.file_name()?.to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).ok()?,
            ))
        })
        .collect()
}

/// Scene folders that contain a manifest, sorted.
pub fn scene_ids(dir: &Path) -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(dir.join("scenes"))
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter(|e| e.path().join("scene.ron").exists())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

/// A scene that parsed and validated.
#[derive(Debug, Clone)]
pub struct SceneSource {
    pub id: String,
    pub manifest: SceneManifest,
    pub assembled: Assembled,
}

pub fn load_scene(
    dir: &Path,
    id: &str,
    prelude: &[(String, String)],
) -> Result<SceneSource, LoadError> {
    let ron_rel = format!("scenes/{id}/scene.ron");
    let wgsl_rel = format!("scenes/{id}/scene.wgsl");
    let manifest =
        SceneManifest::parse(&read(&dir.join(&ron_rel), &ron_rel)?).map_err(|e| LoadError {
            file: ron_rel.clone(),
            line: e.line,
            message: e.message,
        })?;
    let src = read(&dir.join(&wgsl_rel), &wgsl_rel)?;
    // Before validating: a missing entry function would otherwise surface as an error in the
    // generated code.
    for f in required_functions(manifest.kind) {
        if *f != "scene" && !src.contains(&format!("fn {f}")) {
            return Err(LoadError {
                file: wgsl_rel,
                line: None,
                message: format!("{} scenes need fn {f}", manifest.kind.name()),
            });
        }
    }
    let assembled = assemble(&manifest, prelude, &wgsl_rel, &src);
    validate(&assembled).map_err(|e| LoadError {
        file: e.file,
        line: e.line,
        message: e.message,
    })?;
    Ok(SceneSource {
        id: id.into(),
        manifest,
        assembled,
    })
}

/// The scene's saved variants (sorted by name). Broken files are reported and skipped.
pub fn load_variants(dir: &Path, scene: &str) -> (Vec<Variant>, Vec<LoadError>) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("variants").join(scene))
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "ron"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for p in files {
        let stem = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let rel = format!("variants/{scene}/{stem}.ron");
        match read(&p, &rel).and_then(|t| {
            Variant::parse(&t).map_err(|message| LoadError {
                file: rel.clone(),
                line: None,
                message,
            })
        }) {
            Ok(mut v) => {
                v.scene = scene.into();
                if v.name.is_empty() {
                    v.name = stem;
                }
                out.push(v);
            }
            Err(e) => errors.push(e),
        }
    }
    (out, errors)
}

pub fn save_variant(dir: &Path, v: &Variant) -> std::io::Result<()> {
    let path = dir
        .join("variants")
        .join(&v.scene)
        .join(format!("{}.ron", v.name));
    std::fs::create_dir_all(path.parent().expect("has parent"))?;
    std::fs::write(path, v.to_ron())
}

pub fn load_rules(dir: &Path) -> Result<Rules, LoadError> {
    let text = read(&dir.join("director.ron"), "director.ron")?;
    Rules::parse(&text).map_err(|message| LoadError {
        file: "director.ron".into(),
        line: None,
        message,
    })
}

/// What changed on disk.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Changes {
    /// The prelude changed: every scene must be rebuilt.
    pub prelude: bool,
    pub scenes: BTreeSet<String>,
    pub variants: BTreeSet<String>,
    pub director: bool,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        *self == Changes::default()
    }

    fn add(&mut self, dir: &Path, path: &Path) {
        let Ok(rel) = path.strip_prefix(dir) else {
            return;
        };
        let parts: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        match parts.as_slice() {
            [p, ..] if p == "prelude" => self.prelude = true,
            [s, id, ..] if s == "scenes" => {
                self.scenes.insert(id.clone());
            }
            [v, id, ..] if v == "variants" => {
                self.variants.insert(id.clone());
            }
            [d] if d == "director.ron" => self.director = true,
            _ => {}
        }
    }
}

/// Watches the asset folder; changes are delivered once the files have been quiet briefly
/// (editors often write in several steps).
pub struct Watcher {
    dir: PathBuf,
    _watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<PathBuf>,
    pending: Changes,
    last_event: Option<Instant>,
}

const QUIET: Duration = Duration::from_millis(60);

impl Watcher {
    pub fn new(dir: &Path) -> notify::Result<Self> {
        use notify::Watcher as _;
        let (tx, rx) = mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(ev) = res
                    && !ev.kind.is_access()
                {
                    for p in ev.paths {
                        let _ = tx.send(p);
                    }
                }
            })?;
        watcher.watch(dir, notify::RecursiveMode::Recursive)?;
        // Paths from FSEvents are canonical; compare against the canonical dir.
        let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        Ok(Self {
            dir,
            _watcher: watcher,
            rx,
            pending: Changes::default(),
            last_event: None,
        })
    }

    /// Changes that have settled, if any (non-blocking; call once per frame).
    pub fn poll(&mut self) -> Option<Changes> {
        while let Ok(p) = self.rx.try_recv() {
            let p = p.canonicalize().unwrap_or(p);
            self.pending.add(&self.dir, &p);
            self.last_event = Some(Instant::now());
        }
        match self.last_event {
            Some(t) if t.elapsed() >= QUIET && !self.pending.is_empty() => {
                self.last_event = None;
                Some(std::mem::take(&mut self.pending))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("visuals-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn bundled_list_matches_the_assets_folder() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let mut on_disk = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    on_disk.push(
                        p.strip_prefix(&root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                    );
                }
            }
        }
        on_disk.sort();
        let mut listed: Vec<String> = BUNDLED.iter().map(|(p, _)| p.to_string()).collect();
        listed.sort();
        assert_eq!(listed, on_disk, "add new asset files to BUNDLED");
    }

    #[test]
    fn install_loads_everything_and_keeps_user_edits() {
        let dir = temp_dir("install");
        assert_eq!(install(&dir).unwrap(), BUNDLED.len());
        std::fs::write(dir.join("director.ron"), "(rules: [])").unwrap();
        assert_eq!(install(&dir).unwrap(), 0);
        assert!(load_rules(&dir).unwrap().rules.is_empty(), "user edit kept");

        let prelude = load_prelude(&dir);
        assert_eq!(prelude.len(), 5);
        let ids = scene_ids(&dir);
        assert_eq!(
            ids,
            [
                "coral_tunnel",
                "flame",
                "julia_tunnel",
                "kifs_cathedral",
                "liquid_feedback",
                "polar_life"
            ]
        );
        for id in &ids {
            let s = load_scene(&dir, id, &prelude).unwrap_or_else(|e| panic!("{e}"));
            let (vars, errs) = load_variants(&dir, id);
            assert!(errs.is_empty(), "{errs:?}");
            assert!(vars.len() >= 2, "{id}");
            for v in vars {
                // Bundled variants only use params their scene declares.
                for k in v.params.keys() {
                    assert!(
                        s.manifest.param_index(k).is_some(),
                        "{id}/{}: unknown param {k}",
                        v.name
                    );
                }
            }
        }
        assert!(
            ids.iter()
                .flat_map(|id| load_variants(&dir, id).0)
                .any(|v| v.tags.contains(&"calm".to_string())),
            "a calm variant exists for the default fall rule"
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn errors_name_file_and_line() {
        let dir = temp_dir("errors");
        install(&dir).unwrap();
        let prelude = load_prelude(&dir);
        let wgsl = dir.join("scenes/julia_tunnel/scene.wgsl");
        let good = std::fs::read_to_string(&wgsl).unwrap();
        std::fs::write(
            &wgsl,
            good.replacen("let r = length(uv);", "let r = length(uv)", 1),
        )
        .unwrap();
        let e = load_scene(&dir, "julia_tunnel", &prelude).unwrap_err();
        assert_eq!(e.file, "scenes/julia_tunnel/scene.wgsl");
        assert!(matches!(e.line, Some(3..=4)), "{e}");
        std::fs::write(
            dir.join("scenes/flame/scene.ron"),
            "(\n  name: \"x\",\n  kind: Nope,\n)",
        )
        .unwrap();
        let e = load_scene(&dir, "flame", &prelude).unwrap_err();
        assert_eq!(
            (e.file.as_str(), e.line),
            ("scenes/flame/scene.ron", Some(3)),
            "{e}"
        );
        // A missing entry function is reported against the author's file.
        let wgsl = dir.join("scenes/polar_life/scene.wgsl");
        let good = std::fs::read_to_string(&wgsl).unwrap();
        std::fs::write(&wgsl, good.replace("fn rule(", "fn evolve(")).unwrap();
        let e = load_scene(&dir, "polar_life", &prelude).unwrap_err();
        assert_eq!(e.file, "scenes/polar_life/scene.wgsl");
        assert!(e.message.contains("fn rule"), "{e}");
        std::fs::write(dir.join("variants/flame/bad.ron"), "(oops").unwrap();
        let (vars, errs) = load_variants(&dir, "flame");
        assert_eq!((vars.len(), errs.len()), (2, 1));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn variants_save_and_reload() {
        let dir = temp_dir("save");
        install(&dir).unwrap();
        let (mut vars, _) = load_variants(&dir, "flame");
        let mut v = vars.remove(0);
        v.name = "kept-0001".into();
        v.rating = 5;
        save_variant(&dir, &v).unwrap();
        let (vars, _) = load_variants(&dir, "flame");
        assert!(vars.iter().any(|x| x.name == "kept-0001" && x.rating == 5));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn classify_changes() {
        let dir = Path::new("/v");
        let mut c = Changes::default();
        c.add(dir, Path::new("/v/scenes/flame/scene.wgsl"));
        c.add(dir, Path::new("/v/variants/flame/silk.ron"));
        c.add(dir, Path::new("/v/director.ron"));
        c.add(dir, Path::new("/elsewhere/x"));
        assert!(!c.prelude && c.director);
        assert_eq!(c.scenes.iter().collect::<Vec<_>>(), ["flame"]);
        assert_eq!(c.variants.iter().collect::<Vec<_>>(), ["flame"]);
        c.add(dir, Path::new("/v/prelude/noise.wgsl"));
        assert!(c.prelude);
    }

    #[test]
    fn watcher_reports_edits() {
        let dir = temp_dir("watch");
        install(&dir).unwrap();
        let mut w = Watcher::new(&dir).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        while w.poll().is_some() {}
        std::fs::write(dir.join("scenes/flame/scene.wgsl"), BUNDLED[12].1).unwrap();
        let start = Instant::now();
        let mut got = None;
        while start.elapsed() < Duration::from_secs(5) {
            if let Some(c) = w.poll() {
                got = Some(c);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let c = got.expect("change seen within 5 s");
        assert!(c.scenes.contains("flame"), "{c:?}");
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "within the 500 ms budget: {:?}",
            start.elapsed()
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
