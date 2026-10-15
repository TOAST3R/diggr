//! Previews: clips fetched as audio with the user's own yt-dlp (or, for Juno samples, one
//! plain HTTPS request), just ahead of playback.

pub mod clip;
pub mod fetcher;
pub mod sample;
pub mod scheduler;
pub mod search;
pub mod store;
