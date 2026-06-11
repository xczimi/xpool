//! One-off migration: fix FWC26 Group G/H standings-prediction mislabels.
//!
//! The fixture had the Group G and Group H labels swapped (the draw of
//! 5 Dec 2025 put Belgium/Egypt/Iran/New Zealand in G and
//! Spain/Cape Verde/Saudi Arabia/Uruguay in H). Correcting the tournament fixes
//! the games and group nodes, but a player's *standings prediction* is keyed by
//! group letter (`StandingsPrediction.group_id`) with an ordering of team ids —
//! so a prediction saved under "G" still holds the old G team-set and now points
//! at the wrong group.
//!
//! Match (score) predictions need no migration: they are keyed by `game_id`,
//! and the relabel left every match id bound to the same teams, venue and
//! kickoff — only the group label moved.
//!
//! This migration is **content-driven**: it looks at the team-set inside each
//! G/H standings prediction and relabels it to the group those teams actually
//! belong to. That makes it **idempotent** (a correctly-labeled prediction is
//! left alone) and order-independent with respect to the tournament re-import.
//! A prediction whose teams fit neither group cleanly (empty, or a foreign
//! team) is reported as ambiguous and never auto-changed.

use anyhow::Context;
use domain::{Player, StandingsPrediction, Tournament};
use std::collections::BTreeSet;
use storage::Repository;

/// The corrected Group G membership (FIFA final draw, 5 Dec 2025). Source of
/// truth for classifying a mislabeled standings prediction.
pub const CORRECT_G: [&str; 4] = ["BEL", "EGY", "IRN", "NZL"];
/// The corrected Group H membership (FIFA final draw, 5 Dec 2025).
pub const CORRECT_H: [&str; 4] = ["ESP", "CPV", "KSA", "URU"];

/// What to do with one standings prediction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhDecision {
    /// The label already matches the team-set — leave it.
    Consistent,
    /// The team-set belongs to the other group — relabel to `to`.
    Relabel { to: &'static str },
    /// The team-set fits neither G nor H cleanly — report, never auto-change.
    Ambiguous(String),
    /// Not a Group G or H prediction — out of scope.
    OutOfScope,
}

/// Classify a standings prediction by the group letter it is labeled with and
/// the set of team ids it references (its ordering ∪ draw_order).
pub fn classify_standings(group_id: &str, teams: &BTreeSet<String>) -> GhDecision {
    if group_id != "G" && group_id != "H" {
        return GhDecision::OutOfScope;
    }
    if teams.is_empty() {
        return GhDecision::Ambiguous("no teams in ordering/draw_order".to_string());
    }
    let g: BTreeSet<String> = CORRECT_G.iter().map(|s| s.to_string()).collect();
    let h: BTreeSet<String> = CORRECT_H.iter().map(|s| s.to_string()).collect();
    let in_g = teams.is_subset(&g);
    let in_h = teams.is_subset(&h);

    match group_id {
        "G" if in_g => GhDecision::Consistent,
        "G" if in_h => GhDecision::Relabel { to: "H" },
        "H" if in_h => GhDecision::Consistent,
        "H" if in_g => GhDecision::Relabel { to: "G" },
        _ => GhDecision::Ambiguous(format!(
            "team-set {teams:?} is not contained in group G {CORRECT_G:?} or H {CORRECT_H:?}"
        )),
    }
}

/// One relabel the migration made (or would make in a dry run).
#[derive(Debug, Clone)]
pub struct RelabelRecord {
    pub player_id: String,
    pub nick: String,
    pub from: String,
    pub to: String,
    pub ordering: Vec<String>,
}

/// A G/H standings prediction that could not be classified and was left as-is.
#[derive(Debug, Clone)]
pub struct AmbiguousRecord {
    pub player_id: String,
    pub nick: String,
    pub group_id: String,
    pub ordering: Vec<String>,
    pub reason: String,
}

/// Outcome of a migration run.
#[derive(Debug, Default)]
pub struct GhReport {
    pub players_scanned: usize,
    pub relabels: Vec<RelabelRecord>,
    pub ambiguous: Vec<AmbiguousRecord>,
    pub players_written: usize,
}

impl GhReport {
    /// Print a human-readable summary. `applied` distinguishes a real run from a
    /// dry run in the wording.
    pub fn print(&self, applied: bool) {
        let mode = if applied {
            "APPLIED"
        } else {
            "DRY RUN (read-only)"
        };
        println!("== fix-groups-gh — {mode} ==");
        println!("players scanned: {}", self.players_scanned);

        if self.relabels.is_empty() {
            println!("standings predictions to relabel: none");
        } else {
            let verb = if applied {
                "relabeled"
            } else {
                "would relabel"
            };
            println!("standings predictions {verb}: {}", self.relabels.len());
            for r in &self.relabels {
                println!(
                    "  {} ({}): {} -> {}  [{}]",
                    r.nick,
                    r.player_id,
                    r.from,
                    r.to,
                    r.ordering.join(", ")
                );
            }
        }

        if !self.ambiguous.is_empty() {
            println!(
                "ambiguous (left unchanged — needs manual review): {}",
                self.ambiguous.len()
            );
            for a in &self.ambiguous {
                println!(
                    "  {} ({}): group {} [{}] — {}",
                    a.nick,
                    a.player_id,
                    a.group_id,
                    a.ordering.join(", "),
                    a.reason
                );
            }
        }

        if applied {
            println!("players written: {}", self.players_written);
        } else if !self.relabels.is_empty() {
            println!("re-run with --apply to write these changes");
        }
    }
}

/// Team ids appearing on the games of a group node (direct membership).
fn group_team_set(t: &Tournament, group_id: &str) -> BTreeSet<String> {
    t.games_in(group_id)
        .iter()
        .flat_map(|g| [g.home.team_id.clone(), g.away.team_id.clone()])
        .flatten()
        .collect()
}

/// Run the migration. With `apply == false` nothing is written — the returned
/// report is the read-only check. The pre-flight guard refuses to run unless the
/// tournament in the table is already in the corrected G/H state, so predictions
/// are never migrated against a stale (still-swapped) tournament.
pub async fn run<R: Repository>(repo: &R, apply: bool) -> anyhow::Result<GhReport> {
    let tournament = repo
        .get_tournament()
        .await?
        .context("no tournament in table — run `xtask import tournaments/fwc26.json` first")?;

    let want_g: BTreeSet<String> = CORRECT_G.iter().map(|s| s.to_string()).collect();
    let want_h: BTreeSet<String> = CORRECT_H.iter().map(|s| s.to_string()).collect();
    let have_g = group_team_set(&tournament, "G");
    let have_h = group_team_set(&tournament, "H");
    if have_g != want_g || have_h != want_h {
        anyhow::bail!(
            "tournament groups are not in the corrected state (G={have_g:?}, H={have_h:?}); \
             run `xtask import tournaments/fwc26.json` with the corrected fixture before \
             migrating standings predictions"
        );
    }

    let players = repo.list_players().await?;
    let mut report = GhReport {
        players_scanned: players.len(),
        ..Default::default()
    };

    for p in &players {
        let mut changed = false;
        let mut new_standings = Vec::with_capacity(p.standings_predictions.len());

        for prediction in &p.standings_predictions {
            let teams: BTreeSet<String> = prediction
                .ordering
                .iter()
                .chain(prediction.draw_order.iter())
                .cloned()
                .collect();

            match classify_standings(&prediction.group_id, &teams) {
                GhDecision::Relabel { to } => {
                    report.relabels.push(RelabelRecord {
                        player_id: p.id.clone(),
                        nick: p.nick.clone(),
                        from: prediction.group_id.clone(),
                        to: to.to_string(),
                        ordering: prediction.ordering.clone(),
                    });
                    new_standings.push(StandingsPrediction {
                        group_id: to.to_string(),
                        ..prediction.clone()
                    });
                    changed = true;
                }
                GhDecision::Ambiguous(reason) => {
                    report.ambiguous.push(AmbiguousRecord {
                        player_id: p.id.clone(),
                        nick: p.nick.clone(),
                        group_id: prediction.group_id.clone(),
                        ordering: prediction.ordering.clone(),
                        reason,
                    });
                    new_standings.push(prediction.clone());
                }
                GhDecision::Consistent | GhDecision::OutOfScope => {
                    new_standings.push(prediction.clone());
                }
            }
        }

        if changed && apply {
            let updated = Player {
                standings_predictions: new_standings,
                ..p.clone()
            };
            repo.put_player(&updated).await?;
            report.players_written += 1;
        }
    }

    Ok(report)
}
