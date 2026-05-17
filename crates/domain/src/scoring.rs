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

/// Per-match points: prediction `p` vs result `r`, both 90-minute scores.
/// Max `2*exact + outcome`. Implements the per-side, symmetric 4-goal rule
/// (`SCORING.md` §3).
pub fn score_match(_p: &MatchPrediction, _r: &MatchPrediction, _c: &ScoringConfig) -> i64 {
    todo!("P1: implement per SCORING.md §3")
}

/// A prediction is a "perfect" when it scored the maximum (`SCORING.md` §7).
pub fn is_perfect(_p: &MatchPrediction, _r: &MatchPrediction, _c: &ScoringConfig) -> bool {
    todo!("P1: implement per SCORING.md §7")
}

/// Effective-locked (`DATA_MODEL.md` §7): `locked OR (now > deadline AND complete)`.
pub fn effective_locked(
    _locked: bool,
    _now: DateTime<Utc>,
    _deadline: DateTime<Utc>,
    _complete: bool,
) -> bool {
    todo!("P1: implement per DATA_MODEL.md §7")
}

/// Rank a group's teams from a player's predicted match scores, applying the
/// `SCORING.md` §4 ladder. `draw_order` resolves residual ties.
pub fn rank_group(
    _group: &GroupGame,
    _games: &[&SingleGame],
    _predictions: &[&MatchPrediction],
    _draw_order: &[TeamId],
) -> Vec<TeamId> {
    todo!("P1: implement per SCORING.md §4")
}

/// Standings bonus: `standings_pair_point` per team-pair whose relative order
/// in `predicted` matches `official` (`SCORING.md` §4).
pub fn standings_bonus(_predicted: &[TeamId], _official: &[TeamId], _c: &ScoringConfig) -> i64 {
    todo!("P1: implement per SCORING.md §4")
}

/// Whole-tournament score of one prediction-set against a baseline result-set.
/// Per-stage breakdown (`SCORING.md` §8), multipliers applied. Only
/// effective-locked predictions contribute.
pub fn score_tournament(
    _t: &Tournament,
    _prediction: &Player,
    _result: &Player,
    _now: DateTime<Utc>,
    _c: &ScoringConfig,
) -> HashMap<Round, i64> {
    todo!("P1: implement per SCORING.md §3-6,8")
}
