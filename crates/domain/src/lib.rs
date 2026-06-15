//! xpool domain crate — pure entities and the scoring engine. No I/O.

pub mod invite;
pub mod model;
pub mod participation;
pub mod pool;
pub mod scoring;

pub use model::*;
pub use scoring::*;
