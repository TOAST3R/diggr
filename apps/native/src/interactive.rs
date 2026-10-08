//! Terminal player: the classic player keys and a live status line.

use std::io::Write;
use std::time::Duration;

use audio::{EngineEvent, PlayState};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::terminal;
use platform::TrackRef;

const HELP: &str = "z prev  x play  c pause  v stop  b next  ←/→ seek 5s  ↑/↓ volume  e EQ  q quit";

pub fn run(files: Vec<TrackRef>, volume: f32) -> Result<(), String> {
    let mut engine = crate::open_engine()?;
    engine.set_volume(volume);
    engine.set_queue(files);
    engine.play_index(0);

    println!("{HELP}");
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        let mut title = String::new();
        loop {
            if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())?
                && let Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) = event::read().map_err(|e| e.to_string())?
            {
                let pos = engine.position().seconds();
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('z') => engine.previous(),
                    KeyCode::Char('x') => engine.play(),
                    KeyCode::Char('c') => engine.toggle_pause(),
                    KeyCode::Char('v') => engine.stop(),
                    KeyCode::Char('b') => engine.next(),
                    KeyCode::Left => engine.seek(pos - 5.0),
                    KeyCode::Right => engine.seek(pos + 5.0),
                    KeyCode::Up => engine.set_volume(engine.volume() + 0.05),
                    KeyCode::Down => engine.set_volume(engine.volume() - 0.05),
                    KeyCode::Char('e') => {
                        let mut eq = engine.eq();
                        eq.enabled = !eq.enabled;
                        engine.set_eq(eq);
                    }
                    _ => {}
                }
            }
            for e in engine.poll_events() {
                match e {
                    EngineEvent::TrackInfo { info, .. } => {
                        title = if info.artist.is_empty() {
                            info.title
                        } else {
                            format!("{} - {}", info.artist, info.title)
                        };
                    }
                    EngineEvent::TrackFailed { track, error, .. } => {
                        print!("\r\x1b[2Kskipped {}: {error}\r\n", track.file_name());
                    }
                    _ => {}
                }
            }
            let p = engine.position();
            let s = engine.stats();
            let state = match p.state {
                PlayState::Playing => "▶",
                PlayState::Paused => "‖",
                PlayState::Stopped => "■",
            };
            let secs = p.seconds();
            print!(
                "\r\x1b[2K{state} {:02}:{:05.2}  {title:.40}  vol {:>3.0}%  EQ {}  {} Hz  start {:.1} ms  underruns {}  rt-alloc {}",
                (secs / 60.0) as u32,
                secs % 60.0,
                engine.volume() * 100.0,
                if engine.eq().enabled { "on " } else { "off" },
                s.format.sample_rate,
                s.last_start_latency_ms,
                s.underruns,
                s.rt_alloc_violations,
            );
            let _ = std::io::stdout().flush();
        }
    })();
    let _ = terminal::disable_raw_mode();
    println!();
    result
}
