//! The GraphQL query root (`API.md` §4).
//!
//! Each query loads the coarse storage items it needs once, then assembles
//! the response from memory. Resolvers call the pure `domain`/`fwc26`
//! functions; they contain no domain logic.

use crate::auth::CurrentPlayer;
use crate::gql::types::*;
use async_graphql::{Context, Object};
use chrono::Utc;
use domain::scoring::{is_perfect, ScoringConfig};
use std::collections::HashMap;
use storage::Repository;

pub struct QueryRoot;

/// Pull the `Repository` out of the GraphQL context.
fn repo<'a>(ctx: &'a Context<'_>) -> &'a dyn Repository {
    ctx.data_unchecked::<std::sync::Arc<dyn Repository>>()
        .as_ref()
}

#[Object]
impl QueryRoot {
    /// The `<t>#TOURNAMENT` structure — tree, matches, teams.
    async fn tournament(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<Tournament>> {
        let t = repo(ctx).get_tournament().await?;
        Ok(t.as_ref().map(Tournament::from))
    }

    /// The materialised scoreboard, optionally filtered to a pool's members.
    async fn scoreboard(
        &self,
        ctx: &Context<'_>,
        pool: Option<String>,
    ) -> async_graphql::Result<Vec<ScoreEntry>> {
        let repo = repo(ctx);
        let board = repo.get_scoreboard().await?.unwrap_or_default();
        let players = repo.list_players().await?;
        let nick_by_id: HashMap<&str, &str> = players
            .iter()
            .map(|p| (p.id.as_str(), p.nick.as_str()))
            .collect();

        // Restrict to a pool's members if requested.
        let allowed: Option<Vec<String>> = match pool {
            Some(pool_id) => {
                let pools = repo.list_pools().await?;
                let p = pools
                    .into_iter()
                    .find(|p| p.id == pool_id)
                    .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
                Some(p.members)
            }
            None => None,
        };

        let mut entries: Vec<ScoreEntry> = board
            .entries
            .iter()
            .filter(|(pid, _)| allowed.as_ref().is_none_or(|m| m.contains(pid)))
            .map(|(pid, breakdown)| {
                let stages: Vec<StageScore> = breakdown
                    .iter()
                    .map(|(round, points)| StageScore {
                        round: (*round).into(),
                        points: *points,
                    })
                    .collect();
                let total: i64 = breakdown.values().sum();
                ScoreEntry {
                    player_id: pid.clone(),
                    nick: nick_by_id
                        .get(pid.as_str())
                        .copied()
                        .unwrap_or("")
                        .to_owned(),
                    total,
                    stages,
                }
            })
            .collect();
        entries.sort_by(|a, b| b.total.cmp(&a.total).then(a.player_id.cmp(&b.player_id)));
        Ok(entries)
    }

    /// The current player + their predictions. Requires authentication.
    async fn me(&self, ctx: &Context<'_>) -> async_graphql::Result<Player> {
        let player = CurrentPlayer::require(ctx)?;
        Ok(Player::from(player))
    }

    /// The pools the current player belongs to. Requires authentication.
    async fn pools(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Pool>> {
        let player = CurrentPlayer::require(ctx)?;
        let pools = repo(ctx).list_pools().await?;
        Ok(pools
            .iter()
            .filter(|p| p.members.contains(&player.id) || p.owner == player.id)
            .map(Pool::from)
            .collect())
    }

    /// Every player's *visible* predictions for a group's matches (`API.md`
    /// §6, UC-9). A prediction is visible to others only once it is
    /// effective-locked or the match has kicked off.
    async fn tips(&self, ctx: &Context<'_>, group_id: String) -> async_graphql::Result<Vec<Tip>> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let tournament = repo
            .get_tournament()
            .await?
            .ok_or_else(|| async_graphql::Error::new("no tournament loaded"))?;
        let players = repo.list_players().await?;

        let games = tournament.games_in(&group_id);
        let deadline = tournament.deadline(&group_id);
        let now = Utc::now();

        let mut tips = Vec::new();
        for player in &players {
            if player.is_result_user {
                continue;
            }
            for game in &games {
                let prediction = player.match_prediction(&game.id);
                // Own predictions are always visible to the viewer.
                let is_own = player.id == viewer.id;
                let visible = is_own
                    || prediction.is_some_and(|p| {
                        // effective-locked: locked, OR deadline passed, OR
                        // the match itself kicked off.
                        p.locked || now >= game.kickoff || deadline.is_some_and(|d| now > d)
                    });
                tips.push(Tip {
                    player_id: player.id.clone(),
                    nick: player.nick.clone(),
                    game_id: game.id.clone(),
                    prediction: if visible {
                        prediction.map(MatchPrediction::from)
                    } else {
                        None
                    },
                });
            }
        }
        Ok(tips)
    }

    /// Every "perfect" (maximum-scoring) prediction across all players (UC-10).
    async fn perfects(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Perfect>> {
        let repo = repo(ctx);
        let players = repo.list_players().await?;
        let config = ScoringConfig::default();

        let result_user = match players.iter().find(|p| p.is_result_user) {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        let mut perfects = Vec::new();
        for player in &players {
            if player.is_result_user {
                continue;
            }
            for prediction in &player.match_predictions {
                if let Some(result) = result_user.match_prediction(&prediction.game_id) {
                    if result.locked && is_perfect(prediction, result, &config) {
                        perfects.push(Perfect {
                            player_id: player.id.clone(),
                            nick: player.nick.clone(),
                            game_id: prediction.game_id.clone(),
                        });
                    }
                }
            }
        }
        Ok(perfects)
    }

    /// The site-wide banner message, if set.
    async fn motd(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<String>> {
        Ok(repo(ctx).get_motd().await?.map(|m| m.text))
    }

    /// The result user's *locked* match predictions — the official scores.
    /// Public so any client can overlay official results onto the schedule.
    async fn results(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<MatchPrediction>> {
        let players = repo(ctx).list_players().await?;
        Ok(players
            .iter()
            .find(|p| p.is_result_user)
            .map(|r| {
                r.match_predictions
                    .iter()
                    .filter(|p| p.locked)
                    .map(MatchPrediction::from)
                    .collect()
            })
            .unwrap_or_default())
    }
}
