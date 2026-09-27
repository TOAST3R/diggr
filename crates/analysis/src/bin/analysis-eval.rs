//! Scores the analyzer against your annotations.
//!
//! ```text
//! cargo run --release -p analysis --bin analysis-eval [ANNOTATIONS_DIR]
//! ```
//! The default directory is `<cache>/annotations` (where the player saves them in annotation
//! mode). For each annotated track it prints the beat F-measure (±70 ms, within the tapped
//! stretch) and the share of marked boundaries found within ±1 bar.

use std::path::PathBuf;

use analysis::cache::ScoreCache;
use analysis::eval::{Annotations, analyze_file, evaluate};
use platform::TrackRef;
use platform::native::NativeFileSource;

fn main() -> std::process::ExitCode {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| ScoreCache::platform_default().map(|c| c.dir().join("annotations")));
    let Some(dir) = dir else {
        eprintln!("no annotations directory");
        return std::process::ExitCode::FAILURE;
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        eprintln!(
            "no annotations in {} (record some in fullscreen with A)",
            dir.display()
        );
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "{:<40} {:>7} {:>9} {:>6} {:>6}",
        "track", "beat F", "bounds", "taps", "marks"
    );
    let (mut f_sum, mut f_n, mut hit_sum, mut hit_n) = (0.0, 0usize, 0.0, 0usize);
    for path in files {
        let Some(ann) = Annotations::load(&path) else {
            eprintln!("skipping unreadable {}", path.display());
            continue;
        };
        let name = TrackRef::new(ann.track.clone()).file_name().to_owned();
        let Some(score) = analyze_file(&NativeFileSource, &TrackRef::new(ann.track.clone())) else {
            println!("{name:<40} cannot open the track");
            continue;
        };
        let e = evaluate(&score, &ann);
        let pct = |v: Option<f64>| v.map_or("-".to_owned(), |x| format!("{:.1}%", x * 100.0));
        println!(
            "{:<40} {:>7} {:>9} {:>6} {:>6}",
            truncate(&name, 40),
            pct(e.beat_f),
            pct(e.boundary_hits),
            e.taps,
            e.boundaries
        );
        if let Some(f) = e.beat_f {
            f_sum += f;
            f_n += 1;
        }
        if let Some(h) = e.boundary_hits {
            hit_sum += h * e.boundaries as f64;
            hit_n += e.boundaries;
        }
    }
    let avg = |s: f64, n: usize| {
        if n == 0 {
            "-".to_owned()
        } else {
            format!("{:.1}%", s / n as f64 * 100.0)
        }
    };
    println!(
        "{:<40} {:>7} {:>9}   (targets: beat F ≥ 90%, boundaries ≥ 70%)",
        "ALL",
        avg(f_sum, f_n),
        avg(hit_sum, hit_n)
    );
    std::process::ExitCode::SUCCESS
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_owned()
    } else {
        s.chars().take(n - 1).chain(['…']).collect()
    }
}
