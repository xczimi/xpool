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

use crate::scoring::{standings_score, ScoringConfig};
use crate::{GroupChildren, GroupGame, Player, Round, SingleGame, Tournament};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

/// One step on a player's trajectory: the per-game points and the running sum
/// through that game. `points` already has the round-stage multiplier applied,
/// so it equals the per-game points shown on the match page — **except** at a
/// group's settling game (the last resulted game of a leaf group that carries
/// standings), where `points` also folds in that group's standings bonus, so the
/// line steps up as the group's final table settles. Per-point `points` always
/// sum to `cumulative`, and the final `cumulative` equals the player's
/// materialised scoreboard total once every game has cleared its result buffer.
/// (The x-axis is as-of *kickoff*, like the per-match line; the materialised
/// scoreboard slices on kickoff + result-buffer, so the timeline leads it by at
/// most one buffer window mid-tournament, then reconciles exactly.)
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

    // The group-standings bonus is part of every player's scoreboard total but
    // is *not* a per-match award. Attribute it to each leaf group's **settling
    // game** — the last resulted game of that group on the x-axis, whose
    // completion settles the group's final table — so the cumulative line steps
    // up there and the final total reconciles with the materialised scoreboard.
    let settled = settled_group_by_game(tournament, &games);

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
                    let mut earned = match player.match_prediction(&g.id) {
                        Some(pred) => {
                            crate::scoring::score_match_parts(pred, result, config).points(config)
                                * config.multiplier(*round)
                        }
                        None => 0,
                    };
                    // Settling game → add this group's standings bonus, using the
                    // exact `standings_score` the scoreboard uses, then the same
                    // round multiplier (so the two totals never drift).
                    if let Some(sg) = settled.get(g.id.as_str()) {
                        let bonus = standings_score(
                            sg.group,
                            &sg.games,
                            player,
                            result_user,
                            now,
                            sg.deadline,
                            config,
                        )
                        .map_or(0, |b| b.bonus);
                        earned += bonus * config.multiplier(*round);
                    }
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

/// A leaf group whose standings have settled, paired with the game on the x-axis
/// that settles them (the bonus is attributed there).
struct SettledGroup<'a> {
    group: &'a GroupGame,
    games: Vec<&'a SingleGame>,
    deadline: DateTime<Utc>,
}

/// Map each settling game id → its [`SettledGroup`].
///
/// A leaf group settles once **all** of its games appear on the x-axis (every
/// game resulted and kicked off). The settling game is that group's last game in
/// x-axis order — exactly where the scoreboard would first credit the group's
/// standings bonus. Groups that carry no standings, or aren't fully resulted
/// yet, are absent (no bonus step). The per-player bonus itself is computed
/// later by [`standings_score`], which also gates on the player's own prediction.
fn settled_group_by_game<'a>(
    tournament: &'a Tournament,
    games: &[(&'a SingleGame, Round)],
) -> HashMap<&'a str, SettledGroup<'a>> {
    let on_axis: HashSet<&str> = games.iter().map(|(g, _)| g.id.as_str()).collect();

    let mut settled: HashMap<&str, SettledGroup<'a>> = HashMap::new();
    let mut seen_groups: HashSet<&str> = HashSet::new();

    // Walk x-axis order; the last game of a settled group is its settling game.
    for (g, _) in games {
        let group_id = g.group_id.as_str();
        if !seen_groups.insert(group_id) {
            // Already evaluated this group on an earlier game.
            continue;
        }
        let Some(group) = tournament.groups.get(group_id) else {
            continue;
        };
        if !group.carries_standings {
            continue;
        }
        let GroupChildren::Games(child_ids) = &group.children else {
            continue;
        };
        // Fully resulted? Every game of the group must be on the x-axis.
        if !child_ids.iter().all(|id| on_axis.contains(id.as_str())) {
            continue;
        }
        let group_games: Vec<&SingleGame> = child_ids
            .iter()
            .filter_map(|id| tournament.games.get(id))
            .collect();
        let Some(deadline) = tournament.deadline(group_id) else {
            continue;
        };
        // The settling game is the group's last game in x-axis (kickoff) order.
        let settling = games
            .iter()
            .rfind(|(gg, _)| gg.group_id == group.id)
            .map(|(gg, _)| gg.id.as_str());
        if let Some(settling_id) = settling {
            settled.insert(
                settling_id,
                SettledGroup {
                    group,
                    games: group_games,
                    deadline,
                },
            );
        }
    }
    settled
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

    // ─── Invariant: timeline final cumulative == scoreboard total ────────────
    //
    // The materialised scoreboard total is `score_tournament` summed over rounds
    // (`recompute` slices the result-user first, but at a far-future `now` the
    // slice is a no-op). The per-match-only timeline omitted the group-standings
    // bonus, so its final cumulative fell short. This is the acceptance test.

    use crate::scoring::score_tournament;
    use crate::StandingsPrediction;

    /// A team slot with a resolved team id.
    fn slot(team: &str) -> TeamSlot {
        TeamSlot {
            team_id: Some(team.into()),
            description: team.into(),
        }
    }

    /// A group-stage game between two real teams, in group `group_id`.
    fn match_in(id: &str, day: u32, group_id: &str, home: &str, away: &str) -> SingleGame {
        SingleGame {
            id: id.into(),
            kickoff: ko(day),
            venue: None,
            group_id: group_id.into(),
            home: slot(home),
            away: slot(away),
            external_id: None,
        }
    }

    fn standings(group_id: &str, ordering: &[&str]) -> StandingsPrediction {
        StandingsPrediction {
            group_id: group_id.into(),
            ordering: ordering.iter().map(|s| s.to_string()).collect(),
            draw_order: ordering.iter().map(|s| s.to_string()).collect(),
            locked: true,
        }
    }

    fn player_full(
        id: &str,
        is_result_user: bool,
        preds: Vec<MatchPrediction>,
        sp: Vec<StandingsPrediction>,
    ) -> Player {
        Player {
            standings_predictions: sp,
            ..player(id, is_result_user, preds)
        }
    }

    /// Two real round-robin groups (A: T1/T2/T3, B: U1/U2/U3) under a root,
    /// every game resolved, both carrying standings.
    fn two_group_tournament() -> Tournament {
        let leaf = |id: &str, gids: Vec<&str>| GroupGame {
            id: id.into(),
            name: id.into(),
            parent: Some("ROOT".into()),
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(gids.into_iter().map(|s| s.to_string()).collect()),
        };
        let root = GroupGame {
            id: "ROOT".into(),
            name: "ROOT".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: false,
            children: GroupChildren::Groups(vec!["A".into(), "B".into()]),
        };
        let games = [
            match_in("A1", 11, "A", "T1", "T2"),
            match_in("A2", 12, "A", "T1", "T3"),
            match_in("A3", 13, "A", "T2", "T3"),
            match_in("B1", 14, "B", "U1", "U2"),
            match_in("B2", 15, "B", "U1", "U3"),
            match_in("B3", 16, "B", "U2", "U3"),
        ];
        Tournament {
            root: "ROOT".into(),
            groups: HashMap::from([
                ("ROOT".to_string(), root),
                ("A".to_string(), leaf("A", vec!["A1", "A2", "A3"])),
                ("B".to_string(), leaf("B", vec!["B1", "B2", "B3"])),
            ]),
            games: HashMap::from(games.map(|g| (g.id.clone(), g))),
            teams: HashMap::new(),
        }
    }

    #[test]
    fn final_cumulative_equals_scoreboard_total_including_standings_bonus() {
        let t = two_group_tournament();
        let c = config();
        let now = ko(30); // far past every game → all groups settled, slice is a no-op

        // Official results: A → T1 beats both, T2 beats T3; B → U1, U2, U3.
        let ru = player_full(
            "result-user",
            true,
            vec![
                pred("A1", 2, 0),
                pred("A2", 1, 0),
                pred("A3", 1, 0),
                pred("B1", 3, 0),
                pred("B2", 2, 0),
                pred("B3", 1, 0),
            ],
            vec![
                standings("A", &["T1", "T2", "T3"]),
                standings("B", &["U1", "U2", "U3"]),
            ],
        );
        // ada: nails group A standings (and most scores), gets group B order wrong.
        let ada = player_full(
            "demo-ada",
            false,
            vec![
                pred("A1", 2, 0),
                pred("A2", 1, 0),
                pred("A3", 1, 0),
                pred("B1", 0, 1),
                pred("B2", 0, 1),
                pred("B3", 0, 1),
            ],
            vec![
                standings("A", &["T1", "T2", "T3"]),
                standings("B", &["U3", "U2", "U1"]),
            ],
        );
        // alan: different scores, partial standings — only predicts group A's table.
        let alan = player_full(
            "demo-alan",
            false,
            vec![
                pred("A1", 1, 1),
                pred("A2", 0, 2),
                pred("A3", 2, 2),
                pred("B1", 3, 0),
                pred("B2", 1, 1),
                pred("B3", 1, 0),
            ],
            vec![standings("A", &["T2", "T1", "T3"])],
        );
        let players = vec![ru, ada.clone(), alan.clone()];

        let out = player_timelines(&t, &players, now, None, &c);
        assert_eq!(out.len(), 2, "result user excluded");

        for tl in &out {
            let player = players.iter().find(|p| p.id == tl.player_id).unwrap();
            let result = players.iter().find(|p| p.is_result_user).unwrap();
            let scoreboard_total: i64 =
                score_tournament(&t, player, result, now, &c).values().sum();
            let final_cumulative = tl.points.last().map(|p| p.cumulative).unwrap_or(0);
            assert_eq!(
                final_cumulative, scoreboard_total,
                "{}: timeline final cumulative ({final_cumulative}) must equal scoreboard total ({scoreboard_total})",
                tl.player_id
            );
            // Internal consistency: the per-point points still sum to the cumulative.
            let summed: i64 = tl.points.iter().map(|p| p.points).sum();
            assert_eq!(
                summed, final_cumulative,
                "{}: points must sum to cumulative",
                tl.player_id
            );
        }

        // And the bonus is a *positive* component for at least one player, so the
        // test would fail loudly if the bonus were silently zero everywhere.
        let ada_total: i64 = score_tournament(&t, &ada, &players[0], now, &c)
            .values()
            .sum();
        let ada_match_only: i64 = out
            .iter()
            .find(|tl| tl.player_id == "demo-ada")
            .unwrap()
            .points
            .iter()
            .map(|p| p.points)
            .sum();
        assert_eq!(ada_match_only, ada_total);
        assert!(ada_total > 0);
    }

    #[test]
    fn standings_bonus_steps_up_at_the_last_resulted_game_of_the_group() {
        let t = two_group_tournament();
        let c = config();
        let now = ko(30);

        let ru = player_full(
            "result-user",
            true,
            vec![
                pred("A1", 2, 0),
                pred("A2", 1, 0),
                pred("A3", 1, 0),
                pred("B1", 3, 0),
                pred("B2", 2, 0),
                pred("B3", 1, 0),
            ],
            vec![
                standings("A", &["T1", "T2", "T3"]),
                standings("B", &["U1", "U2", "U3"]),
            ],
        );
        // ada predicts ONLY group A's standings; her group-A scores are all wrong
        // on exact but we only care that the bonus lands on A3 (the last A game).
        let ada = player_full(
            "demo-ada",
            false,
            vec![
                pred("A1", 5, 4),
                pred("A2", 5, 4),
                pred("A3", 5, 4),
                pred("B1", 5, 4),
                pred("B2", 5, 4),
                pred("B3", 5, 4),
            ],
            vec![standings("A", &["T1", "T2", "T3"])],
        );
        let players = vec![ru, ada];
        let out = player_timelines(&t, &players, now, None, &c);
        let pts = &out[0].points;
        // Six games in kickoff order: A1 A2 A3 B1 B2 B3.
        let ids: Vec<&str> = pts.iter().map(|p| p.game_id.as_str()).collect();
        assert_eq!(ids, vec!["A1", "A2", "A3", "B1", "B2", "B3"]);
        // The step at A3 must exceed the bare per-match points for A3, because the
        // group-A standings bonus is attributed to A3 (the group's settling game).
        let a3 = pts.iter().find(|p| p.game_id == "A3").unwrap();
        let a3_match_only =
            crate::scoring::score_match_parts(&pred("A3", 5, 4), &pred("A3", 1, 0), &c).points(&c);
        assert!(
            a3.points > a3_match_only,
            "A3 step ({}) should include group-A standings bonus on top of match points ({a3_match_only})",
            a3.points
        );
        // Group B carries no standings prediction for ada → no bonus step on B3.
        let b3 = pts.iter().find(|p| p.game_id == "B3").unwrap();
        let b3_match_only =
            crate::scoring::score_match_parts(&pred("B3", 5, 4), &pred("B3", 1, 0), &c).points(&c);
        assert_eq!(
            b3.points, b3_match_only,
            "no group-B standings → no B3 bonus"
        );
    }
}
