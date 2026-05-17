//! GraphQL input types for mutations.

use async_graphql::InputObject;

/// One match prediction submitted via `submitGroup`.
#[derive(InputObject, Clone, Debug)]
pub struct MatchPredictionInput {
    pub game_id: String,
    pub home_score: i32,
    pub away_score: i32,
}
