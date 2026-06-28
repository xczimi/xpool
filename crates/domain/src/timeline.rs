//! Per-game cumulative points over time — the data backbone for the
//! points-trajectory chart (`.scratch/player-points-timeline-chart`).
//!
//! Pure and I/O-free, like the rest of `domain`. The resolver loads the coarse
//! items once and calls [`player_timelines`]; it holds no logic of its own.
//!
//! Unlike the by-round scoreboard slice, this walks the schedule **game by
//! game** in kickoff order, so every player's cumulative line climbs as each
//! match is scored — a flat group-stage line (all points in one round bucket)
//! is impossible here.

use crate::scoring::ScoringConfig;
use crate::{Player, Round, SingleGame, Tournament};
use chrono::{DateTime, Utc};

/// One step on a player's trajectory: the per-game points and the running sum
/// through that game. `points` already has the round-stage multiplier applied,
/// so it equals the per-game points shown on the match page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelinePoint {
    pub game_id: String,
    pub kickoff: DateTime<Utc>,
    pub points: i64,
    pub cumulative: i64,
}

/// One player's whole trajectory over the scored-so-far schedule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerTimeline {
    pub player_id: String,
    pub nick: String,
    pub points: Vec<TimelinePoint>,
}

/// The per-game cumulative trajectory for each competitor.
///
/// The x-axis is the set of games that (a) have an official result (the result
/// user entered a prediction for them) and (b) kicked off at/before `now`,
/// ordered by kickoff (ties broken by game id for determinism). Every included
/// player gets one [`TimelinePoint`] per such game — `points = 0` when the
/// player has no prediction for it — so all series share the same x-axis and
/// overlay cleanly.
///
/// Included players are the **participants** (excludes the result user and
/// anyone who never predicted), optionally restricted to `allowed` (pool member
/// ids; `None` = the global board). Output is sorted by player id.
pub fn player_timelines(
    tournament: &Tournament,
    players: &[Player],
    now: DateTime<Utc>,
    allowed: Option<&[String]>,
    config: &ScoringConfig,
) -> Vec<PlayerTimeline> {
    let Some(result_user) = players.iter().find(|p| p.is_result_user) else {
        return Vec::new();
    };

    // The shared x-axis: resulted, already-kicked-off games in kickoff order.
    let mut games: Vec<(&SingleGame, Round)> = tournament
        .games
        .values()
        .filter(|g| g.kickoff <= now)
        .filter(|g| result_user.match_prediction(&g.id).is_some())
        .map(|g| {
            let round = tournament
                .groups
                .get(&g.group_id)
                .map(|gr| gr.round)
                .unwrap_or(Round::GroupStage);
            (g, round)
        })
        .collect();
    games.sort_by(|a, b| a.0.kickoff.cmp(&b.0.kickoff).then(a.0.id.cmp(&b.0.id)));

    let mut out: Vec<PlayerTimeline> = crate::participation::participants(players)
        .into_iter()
        .filter(|p| allowed.is_none_or(|ids| ids.contains(&p.id)))
        .map(|player| {
            let mut cumulative = 0i64;
            let points = games
                .iter()
                .map(|(g, round)| {
                    let result = result_user
                        .match_prediction(&g.id)
                        .expect("games filtered to those with an official result");
                    let earned = match player.match_prediction(&g.id) {
                        Some(pred) => {
                            crate::scoring::score_match_parts(pred, result, config).points(config)
                                * config.multiplier(*round)
                        }
                        None => 0,
                    };
                    cumulative += earned;
                    TimelinePoint {
                        game_id: g.id.clone(),
                        kickoff: g.kickoff,
                        points: earned,
                        cumulative,
                    }
                })
                .collect();
            PlayerTimeline {
                player_id: player.id.clone(),
                nick: player.nick.clone(),
                points,
            }
        })
        .collect();
    out.sort_by(|a, b| a.player_id.cmp(&b.player_id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame, TeamSlot,
        Tournament,
    };
    use chrono::TimeZone;
    use std::collections::HashMap;

    fn ko(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, day, 18, 0, 0).unwrap()
    }

    fn game(id: &str, day: u32) -> SingleGame {
        SingleGame {
            id: id.into(),
            kickoff: ko(day),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot {
                team_id: Some("H".into()),
                description: "H".into(),
            },
            away: TeamSlot {
                team_id: Some("A".into()),
                description: "A".into(),
            },
            external_id: None,
        }
    }

    fn pred(game_id: &str, h: u8, a: u8) -> MatchPrediction {
        MatchPrediction {
            game_id: game_id.into(),
            home_score: h,
            away_score: a,
            locked: true,
        }
    }

    fn player(id: &str, is_result_user: bool, preds: Vec<MatchPrediction>) -> Player {
        Player {
            id: id.into(),
            person_id: format!("p-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user,
            version: 0,
            match_predictions: preds,
            standings_predictions: vec![],
        }
    }

    /// Group A with three games M1 (day 11), M2 (day 12), M3 (day 13).
    fn tournament() -> Tournament {
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into(), "M2".into(), "M3".into()]),
        };
        Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([
                ("M1".to_string(), game("M1", 11)),
                ("M2".to_string(), game("M2", 12)),
                ("M3".to_string(), game("M3", 13)),
            ]),
            teams: HashMap::new(),
        }
    }

    fn config() -> ScoringConfig {
        ScoringConfig::default()
    }

    #[test]
    fn cumulative_climbs_in_kickoff_order() {
        let t = tournament();
        // Official: M1 2-0, M2 1-1, M3 0-1.
        let ru = player(
            "result-user",
            true,
            vec![pred("M1", 2, 0), pred("M2", 1, 1), pred("M3", 0, 1)],
        );
        // ada: M1 exact (2-0 → 2 exact +2 outcome = 4), M2 1-1 exact (4),
        // M3 wrong outcome (1-0 vs 0-1 → 0).
        let ada = player(
            "demo-ada",
            false,
            vec![pred("M1", 2, 0), pred("M2", 1, 1), pred("M3", 1, 0)],
        );
        let players = vec![ru, ada];
        let out = player_timelines(&t, &players, ko(20), None, &config());
        assert_eq!(out.len(), 1, "result user excluded");
        let pts = &out[0].points;
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0].game_id, "M1");
        assert_eq!(pts[1].game_id, "M2");
        assert_eq!(pts[2].game_id, "M3");
        // Group stage ×1: 4, then +4, then +0.
        assert_eq!(
            pts.iter().map(|p| p.points).collect::<Vec<_>>(),
            vec![4, 4, 0]
        );
        assert_eq!(
            pts.iter().map(|p| p.cumulative).collect::<Vec<_>>(),
            vec![4, 8, 8]
        );
    }

    #[test]
    fn only_resulted_and_past_games_are_included() {
        let t = tournament();
        // Official result only for M1 and M2 (M3 not yet entered).
        let ru = player(
            "result-user",
            true,
            vec![pred("M1", 2, 0), pred("M2", 1, 1)],
        );
        let ada = player(
            "demo-ada",
            false,
            vec![pred("M1", 2, 0), pred("M2", 1, 1), pred("M3", 1, 0)],
        );
        let players = vec![ru, ada];
        // now is day-12 noon: M3 (day 13) is in the future AND has no result.
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 12, 0, 0).unwrap();
        // M2 kicks off day-12 18:00 → after `now` → excluded by the clock too.
        let out = player_timelines(&t, &players, now, None, &config());
        let ids: Vec<&str> = out[0].points.iter().map(|p| p.game_id.as_str()).collect();
        assert_eq!(ids, vec!["M1"], "only M1 has a result AND kicked off");
    }

    #[test]
    fn pool_filter_restricts_to_members() {
        let t = tournament();
        let ru = player("result-user", true, vec![pred("M1", 2, 0)]);
        let ada = player("demo-ada", false, vec![pred("M1", 2, 0)]);
        let alan = player("demo-alan", false, vec![pred("M1", 0, 2)]);
        let players = vec![ru, ada, alan];
        let allowed = vec!["demo-alan".to_string()];
        let out = player_timelines(&t, &players, ko(20), Some(&allowed), &config());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].player_id, "demo-alan");
    }

    #[test]
    fn non_participants_are_excluded() {
        let t = tournament();
        let ru = player("result-user", true, vec![pred("M1", 2, 0)]);
        let ada = player("demo-ada", false, vec![pred("M1", 2, 0)]);
        // never predicted → not a participant.
        let ghost = player("ghost", false, vec![]);
        let players = vec![ru, ada, ghost];
        let out = player_timelines(&t, &players, ko(20), None, &config());
        let ids: Vec<&str> = out.iter().map(|p| p.player_id.as_str()).collect();
        assert_eq!(ids, vec!["demo-ada"]);
    }

    #[test]
    fn missing_prediction_scores_zero_but_keeps_the_x_axis_aligned() {
        let t = tournament();
        let ru = player(
            "result-user",
            true,
            vec![pred("M1", 2, 0), pred("M2", 1, 1)],
        );
        // ada predicted only M2.
        let ada = player("demo-ada", false, vec![pred("M2", 1, 1)]);
        let players = vec![ru, ada];
        let out = player_timelines(&t, &players, ko(20), None, &config());
        let pts = &out[0].points;
        assert_eq!(pts.len(), 2, "one point per resulted game, aligned");
        assert_eq!(pts[0].points, 0, "no M1 prediction → 0");
        assert_eq!(pts[1].points, 4);
        assert_eq!(pts[1].cumulative, 4);
    }

    #[test]
    fn empty_without_a_result_user() {
        let t = tournament();
        let ada = player("demo-ada", false, vec![pred("M1", 2, 0)]);
        let out = player_timelines(&t, &[ada], ko(20), None, &config());
        assert!(out.is_empty());
    }
}
