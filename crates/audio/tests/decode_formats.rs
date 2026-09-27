//! Decoder behavior on real files in every supported format (fixtures: 2 s, 440 Hz, 44.1 kHz).

use audio::decode::{DecodeError, StereoResampler, TrackDecoder};
use platform::TrackRef;
use platform::native::NativeFileSource;

fn fixture(name: &str) -> TrackRef {
    TrackRef::new(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn open(name: &str) -> TrackDecoder {
    TrackDecoder::open(&NativeFileSource, &fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Decodes the rest of the track, resampled to `out_rate` like the engine does.
fn decode_all(dec: &mut TrackDecoder, out_rate: u32) -> Vec<f32> {
    let mut raw = Vec::new();
    while dec.decode_next(&mut raw).unwrap() {}
    if dec.src_rate() == out_rate {
        return raw;
    }
    let mut r = StereoResampler::new(dec.src_rate(), out_rate).unwrap();
    let mut out = Vec::new();
    r.process(&raw, &mut out);
    r.finish(&mut out);
    out
}

/// Positive-going zero crossings of the left channel.
fn cycles(stereo: &[f32]) -> usize {
    let left: Vec<f32> = stereo.chunks(2).map(|f| f[0]).collect();
    left.windows(2)
        .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
        .count()
}

/// Frequency of the left channel measured between 0.25 s and 1.75 s (away from codec edges).
fn frequency(stereo: &[f32], rate: u32) -> f64 {
    let (a, b) = (rate as usize / 4, rate as usize * 7 / 4);
    cycles(&stereo[a * 2..b * 2]) as f64 / 1.5
}

/// Gapless trimming keeps lossy codecs exact to a few ms, except AAC in MP4: symphonia 0.6.1
/// ignores the MP4 edit list, leaving 1024 priming + padding frames (~43 ms).
fn length_tolerance(name: &str) -> f64 {
    if name.ends_with(".m4a") { 0.05 } else { 0.005 }
}

#[test]
fn every_format_decodes_at_correct_pitch_and_length() {
    for name in ["tone.wav", "tone.mp3", "tone.flac", "tone.ogg", "tone.m4a"] {
        for out_rate in [44_100, 48_000] {
            let pcm = decode_all(&mut open(name), out_rate);
            let secs = pcm.len() as f64 / 2.0 / out_rate as f64;
            assert!(
                (secs - 2.0).abs() < length_tolerance(name),
                "{name}@{out_rate}: {secs:.4} s"
            );
            let hz = frequency(&pcm, out_rate);
            assert!(
                (hz - 440.0).abs() < 1.5,
                "{name}@{out_rate}: {hz:.1} Hz (want 440)"
            );
            assert!(
                pcm.iter().all(|s| s.is_finite() && s.abs() <= 1.01),
                "{name}: bad samples"
            );
        }
    }
}

#[test]
fn tags_are_read_for_every_format() {
    for name in ["tone.wav", "tone.mp3", "tone.flac", "tone.ogg", "tone.m4a"] {
        let mut dec = open(name);
        let info = dec.complete_info(&NativeFileSource, &fixture(name));
        assert_eq!(info.artist, "M83", "{name}");
        assert_eq!(info.title, "Midnight_City", "{name}");
        assert_eq!(info.album, "Hurry_Up", "{name}");
        assert_eq!((info.sample_rate, info.channels), (44_100, 2), "{name}");
        let d = info.duration_secs.expect("duration");
        assert!((d - 2.0).abs() < 0.03, "{name}: duration {d}");
    }
    let kbps = open("tone.mp3").info().bitrate_kbps.unwrap();
    assert!((120..=140).contains(&kbps), "mp3 bitrate {kbps}");
}

#[test]
fn lossless_follows_the_codec() {
    for (name, lossless) in [
        ("tone.wav", true),
        ("tone.flac", true),
        ("tone.mp3", false),
        ("tone.ogg", false),
        ("tone.m4a", false), // AAC in MP4
    ] {
        assert_eq!(open(name).info().lossless, lossless, "{name}");
    }
}

#[test]
fn missing_tags_fall_back_to_file_name() {
    let dec = open("untagged.mp3");
    assert_eq!(dec.info().title, "untagged");
    assert_eq!(dec.info().artist, "");
}

#[test]
fn mono_is_duplicated_and_resampled() {
    let mut dec = open("mono48k.wav");
    assert_eq!(dec.info().channels, 1);
    let pcm = decode_all(&mut dec, 44_100);
    assert_eq!(pcm.len() / 2, 44_100);
    assert!(pcm.chunks(2).all(|f| f[0] == f[1]));
    let c = cycles(&pcm);
    assert!((439..=441).contains(&c), "{c}");
}

#[test]
fn corrupt_frames_are_skipped() {
    let pcm = decode_all(&mut open("corrupt.mp3"), 44_100);
    let secs = pcm.len() as f64 / 2.0 / 44_100.0;
    assert!(
        secs > 1.5,
        "decoding continued past the damage, got {secs:.2} s"
    );
}

#[test]
fn unplayable_files_are_errors() {
    match TrackDecoder::open(&NativeFileSource, &fixture("garbage.mp3")) {
        Err(DecodeError::Unsupported(_)) => {}
        Err(other) => panic!("unexpected error kind {other}"),
        Ok(mut dec) => {
            let mut out = Vec::new();
            let mut result = Ok(true);
            while matches!(result, Ok(true)) {
                result = dec.decode_next(&mut out);
            }
            assert!(result.is_err(), "random bytes must not play as audio");
        }
    }
    assert!(matches!(
        TrackDecoder::open(&NativeFileSource, &fixture("nope.mp3")),
        Err(DecodeError::Open(_))
    ));
}

#[test]
fn seek_is_accurate() {
    for name in ["tone.wav", "tone.flac", "tone.mp3", "tone.ogg", "tone.m4a"] {
        let mut reference = decode_all(&mut open(name), 44_100);
        let mut dec = open(name);
        dec.seek(1.25).unwrap();
        let after = decode_all(&mut dec, 44_100);
        let remaining = after.len() as f64 / 2.0 / 44_100.0;
        assert!(
            (remaining - 0.75).abs() < length_tolerance(name),
            "{name}: {remaining:.4} s left"
        );
        // The audio after the seek matches the reference at the same position (within 2 ms).
        reference.drain(..55_125 * 2);
        let best = (0..=88)
            .map(|lag| {
                let (a, b) = (&after[..8000], &reference[lag * 2..lag * 2 + 8000]);
                let e: f32 = a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum();
                (e / 8000.0, lag)
            })
            .fold((f32::MAX, 0), |m, x| if x.0 < m.0 { x } else { m });
        assert!(best.0 < 0.02, "{name}: post-seek audio mismatch {best:?}");
    }
}
