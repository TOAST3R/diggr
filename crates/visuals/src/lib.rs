//! The fullscreen visual engine: signals, modulation, scenes, variants, director, compositor.

pub mod automaton;
pub mod codegen;
pub mod compositor;
pub mod deck;
pub mod director;
pub mod engine;
pub mod gpu;
pub mod library;
pub mod manifest;
pub mod modulation;
pub mod overlay;
pub mod render;
mod render_overlay;
pub mod signals;
pub mod variants;

pub use engine::VisualEngine;
