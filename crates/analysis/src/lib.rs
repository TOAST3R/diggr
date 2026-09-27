//! Music analysis ahead of the playhead: beat grid, tempo segments, sections (intro, build,
//! drop, …), tension and drop countdown, published as immutable `SongScore` snapshots.

pub mod assemble;
pub mod cache;
pub mod detail;
pub mod eval;
pub mod frontend;
pub mod onsets;
pub mod overview;
pub mod region;
pub mod rhythm;
pub mod score;
pub mod service;
pub mod source;
pub mod spectral;
pub mod structure;
pub mod synth;

pub use score::{SectionKind, SongScore};
pub use service::AnalysisService;
