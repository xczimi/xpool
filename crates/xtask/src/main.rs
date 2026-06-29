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
    /// Export the whole table to a snapshot JSON file (the prod→local data
    /// pull). Read-only on the source; e-mails are anonymised to
    /// `<nick>@dev.invalid` by default.
    Export {
        /// Output path for the snapshot JSON (e.g. .scratch/prod-snapshot.json).
        output: PathBuf,
        /// Keep real e-mails verbatim instead of anonymising. Only for a
        /// same-environment backup — never write prod e-mails to disk.
        #[arg(long)]
        raw_emails: bool,
    },
    /// Load a snapshot JSON file into the table named by XPOOL_TABLE
    /// (unconditional overwrite, idempotent). Creates the table if missing.
    Load {
        /// Path to a snapshot JSON produced by `export`.
        input: PathBuf,
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
    /// One-off: clean up the best-thirds placement bug. Re-resolves the bracket
    /// (re-nulls premature best-third R32 slots) and unlocks locked predictions on
    /// knockout matches whose teams are no longer placed. Idempotent; a read-only
    /// report unless `--apply` is given.
    CleanupBestThirds {
        /// Write the changes. Without this flag the command reports only.
        #[arg(long)]
        apply: bool,
    },
    /// Reconcile xpool games against TheSportsDB and print proposed
    /// `M# -> idEvent` mappings. Read-only by default: prints a table for the
    /// human to paste into `tournaments/fwc26.json`. With `--apply` it writes,
    /// in place and non-destructively: the matched `external_id`s on every game,
    /// and the real `kickoff` on **knockout** games (broadcast scheduling shifts
    /// those after the draw, breaking the live-score window + the per-match
    /// lock). Resolved knockout team slots are preserved, so it is safe to run
    /// mid-tournament and is idempotent — re-run it as rounds resolve or times
    /// move. Requires THESPORTSDB_API_KEY.
    ReconcileEvents {
        /// Write the matched `external_id`s + corrected knockout kickoffs to the
        /// table. Without this flag the command reports only.
        #[arg(long)]
        apply: bool,
    },
    /// Run a deadline-reminder sweep against the configured table + mail
    /// transport (local: MailHog SMTP). Honours XPOOL_NOW. This is how the
    /// scheduled Lambda path is exercised locally.
    SendReminders {
        /// `last-call` (hourly) or `digest` (daily matchday).
        #[arg(long, default_value = "last-call")]
        mode: String,
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
        Command::Export { output, raw_emails } => {
            let summary = xtask::export::export(&repo, &output, !raw_emails).await?;
            let mode = if raw_emails {
                "raw e-mails"
            } else {
                "e-mails anonymised"
            };
            println!(
                "exported {} from `{}` to `{}` ({mode})",
                summary,
                repo.table,
                output.display()
            );
        }
        Command::Load { input } => {
            let summary = xtask::export::load(&repo, &input).await?;
            println!("loaded {} into `{}`", summary, repo.table);
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
        Command::CleanupBestThirds { apply } => {
            let report = xtask::cleanup_thirds::run(&repo, apply).await?;
            report.print(apply);
        }
        Command::ReconcileEvents { apply } => {
            let client = sportsdb::SportsDb::from_env()
                .ok_or_else(|| anyhow::anyhow!("THESPORTSDB_API_KEY not set"))?;
            let tournament = repo
                .get_tournament()
                .await?
                .ok_or_else(|| anyhow::anyhow!("no tournament loaded — run `import` first"))?;
            let events = client.season_schedule().await?;
            let team_rows = client.teams().await?;

            // Build our_teams: (our_team_id, our_name, committed_external_id)
            let our_teams: Vec<(String, String, Option<String>)> = tournament
                .teams
                .values()
                .map(|t| (t.id.clone(), t.name.clone(), t.external_id.clone()))
                .collect();

            let (team_ext, unresolved_teams) =
                xtask::reconcile::resolve_team_ids(&our_teams, &team_rows);

            let games: Vec<xtask::reconcile::GameStub> = tournament
                .games
                .values()
                .map(|g| xtask::reconcile::GameStub {
                    game_id: g.id.clone(),
                    kickoff: g.kickoff,
                    home_team_id: g.home.team_id.clone(),
                    away_team_id: g.away.team_id.clone(),
                })
                .collect();

            let report = xtask::reconcile::reconcile(&games, &team_ext, &events);
            println!(
                "# Proposed game -> idEvent mappings ({} matched):",
                report.matched.len()
            );
            let mut matched = report.matched;
            matched.sort_by(|a, b| a.game_id.cmp(&b.game_id));
            for m in &matched {
                println!("{}\t{}", m.game_id, m.id_event);
            }
            if !report.unmatched_games.is_empty() {
                let mut un = report.unmatched_games;
                un.sort();
                eprintln!("# Unmatched ({}): {}", un.len(), un.join(", "));
            }
            eprintln!(
                "# Unresolved teams ({}): {}",
                unresolved_teams.len(),
                unresolved_teams.join(", ")
            );
            if apply {
                let (next, report) = xtask::reconcile::apply_matches(&tournament, &matched);
                for c in &report.kickoff_changes {
                    println!("# kickoff {}: {} -> {}", c.game_id, c.from, c.to);
                }
                repo.put_tournament(&next).await?;
                println!(
                    "# Applied: wrote {} external_id(s) and corrected {} knockout kickoff(s) \
                     in the table (idempotent).",
                    report.external_id_changed,
                    report.kickoff_changes.len()
                );
                println!(
                    "# Also paste the mappings + corrected kickoffs into tournaments/fwc26.json \
                     so they survive a future re-import."
                );
            } else {
                println!(
                    "# Review, then re-run with --apply to write them to the table, and \
                     paste them into tournaments/fwc26.json."
                );
            }
        }
        Command::SendReminders { mode } => {
            let mode_label = mode.clone();
            let mode = mail::ReminderMode::parse(&mode)
                .ok_or_else(|| anyhow::anyhow!("unknown mode `{mode}` (use last-call|digest)"))?;
            let mail_sender = mail::build_sender_from_env().await?;
            let now = mail::now_from_env();
            let repo: std::sync::Arc<dyn Repository> = std::sync::Arc::new(repo);
            let summary = match mode {
                mail::ReminderMode::LastCall => {
                    mail::run_last_call_sweep(repo.as_ref(), mail_sender.as_ref(), now).await?
                }
                mail::ReminderMode::Digest => {
                    mail::run_digest_sweep(repo.as_ref(), mail_sender.as_ref(), now).await?
                }
            };
            println!(
                "reminders ({mode_label}): {} recipients, {} sent, {} skipped (no email), {} deduped",
                summary.recipients, summary.sent, summary.skipped_no_email, summary.deduped
            );
        }
    }

    Ok(())
}
