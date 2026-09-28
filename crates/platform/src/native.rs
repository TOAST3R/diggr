//! Native seam implementations: cpal, std threads, std::fs.

use std::fs::File;
use std::sync::OnceLock;
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{
    AudioSink, CallbackInfo, ErrorFn, FileSource, MediaSource, PlatformError, Priority, RenderFn,
    SinkConfig, SinkError, SinkHandle, Spawner, StreamFormat, TrackRef,
};

/// Monotonic nanoseconds since the first call in this process. Cheap enough for audio callbacks
/// (no syscall on macOS).
pub fn now_ns() -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NativeSpawner;

impl Spawner for NativeSpawner {
    fn spawn(
        &self,
        name: &str,
        priority: Priority,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), PlatformError> {
        std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                apply_priority(priority);
                f()
            })
            .map(|_| ())
            .map_err(|e| PlatformError::Spawn(e.to_string()))
    }
}

fn apply_priority(priority: Priority) {
    use thread_priority::{ThreadPriority, set_current_thread_priority};
    let p = match priority {
        Priority::High => ThreadPriority::Max,
        Priority::Low => ThreadPriority::Min,
        Priority::Normal => return,
    };
    // Best effort: raising priority may be refused without privileges on some systems.
    let _ = set_current_thread_priority(p);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NativeFileSource;

struct FileMedia {
    file: File,
    len: Option<u64>,
}

impl std::io::Read for FileMedia {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl std::io::Seek for FileMedia {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl MediaSource for FileMedia {
    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}

impl FileSource for NativeFileSource {
    fn open(&self, track: &TrackRef) -> Result<Box<dyn MediaSource>, PlatformError> {
        let file = File::open(&track.0)?;
        let len = file
            .metadata()
            .ok()
            .filter(|m| m.is_file())
            .map(|m| m.len());
        Ok(Box::new(FileMedia { file, len }))
    }
}

/// Output through the default cpal device, always as interleaved f32.
#[derive(Debug, Default, Clone, Copy)]
pub struct CpalSink;

struct CpalHandle {
    _stream: cpal::Stream,
    format: StreamFormat,
}

impl SinkHandle for CpalHandle {
    fn format(&self) -> StreamFormat {
        self.format
    }
}

fn map_error(err: cpal::Error) -> SinkError {
    use cpal::ErrorKind as K;
    match err.kind() {
        K::DeviceNotAvailable | K::DeviceChanged | K::StreamInvalidated => SinkError::DeviceLost,
        K::Xrun => SinkError::Xrun,
        _ => SinkError::Other(err.to_string()),
    }
}

fn backend(e: impl std::fmt::Display) -> PlatformError {
    PlatformError::Backend(e.to_string())
}

impl AudioSink for CpalSink {
    fn now_ns(&self) -> u64 {
        now_ns()
    }

    fn default_format(&self, cfg: &SinkConfig) -> Result<StreamFormat, PlatformError> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(PlatformError::NoDevice)?;
        let supported = device.default_output_config().map_err(backend)?;
        let buffer_frames = match (cfg.buffer_frames, supported.buffer_size()) {
            (Some(want), cpal::SupportedBufferSize::Range { min, max }) => {
                Some(want.clamp(*min, *max))
            }
            _ => None,
        };
        Ok(StreamFormat {
            sample_rate: supported.sample_rate(),
            channels: supported.channels(),
            buffer_frames,
        })
    }

    fn start(
        &self,
        format: &StreamFormat,
        mut render: RenderFn,
        mut on_error: ErrorFn,
    ) -> Result<Box<dyn SinkHandle>, PlatformError> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(PlatformError::NoDevice)?;
        let config = cpal::StreamConfig {
            channels: format.channels,
            sample_rate: format.sample_rate,
            buffer_size: match format.buffer_frames {
                Some(n) => cpal::BufferSize::Fixed(n),
                None => cpal::BufferSize::Default,
            },
        };
        let stream = device
            .build_output_stream::<f32, _, _>(
                config,
                move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                    let ts = info.timestamp();
                    let latency = ts.playback - ts.callback;
                    render(
                        data,
                        CallbackInfo {
                            host_ns: now_ns(),
                            output_latency_ns: latency.as_nanos() as u64,
                        },
                    );
                },
                move |err| on_error(map_error(err)),
                None,
            )
            .map_err(backend)?;
        stream.play().map_err(backend)?;
        Ok(Box::new(CpalHandle {
            _stream: stream,
            format: *format,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::mpsc;

    #[test]
    fn now_is_monotonic() {
        let a = now_ns();
        let b = now_ns();
        assert!(b >= a);
    }

    #[test]
    fn file_source_streams_chunks() {
        let dir = crate::testing::TestDir::new("platform-file-source");
        let path = dir.join("data.bin");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&[7u8; 10_000])
            .unwrap();

        let mut media = NativeFileSource
            .open(&TrackRef::new(path.to_string_lossy()))
            .unwrap();
        assert_eq!(media.byte_len(), Some(10_000));
        let mut chunk = [0u8; 4096];
        assert_eq!(media.read(&mut chunk).unwrap(), 4096);
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(
            NativeFileSource
                .open(&TrackRef::new("/definitely/not/here.mp3"))
                .is_err()
        );
    }

    #[test]
    fn spawner_runs_with_priorities() {
        let (tx, rx) = mpsc::channel();
        for p in [Priority::Low, Priority::Normal, Priority::High] {
            let tx = tx.clone();
            NativeSpawner
                .spawn("test", p, Box::new(move || tx.send(p).unwrap()))
                .unwrap();
        }
        let mut got: Vec<_> = (0..3).map(|_| rx.recv().unwrap()).collect();
        got.sort_by_key(|p| *p as u8);
        assert_eq!(got, vec![Priority::Low, Priority::Normal, Priority::High]);
    }
}
