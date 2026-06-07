//! The post-result hook (`SCORING.md` §8, `DATA_SOURCES.md` §5).
//!
//! After the result user's `submitGroup` mutates their predictions, the whole
//! derived state is rebuilt wholesale — no per-node cache, no invalidation
//! cascade:
//!
//! 1. `domain::score_tournament` for every real player vs the result user →
//!    write the materialised `Scoreboard`.
//! 2. `fwc26::resolve_bracket` → write resolved team slots back onto the
//!    tournament's knockout games.
//!
//! Both are pure-function calls; this module is glue only.

use chrono::{DateTime, Utc};
use domain::scoring::{score_tournament, ScoringConfig};
use storage::{Repository, Scoreboard};

/// Run the wholesale recompute. Loads the coarse items, calls the pure
/// `domain`/`fwc26` functions, and writes back the `Scoreboard` and the
/// bracket-resolved `Tournament`.
pub async fn recompute(repo: &dyn Repository, now: DateTime<Utc>) -> anyhow::Result<()> {
    let players = repo.list_players().await?;

    let result_user = players
        .iter()
        .find(|p| p.is_result_user)
        .ok_or_else(|| anyhow::anyhow!("no result user found — cannot recompute"))?;

    let tournament = repo
        .get_tournament()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no tournament loaded — cannot recompute"))?;

    // 1. Scoreboard: score every real player vs the result user.
    let config = ScoringConfig::default();
    let mut scoreboard = Scoreboard::default();
    for player in &players {
        if player.is_result_user {
            continue;
        }
        let breakdown = score_tournament(&tournament, player, result_user, now, &config);
        scoreboard.entries.insert(player.id.clone(), breakdown);
    }
    repo.put_scoreboard(&scoreboard).await?;

    // 2. Bracket resolution: write resolved team ids onto knockout games only.
    // Group-stage games carry fixed team ids from the JSON and must never be
    // touched — their slot descriptions ("A1") are not knockout grammar and
    // would resolve to `None`, wiping the real teams.
    let resolved = fwc26::resolve_bracket(&tournament, result_user);
    let mut next = tournament.clone();
    for (game_id, (home_team, away_team)) in resolved {
        let is_knockout = next
            .games
            .get(&game_id)
            .and_then(|g| next.groups.get(&g.group_id))
            .is_some_and(|grp| grp.round != domain::Round::GroupStage);
        if !is_knockout {
            continue;
        }
        if let Some(game) = next.games.get_mut(&game_id) {
            game.home.team_id = home_team;
            game.away.team_id = away_team;
        }
    }
    repo.put_tournament(&next).await?;

    Ok(())
}
