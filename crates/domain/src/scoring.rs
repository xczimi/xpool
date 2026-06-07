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

/// Rank a set of teams by the `SCORING.md` §4 ladder.
///
/// This is the recursive core and the **entry point**. It handles one tied
/// set per call:
///
/// 1. Partition by **overall points** (descending). Single-team partitions are
///    placed directly.
/// 2. Each still-tied partition runs the head-to-head sub-ladder
///    (`rank_h2h`) — H2H points, then H2H goal difference, then H2H goals —
///    with H2H stats recomputed among *only* that partition's teams.
/// 3. A subset still tied after H2H falls to all-match goal difference, then
///    all-match goals (`rank_all_match`).
/// 4. Anything still tied falls to the player's manual `draw_order`.
///
/// **Strict-FIFA reapplication (issue #12):** whenever an H2H rung *separates*
/// part of a tied set but leaves a smaller subset tied, that subset re-enters
/// here at step 1 — so its H2H table is recomputed among only its own teams.
/// `all_stats` (the whole-group table, used for steps 3–4) never changes.
fn rank_tied(
    teams: &[TeamId],
    all_stats: &HashMap<TeamId, TeamStats>,
    games: &[&SingleGame],
    predictions: &[&MatchPrediction],
    draw_order: &[TeamId],
) -> Vec<TeamId> {
    if teams.len() <= 1 {
        return teams.to_vec();
    }

    partition_and_rank(
        teams,
        |t| all_stats.get(t).map_or(0, |s| s.points),
        |tied| rank_h2h(tied, all_stats, games, predictions, draw_order),
    )
}

/// Head-to-head sub-ladder (`SCORING.md` §4 step 2) for a points-tied set.
///
/// H2H stats are **recomputed among only `teams`** — their games against each
/// other — then the three H2H rungs (points, goal difference, goals) are
/// applied in order. When a rung separates part of the set, each still-tied
/// subset that is *strictly smaller* restarts the whole ladder via `rank_tied`
/// (strict-FIFA reapplication, issue #12); a subset that did not shrink simply
/// advances to the next rung. A set that survives all three H2H rungs intact
/// falls through to the all-match steps.
fn rank_h2h(
    teams: &[TeamId],
    all_stats: &HashMap<TeamId, TeamStats>,
    games: &[&SingleGame],
    predictions: &[&MatchPrediction],
    draw_order: &[TeamId],
) -> Vec<TeamId> {
    if teams.len() <= 1 {
        return teams.to_vec();
    }

    // Recompute H2H stats over *only* the still-tied teams.
    let team_refs: Vec<&TeamId> = teams.iter().collect();
    let h2h = h2h_games(&team_refs, games);
    let h2h_stats = compute_stats(teams, &h2h, predictions);

    // The three H2H rungs, each a metric over the recomputed H2H table.
    let h2h_metrics: [fn(&TeamStats) -> i32; 3] =
        [|s| s.points, |s| s.goal_diff(), |s| s.goals_for];

    rank_h2h_rung(
        teams,
        &h2h_stats,
        all_stats,
        games,
        predictions,
        draw_order,
        &h2h_metrics,
        0,
    )
}

/// Apply one H2H rung (`metrics[idx]`) to a tied set.
///
/// After sorting by the rung's metric, each equal-valued run is examined:
/// - a run that *shrank* (separated from siblings) and is still tied restarts
///   the whole ladder at `rank_tied` — strict-FIFA reapplication;
/// - a run equal in size to the input (nothing separated) advances to the next
///   H2H rung, or to the all-match steps once the H2H rungs are exhausted.
#[allow(clippy::too_many_arguments)]
fn rank_h2h_rung(
    teams: &[TeamId],
    h2h_stats: &HashMap<TeamId, TeamStats>,
    all_stats: &HashMap<TeamId, TeamStats>,
    games: &[&SingleGame],
    predictions: &[&MatchPrediction],
    draw_order: &[TeamId],
    metrics: &[fn(&TeamStats) -> i32; 3],
    idx: usize,
) -> Vec<TeamId> {
    if teams.len() <= 1 {
        return teams.to_vec();
    }
    if idx >= metrics.len() {
        // H2H exhausted with the set still intact — fall to all-match steps.
        return rank_all_match(teams, all_stats, draw_order, 0);
    }

    let metric = metrics[idx];
    partition_and_rank(
        teams,
        |t| h2h_stats.get(t).map_or(0, metric),
        |run| {
            if run.len() == teams.len() {
                // Nothing separated on this rung — advance within the same
                // H2H table to the next rung.
                rank_h2h_rung(
                    run,
                    h2h_stats,
                    all_stats,
                    games,
                    predictions,
                    draw_order,
                    metrics,
                    idx + 1,
                )
            } else {
                // This rung separated the set; the still-tied subset restarts
                // the whole ladder so its H2H table is recomputed for itself.
                rank_tied(run, all_stats, games, predictions, draw_order)
            }
        },
    )
}

/// All-match tiebreak steps (`SCORING.md` §4 steps 3–5) for a subset still tied
/// after the H2H sub-ladder: all-match goal difference, then all-match goals,
/// then the manual `draw_order`. These use whole-group stats and never trigger
/// H2H recomputation.
fn rank_all_match(
    teams: &[TeamId],
    all_stats: &HashMap<TeamId, TeamStats>,
    draw_order: &[TeamId],
    step: usize,
) -> Vec<TeamId> {
    if teams.len() <= 1 {
        return teams.to_vec();
    }

    let metric: fn(&TeamStats) -> i32 = match step {
        0 => |s| s.goal_diff(),
        1 => |s| s.goals_for,
        _ => {
            // Terminal: order by the player's manual draw_order.
            let mut result = teams.to_vec();
            result.sort_by_key(|t| draw_order.iter().position(|d| d == t).unwrap_or(usize::MAX));
            return result;
        }
    };

    partition_and_rank(
        teams,
        |t| all_stats.get(t).map_or(0, metric),
        |run| rank_all_match(run, all_stats, draw_order, step + 1),
    )
}

/// Sort `teams` by `key` (descending = better rank), then split into runs of
/// equal key. Single-team runs are placed as-is; multi-team runs (still tied)
/// are passed to `resolve` and the results spliced in order.
fn partition_and_rank<KeyFn, ResolveFn>(
    teams: &[TeamId],
    key: KeyFn,
    resolve: ResolveFn,
) -> Vec<TeamId>
where
    KeyFn: Fn(&TeamId) -> i32,
    ResolveFn: Fn(&[TeamId]) -> Vec<TeamId>,
{
    let mut sorted = teams.to_vec();
    sorted.sort_by_key(|t| std::cmp::Reverse(key(t))); // descending

    let mut result: Vec<TeamId> = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let k = key(&sorted[i]);
        let mut j = i + 1;
        while j < sorted.len() && key(&sorted[j]) == k {
            j += 1;
        }
        let run = &sorted[i..j];
        if run.len() == 1 {
            result.push(run[0].clone());
        } else {
            result.extend(resolve(run));
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

    rank_tied(&teams, &all_stats, games, predictions, draw_order)
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
            // Result must be effective-locked — the SAME rule as a prediction.
            // The group `deadline` (the earliest kickoff in the group, NOT each
            // game's own kickoff) implicitly locks an entered result; results are
            // entered after the match, so they are always past it. No explicit-
            // lock requirement and no result-user special case (unified entry).
            let r_locked = effective_locked(result_mp.locked, now, deadline, true);
            if !r_locked {
                continue;
            }
            // Prediction must be effective-locked. `complete` is read
            // PER MATCH (`DATA_MODEL.md` §7, `SCORING.md` §3): every
            // `MatchPrediction` is its own draft and always carries both
            // `u8` scores, so it is always complete. It is therefore passed
            // `true` here. Consequence: in an unlocked `LockTogether` group,
            // each game that *was* predicted auto-counts after the deadline
            // independently; games with no `MatchPrediction` at all simply
            // score 0 (they never reach this loop body).
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
            // Result standings must be effective-locked — same rule as the
            // predicted standings below.
            let r_sp_locked = effective_locked(
                result_sp.locked,
                now,
                deadline,
                !result_sp.ordering.is_empty(),
            );
            if !r_sp_locked {
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

            // A leaf group with no resolvable games has no deadline. `domain`
            // is clock-free (CLAUDE.md "Server-authoritative clock") — never
            // fall back to the wall clock. Treat an unresolvable deadline as
            // "far in the future" so an unscored leaf group is never silently
            // auto-locked (`now > deadline` stays false for any real `now`).
            let deadline = t.deadline(group_id).unwrap_or(DateTime::<Utc>::MAX_UTC);
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
        // Fixed timestamps — `domain` is clock-free, even in tests.
        let now = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 6, 1, 12, 0, 0).unwrap();
        let past = now - chrono::Duration::hours(1);
        let future = now + chrono::Duration::hours(1);

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
