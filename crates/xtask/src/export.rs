//! Bulk export / import of a table — the prod→local data pull.
//!
//! `export` scans a (read-only) source table into a [`Snapshot`] JSON file,
//! anonymising e-mails by default. `load` writes a snapshot into a target table
//! unconditionally, so a re-load is idempotent. Both talk to a
//! [`DynamoRepository`]; the source/target is chosen by that repo's env
//! (`XPOOL_TABLE`, `DYNAMO_ENDPOINT`).

use crate::snapshot::{anonymize_emails, Snapshot};
use anyhow::Context;
use std::path::Path;
use storage::DynamoRepository;

/// Scan the repository's table and write a [`Snapshot`] JSON file.
///
/// Read-only on the source — it never creates or mutates the table. When
/// `anonymize` is set (the default for prod pulls) e-mails are remapped to
/// `<nick>@dev.invalid` before writing.
pub async fn export(
    repo: &DynamoRepository,
    output: &Path,
    anonymize: bool,
) -> anyhow::Result<ExportSummary> {
    let rows = repo
        .scan_all()
        .await
        .with_context(|| format!("scanning table `{}`", repo.table))?;
    let snapshot = Snapshot::new(rows);
    let snapshot = if anonymize {
        anonymize_emails(snapshot)
    } else {
        snapshot
    };

    let json = serde_json::to_string_pretty(&snapshot).context("serialising snapshot")?;
    std::fs::write(output, json).with_context(|| format!("writing `{}`", output.display()))?;

    Ok(ExportSummary::of(&snapshot))
}

/// Read a [`Snapshot`] JSON file and write every row into the repository's
/// table, unconditionally (idempotent overwrite). Ensures the table exists
/// first, so it works against a fresh local/dev table.
pub async fn load(repo: &DynamoRepository, input: &Path) -> anyhow::Result<ExportSummary> {
    let json =
        std::fs::read_to_string(input).with_context(|| format!("reading `{}`", input.display()))?;
    let snapshot: Snapshot =
        serde_json::from_str(&json).with_context(|| format!("parsing `{}`", input.display()))?;

    repo.ensure_table().await?;
    for item in &snapshot.items {
        repo.put_raw(item).await?;
    }

    Ok(ExportSummary::of(&snapshot))
}

/// Row counts by kind, for a human-readable one-liner after export/load.
pub struct ExportSummary {
    pub total: usize,
    pub players: usize,
    pub pools: usize,
    pub invites: usize,
    pub identities: usize,
    pub persons: usize,
}

impl ExportSummary {
    fn of(snapshot: &Snapshot) -> Self {
        let count = |pred: &dyn Fn(&str) -> bool| snapshot.rows_where(pred).count();
        Self {
            total: snapshot.items.len(),
            players: count(&|pk| pk.ends_with("#PLAYER")),
            pools: count(&|pk| pk.ends_with("#POOL")),
            invites: count(&|pk| pk.ends_with("#INVITE")),
            identities: count(&|pk| pk.starts_with("IDENTITY#")),
            persons: count(&|pk| pk.starts_with("PERSON#")),
        }
    }
}

impl std::fmt::Display for ExportSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} rows ({} players, {} pools, {} invites, {} identities, {} persons)",
            self.total, self.players, self.pools, self.invites, self.identities, self.persons
        )
    }
}
