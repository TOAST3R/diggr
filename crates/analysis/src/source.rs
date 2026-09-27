//! Reads a track for analysis: its own `TrackDecoder` (never the playback one) → mono →
//! 22,050 Hz, in chunks, from any start time.

use audio::decode::{ChannelResampler, DecodeError, TrackDecoder};
use platform::{FileSource, TrackRef};

use crate::frontend::SR;

pub struct Source {
    dec: TrackDecoder,
    resampler: Option<ChannelResampler>,
    raw: Vec<f32>,
    mono: Vec<f32>,
    finished: bool,
}

impl Source {
    pub fn open(
        files: &dyn FileSource,
        track: &TrackRef,
        start_secs: f64,
    ) -> Result<Self, DecodeError> {
        let mut dec = TrackDecoder::open(files, track)?;
        if start_secs > 0.0 {
            dec.seek(start_secs)?;
        }
        let rate = dec.src_rate();
        let resampler = (rate != SR)
            .then(|| ChannelResampler::with_channels(rate, SR, 1))
            .transpose()?;
        Ok(Self {
            dec,
            resampler,
            raw: Vec::new(),
            mono: Vec::new(),
            finished: false,
        })
    }

    pub fn duration(&self) -> Option<f64> {
        self.dec.info().duration_secs
    }

    /// About `secs` of mono 22,050 Hz audio, or `None` once the track is exhausted.
    pub fn read(&mut self, secs: f64) -> Option<Vec<f32>> {
        if self.finished {
            return None;
        }
        let want = (secs * SR as f64) as usize;
        let mut out = Vec::with_capacity(want + 4096);
        while out.len() < want {
            self.raw.clear();
            let more = self.dec.decode_next(&mut self.raw).unwrap_or(false);
            self.mono.clear();
            self.mono.extend(
                self.raw
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|f| 0.5 * (f[0] + f[1])),
            );
            match &mut self.resampler {
                Some(r) => r.process(&self.mono, &mut out),
                None => out.extend_from_slice(&self.mono),
            }
            if !more {
                if let Some(r) = &mut self.resampler {
                    r.finish(&mut out);
                }
                self.finished = true;
                break;
            }
        }
        (!out.is_empty()).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::native::NativeFileSource;

    fn fixture(name: &str) -> TrackRef {
        TrackRef::new(format!(
            "{}/../audio/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
    }

    #[test]
    fn reads_mono_22k_from_start_and_middle() {
        for name in ["tone.flac", "tone.mp3"] {
            let mut s = Source::open(&NativeFileSource, &fixture(name), 0.0).unwrap();
            let mut all = Vec::new();
            while let Some(c) = s.read(0.5) {
                all.extend(c);
            }
            let secs = all.len() as f64 / SR as f64;
            assert!((secs - 2.0).abs() < 0.01, "{name}: {secs}");
            let crossings =
                all.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f64 / secs;
            assert!((crossings - 440.0).abs() < 2.0, "{name}: {crossings} Hz");

            let mut s = Source::open(&NativeFileSource, &fixture(name), 1.5).unwrap();
            let rest: usize = std::iter::from_fn(|| s.read(0.2)).map(|c| c.len()).sum();
            assert!(
                (rest as f64 / SR as f64 - 0.5).abs() < 0.01,
                "{name}: from 1.5 s"
            );
        }
    }
}
