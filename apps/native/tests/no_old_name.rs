//! The repo holds no other product's name: the player that inspired Diggr's look is described,
//! never named (`app-identity`). Every tracked text file is checked, in any letter case. The
//! name is put together here from two halves, so this file passes its own check.

use std::path::Path;
use std::process::Command;

#[test]
fn no_tracked_file_names_the_old_player() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(out) = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(&root)
        .output()
    else {
        eprintln!("skipped: git is not available");
        return;
    };
    if !out.status.success() {
        eprintln!("skipped: not a git checkout");
        return;
    }
    let name = ["win", "amp"].concat();
    let mut found = Vec::new();
    for file in out.stdout.split(|&b| b == 0).filter(|f| !f.is_empty()) {
        let file = String::from_utf8_lossy(file);
        // Binary files (images, audio fixtures) can't spell it; a deleted one isn't there.
        let Ok(text) = std::fs::read_to_string(root.join(&*file)) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(&name) {
                found.push(format!("{file}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the old name is still in the repo:\n{}",
        found.join("\n")
    );
}
