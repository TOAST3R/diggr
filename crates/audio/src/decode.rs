//! One track's decoder (symphonia → interleaved stereo f32 at the source rate) and the stream
//! resampler that converts to the output rate.

use audioadapter_buffers::direct::InterleavedSlice;
use platform::{FileSource, MediaSource, TrackRef};
use rubato::{Fft, FixedSync, Indexing, Resampler};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, StandardTag};
use symphonia::core::units::{Time, TimeBase};

use crate::TrackInfo;
use crate::ring::CHANNELS;

/// Consecutive undecodable packets tolerated before giving up on a track.
const MAX_CONSECUTIVE_ERRORS: u32 = 64;
const RESAMPLER_CHUNK: usize = 1024;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DecodeError {
    #[error("cannot open: {0}")]
    Open(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("undecodable: {0}")]
    Fatal(String),
}

/// Adapts the platform seam to symphonia's source trait.
struct Source(Box<dyn MediaSource>);

impl std::io::Read for Source {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Seek for Source {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

impl symphonia::core::io::MediaSource for Source {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.0.byte_len()
    }
}

pub struct TrackDecoder {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    time_base: Option<TimeBase>,
    src_rate: u32,
    info: TrackInfo,
    src_buf: Vec<f32>,
    stereo_buf: Vec<f32>,
    /// After a seek: drop decoded audio before this time (seconds).
    skip_until: Option<f64>,
    eof: bool,
    errors_in_row: u32,
    produced_any: bool,
}

impl TrackDecoder {
    /// Opens and probes a track. Only headers are read; decoding starts on [`Self::decode_next`].
    pub fn open(files: &dyn FileSource, track: &TrackRef) -> Result<Self, DecodeError> {
        let media = files
            .open(track)
            .map_err(|e| DecodeError::Open(e.to_string()))?;
        let byte_len = media.byte_len();
        let mss = MediaSourceStream::new(Box::new(Source(media)), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = track.extension() {
            hint.with_extension(&ext);
        }
        let mut reader = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| DecodeError::Unsupported(e.to_string()))?;
        let t = reader
            .default_track(TrackType::Audio)
            .ok_or_else(|| DecodeError::Unsupported("no audio track".into()))?;
        let params = t
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or_else(|| DecodeError::Unsupported("no audio codec parameters".into()))?
            .clone();
        let track_id = t.id;
        let time_base = t.time_base;
        let src_rate = params
            .sample_rate
            .ok_or_else(|| DecodeError::Unsupported("unknown sample rate".into()))?;
        let channels = params.channels.as_ref().map_or(2, |c| c.count()) as u16;
        let duration_secs = match (time_base, t.duration, t.num_frames) {
            (Some(tb), Some(d), _) => tb.calc_duration(d).map(|t| t.as_secs_f64()),
            (_, _, Some(n)) => Some(n as f64 / src_rate as f64),
            _ => None,
        };
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .map_err(|e| DecodeError::Unsupported(e.to_string()))?;

        let mut info = TrackInfo {
            title: track.stem().to_owned(),
            duration_secs,
            bitrate_kbps: match (byte_len, duration_secs) {
                (Some(bytes), Some(d)) if d > 0.0 => Some((bytes as f64 * 8.0 / d / 1000.0) as u32),
                _ => None,
            },
            sample_rate: src_rate,
            channels,
            lossless: is_lossless(params.codec),
            ..Default::default()
        };
        if let Some(rev) = reader.metadata().skip_to_latest() {
            apply_tags(&mut info, rev);
        }

        Ok(Self {
            reader,
            decoder,
            track_id,
            time_base,
            src_rate,
            info,
            src_buf: Vec::new(),
            stereo_buf: Vec::new(),
            skip_until: None,
            eof: false,
            errors_in_row: 0,
            produced_any: false,
        })
    }

    pub fn info(&self) -> &TrackInfo {
        &self.info
    }

    /// Metadata that needs extra reads beyond the probe. Call after playback has started.
    pub fn complete_info(&mut self, files: &dyn FileSource, track: &TrackRef) -> &TrackInfo {
        // symphonia 0.6.1's WAV reader parses RIFF INFO tags but drops them; read them directly.
        if self.info.artist.is_empty()
            && track.extension().as_deref() == Some("wav")
            && let Ok(mut media) = files.open(track)
        {
            apply_riff_info(&mut self.info, &mut media);
        }
        &self.info
    }

    pub fn src_rate(&self) -> u32 {
        self.src_rate
    }

    /// Decodes at least one packet and appends stereo samples at the source rate to `out`.
    /// Returns `Ok(false)` once the track is finished.
    pub fn decode_next(&mut self, out: &mut Vec<f32>) -> Result<bool, DecodeError> {
        if self.eof {
            return Ok(false);
        }
        loop {
            let packet = match self.reader.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) | Err(SymError::ResetRequired) => return self.finish(out),
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return self.finish(out);
                }
                Err(e) => {
                    self.note_error(e)?;
                    continue;
                }
            };
            if packet.track_id != self.track_id {
                continue;
            }
            let packet_time = self
                .time_base
                .and_then(|tb| tb.calc_time(packet.pts))
                .map(|t| t.as_secs_f64());
            let buf = match self.decoder.decode(&packet) {
                Ok(buf) => buf,
                Err(e @ (SymError::DecodeError(_) | SymError::IoError(_))) => {
                    self.note_error(e)?;
                    continue;
                }
                Err(e) => return Err(DecodeError::Fatal(e.to_string())),
            };
            self.errors_in_row = 0;
            let frames = buf.frames();
            let src_channels = buf.spec().channels().count().max(1);
            self.src_buf.resize(frames * src_channels, 0.0);
            buf.copy_to_slice_interleaved::<f32, _>(&mut self.src_buf[..]);

            let mut skip = 0;
            if let (Some(target), Some(t0)) = (self.skip_until, packet_time) {
                skip =
                    (((target - t0) * self.src_rate as f64).round().max(0.0) as usize).min(frames);
                if skip < frames {
                    self.skip_until = None;
                }
            } else {
                self.skip_until = None;
            }
            to_stereo(
                &self.src_buf[skip * src_channels..],
                src_channels,
                &mut self.stereo_buf,
            );
            if self.stereo_buf.is_empty() {
                continue;
            }
            self.produced_any = true;
            out.extend_from_slice(&self.stereo_buf);
            return Ok(true);
        }
    }

    fn note_error(&mut self, e: SymError) -> Result<(), DecodeError> {
        self.errors_in_row += 1;
        if self.errors_in_row > MAX_CONSECUTIVE_ERRORS {
            return Err(DecodeError::Fatal(e.to_string()));
        }
        Ok(())
    }

    fn finish(&mut self, _out: &mut Vec<f32>) -> Result<bool, DecodeError> {
        self.eof = true;
        if !self.produced_any {
            return Err(DecodeError::Fatal("no audio in track".into()));
        }
        Ok(false)
    }

    /// Seeks precisely to `secs`; decoding resumes at exactly that time.
    pub fn seek(&mut self, secs: f64) -> Result<(), DecodeError> {
        let secs = secs.max(0.0);
        let time = Time::try_from_secs_f64(secs)
            .ok_or_else(|| DecodeError::Fatal(format!("bad seek time {secs}")))?;
        self.reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| DecodeError::Fatal(format!("seek failed: {e}")))?;
        self.decoder.reset();
        self.skip_until = Some(secs);
        self.eof = false;
        self.errors_in_row = 0;
        Ok(())
    }
}

/// Uncompressed PCM (not A-law/μ-law) and the compressed lossless codecs.
fn is_lossless(codec: symphonia::core::codecs::audio::AudioCodecId) -> bool {
    use symphonia::core::codecs::audio::well_known::*;
    (CODEC_ID_PCM_S32LE..=CODEC_ID_PCM_F64BE_PLANAR).contains(&codec)
        || (CODEC_ID_FLAC..=CODEC_ID_TRUEHD).contains(&codec)
}

fn apply_tags(info: &mut TrackInfo, rev: &MetadataRevision) {
    let tags = rev
        .media
        .tags
        .iter()
        .chain(rev.per_track.iter().flat_map(|t| t.metadata.tags.iter()));
    for tag in tags {
        match &tag.std {
            Some(StandardTag::TrackTitle(s)) if !s.is_empty() => info.title = s.to_string(),
            Some(StandardTag::Artist(s)) if !s.is_empty() => info.artist = s.to_string(),
            Some(StandardTag::Album(s)) if !s.is_empty() => info.album = s.to_string(),
            _ => {}
        }
    }
}

/// Reads artist/title/album from a RIFF `LIST/INFO` chunk (only the first 1 MB is scanned).
fn apply_riff_info(info: &mut TrackInfo, media: &mut Box<dyn MediaSource>) {
    use std::io::Read;
    let mut head = Vec::new();
    if media.take(1 << 20).read_to_end(&mut head).is_err() || head.get(..4) != Some(b"RIFF") {
        return;
    }
    let u32_at = |b: &[u8], i: usize| {
        b.get(i..i + 4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
    };
    let mut pos = 12;
    while let Some(len) = u32_at(&head, pos + 4) {
        let (id, body) = (&head[pos..pos + 4], pos + 8);
        let end = (body + len as usize).min(head.len());
        if id == b"LIST" && head.get(body..body + 4) == Some(b"INFO") {
            let mut p = body + 4;
            while let Some(sub_len) = u32_at(&head, p + 4) {
                let text_end = (p + 8 + sub_len as usize).min(end);
                let text = String::from_utf8_lossy(&head[p + 8..text_end]);
                let text = text.trim_end_matches('\0').trim().to_owned();
                match &head[p..p + 4] {
                    b"IART" if !text.is_empty() => info.artist = text,
                    b"INAM" if !text.is_empty() => info.title = text,
                    b"IPRD" if !text.is_empty() => info.album = text,
                    _ => {}
                }
                p += 8 + sub_len as usize + (sub_len as usize & 1);
                if p + 8 > end {
                    break;
                }
            }
        }
        pos = body + len as usize + (len as usize & 1);
        if pos + 8 > head.len() {
            break;
        }
    }
}

/// Mono is duplicated; for more than two channels the first two (front L/R) are kept.
fn to_stereo(src: &[f32], channels: usize, dst: &mut Vec<f32>) {
    dst.clear();
    let frames = src.len() / channels;
    dst.reserve(frames * CHANNELS);
    if channels == 1 {
        for &s in src {
            dst.extend_from_slice(&[s, s]);
        }
    } else {
        for f in src.chunks_exact(channels) {
            dst.extend_from_slice(&f[..2]);
        }
    }
}

/// FFT resampler for interleaved audio with any channel count that trims its startup delay and
/// flushes its tail, so `n` input frames always become exactly `round(n * ratio)` output frames.
pub struct ChannelResampler {
    inner: Fft<f32>,
    channels: usize,
    ratio: f64,
    input: Vec<f32>,
    output: Vec<f32>,
    delay_left: usize,
    total_in: u64,
    total_out: u64,
}

/// The playback path's resampler (interleaved stereo).
pub type StereoResampler = ChannelResampler;

impl ChannelResampler {
    /// Interleaved stereo.
    pub fn new(in_rate: u32, out_rate: u32) -> Result<Self, DecodeError> {
        Self::with_channels(in_rate, out_rate, CHANNELS)
    }

    pub fn with_channels(
        in_rate: u32,
        out_rate: u32,
        channels: usize,
    ) -> Result<Self, DecodeError> {
        let fft = Fft::<f32>::new(
            in_rate as usize,
            out_rate as usize,
            RESAMPLER_CHUNK,
            channels,
            FixedSync::Input,
        )
        .map_err(|e| DecodeError::Unsupported(format!("resampler: {e}")))?;
        let delay = fft.output_delay();
        let out_max = fft.output_frames_max();
        Ok(Self {
            inner: fft,
            channels,
            ratio: out_rate as f64 / in_rate as f64,
            input: Vec::with_capacity(RESAMPLER_CHUNK * channels * 2),
            output: vec![0.0; out_max * channels],
            delay_left: delay,
            total_in: 0,
            total_out: 0,
        })
    }

    pub fn process(&mut self, samples: &[f32], out: &mut Vec<f32>) {
        self.input.extend_from_slice(samples);
        self.total_in += (samples.len() / self.channels) as u64;
        loop {
            let need = self.inner.input_frames_next();
            if self.input.len() / self.channels < need {
                break;
            }
            self.run(None, out);
        }
    }

    /// Flushes buffered input and the filter tail.
    pub fn finish(&mut self, out: &mut Vec<f32>) {
        let expected = (self.total_in as f64 * self.ratio).round() as u64;
        let pending = self.input.len() / self.channels;
        if pending > 0 {
            self.run(Some(pending), out);
        }
        while self.total_out < expected {
            self.run(Some(0), out);
        }
        let extra = (self.total_out - expected) as usize;
        out.truncate(out.len() - extra * self.channels);
        self.total_out = expected;
    }

    fn run(&mut self, partial: Option<usize>, out: &mut Vec<f32>) {
        let r = &mut self.inner;
        let need = r.input_frames_next();
        if self.input.len() < need * self.channels {
            self.input.resize(need * self.channels, 0.0);
        }
        let out_frames = self.output.len() / self.channels;
        let input =
            InterleavedSlice::new(&self.input[..], self.channels, need).expect("sized input");
        let mut output = InterleavedSlice::new_mut(&mut self.output[..], self.channels, out_frames)
            .expect("sized output");
        let indexing = Indexing {
            partial_len: partial,
            ..Default::default()
        };
        let (used, produced) = r
            .process_into_buffer(&input, &mut output, Some(&indexing))
            .expect("buffers sized from the resampler");
        let consumed = partial.unwrap_or(used).min(used);
        self.input.drain(..consumed * self.channels);
        if partial.is_some() {
            self.input.clear();
        }
        let skip = self.delay_left.min(produced);
        self.delay_left -= skip;
        out.extend_from_slice(&self.output[skip * self.channels..produced * self.channels]);
        self.total_out += (produced - skip) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, hz: f64, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v =
                    (2.0 * std::f64::consts::PI * hz * i as f64 / rate as f64).sin() as f32 * 0.5;
                [v, v]
            })
            .collect()
    }

    fn zero_crossings(stereo: &[f32]) -> usize {
        stereo
            .chunks(2)
            .map(|f| f[0])
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count()
    }

    #[test]
    fn resampler_preserves_length_and_pitch() {
        let mut r = StereoResampler::new(44_100, 48_000).unwrap();
        let input = sine(44_100, 1_000.0, 44_100);
        let mut out = Vec::new();
        for chunk in input.chunks(1152 * 2) {
            r.process(chunk, &mut out);
        }
        r.finish(&mut out);
        assert_eq!(out.len() / 2, 48_000, "exact output length");
        let zc = zero_crossings(&out);
        assert!(
            (999..=1001).contains(&zc),
            "1 kHz stays 1 kHz, got {zc} cycles"
        );
        // Delay trimmed: the output starts like the input (near zero, rising).
        assert!(out[0].abs() < 0.05 && out[2 * 5] > 0.0);
    }

    #[test]
    fn mono_resampling_keeps_length_and_pitch() {
        let mut r = ChannelResampler::with_channels(48_000, 22_050, 1).unwrap();
        let input: Vec<f32> = sine(48_000, 440.0, 48_000)
            .chunks(2)
            .map(|f| f[0])
            .collect();
        let mut out = Vec::new();
        for chunk in input.chunks(1000) {
            r.process(chunk, &mut out);
        }
        r.finish(&mut out);
        assert_eq!(out.len(), 22_050);
        let crossings = out.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((439..=441).contains(&crossings), "{crossings}");
    }

    #[test]
    fn stereo_conversion() {
        let mut dst = Vec::new();
        to_stereo(&[0.1, 0.2], 1, &mut dst);
        assert_eq!(dst, [0.1, 0.1, 0.2, 0.2]);
        to_stereo(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, &mut dst);
        assert_eq!(dst, [1.0, 2.0, 4.0, 5.0]);
    }
}
