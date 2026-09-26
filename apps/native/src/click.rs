//! Acoustic check of the playback clock (task 6.4): play clicks, capture them with the default
//! input device, and compare when each click was heard against when the clock said it would be.
//!
//! Needs a speaker → microphone path (built-in mic next to the speakers works; a loopback cable
//! is more precise). macOS asks for microphone permission on first run.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use platform::TrackRef;
use platform::native::now_ns;

const CLICK_EVERY_SECS: f64 = 0.5;
const CLICKS: usize = 16;
const THRESHOLD: f32 = 0.2;

pub fn run() -> Result<(), String> {
    let mut engine = crate::open_engine()?;
    let rate = engine.stats().format.sample_rate;
    let path = std::env::temp_dir().join("winamp-click-test.wav");
    let click_frames = write_clicks(&path, rate)?;

    // Microphone capture, timestamped in the same monotonic base as the playback clock.
    let heard: Arc<Mutex<Vec<u64>>> = Arc::default();
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("no input device")?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    let (in_rate, in_ch) = (config.sample_rate() as f64, config.channels() as usize);
    let sink = heard.clone();
    let mut refractory_until = 0u64;
    let stream = device
        .build_input_stream::<f32, _, _>(
            config.config(),
            move |data: &[f32], info: &cpal::InputCallbackInfo| {
                let ts = info.timestamp();
                let capture_ns =
                    now_ns().saturating_sub((ts.callback - ts.capture).as_nanos() as u64);
                for (i, frame) in data.chunks(in_ch).enumerate() {
                    let t = capture_ns + (i as f64 / in_rate * 1e9) as u64;
                    if frame[0].abs() > THRESHOLD && t > refractory_until {
                        sink.lock().unwrap().push(t);
                        refractory_until = t + 200_000_000;
                    }
                }
            },
            |e| eprintln!("input error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;

    println!("playing {CLICKS} clicks — keep the volume up and the microphone near the speaker");
    engine.set_volume(1.0);
    engine.set_queue(vec![TrackRef::new(path.to_string_lossy())]);
    engine.play_index(0);
    let mut samples = Vec::new(); // (time, audible frame) pairs from the clock
    let mut reader = engine.clock_reader();
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs_f64(CLICKS as f64 * CLICK_EVERY_SECS + 2.0) {
        let t = now_ns();
        let p = reader.position(t);
        if p.state == audio::PlayState::Playing && p.frame > 0 {
            samples.push((t, p.frame));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    drop(stream);

    // When the clock predicted each click to be audible.
    let predicted: Vec<u64> = click_frames
        .iter()
        .filter_map(|&f| {
            let &(t, frame) = samples.iter().rev().find(|(_, fr)| *fr <= f)?;
            Some(t + ((f - frame) as f64 / rate as f64 * 1e9) as u64)
        })
        .collect();
    let heard = heard.lock().unwrap().clone();
    let offsets: Vec<f64> = predicted
        .iter()
        .filter_map(|&p| {
            heard
                .iter()
                .map(|&h| (h as f64 - p as f64) / 1e6)
                .filter(|d| d.abs() < 150.0)
                .min_by(|a, b| a.abs().total_cmp(&b.abs()))
        })
        .collect();
    if offsets.len() < CLICKS / 2 {
        return Err(format!(
            "only {} of {CLICKS} clicks were captured — is the microphone hearing the speaker?",
            offsets.len()
        ));
    }
    let mean = offsets.iter().sum::<f64>() / offsets.len() as f64;
    let max = offsets.iter().fold(0f64, |m, d| m.max(d.abs()));
    println!(
        "captured {}/{CLICKS} clicks: mean offset {mean:+.2} ms, max |offset| {max:.2} ms (heard − predicted)",
        offsets.len()
    );
    println!("(includes acoustic travel ≈ 3 ms per metre and any unreported input latency)");
    if mean.abs() <= 2.0 {
        println!("clock accuracy within ±2 ms ✓");
    } else {
        println!(
            "consider an A/V offset of {:+.0} ms (engine.set_av_offset_ms)",
            mean
        );
    }
    Ok(())
}

/// Writes 2 ms full-scale bursts every half second after a 1 s lead-in; returns their frames.
fn write_clicks(path: &std::path::Path, rate: u32) -> Result<Vec<u64>, String> {
    let total = ((1.0 + CLICKS as f64 * CLICK_EVERY_SECS + 1.0) * rate as f64) as u64;
    let clicks: Vec<u64> = (0..CLICKS)
        .map(|k| ((1.0 + k as f64 * CLICK_EVERY_SECS) * rate as f64) as u64)
        .collect();
    let burst = rate as u64 / 500;
    let mut wav = Vec::with_capacity(44 + total as usize * 4);
    let data_len = (total * 4) as u32; // 16-bit stereo
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 4).to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for f in 0..total {
        let on = clicks.iter().any(|&c| f >= c && f < c + burst);
        let s: i16 = if on {
            if (f / 12) % 2 == 0 { 30_000 } else { -30_000 }
        } else {
            0
        };
        wav.extend_from_slice(&s.to_le_bytes());
        wav.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, wav).map_err(|e| e.to_string())?;
    Ok(clicks)
}
