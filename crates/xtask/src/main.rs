//! xtask — the import/seed CLI (`DATA_SOURCES.md` §5, plan task P5).
//!
//! ```text
//! xtask import <path>   load a tournament JSON into the repository
//! xtask seed            create demo players + a result user + a demo pool
//! xtask bootstrap       create only the result-user (production, no demo data)
//! xtask drop-table      drop the DynamoDB table named by XPOOL_TABLE
//! ```
//!
//! `import` and `seed` talk to `DynamoRepository::from_env()` and call
//! `ensure_table()` first. Both are idempotent.
//! `drop-table` also uses `DynamoRepository::from_env()` but does NOT call
//! `ensure_table()` — it drops the table instead.

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
    /// Bootstrap production: seed only the result-user (no demo data).
    Bootstrap,
    /// Drop the DynamoDB table named by XPOOL_TABLE (e2e teardown).
    DropTable,
    /// Seed a generated scenario (full results + ~12 players' predictions).
    Scenario {
        /// Scenario id: `chalk`, `balanced`, or `chaos`.
        id: String,
    },
    /// One-off: fix FWC26 Group G/H standings-prediction mislabels. Idempotent;
    /// a read-only report unless `--apply` is given.
    FixGroupsGh {
        /// Write the relabels. Without this flag the command reports only.
        #[arg(long)]
        apply: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Workspace-root .env (DYNAMO_ENDPOINT, AWS_*, etc.) — silent no-op if missing.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let repo = DynamoRepository::from_env().await?;

    match cli.command {
        Command::Import { path } => {
            repo.ensure_table().await?;
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
            repo.ensure_table().await?;
            xtask::seed::seed(&repo).await?;
            println!("seeded demo data: result user + 6 demo players + 1 demo pool");
        }
        Command::Bootstrap => {
            repo.ensure_table().await?;
            xtask::seed::bootstrap(&repo).await?;
            println!("bootstrapped: result user only (no demo data)");
        }
        Command::DropTable => {
            repo.delete_table().await?;
            println!("dropped table {}", repo.table);
        }
        Command::Scenario { id } => {
            repo.ensure_table().await?;
            let rankings = PathBuf::from(xtask::scenario::DEFAULT_RANKINGS_PATH);
            xtask::scenario::seed_scenario(&repo, &id, &rankings).await?;
            println!(
                "seeded scenario `{id}`: official results + 6 demo + 5 whacky players. \
                 Move the dev clock and call the `devRematerialize` mutation to build the \
                 scoreboard as-of that time."
            );
        }
        Command::FixGroupsGh { apply } => {
            let report = xtask::migrate_gh::run(&repo, apply).await?;
            report.print(apply);
        }
    }

    Ok(())
}
