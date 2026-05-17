//! xtask — the import/seed CLI (`DATA_SOURCES.md` §5, plan task P5).
//!
//! ```text
//! xtask import <path>   load a tournament JSON into the repository
//! xtask seed            create demo players + a result user + a demo pool
//! ```
//!
//! Both subcommands talk to `DynamoRepository::from_env()` and call
//! `ensure_table()` first. Both are idempotent.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use storage::{DynamoRepository, Repository};

#[derive(Parser)]
#[command(name = "xtask", about = "xpool import / seed CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import a tournament JSON file into the repository.
    Import {
        /// Path to the tournament JSON (e.g. tournaments/fwc26.json).
        path: PathBuf,
    },
    /// Seed demo data (result user, demo players, a demo pool).
    Seed,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let repo = DynamoRepository::from_env().await?;
    repo.ensure_table().await?;

    match cli.command {
        Command::Import { path } => {
            let tournament = xtask::load_tournament(&path)?;
            repo.put_tournament(&tournament).await?;
            println!(
                "imported tournament: {} groups, {} games, {} teams",
                tournament.groups.len(),
                tournament.games.len(),
                tournament.teams.len()
            );
        }
        Command::Seed => {
            xtask::seed::seed(&repo).await?;
            println!("seeded demo data: result user + 6 demo players + 1 demo pool");
        }
    }

    Ok(())
}
