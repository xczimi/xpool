//! GraphQL input types for mutations.

use async_graphql::InputObject;

/// The largest score the API accepts for a single side of a match. A generous
/// cap that still rejects nonsense — no soccer match reaches double digits in
/// regulation, but the bound is kept loose to avoid surprises.
pub const MAX_SCORE: i32 = 99;

/// Validate a single match score: it must be non-negative and within
/// [`MAX_SCORE`]. Returns the score as a `u8`, or a GraphQL error — scores are
/// rejected, never coerced (`.specs` input-validation rules).
pub fn validate_score(label: &str, score: i32) -> async_graphql::Result<u8> {
    if !(0..=MAX_SCORE).contains(&score) {
        return Err(async_graphql::Error::new(format!(
            "{label} score {score} is out of range (0..={MAX_SCORE})"
        )));
    }
    Ok(score as u8)
}

/// One match prediction submitted via `submitGroup`.
#[derive(InputObject, Clone, Debug)]
pub struct MatchPredictionInput {
    pub game_id: String,
    pub home_score: i32,
    pub away_score: i32,
}

/// A group's standings prediction submitted via `submitGroup`.
#[derive(InputObject, Clone, Debug)]
pub struct StandingsInput {
    /// The predicted final ordering of the node's teams.
    pub ordering: Vec<String>,
    /// Manual tiebreak for everything not score-derivable.
    pub draw_order: Vec<String>,
}
