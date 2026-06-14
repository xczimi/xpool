//! xtask library — tournament import & demo seeding (plan task P5).
//!
//! Split from `main.rs` so the importer can be exercised by integration tests
//! without the CLI shell.

pub mod dto;
pub mod export;
pub mod migrate_gh;
pub mod scenario;
pub mod seed;
pub mod snapshot;
pub mod validate;

use anyhow::Context;
use domain::Tournament;
use std::path::Path;

/// Read a tournament JSON file, deserialise into `domain::Tournament`, and
/// validate it loudly. Does not touch storage.
pub fn load_tournament(path: &Path) -> anyhow::Result<Tournament> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading tournament file `{}`", path.display()))?;
    let dto: dto::TournamentDto = serde_json::from_str(&raw)
        .with_context(|| format!("parsing tournament JSON `{}`", path.display()))?;
    let tournament = dto
        .into_domain()
        .with_context(|| format!("converting tournament `{}`", path.display()))?;
    validate::validate(&tournament)
        .with_context(|| format!("validating tournament `{}`", path.display()))?;
    Ok(tournament)
}
