//! The Discogs side: HTTP, rate limit, disk cache, addresses, and turning a page into records
//! and their clips.

pub mod cache;
pub mod cart;
pub mod client;
pub mod expand;
pub mod matching;
pub mod model;
pub mod ratelimit;
pub mod seller;
pub mod transport;
pub mod url;
