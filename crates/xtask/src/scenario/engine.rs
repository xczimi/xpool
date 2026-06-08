//! Forward-simulate a coherent outcome-set from a `ScorelinePolicy`. Every
//! knockout pairing is derived from the predictor's own earlier results via the
//! same `fwc26::resolve_bracket` / `domain::rank_group` the live app uses.

use crate::scenario::policy::{GameContext, ScorelinePolicy};
use crate::scenario::ranking::Ranking;
use domain::{
    GroupChildren, MatchPrediction, Player, Round, SingleGame, StandingsPrediction, TeamId,
    Tournament,
};
use std::collections::HashMap;

/// Rounds in the only valid simulation order.
const ROUND_ORDER: [Round; 7] = [
    Round::GroupStage,
    Round::R32,
    Round::R16,
    Round::QF,
    Round::SF,
    Round::ThirdPlace,
    Round::Final,
];

/// A complete generated outcome-set.
pub struct Outcome {
    pub match_predictions: Vec<MatchPrediction>,
    pub standings_predictions: Vec<StandingsPrediction>,
    /// The concrete teams the engine used for each game — for the coherence
    /// round-trip test and debugging.
    pub resolved_teams: HashMap<String, (TeamId, TeamId)>,
}

/// A throwaway result-user-shaped player carrying the predictions so far, so
/// `resolve_bracket` can resolve the next round.
fn interim_player(mps: &[MatchPrediction], sps: &[StandingsPrediction]) -> Player {
    Player {
        id: "__gen".into(),
        person_id: String::new(),
        nick: String::new(),
        full_name: String::new(),
        referrer: None,
        is_result_user: true,
        version: 0,
        match_predictions: mps.to_vec(),
        standings_predictions: sps.to_vec(),
    }
}

/// All `SingleGame`s belonging to a leaf group (`GroupChildren::Games`).
fn group_games<'a>(t: &'a Tournament, group_id: &str) -> Vec<&'a SingleGame> {
    match t.groups.get(group_id).map(|g| &g.children) {
        Some(GroupChildren::Games(ids)) => ids.iter().filter_map(|id| t.games.get(id)).collect(),
        _ => Vec::new(),
    }
}

/// A group's teams sorted by strength descending (stable) — a total draw_order
/// so ranking and bracket resolution are never ambiguous.
fn draw_order_by_strength(team_ids: &[TeamId], ranking: &Ranking) -> Vec<TeamId> {
    let mut ids = team_ids.to_vec();
    ids.sort_by_key(|b| std::cmp::Reverse(ranking.strength(b)));
    ids
}

/// Forward-simulate the whole tournament under `policy`.
pub fn generate(t: &Tournament, ranking: &Ranking, policy: &mut dyn ScorelinePolicy) -> Outcome {
    let mut mps: Vec<MatchPrediction> = Vec::new();
    let mut sps: Vec<StandingsPrediction> = Vec::new();
    let mut resolved_teams: HashMap<String, (TeamId, TeamId)> = HashMap::new();

    for round in ROUND_ORDER {
        // Games in this round, ordered deterministically (kickoff, then id).
        let mut games: Vec<&SingleGame> = t
            .games
            .values()
            .filter(|g| t.groups.get(&g.group_id).map(|gr| gr.round) == Some(round))
            .collect();
        games.sort_by(|a, b| a.kickoff.cmp(&b.kickoff).then_with(|| a.id.cmp(&b.id)));

        // Resolve knockout teams for this round from results so far.
        let resolved = if round == Round::GroupStage {
            HashMap::new()
        } else {
            let interim = interim_player(&mps, &sps);
            fwc26::resolve_bracket(t, &interim)
        };

        for game in &games {
            let (home, away) = if round == Round::GroupStage {
                (
                    game.home.team_id.clone().expect("group-stage home team"),
                    game.away.team_id.clone().expect("group-stage away team"),
                )
            } else {
                let (h, a) = resolved.get(&game.id).cloned().unwrap_or((None, None));
                (
                    h.expect("knockout home resolved by now"),
                    a.expect("knockout away resolved by now"),
                )
            };

            let ctx = GameContext {
                home: home.clone(),
                away: away.clone(),
                home_strength: ranking.strength(&home),
                away_strength: ranking.strength(&away),
                round,
            };
            let (hs, as_) = policy.score(&ctx);
            mps.push(MatchPrediction {
                game_id: game.id.clone(),
                home_score: hs,
                away_score: as_,
                locked: false,
            });
            resolved_teams.insert(game.id.clone(), (home.clone(), away.clone()));

            // Knockout: record the advancer in the one-match group's standings
            // so `resolve_bracket` resolves draws the same way next round.
            if round != Round::GroupStage {
                let (adv, elim) = advancer(hs, as_, &home, &away, ranking);
                sps.push(StandingsPrediction {
                    group_id: game.group_id.clone(),
                    ordering: vec![adv.clone(), elim.clone()],
                    draw_order: vec![adv, elim],
                    locked: false,
                });
            }
        }

        // Group-stage standings: rank each leaf group from the scores just set.
        // Iterate group ids in sorted order so the `standings_predictions` Vec
        // is byte-for-byte reproducible (HashMap iteration order is not stable).
        if round == Round::GroupStage {
            let mut group_ids: Vec<&String> = t.groups.keys().collect();
            group_ids.sort();
            for gid in group_ids {
                let group = &t.groups[gid];
                if group.round != Round::GroupStage || !group.carries_standings {
                    continue;
                }
                let games_g = group_games(t, gid);
                if games_g.is_empty() {
                    continue;
                }
                let team_ids: Vec<TeamId> = games_g
                    .iter()
                    .flat_map(|g| {
                        [g.home.team_id.clone(), g.away.team_id.clone()]
                            .into_iter()
                            .flatten()
                    })
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                let draw_order = draw_order_by_strength(&team_ids, ranking);
                let pred_refs: Vec<&MatchPrediction> = games_g
                    .iter()
                    .filter_map(|g| mps.iter().find(|p| p.game_id == g.id))
                    .collect();
                let ordering = domain::rank_group(group, &games_g, &pred_refs, &draw_order);
                sps.push(StandingsPrediction {
                    group_id: gid.clone(),
                    ordering,
                    draw_order,
                    locked: false,
                });
            }
        }
    }

    Outcome {
        match_predictions: mps,
        standings_predictions: sps,
        resolved_teams,
    }
}

/// Decide the knockout advancer: higher score wins; a 90-minute draw goes to the
/// higher-strength side (home on a tie) — matching `fwc26`'s draw fallback.
fn advancer(hs: u8, as_: u8, home: &str, away: &str, ranking: &Ranking) -> (TeamId, TeamId) {
    use std::cmp::Ordering::*;
    match hs.cmp(&as_) {
        Greater => (home.to_string(), away.to_string()),
        Less => (away.to_string(), home.to_string()),
        Equal => {
            if ranking.strength(away) > ranking.strength(home) {
                (away.to_string(), home.to_string())
            } else {
                (home.to_string(), away.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::policy::{AlwaysDraw, AlwaysHome, Chalk};
    use chrono::{TimeZone, Utc};
    use domain::{GroupGame, LockMode, Team, TeamSlot};
    use std::collections::HashMap as Map;

    // A minimal 1-group, 1-game tournament (no knockout) for group-stage tests.
    fn one_group_tournament() -> Tournament {
        let mk_team = |id: &str| Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        };
        let game = SingleGame {
            id: "G1".into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 19, 0, 0).unwrap(),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot {
                team_id: Some("HOME".into()),
                description: "A1".into(),
            },
            away: TeamSlot {
                team_id: Some("AWAY".into()),
                description: "A2".into(),
            },
        };
        let group = GroupGame {
            id: "A".into(),
            name: "Group A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["G1".into()]),
        };
        Tournament {
            root: "A".into(),
            groups: Map::from([("A".to_string(), group)]),
            games: Map::from([("G1".to_string(), game)]),
            teams: Map::from([
                ("HOME".to_string(), mk_team("HOME")),
                ("AWAY".to_string(), mk_team("AWAY")),
            ]),
        }
    }

    fn ranking() -> Ranking {
        // HOME stronger than AWAY. `Ranking` derives transparent `Deserialize`
        // (added in Step 1), so it builds straight from a JSON object without a
        // public constructor and the test does not depend on real team ids.
        serde_json::from_value(serde_json::json!({ "HOME": 80, "AWAY": 40 })).unwrap()
    }

    #[test]
    fn group_stage_produces_a_match_and_a_standings_row() {
        let t = one_group_tournament();
        let r = ranking();
        let out = generate(&t, &r, &mut AlwaysHome);

        // One match prediction, 1-0 to HOME.
        assert_eq!(out.match_predictions.len(), 1);
        let mp = &out.match_predictions[0];
        assert_eq!((mp.home_score, mp.away_score), (1, 0));
        assert!(!mp.locked);

        // One standings row, HOME ranked first (it won).
        let sp = out
            .standings_predictions
            .iter()
            .find(|s| s.group_id == "A")
            .expect("group A standings");
        assert_eq!(sp.ordering.first().map(String::as_str), Some("HOME"));
    }

    fn real_tournament() -> Tournament {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26.json");
        crate::load_tournament(&path).expect("load fwc26")
    }

    fn real_ranking() -> Ranking {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tournaments/fwc26-rankings.json");
        Ranking::load(&path).expect("load rankings")
    }

    #[test]
    fn full_tournament_resolves_every_game_including_the_final() {
        let t = real_tournament();
        let r = real_ranking();
        let out = generate(&t, &r, &mut Chalk);

        // Every game in the tournament gets a prediction and resolved teams.
        assert_eq!(out.match_predictions.len(), t.games.len());
        assert_eq!(out.resolved_teams.len(), t.games.len());

        // No knockout game left with a placeholder (all teams concrete).
        for (gid, (home, away)) in &out.resolved_teams {
            assert!(!home.is_empty(), "game {gid} home unresolved");
            assert!(!away.is_empty(), "game {gid} away unresolved");
        }
    }

    #[test]
    fn coherence_round_trip_matches_resolve_bracket() {
        let t = real_tournament();
        let r = real_ranking();
        // AlwaysDraw stresses the advancer path (every knockout is a 1-1 draw).
        let out = generate(&t, &r, &mut AlwaysDraw);

        // Assign the outcome as the result-user and re-resolve the bracket.
        let result = interim_player(&out.match_predictions, &out.standings_predictions);
        let bracket = fwc26::resolve_bracket(&t, &result);

        // For every knockout game, resolve_bracket must reproduce exactly the
        // teams the engine used.
        for (gid, game) in &t.games {
            let round = t.groups.get(&game.group_id).map(|g| g.round);
            if round == Some(Round::GroupStage) {
                continue;
            }
            let (eh, ea) = out.resolved_teams.get(gid).expect("engine teams");
            let (rh, ra) = bracket.get(gid).cloned().unwrap_or((None, None));
            assert_eq!(rh.as_ref(), Some(eh), "home mismatch on {gid}");
            assert_eq!(ra.as_ref(), Some(ea), "away mismatch on {gid}");
        }
    }
}
