//! Starting helper processes (yt-dlp) the same way on every system.
//!
//! On Windows a program without a window of its own, which the player is, makes Windows open a
//! console for each console program it starts, and previews start one constantly. The flag
//! that stops that lives here, so every start gets it; a test fails when code elsewhere in
//! this crate starts a process any other way.

use std::ffi::OsStr;
use std::process::Command;

/// `CREATE_NO_WINDOW`: the child runs without a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A command for a helper process, which opens no window on Windows.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn a_helper_runs_and_its_output_is_read() {
        // The test binary itself, which every system has and which prints its tests for `--list`.
        let out = command(std::env::current_exe().unwrap())
            .arg("--list")
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("a_helper_runs_and_its_output_is_read")
        );
    }

    fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, found);
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }

    /// Only code before a file's `#[cfg(test)]` is checked, which is where every file here keeps
    /// its tests (the ones that run a fake yt-dlp may start it directly).
    #[test]
    fn no_other_code_starts_a_process_directly() {
        let needle = ["Command", "::new"].concat();
        let mut files = Vec::new();
        sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let offenders: Vec<_> = files
            .into_iter()
            .filter(|p| p.file_name().is_some_and(|n| n != "proc.rs"))
            .filter(|p| {
                let text = std::fs::read_to_string(p).unwrap();
                text.split("#[cfg(test)]").next().unwrap().contains(&needle)
            })
            .collect();
        assert!(
            offenders.is_empty(),
            "start helper processes with crate::proc::command, not directly: {offenders:?}"
        );
    }
}
