//! The classic Winamp-style user interface: skin, main player, equalizer, playlist, and the
//! fullscreen host for the visual engine.

pub mod app;
pub mod eqcurve;
pub mod files;
pub mod format;
pub mod fullscreen;
pub mod help;
pub mod metadata;
pub mod navigation;
pub mod playlist;
pub mod render_job;
pub mod settings;
pub mod skin;
pub mod spectrogram;
pub mod spectrum;
pub mod timeline;
pub mod waveform;
pub mod widgets;

pub use app::{AppContext, Startup, WinampApp};
