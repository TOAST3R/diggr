//! The classic skinned user interface: skin, main player, equalizer, playlist, waveform and
//! spectrogram.

pub mod app;
pub mod columns;
pub mod crates;
pub mod eqcurve;
pub mod files;
pub mod format;
pub mod help;
pub mod insights;
pub mod layout;
pub mod metadata;
pub mod navigation;
pub mod playlist;
pub mod records;
pub mod settings;
pub mod skin;
pub mod spectrogram;
pub mod spectrum;
pub mod sync;
pub mod waveform;
pub mod widgets;

pub use app::{AppContext, DiggrApp, Startup};
#[cfg(not(target_arch = "wasm32"))]
pub use app::{BridgeSetup, DigAction, DigSetup, SendMode};
