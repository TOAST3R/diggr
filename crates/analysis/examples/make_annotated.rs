//! Writes synthetic tracks and "listener" annotations of their ground truth into a folder, to
//! try `analysis-eval` without real music:
//! `cargo run -p analysis --example make_annotated -- DIR && cargo run -p analysis --bin analysis-eval -- DIR/annotations`
use analysis::eval::Annotations;
use analysis::synth;
use platform::TrackRef;
use platform::native::NativeFileSource;

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("usage: make_annotated DIR"));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, bpm) in [("house_124.wav", 124.0), ("dnb_174.wav", 174.0)] {
        let s = synth::render(&synth::standard_track(bpm), 44_100, bpm as u64);
        let path = dir.join(name);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for &x in &s.samples {
            w.write_sample((x * 32_000.0) as i16).unwrap();
        }
        w.finalize().unwrap();
        let track = TrackRef::new(path.to_string_lossy());
        let hash = analysis::cache::content_hash(&NativeFileSource, &track).unwrap();
        let mut ann = Annotations::new(&track, hash);
        for &t in s.beats.iter().take(64) {
            ann.tap(t + 0.015);
        }
        for &(t, kind) in &s.sections[1..] {
            ann.boundary(t, kind);
        }
        println!(
            "wrote {}",
            ann.save(&dir.join("annotations")).unwrap().display()
        );
    }
}
