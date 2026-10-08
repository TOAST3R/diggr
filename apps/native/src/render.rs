//! `--render-show`: render a track's visual show to an MP4 from the command line.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use platform::TrackRef;
use platform::native::NativeFileSource;
use visuals::render::{Options, RenderError, render_show};

pub const USAGE: &str = "diggr --render-show TRACK -o OUT.mp4 [--size WxH] [--fps N] [--from TIME] [--to TIME] [--overlay] [--look SCENE/VARIANT]";

/// `90`, `1:30` or `1:02:03` → seconds.
pub fn parse_time(s: &str) -> Option<f64> {
    ui::format::parse_clock(s)
}

fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('x')?;
    let size = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
    visuals::render::size_ok(size).then_some(size)
}

/// Parses the arguments after `--render-show`.
pub fn parse(args: &[String]) -> Result<Options, String> {
    let mut track = None;
    let mut out = None;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(value(a)?)),
            "--size" | "--fps" | "--from" | "--to" | "--look" => rest.push((a.clone(), value(a)?)),
            "--overlay" => rest.push((a.clone(), String::new())),
            _ if track.is_none() && !a.starts_with('-') => track = Some(a.clone()),
            _ => return Err(format!("unexpected argument {a}")),
        }
    }
    let track = track.ok_or("no track given")?;
    let out = out.ok_or("no output file given (-o OUT.mp4)")?;
    let mut o = Options::new(TrackRef::new(track), out);
    for (k, v) in rest {
        match k.as_str() {
            "--size" => {
                o.size = parse_size(&v).ok_or(format!(
                    "bad --size {v} (like 1920x1080: width divisible by 8, even height)"
                ))?
            }
            "--fps" => {
                o.fps = v
                    .parse()
                    .ok()
                    .filter(|f| (1..=240).contains(f))
                    .ok_or(format!("bad --fps {v}"))?
            }
            "--from" => o.from = parse_time(&v).ok_or(format!("bad --from {v}"))?,
            "--to" => o.to = Some(parse_time(&v).ok_or(format!("bad --to {v}"))?),
            "--look" => {
                let (s, v2) = v
                    .split_once('/')
                    .ok_or(format!("bad --look {v} (like julia_tunnel/solar)"))?;
                o.look = Some((s.to_string(), v2.to_string()));
            }
            "--overlay" => o.overlay = true,
            _ => unreachable!(),
        }
    }
    if o.to.is_some_and(|t| t <= o.from) {
        return Err("--to must be after --from".into());
    }
    Ok(o)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args).map_err(|e| format!("{e}\nusage: {USAGE}"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let c = cancel.clone();
    let _ = ctrlc::set_handler(move || c.store(true, Ordering::Relaxed));
    let name = opts.out.display().to_string();
    let mut last_print = Instant::now();
    let fps = opts.fps as f64;
    eprintln!("Analyzing and rendering {} → {name}", opts.track.0);
    let result = render_show(&NativeFileSource, &opts, |p| {
        if last_print.elapsed().as_millis() >= 250 || p.frame == p.frames {
            last_print = Instant::now();
            let eta = p.eta_secs().map_or_else(|| "…".into(), ui::format::clock);
            eprint!(
                "\rRendering {:>3.0}% · {eta} left · {:.1}× real time   ",
                p.fraction() * 100.0,
                p.speed() / fps
            );
            let _ = std::io::stderr().flush();
        }
        !cancel.load(Ordering::Relaxed)
    });
    eprintln!();
    match result {
        Ok(s) => {
            eprintln!(
                "Wrote {name}: {} frames in {} ({:.1}× real time)",
                s.frames,
                ui::format::clock(s.seconds),
                s.frames as f64 / fps / s.seconds.max(1e-6)
            );
            Ok(())
        }
        Err(RenderError::Cancelled) => Err("render cancelled; nothing was written".into()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn times() {
        assert_eq!(parse_time("90"), Some(90.0));
        assert_eq!(parse_time("1:30"), Some(90.0));
        assert_eq!(parse_time("1:02:03.5"), Some(3723.5));
        assert_eq!(parse_time("x"), None);
    }

    #[test]
    fn arguments() {
        let o = parse(&args("t.flac -o s.mp4")).unwrap();
        assert_eq!(
            (o.size, o.fps, o.from, o.to, o.overlay),
            ((1920, 1080), 60, 0.0, None, false)
        );
        let o = parse(&args("t.flac -o s.mp4 --size 1280x720 --fps 30 --from 1:00 --to 1:30 --overlay --look flame/silk")).unwrap();
        assert_eq!(
            (o.size, o.fps, o.from, o.to, o.overlay),
            ((1280, 720), 30, 60.0, Some(90.0), true)
        );
        assert_eq!(o.look, Some(("flame".into(), "silk".into())));
        assert!(parse(&args("t.flac")).unwrap_err().contains("-o"));
        assert!(
            parse(&args("t.flac -o s.mp4 --size 1284x720")).is_err(),
            "width must divide by 8"
        );
        assert!(
            parse(&args("t.flac -o s.mp4 --size 1080x1920")).is_ok(),
            "vertical video"
        );
        assert!(parse(&args("t.flac -o s.mp4 --from 2:00 --to 1:00")).is_err());
    }
}
