//! GraphQL input types for mutations.

use async_graphql::InputObject;

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
