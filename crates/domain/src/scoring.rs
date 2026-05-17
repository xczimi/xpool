//! The scoring engine — pure functions, no I/O (`SCORING.md`).
//!
//! Signatures here are a **locked contract**. The `todo!()` bodies are filled
//! by the `domain`-crate subagent (plan task P1) along with the test suite.

use crate::model::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Centralized scoring constants (`SCORING.md` §2). Seeded defaults; tuned
/// before launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoringConfig {
    pub exact_score_point: i64,
    pub outcome_point: i64,
    pub high_scoring_threshold: u8,
    pub standings_pair_point: i64,
    pub perfect_threshold: i64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            exact_score_point: 1,
            outcome_point: 2,
            high_scoring_threshold: 4,
            standings_pair_point: 1,
            perfect_threshold: 4,
        }
    }
}

impl ScoringConfig {
    /// Per-round stage multiplier (`SCORING.md` §6) — an explicit table, never
    /// derived from start-time order.
    pub fn multiplier(&self, round: Round) -> i64 {
        match round {
            Round::GroupStage => 1,
            Round::R32 => 2,
            Round::R16 => 3,
            Round::QF => 4,
            Round::SF => 5,
            Round::ThirdPlace => 5,
            Round::Final => 6,
        }
    }
}

/// Outcome sign: positive for home win, zero for draw, negative for away win.
fn outcome_sign(home: u8, away: u8) -> std::cmp::Ordering {
    home.cmp(&away)
}

/// Per-match points: prediction `p` vs result `r`, both 90-minute scores.
/// Max `2*exact + outcome`. Implements the per-side, symmetric 4-goal rule
/// (`SCORING.md` §3).
pub fn score_match(p: &MatchPrediction, r: &MatchPrediction, c: &ScoringConfig) -> i64 {
    let thr = c.high_scoring_threshold;
    let mut pts: i64 = 0;

    // Home side: exact match OR both ≥ threshold (4-goal rule, §3)
    if p.home_score == r.home_score || (p.home_score >= thr && r.home_score >= thr) {
        pts += c.exact_score_point;
    }

    // Away side: exact match OR both ≥ threshold (per-side, symmetric — fixes legacy bug §10 #1)
    if p.away_score == r.away_score || (p.away_score >= thr && r.away_score >= thr) {
        pts += c.exact_score_point;
    }

    // Outcome: correct W/D/L (sign of home − away)
    if outcome_sign(p.home_score, p.away_score) == outcome_sign(r.home_score, r.away_score) {
        pts += c.outcome_point;
    }

    pts
}

/// A prediction is a "perfect" when it scored the maximum (`SCORING.md` §7).
pub fn is_perfect(p: &MatchPrediction, r: &MatchPrediction, c: &ScoringConfig) -> bool {
    score_match(p, r, c) >= c.perfect_threshold
}

/// Effective-locked (`DATA_MODEL.md` §7): `locked OR (now > deadline AND complete)`.
pub fn effective_locked(
    locked: bool,
    now: DateTime<Utc>,
    deadline: DateTime<Utc>,
    complete: bool,
) -> bool {
    locked || (now > deadline && complete)
}

// ─── Group ranking helpers ───────────────────────────────────────────────────

/// Per-team stats accumulated while ranking a group.
#[derive(Clone, Debug, Default)]
struct TeamStats {
    points: i32,
    goals_for: i32,
    goals_against: i32,
}

impl TeamStats {
    fn goal_diff(&self) -> i32 {
        self.goals_for - self.goals_against
    }
}

/// Collect all teams involved in the games.
fn collect_teams(games: &[&SingleGame]) -> Vec<TeamId> {
    let mut teams: Vec<TeamId> = Vec::new();
    for game in games {
        if let Some(ref tid) = game.home.team_id {
            if !teams.contains(tid) {
                teams.push(tid.clone());
            }
        }
        if let Some(ref tid) = game.away.team_id {
            if !teams.contains(tid) {
                teams.push(tid.clone());
            }
        }
    }
    teams
}

/// Compute per-team stats from the given games and predictions.
/// Only games where both team slots are resolved are considered.
fn compute_stats(
    teams: &[TeamId],
    games: &[&SingleGame],
    predictions: &[&MatchPrediction],
) -> HashMap<TeamId, TeamStats> {
    let mut stats: HashMap<TeamId, TeamStats> = teams
        .iter()
        .map(|t| (t.clone(), TeamStats::default()))
        .collect();

    for game in games {
        let pred = predictions.iter().find(|p| p.game_id == game.id);
        if let (Some(pred), Some(home_id), Some(away_id)) =
            (pred, game.home.team_id.as_ref(), game.away.team_id.as_ref())
        {
            let (home_g, away_g) = (pred.home_score as i32, pred.away_score as i32);
            if let Some(s) = stats.get_mut(home_id) {
                s.goals_for += home_g;
                s.goals_against += away_g;
                s.points += match home_g.cmp(&away_g) {
                    std::cmp::Ordering::Greater => 3,
                    std::cmp::Ordering::Equal => 1,
                    std::cmp::Ordering::Less => 0,
                };
            }
            if let Some(s) = stats.get_mut(away_id) {
                s.goals_for += away_g;
                s.goals_against += home_g;
                s.points += match away_g.cmp(&home_g) {
                    std::cmp::Ordering::Greater => 3,
                    std::cmp::Ordering::Equal => 1,
                    std::cmp::Ordering::Less => 0,
                };
            }
        }
    }
    stats
}

/// Filter games to only those between a given set of teams.
fn h2h_games<'a>(teams: &[&TeamId], games: &[&'a SingleGame]) -> Vec<&'a SingleGame> {
    games
        .iter()
        .filter(|g| {
            let home_in = g.home.team_id.as_ref().is_some_and(|t| teams.contains(&t));
            let away_in = g.away.team_id.as_ref().is_some_and(|t| teams.contains(&t));
            home_in && away_in
        })
        .copied()
        .collect()
}

/// Compare two teams using the §4 ladder. Returns `Less` if `a` ranks above `b`
/// (i.e., lower index = better rank = "less" in sort order).
///
/// Ladder:
/// 1. Overall points (desc)
/// 2. H2H points (desc) among the tied subgroup
/// 3. H2H GD (desc)
/// 4. H2H goals for (desc)
/// 5. All-match GD (desc)
/// 6. All-match goals for (desc)
/// 7. draw_order position (asc)
fn rank_group_sort(
    teams: &[TeamId],
    all_stats: &HashMap<TeamId, TeamStats>,
    games: &[&SingleGame],
    predictions: &[&MatchPrediction],
    draw_order: &[TeamId],
) -> Vec<TeamId> {
    if teams.is_empty() {
        return vec![];
    }
    if teams.len() == 1 {
        return teams.to_vec();
    }

    // Group by overall points.
    let mut by_points: HashMap<i32, Vec<TeamId>> = HashMap::new();
    for t in teams {
        let pts = all_stats.get(t).map_or(0, |s| s.points);
        by_points.entry(pts).or_default().push(t.clone());
    }

    let mut pts_sorted: Vec<i32> = by_points.keys().copied().collect();
    pts_sorted.sort_by(|a, b| b.cmp(a)); // descending

    let mut result: Vec<TeamId> = Vec::new();

    for pts in pts_sorted {
        let group = by_points.remove(&pts).unwrap();
        if group.len() == 1 {
            result.push(group.into_iter().next().unwrap());
            continue;
        }
        // Tied group — resolve recursively with h2h and GD criteria.
        let group_refs: Vec<&TeamId> = group.iter().collect();
        let h2h = h2h_games(&group_refs, games);
        let h2h_stats = compute_stats(&group, &h2h, predictions);

        // Try h2h tiebreak sub-sort within this tied group.
        let sorted = sort_tied_group(&group, all_stats, &h2h_stats, &h2h, predictions, draw_order);
        result.extend(sorted);
    }

    result
}

/// Recursively break a tied group using H2H, then all-match criteria, then draw_order.
fn sort_tied_group(
    teams: &[TeamId],
    all_stats: &HashMap<TeamId, TeamStats>,
    h2h_stats: &HashMap<TeamId, TeamStats>,
    h2h_games_list: &[&SingleGame],
    predictions: &[&MatchPrediction],
    draw_order: &[TeamId],
) -> Vec<TeamId> {
    if teams.len() == 1 {
        return teams.to_vec();
    }

    // Sort by h2h points desc
    let sub = sort_by_criteria(teams, |t| {
        let pts = h2h_stats.get(t).map_or(0, |s| s.points);
        -pts // negate for descending
    });

    // If any sub-group further resolves, split and recurse.
    let resolved = split_and_resolve_groups(
        &sub,
        |group| {
            let pts: Vec<i32> = group
                .iter()
                .map(|t| h2h_stats.get(t).map_or(0, |s| s.points))
                .collect();
            pts.windows(2).all(|w| w[0] == w[1])
        },
        |group| {
            // h2h GD, then h2h goals, then all GD, then all goals, then draw_order
            resolve_sub_group(
                group,
                all_stats,
                h2h_stats,
                h2h_games_list,
                predictions,
                draw_order,
                1,
            )
        },
    );

    resolved
}

/// Resolve a group that was tied on H2H points: try h2h GD.
#[allow(clippy::only_used_in_recursion)]
fn resolve_sub_group(
    teams: &[TeamId],
    all_stats: &HashMap<TeamId, TeamStats>,
    h2h_stats: &HashMap<TeamId, TeamStats>,
    h2h_games_list: &[&SingleGame],
    predictions: &[&MatchPrediction],
    draw_order: &[TeamId],
    step: u8,
) -> Vec<TeamId> {
    if teams.len() == 1 {
        return teams.to_vec();
    }

    match step {
        1 => {
            // H2H GD
            let sub = sort_by_criteria(teams, |t| -(h2h_stats.get(t).map_or(0, |s| s.goal_diff())));
            split_and_resolve_groups(
                &sub,
                |group| {
                    let vals: Vec<i32> = group
                        .iter()
                        .map(|t| h2h_stats.get(t).map_or(0, |s| s.goal_diff()))
                        .collect();
                    vals.windows(2).all(|w| w[0] == w[1])
                },
                |group| {
                    resolve_sub_group(
                        group,
                        all_stats,
                        h2h_stats,
                        h2h_games_list,
                        predictions,
                        draw_order,
                        2,
                    )
                },
            )
        }
        2 => {
            // H2H goals for
            let sub = sort_by_criteria(teams, |t| -(h2h_stats.get(t).map_or(0, |s| s.goals_for)));
            split_and_resolve_groups(
                &sub,
                |group| {
                    let vals: Vec<i32> = group
                        .iter()
                        .map(|t| h2h_stats.get(t).map_or(0, |s| s.goals_for))
                        .collect();
                    vals.windows(2).all(|w| w[0] == w[1])
                },
                |group| {
                    resolve_sub_group(
                        group,
                        all_stats,
                        h2h_stats,
                        h2h_games_list,
                        predictions,
                        draw_order,
                        3,
                    )
                },
            )
        }
        3 => {
            // All-match GD
            let sub = sort_by_criteria(teams, |t| -(all_stats.get(t).map_or(0, |s| s.goal_diff())));
            split_and_resolve_groups(
                &sub,
                |group| {
                    let vals: Vec<i32> = group
                        .iter()
                        .map(|t| all_stats.get(t).map_or(0, |s| s.goal_diff()))
                        .collect();
                    vals.windows(2).all(|w| w[0] == w[1])
                },
                |group| {
                    resolve_sub_group(
                        group,
                        all_stats,
                        h2h_stats,
                        h2h_games_list,
                        predictions,
                        draw_order,
                        4,
                    )
                },
            )
        }
        4 => {
            // All-match goals for
            let sub = sort_by_criteria(teams, |t| -(all_stats.get(t).map_or(0, |s| s.goals_for)));
            split_and_resolve_groups(
                &sub,
                |group| {
                    let vals: Vec<i32> = group
                        .iter()
                        .map(|t| all_stats.get(t).map_or(0, |s| s.goals_for))
                        .collect();
                    vals.windows(2).all(|w| w[0] == w[1])
                },
                |group| {
                    resolve_sub_group(
                        group,
                        all_stats,
                        h2h_stats,
                        h2h_games_list,
                        predictions,
                        draw_order,
                        5,
                    )
                },
            )
        }
        _ => {
            // draw_order fallback
            let mut result = teams.to_vec();
            result.sort_by_key(|t| draw_order.iter().position(|d| d == t).unwrap_or(usize::MAX));
            result
        }
    }
}

/// Sort teams by a key function (ascending key = better rank).
fn sort_by_criteria<F>(teams: &[TeamId], key_fn: F) -> Vec<TeamId>
where
    F: Fn(&TeamId) -> i32,
{
    let mut sorted = teams.to_vec();
    sorted.sort_by_key(|t| key_fn(t));
    sorted
}

/// Given a sorted list, split into groups with equal criteria values and apply
/// `resolve_fn` to sub-groups that still have ties.
fn split_and_resolve_groups<EqualFn, ResolveFn>(
    sorted: &[TeamId],
    is_tied: EqualFn,
    resolve_fn: ResolveFn,
) -> Vec<TeamId>
where
    EqualFn: Fn(&[TeamId]) -> bool,
    ResolveFn: Fn(&[TeamId]) -> Vec<TeamId>,
{
    if sorted.is_empty() {
        return vec![];
    }

    let mut result: Vec<TeamId> = Vec::new();
    let mut i = 0;

    while i < sorted.len() {
        // Find how many consecutive teams have the same criteria value
        let mut j = i + 1;
        while j < sorted.len() {
            let window = &sorted[i..=j];
            if is_tied(window) {
                j += 1;
            } else {
                break;
            }
        }
        let group = &sorted[i..j];
        if group.len() == 1 {
            result.push(group[0].clone());
        } else {
            let resolved = resolve_fn(group);
            result.extend(resolved);
        }
        i = j;
    }

    result
}

/// Rank a group's teams from a player's predicted match scores, applying the
/// `SCORING.md` §4 ladder. `draw_order` resolves residual ties.
pub fn rank_group(
    _group: &GroupGame,
    games: &[&SingleGame],
    predictions: &[&MatchPrediction],
    draw_order: &[TeamId],
) -> Vec<TeamId> {
    let teams = collect_teams(games);
    if teams.is_empty() {
        return vec![];
    }

    let all_stats = compute_stats(&teams, games, predictions);

    rank_group_sort(&teams, &all_stats, games, predictions, draw_order)
}

/// Standings bonus: `standings_pair_point` per team-pair whose relative order
/// in `predicted` matches `official` (`SCORING.md` §4).
pub fn standings_bonus(predicted: &[TeamId], official: &[TeamId], c: &ScoringConfig) -> i64 {
    let mut bonus: i64 = 0;

    // For every pair (i, j) where i < j in one ordering, check if same relative order in other.
    for i in 0..official.len() {
        for j in (i + 1)..official.len() {
            let team_a = &official[i]; // official: a ranks above b
            let team_b = &official[j];

            // Find positions in predicted
            let pos_a = predicted.iter().position(|t| t == team_a);
            let pos_b = predicted.iter().position(|t| t == team_b);

            if let (Some(pa), Some(pb)) = (pos_a, pos_b) {
                // In official, a (at pos i) is before b (at pos j), i.e., official order = a > b
                // In predicted, same order iff pa < pb
                if pa < pb {
                    bonus += c.standings_pair_point;
                }
            }
        }
    }

    bonus
}

// ─── Tournament scoring internals ───────────────────────────────────────────

/// Score a single leaf group node (has direct SingleGames).
/// Returns raw points (before multiplier).
fn score_leaf_group(
    group: &GroupGame,
    games: &[&SingleGame],
    prediction: &Player,
    result: &Player,
    now: DateTime<Utc>,
    deadline: DateTime<Utc>,
    c: &ScoringConfig,
) -> i64 {
    let mut raw: i64 = 0;

    // Per-match scoring
    for game in games {
        let pred_mp = prediction.match_prediction(&game.id);
        let result_mp = result.match_prediction(&game.id);

        if let (Some(pred_mp), Some(result_mp)) = (pred_mp, result_mp) {
            // Result must be locked (not just effective-locked) per spec §1
            if !result_mp.locked {
                continue;
            }
            // Prediction must be effective-locked
            // "complete" for a MatchPrediction: it always has both scores (no Option),
            // so a MatchPrediction's existence implies it's complete.
            let p_locked = effective_locked(pred_mp.locked, now, deadline, true);
            if !p_locked {
                continue;
            }
            raw += score_match(pred_mp, result_mp, c);
        }
    }

    // Standings bonus (if group carries standings)
    if group.carries_standings {
        let pred_sp = prediction.standings_prediction(&group.id);
        let result_sp = result.standings_prediction(&group.id);

        if let (Some(pred_sp), Some(result_sp)) = (pred_sp, result_sp) {
            // Result standings must be locked
            if !result_sp.locked {
                return raw; // no bonus
            }
            // Predicted standings must be effective-locked
            let p_locked =
                effective_locked(pred_sp.locked, now, deadline, !pred_sp.ordering.is_empty());
            if !p_locked {
                return raw; // no bonus
            }

            // Compute official standings from result's predictions via rank_group
            let official_ranking = rank_group(
                group,
                games,
                &result_mp_refs(result, games),
                &result_sp.draw_order,
            );
            // Compute predicted standings from prediction's predictions via rank_group
            let predicted_ranking = rank_group(
                group,
                games,
                &result_mp_refs(prediction, games),
                &pred_sp.draw_order,
            );

            // If we have explicit orderings in the StandingsPredictions, use them directly
            // when rank_group would produce empty results (e.g., no predictions).
            // Actually: rank_group uses predictions to compute standings, which is what we want.
            // The StandingsPrediction.ordering is the player's explicit ordering (for display/storage),
            // but ranking is computed from match results for the bonus comparison.
            let bonus = standings_bonus(&predicted_ranking, &official_ranking, c);
            raw += bonus;
        }
    }

    raw
}

/// Helper: collect MatchPrediction references for a player's games.
fn result_mp_refs<'a>(player: &'a Player, games: &[&SingleGame]) -> Vec<&'a MatchPrediction> {
    games
        .iter()
        .filter_map(|g| player.match_prediction(&g.id))
        .collect()
}

/// Recursively score a group node and all its descendants.
/// Returns a per-Round breakdown of points (with multipliers already applied).
fn score_group_node(
    group_id: &str,
    t: &Tournament,
    prediction: &Player,
    result: &Player,
    now: DateTime<Utc>,
    c: &ScoringConfig,
    breakdown: &mut HashMap<Round, i64>,
) {
    let group = match t.groups.get(group_id) {
        Some(g) => g,
        None => return,
    };

    match &group.children {
        GroupChildren::Games(game_ids) => {
            // Leaf group: score matches + standings bonus.
            let games: Vec<&SingleGame> =
                game_ids.iter().filter_map(|id| t.games.get(id)).collect();

            let deadline = t.deadline(group_id).unwrap_or_else(Utc::now);
            let raw = score_leaf_group(group, &games, prediction, result, now, deadline, c);
            let multiplied = raw * c.multiplier(group.round);
            *breakdown.entry(group.round).or_insert(0) += multiplied;
        }
        GroupChildren::Groups(child_ids) => {
            // Internal node: recurse into children.
            // Internal nodes themselves don't carry match points (their children do).
            for child_id in child_ids {
                score_group_node(child_id, t, prediction, result, now, c, breakdown);
            }
        }
    }
}

/// Whole-tournament score of one prediction-set against a baseline result-set.
/// Per-stage breakdown (`SCORING.md` §8), multipliers applied. Only
/// effective-locked predictions contribute.
pub fn score_tournament(
    t: &Tournament,
    prediction: &Player,
    result: &Player,
    now: DateTime<Utc>,
    c: &ScoringConfig,
) -> HashMap<Round, i64> {
    let mut breakdown: HashMap<Round, i64> = HashMap::new();
    score_group_node(&t.root, t, prediction, result, now, c, &mut breakdown);
    breakdown
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn score_match_max_points() {
        let c = ScoringConfig::default();
        let p = MatchPrediction {
            game_id: "x".into(),
            home_score: 1,
            away_score: 0,
            locked: true,
        };
        let r = MatchPrediction {
            game_id: "x".into(),
            home_score: 1,
            away_score: 0,
            locked: true,
        };
        assert_eq!(score_match(&p, &r, &c), 4);
    }

    #[test]
    fn effective_locked_truth_table() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let future = Utc::now() + chrono::Duration::hours(1);
        let now = Utc::now();

        // locked=true → always true
        assert!(effective_locked(true, now, future, false));
        assert!(effective_locked(true, now, future, true));
        assert!(effective_locked(true, now, past, false));

        // locked=false, now > deadline, complete=true → true
        assert!(effective_locked(false, now, past, true));

        // locked=false, now > deadline, complete=false → false
        assert!(!effective_locked(false, now, past, false));

        // locked=false, now <= deadline → false regardless of complete
        assert!(!effective_locked(false, now, future, true));
        assert!(!effective_locked(false, now, future, false));
    }

    #[test]
    fn standings_bonus_empty_teams() {
        let c = ScoringConfig::default();
        assert_eq!(standings_bonus(&[], &[], &c), 0);
    }
}
