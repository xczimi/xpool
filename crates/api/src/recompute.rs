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
//! Both run against an **as-of `now` projection** of the result user
//! (`slice_result_as_of`): only matches played by `now` (and standings for
//! fully-played groups) count, so the scoreboard + bracket materialise
//! correctly for the request clock. For normal incremental entry this is a
//! no-op (a result is entered after its match is played); it is load-bearing
//! when a full-tournament scenario seed is inspected at an earlier dev clock.
//!
//! Both are pure-function calls; this module is glue only.

use chrono::{DateTime, Utc};
use domain::scoring::{score_tournament, ScoringConfig};
use domain::{GroupChildren, Player, Round, Tournament};
use storage::{Repository, Scoreboard};

/// Has a game been played as-of `now`? True once `now` passes
/// `kickoff + result_buffer(round)` — the inverse of `result_pending`.
fn game_played(t: &Tournament, game_id: &str, now: DateTime<Utc>) -> bool {
    t.games.get(game_id).is_some_and(|g| {
        let round = t
            .groups
            .get(&g.group_id)
            .map(|gr| gr.round)
            .unwrap_or(Round::GroupStage);
        now > g.kickoff + crate::timeflags::result_buffer(round)
    })
}

/// Are all of a leaf group's games played as-of `now`? Internal nodes (which do
/// not carry sliceable standings here) return false.
fn group_complete(t: &Tournament, group_id: &str, now: DateTime<Utc>) -> bool {
    match t.groups.get(group_id).map(|g| &g.children) {
        Some(GroupChildren::Games(ids)) => ids.iter().all(|id| game_played(t, id, now)),
        _ => false,
    }
}

/// Project the result-user onto "what has actually happened by `now`": keep
/// match results only for played games, and standings only for fully-played
/// groups. For real production entries this is a no-op (unplayed games carry no
/// entered result), so it is safe to apply unconditionally and makes the
/// materialised scoreboard/bracket correct as-of any clock.
fn slice_result_as_of(t: &Tournament, result: &Player, now: DateTime<Utc>) -> Player {
    let match_predictions = result
        .match_predictions
        .iter()
        .filter(|mp| game_played(t, &mp.game_id, now))
        .cloned()
        .collect();
    let standings_predictions = result
        .standings_predictions
        .iter()
        .filter(|sp| group_complete(t, &sp.group_id, now))
        .cloned()
        .collect();
    Player {
        match_predictions,
        standings_predictions,
        ..result.clone()
    }
}

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

    // Project the result-user onto matches played as-of `now`, so the scoreboard
    // and bracket materialise correctly for the requested clock (no-op for real
    // post-result entries — see `slice_result_as_of`).
    let sliced = slice_result_as_of(&tournament, result_user, now);

    // 1. Scoreboard: score every real player vs the (sliced) result user.
    let config = ScoringConfig::default();
    let mut scoreboard = Scoreboard::default();
    for player in &players {
        if player.is_result_user {
            continue;
        }
        let breakdown = score_tournament(&tournament, player, &sliced, now, &config);
        scoreboard.entries.insert(player.id.clone(), breakdown);
    }
    repo.put_scoreboard(&scoreboard).await?;

    // 2. Bracket resolution: write resolved team ids onto knockout games only.
    // Group-stage games carry fixed team ids from the JSON and must never be
    // touched — their slot descriptions ("A1") are not knockout grammar and
    // would resolve to `None`, wiping the real teams.
    let resolved = fwc26::resolve_bracket(&tournament, &sliced);
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame,
        StandingsPrediction, Team, TeamSlot, Tournament,
    };
    use std::collections::HashMap;

    fn at(y: i32, mo: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, 0, 0).unwrap()
    }

    // Two group-stage games in group A, kickoffs a day apart.
    fn fixture() -> (Tournament, Player) {
        let team = |id: &str| Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        };
        let g1 = SingleGame {
            id: "M1".into(),
            kickoff: at(2026, 6, 11, 19),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot {
                team_id: Some("AAA".into()),
                description: "A1".into(),
            },
            away: TeamSlot {
                team_id: Some("BBB".into()),
                description: "A2".into(),
            },
            external_id: None,
        };
        let g2 = SingleGame {
            id: "M2".into(),
            kickoff: at(2026, 6, 13, 19),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot {
                team_id: Some("AAA".into()),
                description: "A1".into(),
            },
            away: TeamSlot {
                team_id: Some("BBB".into()),
                description: "A2".into(),
            },
            external_id: None,
        };
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into(), "M2".into()]),
        };
        let t = Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), g1), ("M2".to_string(), g2)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let result = Player {
            id: "result-user".into(),
            person_id: "p".into(),
            nick: "official".into(),
            full_name: "Official".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![
                MatchPrediction {
                    game_id: "M1".into(),
                    home_score: 1,
                    away_score: 0,
                    locked: false,
                },
                MatchPrediction {
                    game_id: "M2".into(),
                    home_score: 2,
                    away_score: 0,
                    locked: false,
                },
            ],
            standings_predictions: vec![StandingsPrediction {
                group_id: "A".into(),
                ordering: vec!["AAA".into(), "BBB".into()],
                draw_order: vec!["AAA".into(), "BBB".into()],
                locked: false,
            }],
        };
        (t, result)
    }

    #[test]
    fn slice_keeps_only_played_matches() {
        let (t, result) = fixture();
        // Between the two kickoffs (after M1 + buffer, before M2): only M1 in.
        let now = at(2026, 6, 12, 12);
        let sliced = slice_result_as_of(&t, &result, now);
        assert_eq!(sliced.match_predictions.len(), 1);
        assert_eq!(sliced.match_predictions[0].game_id, "M1");
        // Group is not complete → standings dropped.
        assert!(sliced.standings_predictions.is_empty());
    }

    #[test]
    fn slice_keeps_everything_once_group_is_complete() {
        let (t, result) = fixture();
        let now = at(2026, 6, 20, 12); // after both kickoffs + buffer
        let sliced = slice_result_as_of(&t, &result, now);
        assert_eq!(sliced.match_predictions.len(), 2);
        assert_eq!(sliced.standings_predictions.len(), 1);
    }

    #[test]
    fn slice_is_noop_before_anything_is_played() {
        let (t, result) = fixture();
        let now = at(2026, 6, 1, 12); // before the tournament
        let sliced = slice_result_as_of(&t, &result, now);
        assert!(sliced.match_predictions.is_empty());
        assert!(sliced.standings_predictions.is_empty());
    }
}
