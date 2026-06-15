//! A thin, typed client for [TheSportsDB](https://www.thesportsdb.com) V2.
//!
//! Pure JSON decoding ([`decode`]) is separated from HTTP ([`client`]) so the
//! envelope handling is unit-testable without a network. Structured so it could
//! later be extracted and published as a standalone open-source Rust SDK:
//! the public surface carries no xpool-specific types and no dependency on the
//! `domain`/`storage` crates.

mod client;
mod decode;
mod model;

pub use client::SportsDb;
pub use decode::{decode_livescore, decode_schedule};
pub use model::Event;
