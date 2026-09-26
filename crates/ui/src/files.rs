//! Finding audio files (recursive folder adds, drag-and-drop) and M3U/M3U8 playlists.

use std::path::{Path, PathBuf};

/// Extensions the decoder handles.
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "oga", "m4a", "mp4", "aac"];

pub fn is_audio(path: &Path) -> bool {
    ext(path).is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.as_str()))
}

pub fn is_playlist(path: &Path) -> bool {
    ext(path).is_some_and(|e| e == "m3u" || e == "m3u8")
}

fn ext(path: &Path) -> Option<String> {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
}

/// Expands files and folders (recursively, sorted by name) into absolute audio file paths, so a
/// saved playlist works whatever directory the app starts in. M3U playlists are read and their
/// entries included; everything else (images, text, …) is ignored.
pub fn expand(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        let p = std::path::absolute(p).unwrap_or_else(|_| p.clone());
        collect(&p, &mut out, 0);
    }
    out
}

fn collect(path: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 32 {
        return; // symlink loops
    }
    if path.is_dir() {
        let Ok(read) = std::fs::read_dir(path) else {
            return;
        };
        let mut children: Vec<PathBuf> = read.filter_map(|e| e.ok().map(|e| e.path())).collect();
        children.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
        for c in children {
            if !c
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            {
                collect(&c, out, depth + 1);
            }
        }
    } else if is_audio(path) {
        out.push(path.to_path_buf());
    } else if is_playlist(path)
        && let Ok(text) = std::fs::read_to_string(path)
    {
        out.extend(parse_m3u(&text, path.parent()).into_iter().map(|e| e.path));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct M3uEntry {
    pub path: PathBuf,
    /// From `#EXTINF`, when present.
    pub title: Option<String>,
    pub duration: Option<f64>,
}

/// Parses M3U/M3U8 (extended or plain). Relative paths are resolved against `base`.
pub fn parse_m3u(text: &str, base: Option<&Path>) -> Vec<M3uEntry> {
    let mut out = Vec::new();
    let mut pending: Option<(Option<f64>, Option<String>)> = None;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            let (secs, title) = info.split_once(',').unwrap_or((info, ""));
            let secs = secs.trim().parse::<f64>().ok().filter(|s| *s >= 0.0);
            let title = (!title.trim().is_empty()).then(|| title.trim().to_owned());
            pending = Some((secs, title));
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let raw = line.strip_prefix("file://").unwrap_or(line);
        let path = PathBuf::from(raw);
        let path = match base {
            Some(b) if path.is_relative() => b.join(path),
            _ => path,
        };
        let (duration, title) = pending.take().unwrap_or((None, None));
        out.push(M3uEntry {
            path,
            title,
            duration,
        });
    }
    out
}

/// Writes an extended M3U (UTF-8, suitable for `.m3u8`).
pub fn write_m3u(entries: &[M3uEntry]) -> String {
    let mut s = String::from("#EXTM3U\n");
    for e in entries {
        let secs = e.duration.map_or(-1, |d| d.round() as i64);
        s.push_str(&format!(
            "#EXTINF:{secs},{}\n",
            e.title.as_deref().unwrap_or("")
        ));
        s.push_str(&e.path.to_string_lossy());
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ui-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn dropping_a_folder_adds_only_audio_recursively_in_order() {
        let d = tmp("scan");
        std::fs::create_dir_all(d.join("CD2")).unwrap();
        for f in [
            "02 b.mp3",
            "01 a.FLAC",
            "cover.jpg",
            "notes.txt",
            "CD2/01 c.ogg",
            ".hidden.mp3",
        ] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        let found: Vec<String> = expand(std::slice::from_ref(&d))
            .iter()
            .map(|p| p.strip_prefix(&d).unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(found, ["01 a.FLAC", "02 b.mp3", "CD2/01 c.ogg"]);
    }

    #[test]
    fn relative_paths_become_absolute() {
        let d = tmp("rel");
        std::fs::write(d.join("a.mp3"), b"x").unwrap();
        let cwd = std::env::current_dir().unwrap();
        let rel = pathdiff(&d.join("a.mp3"), &cwd);
        let found = expand(&[rel]);
        assert_eq!(found.len(), 1);
        assert!(found[0].is_absolute(), "{:?}", found[0]);
    }

    /// `target` relative to `base` (both absolute), using `..` as needed.
    fn pathdiff(target: &Path, base: &Path) -> PathBuf {
        let t: Vec<_> = target.components().collect();
        let b: Vec<_> = base.components().collect();
        let common = t.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let mut out = PathBuf::new();
        for _ in common..b.len() {
            out.push("..");
        }
        for c in &t[common..] {
            out.push(c);
        }
        out
    }

    #[test]
    fn two_hundred_files_and_images() {
        let d = tmp("many");
        for i in 0..200 {
            std::fs::write(d.join(format!("{i:03}.mp3")), b"x").unwrap();
            if i % 10 == 0 {
                std::fs::write(d.join(format!("{i:03}.png")), b"x").unwrap();
            }
        }
        assert_eq!(expand(&[d]).len(), 200);
    }

    #[test]
    fn m3u_round_trip_preserves_order_and_info() {
        let entries = vec![
            M3uEntry {
                path: "/music/a.mp3".into(),
                title: Some("M83 - Midnight City".into()),
                duration: Some(243.0),
            },
            M3uEntry {
                path: "/music/b é.flac".into(),
                title: None,
                duration: None,
            },
        ];
        let text = write_m3u(&entries);
        assert!(text.starts_with("#EXTM3U\n#EXTINF:243,M83 - Midnight City\n/music/a.mp3\n"));
        assert_eq!(parse_m3u(&text, None), entries);
    }

    #[test]
    fn m3u_plain_relative_crlf_and_bom() {
        let text = "\u{feff}# comment\r\nsong one.mp3\r\n\r\nsub/two.wav\r\n/abs/three.ogg\r\nfile:///abs/four.mp3\r\n";
        let got: Vec<PathBuf> = parse_m3u(text, Some(Path::new("/lists")))
            .into_iter()
            .map(|e| e.path)
            .collect();
        let want: Vec<PathBuf> = [
            "/lists/song one.mp3",
            "/lists/sub/two.wav",
            "/abs/three.ogg",
            "/abs/four.mp3",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn dropped_m3u_expands_to_its_entries() {
        let d = tmp("m3u");
        std::fs::write(d.join("list.m3u"), "a.mp3\nb.mp3\n").unwrap();
        assert_eq!(
            expand(&[d.join("list.m3u")]),
            [d.join("a.mp3"), d.join("b.mp3")]
        );
    }
}
