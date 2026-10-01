//! Audio core: streaming decode → lock-free ring → real-time renderer (EQ, volume) → device,
//! with a playback clock and a sample tap for visual consumers.

pub mod clock;
pub mod decode;
pub mod engine;
pub mod eq;
pub mod filter;
pub mod renderer;
pub mod ring;
pub mod rt_guard;
pub mod tap;
pub mod trycell;
pub mod worker;

pub use clock::{ClockReader, ClockSnapshot, Position};
pub use engine::{Engine, EngineConfig, EngineError, EngineStats};
pub use eq::{EqPreset, EqPresets, EqSettings};
pub use tap::{TapChunk, TapReader};
pub use worker::{EngineEvent, RepeatMode};

/// Identifies one playback instance of a track (a new id each time a track is loaded). 0 = none.
pub type TrackId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PlayState {
    #[default]
    Stopped = 0,
    Playing = 1,
    Paused = 2,
}

impl PlayState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Playing,
            2 => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

/// Descriptive metadata for a track.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackInfo {
    pub title: String,
    /// Empty when the file has no artist tag.
    pub artist: String,
    pub album: String,
    pub duration_secs: Option<f64>,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Decoded with a lossless codec (PCM, FLAC, ALAC, …), so any band limit is in the source.
    pub lossless: bool,
}
