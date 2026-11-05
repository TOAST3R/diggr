//! Starting and stopping helper processes (yt-dlp) the same way on every system.
//!
//! On Windows a program without a window of its own, which the player is, makes Windows open a
//! console for each console program it starts, and previews start one constantly. The flag
//! that stops that lives here, so every start gets it; a test fails when code elsewhere in
//! this crate starts a process any other way.
//!
//! On macOS and Linux each helper leads a process group of its own, and [`stop`] kills the
//! whole group: yt-dlp's macOS and Linux builds are a launcher that starts Python as a second
//! process, and killing only the launcher left that one running (orphaned) at every time-out.

use std::ffi::OsStr;
use std::process::{Child, Command};

/// `CREATE_NO_WINDOW`: the child runs without a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A command for a helper process, which opens no window on Windows and leads its own process
/// group on Unix.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        c.process_group(0);
    }
    c
}

/// Stops a helper started with [`command`] and every process it started (on Unix, its
/// process group), and waits for it.
pub fn stop(child: &mut Child) {
    #[cfg(unix)]
    if matches!(child.try_wait(), Ok(None))
        && let Ok(pid) = libc::pid_t::try_from(child.id())
    {
        // SAFETY: `killpg` only sends a signal. The group is the child's own (`process_group(0)`),
        // and the child is still running (not reaped), so its id can't belong to anything else.
        unsafe {
            libc::killpg(pid, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
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

    /// Whether any process of group `pgid` is left, waiting up to 1 s for the system to reap them.
    #[cfg(unix)]
    fn group_gone(pgid: libc::pid_t) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            // SAFETY: signal 0 only checks that the group exists.
            if unsafe { libc::killpg(pgid, 0) } != 0 {
                return true;
            }
            if std::time::Instant::now() > deadline {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[cfg(unix)]
    #[test]
    fn stopping_a_helper_stops_what_it_started() {
        // A launcher that starts a second process and waits, as PyInstaller's does.
        let mut child = command("sh")
            .args(["-c", "sleep 30 & sleep 30"])
            .spawn()
            .unwrap();
        let pgid = libc::pid_t::try_from(child.id()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        // SAFETY: signal 0 only checks that the group exists.
        assert_eq!(unsafe { libc::killpg(pgid, 0) }, 0, "its own group");
        stop(&mut child);
        assert!(group_gone(pgid), "a process of the helper was left running");
    }

    #[test]
    fn stopping_a_helper_that_ended_is_harmless() {
        let mut child = command(std::env::current_exe().unwrap())
            .arg("--list")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        stop(&mut child);
        stop(&mut child);
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
