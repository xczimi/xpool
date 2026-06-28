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

/// Which components of a per-match score were earned (`SCORING.md` §3). Each
/// flag drives a fixed point award: `exact_home`/`exact_away` → `exact_score_point`,
/// `outcome` → `outcome_point`. Exposed so callers can show *how* a score was
/// earned without re-deriving the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchScoreParts {
    /// Home score matched exactly, or both sides scored ≥ threshold (4-goal rule).
    pub exact_home: bool,
    /// Away score matched exactly, or both sides scored ≥ threshold (4-goal rule).
    pub exact_away: bool,
    /// Correct outcome (win / draw / loss).
    pub outcome: bool,
}

/// Decompose a per-match score into its components (`SCORING.md` §3). The
/// per-side, symmetric 4-goal rule treats a side as exact when both predicted
/// and actual are ≥ `high_scoring_threshold`.
pub fn score_match_parts(
    p: &MatchPrediction,
    r: &MatchPrediction,
    c: &ScoringConfig,
) -> MatchScoreParts {
    let thr = c.high_scoring_threshold;
    MatchScoreParts {
        exact_home: p.home_score == r.home_score || (p.home_score >= thr && r.home_score >= thr),
        exact_away: p.away_score == r.away_score || (p.away_score >= thr && r.away_score >= thr),
        outcome: outcome_sign(p.home_score, p.away_score)
            == outcome_sign(r.home_score, r.away_score),
    }
}

impl MatchScoreParts {
    /// The points these parts are worth under `c`.
    pub fn points(&self, c: &ScoringConfig) -> i64 {
        (self.exact_home as i64) * c.exact_score_point
            + (self.exact_away as i64) * c.exact_score_point
            + (self.outcome as i64) * c.outcome_point
    }
}

/// Per-match points: prediction `p` vs result `r`, both 90-minute scores.
/// Max `2*exact + outcome`. Implements the per-side, symmetric 4-goal rule
/// (`SCORING.md` §3).
pub fn score_match(p: &MatchPrediction, r: &MatchPrediction, c: &ScoringConfig) -> i64 {
    score_match_parts(p, r, c).points(c)
}

/// A prediction is a "perfect" when it scored the maximum (`SCORING.md` §7).
pub fn is_perfect(p: &MatchPrediction, r: &MatchPrediction, c: &ScoringConfig) -> bool {
    score_match(p, r, c) >= c.perfect_threshold
}

/// The **best score still mathematically reachable** for a prediction `p` given
/// a live score `live`, returned **multiplied** by `multiplier`.
///
/// Goals only go up, so the final `(h, a)` satisfies `h >= live.home_score`,
/// `a >= live.away_score`. `score_match` reads only three flags (`exact_home`,
/// `exact_away`, `outcome`). The `exact` flags saturate once a side passes
/// `max(predicted, threshold)`, but the **`outcome`** flag couples the two
/// sides: to reach a draw (or to let the trailing side overtake for a win) a
/// side may have to climb up to the *other* side's level. So the safe per-axis
/// bound is the **global** max across *both* predicted scores, *both* live
/// scores and the threshold, plus one — beyond that point every flag is settled
/// and pushing further only widens the difference monotonically. (A tighter
/// per-axis bound that ignores the opposing side is wrong: e.g. `p = 0–0`,
/// `live = 0–6` needs the home side to reach 6 to match the draw outcome.)
/// The grid stays tiny (scores are small, threshold is 4), so the brute force is
/// cheap and exact — verified exhaustively against a far larger ground-truth
/// grid in tests.
pub fn max_reachable_score(
    p: &MatchPrediction,
    live: &MatchPrediction,
    c: &ScoringConfig,
    multiplier: i64,
) -> i64 {
    let thr = c.high_scoring_threshold;
    // One past every "interesting" value on EITHER side — the outcome flag
    // couples the axes, so each side must be enumerated up to the global max.
    let hi = p
        .home_score
        .max(p.away_score)
        .max(live.home_score)
        .max(live.away_score)
        .max(thr)
        .saturating_add(1);

    let mut best = 0;
    for fh in live.home_score..=hi {
        for fa in live.away_score..=hi {
            let final_score = MatchPrediction {
                game_id: p.game_id.clone(),
                home_score: fh,
                away_score: fa,
                locked: true,
            };
            best = best.max(score_match(p, &final_score, c));
        }
    }
    best * multiplier
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

/// Count team-pairs whose relative order in `predicted` matches `official`,
/// and the total comparable pairs. `(correct, total)` — the standings bonus is
/// `correct * standings_pair_point` (`SCORING.md` §4).
pub fn standings_pairs(predicted: &[TeamId], official: &[TeamId]) -> (usize, usize) {
    let mut correct = 0;
    let mut total = 0;

    // For every pair (i, j) with i < j in `official` (a ranks above b), check
    // whether `predicted` keeps the same relative order.
    for i in 0..official.len() {
        for j in (i + 1)..official.len() {
            let pos_a = predicted.iter().position(|t| t == &official[i]);
            let pos_b = predicted.iter().position(|t| t == &official[j]);
            if let (Some(pa), Some(pb)) = (pos_a, pos_b) {
                total += 1;
                if pa < pb {
                    correct += 1;
                }
            }
        }
    }

    (correct, total)
}

/// Standings bonus: `standings_pair_point` per team-pair whose relative order
/// in `predicted` matches `official` (`SCORING.md` §4).
pub fn standings_bonus(predicted: &[TeamId], official: &[TeamId], c: &ScoringConfig) -> i64 {
    standings_pairs(predicted, official).0 as i64 * c.standings_pair_point
}

/// How a player's standings bonus for one group was earned (before multiplier).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingsBreakdown {
    /// Pairs ordered correctly vs the official final table.
    pub pairs_correct: usize,
    /// Total comparable pairs (the maximum achievable).
    pub pairs_total: usize,
    /// Raw bonus points (`pairs_correct * standings_pair_point`), pre-multiplier.
    pub bonus: i64,
}

/// Score one player's standings prediction for a leaf group against the
/// official outcome — the per-group sibling of the standings block inside
/// `score_leaf_group`, exposed so the API can show the bonus transparently.
///
/// Returns `None` when the group carries no standings, when either side's
/// standings prediction is not yet effective-locked (so the bonus isn't
/// scoreable), or when there is no comparable ranking. Both the official and
/// predicted final orders are derived from match scores via `rank_group`, the
/// same way the scoreboard computes them.
pub fn standings_score(
    group: &GroupGame,
    games: &[&SingleGame],
    prediction: &Player,
    result: &Player,
    now: DateTime<Utc>,
    deadline: DateTime<Utc>,
    c: &ScoringConfig,
) -> Option<StandingsBreakdown> {
    if !group.carries_standings {
        return None;
    }
    let pred_sp = prediction.standings_prediction(&group.id)?;
    let result_sp = result.standings_prediction(&group.id)?;

    // Both standings must be effective-locked — the same gate `score_leaf_group`
    // applies, so this reconciles with the scoreboard.
    let r_locked = effective_locked(
        result_sp.locked,
        now,
        deadline,
        !result_sp.ordering.is_empty(),
    );
    let p_locked = effective_locked(pred_sp.locked, now, deadline, !pred_sp.ordering.is_empty());
    if !r_locked || !p_locked {
        return None;
    }

    // Completion gate (monotonicity): award the bonus ONLY when the group is
    // complete — every game in the leaf group has an official (result-user)
    // result entered and effective-locked. Until then the provisional ranking
    // can still shift as results land, so `pairs_correct` — and the bonus —
    // could decrease; a player's committed points must never go down. Gating on
    // completion makes the bonus final-and-stable, and reconciles the
    // materialised scoreboard exactly with the points-timeline (which settles a
    // group's bonus at its last game). The per-game check mirrors the
    // result-presence gate in `score_leaf_group` so the definition stays
    // consistent.
    let group_complete = games.iter().all(|game| {
        result
            .match_prediction(&game.id)
            .is_some_and(|r| effective_locked(r.locked, now, deadline, true))
    });
    if !group_complete {
        return None;
    }

    let official = rank_group(
        group,
        games,
        &result_mp_refs(result, games),
        &result_sp.draw_order,
    );
    let predicted = rank_group(
        group,
        games,
        &result_mp_refs(prediction, games),
        &pred_sp.draw_order,
    );
    let (pairs_correct, pairs_total) = standings_pairs(&predicted, &official);
    Some(StandingsBreakdown {
        pairs_correct,
        pairs_total,
        bonus: pairs_correct as i64 * c.standings_pair_point,
    })
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

    // Standings bonus (if group carries standings) — the same computation the
    // API exposes per group via `standings_score`, so the two never drift.
    if let Some(sb) = standings_score(group, games, prediction, result, now, deadline, c) {
        raw += sb.bonus;
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

    fn mp(h: u8, a: u8) -> MatchPrediction {
        MatchPrediction {
            game_id: "x".into(),
            home_score: h,
            away_score: a,
            locked: true,
        }
    }

    #[test]
    fn score_match_parts_decompose_and_sum_to_score_match() {
        let c = ScoringConfig::default();
        // Right outcome, away exact, home wrong: 0 + 1 + 2 = 3.
        let parts = score_match_parts(&mp(2, 1), &mp(3, 1), &c);
        assert_eq!(
            parts,
            MatchScoreParts {
                exact_home: false,
                exact_away: true,
                outcome: true
            }
        );
        assert_eq!(parts.points(&c), 3);
        assert_eq!(parts.points(&c), score_match(&mp(2, 1), &mp(3, 1), &c));
    }

    #[test]
    fn score_match_parts_four_goal_rule_counts_a_side_as_exact() {
        let c = ScoringConfig::default();
        // Home differs (5 vs 4) but both ≥ threshold → exact_home true; away exact;
        // outcome (home win) correct.
        let parts = score_match_parts(&mp(5, 0), &mp(4, 0), &c);
        assert!(parts.exact_home, "4-goal rule makes the home side count");
        assert!(parts.exact_away);
        assert!(parts.outcome);
        assert_eq!(parts.points(&c), 4);
    }

    #[test]
    fn standings_pairs_counts_correct_and_total() {
        let order = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let official = order(&["A", "B", "C"]); // 3 pairs: AB, AC, BC
                                                // Swap the last two → AB, AC correct, BC reversed → 2 of 3.
        let predicted = order(&["A", "C", "B"]);
        assert_eq!(standings_pairs(&predicted, &official), (2, 3));
        assert_eq!(standings_pairs(&official, &official), (3, 3));
    }

    // ─── max_reachable_score ─────────────────────────────────────────────────

    /// Live score helper: a `MatchPrediction` standing in for "the score now".
    fn live(h: u8, a: u8) -> MatchPrediction {
        MatchPrediction {
            game_id: "x".into(),
            home_score: h,
            away_score: a,
            locked: true,
        }
    }

    #[test]
    fn max_reachable_exact_still_reachable() {
        let c = ScoringConfig::default();
        // Predicted 1–0; it's currently 1–0. The exact final 1–0 is reachable.
        // Best base = 2 exact + outcome = 4. multiplier 1 → 4.
        assert_eq!(max_reachable_score(&mp(1, 0), &live(1, 0), &c, 1), 4);
    }

    #[test]
    fn max_reachable_exact_home_lost_but_outcome_and_away_kept() {
        let c = ScoringConfig::default();
        // Predicted 1–0 (home win); it's 2–0. final.home >= 2, so home can never
        // equal predicted 1 → exact_home lost. final.away == 0 reachable →
        // exact_away kept. Home win reachable (2–0) → outcome kept.
        // base = 0 + 1 + 2 = 3.
        assert_eq!(max_reachable_score(&mp(1, 0), &live(2, 0), &c, 1), 3);
    }

    #[test]
    fn max_reachable_predicted_draw_outcome_lost_keeps_only_what_survives() {
        let c = ScoringConfig::default();
        // Predicted 0–0 (draw); it's 0–1. A draw needs final.home == final.away
        // with away >= 1 (e.g. 1–1): exact_home (need 0) lost, exact_away (need 0)
        // lost, draw outcome reachable. base = outcome only = 2.
        assert_eq!(max_reachable_score(&mp(0, 0), &live(0, 1), &c, 1), 2);
    }

    #[test]
    fn max_reachable_multiplier_is_applied() {
        let c = ScoringConfig::default();
        // Same as the exact case but a knockout multiplier (R32 = 2): 4 * 2 = 8.
        let m = c.multiplier(Round::R32);
        assert_eq!(max_reachable_score(&mp(1, 0), &live(1, 0), &c, m), 8);
    }

    #[test]
    fn max_reachable_four_goal_rule_keeps_high_scoring_exact() {
        let c = ScoringConfig::default();
        // Predicted 5–0; it's 4–0. Both home sides >= threshold (4) → exact_home
        // counts for any final.home >= 4. away 0 reachable → exact_away. home win
        // reachable → outcome. base = 4.
        assert_eq!(max_reachable_score(&mp(5, 0), &live(4, 0), &c, 1), 4);
    }

    #[test]
    fn max_reachable_high_scoring_draw_both_sides_exact() {
        let c = ScoringConfig::default();
        // Predicted 4–4; it's 4–4. Both sides >= threshold for any growth, and a
        // draw stays reachable (e.g. 5–5). base = 2 exact + draw outcome = 4.
        assert_eq!(max_reachable_score(&mp(4, 4), &live(4, 4), &c, 1), 4);
    }

    #[test]
    fn max_reachable_never_below_current_best() {
        let c = ScoringConfig::default();
        // The reachable max must be >= the score the prediction already earns
        // against the live score treated as if final (a sanity monotonicity guard).
        let p = mp(2, 1);
        let l = live(2, 1);
        let now_score = score_match(&p, &l, &c);
        assert!(max_reachable_score(&p, &l, &c, 1) >= now_score);
    }

    #[test]
    fn max_reachable_outcome_needs_climbing_past_the_other_side() {
        let c = ScoringConfig::default();
        // Predicted draw 0–0; it's 0–6. A draw is still reachable (6–6), but the
        // home side must climb all the way up to 6 — past `max(p, live.home, thr)`.
        // The bound must account for the opposing side, else this returns 0.
        // base = draw outcome only = 2.
        assert_eq!(max_reachable_score(&mp(0, 0), &live(0, 6), &c, 1), 2);
        // Predicted home win 1–0; it's 0–6. A home win (e.g. 7–6) is reachable →
        // outcome 2 points (exact_home not reachable: final.home != 1 once > live.away
        // forces it past 6). The home side climbs well past `max(1, 0, 4)+1`.
        assert_eq!(max_reachable_score(&mp(1, 0), &live(0, 6), &c, 1), 2);
    }

    #[test]
    fn max_reachable_matches_bruteforce_ground_truth() {
        // Exhaustive check: the bounded grid must equal a far larger brute-force
        // grid for every reachable prediction/live pair in 0..=10. This is the
        // guard that the per-axis bound is provably sufficient — the subtle
        // outcome-coupling bug (a side must climb to the opposing side's level)
        // would surface here.
        let c = ScoringConfig::default();
        let truth = |p: &MatchPrediction, l: &MatchPrediction| -> i64 {
            let mut best = 0;
            for fh in l.home_score..=l.home_score.saturating_add(60) {
                for fa in l.away_score..=l.away_score.saturating_add(60) {
                    best = best.max(score_match(
                        p,
                        &MatchPrediction {
                            game_id: "x".into(),
                            home_score: fh,
                            away_score: fa,
                            locked: true,
                        },
                        &c,
                    ));
                }
            }
            best
        };
        for ph in 0u8..=10 {
            for pa in 0u8..=10 {
                for lh in 0u8..=10 {
                    for la in 0u8..=10 {
                        let p = mp(ph, pa);
                        let l = live(lh, la);
                        assert_eq!(
                            max_reachable_score(&p, &l, &c, 1),
                            truth(&p, &l),
                            "mismatch at p={ph}-{pa} live={lh}-{la}"
                        );
                    }
                }
            }
        }
    }
}
