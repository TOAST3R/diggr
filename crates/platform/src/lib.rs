//! Platform seams: the only way the core crates reach audio devices, threads, and files.
//!
//! Native implementations live in [`native`]; a web target provides its own implementations of
//! the same traits. [`testing`] offers a manually driven sink for deterministic tests.

use std::io::{Read, Seek};

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod testing;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no output device available")]
    NoDevice,
    #[error("audio backend error: {0}")]
    Backend(String),
    #[error("thread spawn failed: {0}")]
    Spawn(String),
}

/// Scheduling hint for spawned threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// Background work that must never compete with playback (analysis, scanning).
    Low,
    Normal,
    /// Latency-sensitive producers (decoding).
    High,
}

pub trait Spawner: Send + Sync {
    fn spawn(
        &self,
        name: &str,
        priority: Priority,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), PlatformError>;
}

/// Identifies a playable item. On native this is a file system path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackRef(pub String);

impl TrackRef {
    pub fn new(location: impl Into<String>) -> Self {
        Self(location.into())
    }

    /// Last path component, e.g. `song.mp3`.
    pub fn file_name(&self) -> &str {
        self.0.rsplit(['/', '\\']).next().unwrap_or(&self.0)
    }

    /// File name without its extension, e.g. `song`.
    pub fn stem(&self) -> &str {
        let name = self.file_name();
        match name.rfind('.') {
            Some(i) if i > 0 => &name[..i],
            _ => name,
        }
    }

    /// Lower-case extension without the dot, if any.
    pub fn extension(&self) -> Option<String> {
        let name = self.file_name();
        match name.rfind('.') {
            Some(i) if i > 0 && i + 1 < name.len() => Some(name[i + 1..].to_ascii_lowercase()),
            _ => None,
        }
    }
}

/// A seekable byte stream that is read in chunks, never loaded whole.
pub trait MediaSource: Read + Seek + Send + Sync {
    fn byte_len(&self) -> Option<u64>;
}

pub trait FileSource: Send + Sync {
    fn open(&self, track: &TrackRef) -> Result<Box<dyn MediaSource>, PlatformError>;
}

/// What the engine would like from the output device.
#[derive(Debug, Clone, Copy, Default)]
pub struct SinkConfig {
    /// Requested device buffer size in frames; `None` uses the backend default.
    pub buffer_frames: Option<u32>,
}

/// The format a sink actually runs at. Samples are always interleaved `f32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: Option<u32>,
}

/// Timing for one render callback.
#[derive(Debug, Clone, Copy, Default)]
pub struct CallbackInfo {
    /// Monotonic time (same base as [`AudioSink::now_ns`]) at which the callback started.
    pub host_ns: u64,
    /// Time from the callback until the first frame of this buffer reaches the speaker.
    pub output_latency_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkError {
    /// The device disappeared or changed; the stream must be rebuilt.
    DeviceLost,
    /// The backend reported an under/overrun.
    Xrun,
    Other(String),
}

pub type RenderFn = Box<dyn FnMut(&mut [f32], CallbackInfo) + Send + 'static>;
pub type ErrorFn = Box<dyn FnMut(SinkError) + Send + 'static>;

/// A running output stream. Dropping it stops the stream.
pub trait SinkHandle {
    fn format(&self) -> StreamFormat;
}

pub trait AudioSink: Send + Sync {
    /// Monotonic time in the same base as [`CallbackInfo::host_ns`].
    fn now_ns(&self) -> u64;

    /// The format the default device would run at with `cfg`.
    fn default_format(&self, cfg: &SinkConfig) -> Result<StreamFormat, PlatformError>;

    /// Start a stream at `format`. `render` is called on the real-time thread.
    fn start(
        &self,
        format: &StreamFormat,
        render: RenderFn,
        on_error: ErrorFn,
    ) -> Result<Box<dyn SinkHandle>, PlatformError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_ref_parts() {
        let t = TrackRef::new("/music/M83 - Midnight City.MP3");
        assert_eq!(t.file_name(), "M83 - Midnight City.MP3");
        assert_eq!(t.stem(), "M83 - Midnight City");
        assert_eq!(t.extension().as_deref(), Some("mp3"));

        let hidden = TrackRef::new("dir/.hidden");
        assert_eq!(hidden.stem(), ".hidden");
        assert_eq!(hidden.extension(), None);
    }
}
