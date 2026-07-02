//! `unlock <game> [--player <id>]` — clear the `locked` flag on a single match
//! prediction so it can be re-entered.
//!
//! Official results are the result-user's predictions, entered via `submitGroup`
//! with `lock: true`. Once a prediction is locked, `submit_group` refuses **any**
//! overwrite — even one that would only unlock it — so a wrong locked result has
//! no in-app fix (`crates/api/src/gql/mutation.rs`, "already locked and cannot be
//! changed"). This flips that one prediction's `locked` back to `false`; the
//! admin then re-enters the correct score through the normal flow. Defaults to
//! the result-user (`--player` targets any player).
//!
//! The score is left untouched — unlocking is orthogonal to correcting the
//! value, and `locked` does not gate scoring (`recompute::slice_result_as_of`
//! slices the result-user by `game_played`, not `locked`).
//!
//! Idempotent; a read-only report unless `--apply`.

use anyhow::Context;
use domain::{MatchPrediction, Player};
use storage::Repository;

/// What the target prediction's state implies for the run.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The prediction was locked; unlocked now (`--apply`) or would be (dry run).
    /// Carries the preserved score for the report.
    Unlocked { home_score: u8, away_score: u8 },
    /// The prediction exists but is already unlocked — nothing to do.
    AlreadyUnlocked,
    /// The player has no prediction for that game — nothing to do.
    NoPrediction,
}

/// Outcome of an unlock run.
#[derive(Debug)]
pub struct UnlockReport {
    pub player_id: String,
    pub nick: String,
    pub game_id: String,
    pub outcome: Outcome,
    pub applied: bool,
}

impl UnlockReport {
    pub fn print(&self) {
        let mode = if self.applied {
            "APPLIED"
        } else {
            "DRY RUN (read-only)"
        };
        println!(
            "== unlock {} on player {} ({}) — {mode} ==",
            self.game_id, self.player_id, self.nick
        );
        match &self.outcome {
            Outcome::NoPrediction => {
                println!(
                    "player has no prediction for {} — nothing to do",
                    self.game_id
                );
            }
            Outcome::AlreadyUnlocked => {
                println!(
                    "prediction for {} is already unlocked — nothing to do",
                    self.game_id
                );
            }
            Outcome::Unlocked {
                home_score,
                away_score,
            } => {
                let verb = if self.applied {
                    "unlocked"
                } else {
                    "would unlock"
                };
                println!(
                    "{verb} {} (score {home_score}-{away_score} preserved)",
                    self.game_id
                );
                if !self.applied {
                    println!(
                        "re-run with --apply to unlock — then re-enter the correct result via submitGroup"
                    );
                }
            }
        }
    }
}

/// Load `player_id`, inspect its prediction for `game_id`, and (when `apply` and
/// the prediction is locked) write it back with `locked = false`. The rewrite is
/// immutable — a fresh predictions vec with the one entry replaced — and goes
/// through `put_player`, whose optimistic-concurrency guard (`version`) protects
/// against a concurrent write.
pub async fn run<R: Repository>(
    repo: &R,
    player_id: &str,
    game_id: &str,
    apply: bool,
) -> anyhow::Result<UnlockReport> {
    let player = repo
        .get_player(player_id)
        .await
        .with_context(|| format!("loading player `{player_id}`"))?
        .ok_or_else(|| anyhow::anyhow!("no player with id `{player_id}`"))?;

    let outcome = match player
        .match_predictions
        .iter()
        .find(|p| p.game_id == game_id)
    {
        None => Outcome::NoPrediction,
        Some(p) if !p.locked => Outcome::AlreadyUnlocked,
        Some(p) => Outcome::Unlocked {
            home_score: p.home_score,
            away_score: p.away_score,
        },
    };

    if apply && matches!(outcome, Outcome::Unlocked { .. }) {
        let match_predictions = player
            .match_predictions
            .iter()
            .map(|p| {
                if p.game_id == game_id {
                    MatchPrediction {
                        locked: false,
                        ..p.clone()
                    }
                } else {
                    p.clone()
                }
            })
            .collect();
        let updated = Player {
            match_predictions,
            ..player.clone()
        };
        repo.put_player(&updated)
            .await
            .with_context(|| format!("unlocking {game_id} on player `{player_id}`"))?;
    }

    Ok(UnlockReport {
        player_id: player.id.clone(),
        nick: player.nick.clone(),
        game_id: game_id.to_string(),
        outcome,
        applied: apply,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{MatchPrediction, Player};
    use storage::InMemoryRepository;

    fn pred(game_id: &str, locked: bool) -> MatchPrediction {
        MatchPrediction {
            game_id: game_id.into(),
            home_score: 2,
            away_score: 0,
            locked,
        }
    }

    fn player(preds: Vec<MatchPrediction>) -> Player {
        Player {
            id: "result-user".into(),
            person_id: "person-r".into(),
            nick: "official".into(),
            full_name: "official".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: preds,
            standings_predictions: vec![],
        }
    }

    async fn repo_with(p: Player) -> InMemoryRepository {
        let repo = InMemoryRepository::new();
        repo.put_player(&p).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn dry_run_reports_but_writes_nothing() {
        let repo = repo_with(player(vec![pred("M84", true)])).await;

        let report = run(&repo, "result-user", "M84", false).await.unwrap();

        assert_eq!(
            report.outcome,
            Outcome::Unlocked {
                home_score: 2,
                away_score: 0
            }
        );
        assert!(!report.applied);
        // Still locked — dry run wrote nothing.
        let stored = repo.get_player("result-user").await.unwrap().unwrap();
        assert!(stored.match_predictions[0].locked);
    }

    #[tokio::test]
    async fn apply_unlocks_then_idempotent() {
        let repo = repo_with(player(vec![pred("M84", true)])).await;

        let report = run(&repo, "result-user", "M84", true).await.unwrap();
        assert!(matches!(report.outcome, Outcome::Unlocked { .. }));
        assert!(report.applied);

        let stored = repo.get_player("result-user").await.unwrap().unwrap();
        let m84 = &stored.match_predictions[0];
        assert!(!m84.locked, "apply must clear the lock");
        // Score preserved — unlock does not change the value.
        assert_eq!((m84.home_score, m84.away_score), (2, 0));

        // Second run finds it already unlocked and is a no-op.
        let report2 = run(&repo, "result-user", "M84", true).await.unwrap();
        assert_eq!(report2.outcome, Outcome::AlreadyUnlocked);
    }

    #[tokio::test]
    async fn already_unlocked_is_a_clean_no_op() {
        let repo = repo_with(player(vec![pred("M84", false)])).await;

        let report = run(&repo, "result-user", "M84", true).await.unwrap();

        assert_eq!(report.outcome, Outcome::AlreadyUnlocked);
    }

    #[tokio::test]
    async fn missing_prediction_is_a_clean_no_op() {
        let repo = repo_with(player(vec![pred("M84", true)])).await;

        let report = run(&repo, "result-user", "M99", true).await.unwrap();

        assert_eq!(report.outcome, Outcome::NoPrediction);
        // The unrelated locked prediction is untouched.
        let stored = repo.get_player("result-user").await.unwrap().unwrap();
        assert!(stored.match_predictions[0].locked);
    }

    #[tokio::test]
    async fn only_the_target_game_is_unlocked() {
        let repo = repo_with(player(vec![
            pred("M83", true),
            pred("M84", true),
            pred("M85", true),
        ]))
        .await;

        run(&repo, "result-user", "M84", true).await.unwrap();

        let stored = repo.get_player("result-user").await.unwrap().unwrap();
        let locked = |g: &str| {
            stored
                .match_predictions
                .iter()
                .find(|p| p.game_id == g)
                .unwrap()
                .locked
        };
        assert!(locked("M83"), "sibling prediction stays locked");
        assert!(!locked("M84"), "only the target is unlocked");
        assert!(locked("M85"), "sibling prediction stays locked");
    }

    #[tokio::test]
    async fn unknown_player_is_an_error() {
        let repo = repo_with(player(vec![pred("M84", true)])).await;

        let err = run(&repo, "nobody", "M84", true).await.unwrap_err();

        assert!(err.to_string().contains("nobody"));
    }
}
