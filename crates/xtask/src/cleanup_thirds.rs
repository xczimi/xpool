//! One-off cleanup for the best-thirds placement bug.
//!
//! Two logically-separate concerns in one idempotent pass:
//! 1. Force-re-resolve the bracket with the fixed `fwc26::resolve_bracket` and
//!    persist it — premature best-third R32 slots revert to `None` (the gate now
//!    requires all 12 groups final).
//! 2. Unlock any *locked* prediction on a knockout match whose slot is now
//!    unresolved, so those players can re-predict once teams are correctly placed.
//!
//! The unlock criterion is **structural** ("locked prediction on a knockout match
//! with an unresolved team slot"), not a hardcoded match list — so re-running is a
//! no-op once the slots are corrected.
//!
//! Re-resolution uses the result-user's raw predictions. In production the
//! result-user only carries results for games actually played, so this matches the
//! API's as-of-`now` recompute (whose slice is a no-op for real entries).

use anyhow::Context;
use domain::{Player, Round, Tournament};
use std::collections::BTreeSet;
use storage::Repository;

/// Knockout game ids in `t` whose home or away slot is unplaced.
fn unresolved_knockout_games(t: &Tournament) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (id, game) in &t.games {
        let is_knockout = t
            .groups
            .get(&game.group_id)
            .is_some_and(|g| g.round != Round::GroupStage);
        if is_knockout && (game.home.team_id.is_none() || game.away.team_id.is_none()) {
            out.insert(id.clone());
        }
    }
    out
}

/// One prediction the cleanup unlocked (or would unlock in a dry run).
#[derive(Debug, Clone)]
pub struct UnlockRecord {
    pub player_id: String,
    pub nick: String,
    pub game_id: String,
}

/// Outcome of a cleanup run.
#[derive(Debug, Default)]
pub struct CleanupReport {
    pub slots_renulled: usize,
    pub unlocks: Vec<UnlockRecord>,
    pub players_written: usize,
    pub tournament_written: bool,
}

impl CleanupReport {
    pub fn print(&self, applied: bool) {
        let mode = if applied {
            "APPLIED"
        } else {
            "DRY RUN (read-only)"
        };
        println!("== cleanup-best-thirds — {mode} ==");
        println!(
            "knockout slots re-nulled by re-resolve: {}",
            self.slots_renulled
        );
        if self.unlocks.is_empty() {
            println!("locked predictions to unlock: none");
        } else {
            let verb = if applied { "unlocked" } else { "would unlock" };
            println!("locked predictions {verb}: {}", self.unlocks.len());
            for u in &self.unlocks {
                println!("  {} ({}): game {}", u.nick, u.player_id, u.game_id);
            }
        }
        if applied {
            println!(
                "tournament written: {}, players written: {}",
                self.tournament_written, self.players_written
            );
        } else if !self.unlocks.is_empty() || self.slots_renulled > 0 {
            println!("re-run with --apply to write these changes");
        }
    }
}

/// Re-resolve the bracket and write the corrected team slots onto knockout games.
/// Returns the corrected tournament and how many previously-placed knockout slots
/// became `None`.
fn reresolved_tournament(t: &Tournament, result_user: &Player) -> (Tournament, usize) {
    let resolved = fwc26::resolve_bracket(t, result_user);
    let mut next = t.clone();
    let mut renulled = 0usize;
    for (game_id, (home_team, away_team)) in resolved {
        let is_knockout = next
            .games
            .get(&game_id)
            .and_then(|g| next.groups.get(&g.group_id))
            .is_some_and(|grp| grp.round != Round::GroupStage);
        if !is_knockout {
            continue;
        }
        if let Some(game) = next.games.get_mut(&game_id) {
            if game.home.team_id.is_some() && home_team.is_none() {
                renulled += 1;
            }
            if game.away.team_id.is_some() && away_team.is_none() {
                renulled += 1;
            }
            game.home.team_id = home_team;
            game.away.team_id = away_team;
        }
    }
    (next, renulled)
}

/// Run the cleanup. With `apply == false` nothing is written.
pub async fn run<R: Repository>(repo: &R, apply: bool) -> anyhow::Result<CleanupReport> {
    let tournament = repo
        .get_tournament()
        .await?
        .context("no tournament in table — run `xtask import` first")?;
    let players = repo.list_players().await?;
    let result_user = players
        .iter()
        .find(|p| p.is_result_user)
        .context("no result user found — cannot re-resolve")?;

    let (next, renulled) = reresolved_tournament(&tournament, result_user);
    let unresolved = unresolved_knockout_games(&next);

    let mut report = CleanupReport {
        slots_renulled: renulled,
        ..Default::default()
    };

    // Unlock locked predictions on knockout matches now lacking a placed team.
    for p in &players {
        let mut changed = false;
        let new_preds: Vec<_> = p
            .match_predictions
            .iter()
            .map(|mp| {
                if mp.locked && unresolved.contains(&mp.game_id) {
                    report.unlocks.push(UnlockRecord {
                        player_id: p.id.clone(),
                        nick: p.nick.clone(),
                        game_id: mp.game_id.clone(),
                    });
                    changed = true;
                    domain::MatchPrediction {
                        locked: false,
                        ..mp.clone()
                    }
                } else {
                    mp.clone()
                }
            })
            .collect();

        if changed && apply {
            let updated = Player {
                match_predictions: new_preds,
                ..p.clone()
            };
            repo.put_player(&updated).await?;
            report.players_written += 1;
        }
    }

    if apply {
        repo.put_tournament(&next).await?;
        report.tournament_written = true;
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        GroupChildren, GroupGame, GroupId, LockMode, MatchPrediction, Round, SingleGame, TeamSlot,
        Tournament,
    };
    use std::collections::HashMap;
    use storage::InMemoryRepository;

    fn kickoff() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-06-20T18:00:00Z")
            .unwrap()
            .into()
    }

    fn knockout_group(id: &str, game_ids: Vec<String>) -> GroupGame {
        GroupGame {
            id: id.to_string(),
            name: id.to_string(),
            parent: Some("root".to_string()),
            round: Round::R32,
            lock_mode: LockMode::LockPerMatch,
            carries_standings: false,
            children: GroupChildren::Games(game_ids),
        }
    }

    fn root_group(child_ids: Vec<GroupId>) -> GroupGame {
        GroupGame {
            id: "root".to_string(),
            name: "FWC26".to_string(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: false,
            children: GroupChildren::Groups(child_ids),
        }
    }

    /// Build a minimal tournament with one R32 knockout game (M74) whose both
    /// slots are prematurely placed: home = ENG ("1E"), away = PAR ("3ABCDF").
    /// No group-stage groups → resolve_bracket returns None for both slots.
    fn build_premature_tournament() -> Tournament {
        let mut groups: HashMap<GroupId, GroupGame> = HashMap::new();
        let mut games = HashMap::new();

        groups.insert("root".to_string(), root_group(vec!["r32-m74".to_string()]));
        groups.insert(
            "r32-m74".to_string(),
            knockout_group("r32-m74", vec!["M74".to_string()]),
        );

        games.insert(
            "M74".to_string(),
            SingleGame {
                id: "M74".to_string(),
                kickoff: kickoff(),
                venue: None,
                group_id: "r32-m74".to_string(),
                home: TeamSlot {
                    team_id: Some("ENG".to_string()),
                    description: "1E".to_string(),
                },
                away: TeamSlot {
                    team_id: Some("PAR".to_string()),
                    description: "3ABCDF".to_string(),
                },
                external_id: None,
            },
        );

        Tournament {
            root: "root".to_string(),
            groups,
            games,
            teams: HashMap::new(),
        }
    }

    fn result_user() -> Player {
        Player {
            id: "result-user".to_string(),
            person_id: "result-person".to_string(),
            nick: "result".to_string(),
            full_name: "Result User".to_string(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    fn normal_player_with_locked_m74() -> Player {
        Player {
            id: "player-1".to_string(),
            person_id: "person-1".to_string(),
            nick: "alice".to_string(),
            full_name: "Alice".to_string(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: vec![MatchPrediction {
                game_id: "M74".to_string(),
                home_score: 2,
                away_score: 1,
                locked: true,
            }],
            standings_predictions: vec![],
        }
    }

    async fn setup_repo() -> InMemoryRepository {
        let repo = InMemoryRepository::new();
        repo.put_tournament(&build_premature_tournament())
            .await
            .unwrap();
        repo.put_player(&result_user()).await.unwrap();
        repo.put_player(&normal_player_with_locked_m74())
            .await
            .unwrap();
        repo
    }

    #[tokio::test]
    async fn dry_run_writes_nothing_but_reports() {
        let repo = setup_repo().await;

        let report = run(&repo, false).await.unwrap();

        // At least one slot was re-nulled (PAR in "3ABCDF" away, ENG in "1E" home).
        assert!(
            report.slots_renulled >= 1,
            "expected at least one slot renulled, got {}",
            report.slots_renulled
        );
        // The locked M74 prediction would be unlocked.
        assert!(
            !report.unlocks.is_empty(),
            "expected at least one unlock record"
        );
        assert!(report.unlocks.iter().any(|u| u.game_id == "M74"));

        // Nothing was actually written — tournament slots still prematurely placed.
        let t = repo.get_tournament().await.unwrap().unwrap();
        assert_eq!(
            t.games["M74"].away.team_id,
            Some("PAR".to_string()),
            "dry run must not modify tournament"
        );
        assert_eq!(
            t.games["M74"].home.team_id,
            Some("ENG".to_string()),
            "dry run must not modify tournament"
        );

        // Player's prediction still locked.
        let players = repo.list_players().await.unwrap();
        let player = players.iter().find(|p| p.id == "player-1").unwrap();
        let m74_pred = player
            .match_predictions
            .iter()
            .find(|mp| mp.game_id == "M74")
            .unwrap();
        assert!(m74_pred.locked, "dry run must not unlock the prediction");
    }

    #[tokio::test]
    async fn apply_renulls_and_unlocks_then_idempotent() {
        let repo = setup_repo().await;

        // First apply.
        run(&repo, true).await.unwrap();

        // Tournament slots re-nulled.
        let t = repo.get_tournament().await.unwrap().unwrap();
        assert!(
            t.games["M74"].away.team_id.is_none(),
            "apply must null the premature away slot"
        );
        assert!(
            t.games["M74"].home.team_id.is_none(),
            "apply must null the premature home slot"
        );

        // Player's M74 prediction unlocked.
        let players = repo.list_players().await.unwrap();
        let player = players.iter().find(|p| p.id == "player-1").unwrap();
        let m74_pred = player
            .match_predictions
            .iter()
            .find(|mp| mp.game_id == "M74")
            .unwrap();
        assert!(!m74_pred.locked, "apply must unlock the prediction");

        // Second apply — idempotent: no further unlocks or renulls.
        let report2 = run(&repo, true).await.unwrap();
        assert!(
            report2.unlocks.is_empty(),
            "second run must produce no unlock records"
        );
        assert_eq!(
            report2.slots_renulled, 0,
            "second run must report zero renulled slots"
        );
    }
}
