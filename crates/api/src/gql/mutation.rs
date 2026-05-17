//! The GraphQL mutation root (`API.md` §5).
//!
//! `submitGroup` saves/locks a whole group's predictions onto the player item
//! with optimistic concurrency (retry once on conflict). Admin mutations
//! (`enterResult`, `setMotd`) require the result user. `enterResult` triggers
//! the wholesale post-result recompute.

use crate::auth::CurrentPlayer;
use crate::gql::inputs::{MatchPredictionInput, StandingsInput};
use crate::gql::types::*;
use crate::recompute::recompute;
use async_graphql::{Context, Object};
use domain::{
    MatchPrediction, Player as DomainPlayer, Pool as DomainPool,
    StandingsPrediction as DomainStandingsPrediction,
};
use std::sync::Arc;
use storage::Repository;

fn repo<'a>(ctx: &'a Context<'_>) -> &'a Arc<dyn Repository> {
    ctx.data_unchecked::<Arc<dyn Repository>>()
}

/// Apply a batch of predictions for one group onto a player, returning the
/// next player state. Pure helper.
///
/// `version` is left **equal to the supplied player's** — the storage layer's
/// optimistic-concurrency guard (`storage::Repository::put_player`) succeeds
/// only when the supplied `version` still matches what is stored. A write that
/// read a stale version therefore fails and the caller retries against fresh
/// state.
fn apply_group_predictions(
    player: &DomainPlayer,
    group_id: &str,
    game_ids: &[String],
    predictions: &[MatchPredictionInput],
    standings: Option<&StandingsInput>,
    lock: bool,
) -> DomainPlayer {
    // Drop existing predictions for the group's games, then re-add.
    let mut match_predictions: Vec<MatchPrediction> = player
        .match_predictions
        .iter()
        .filter(|p| !game_ids.contains(&p.game_id))
        .cloned()
        .collect();

    for input in predictions {
        // Ignore inputs that do not belong to the group being submitted.
        if !game_ids.contains(&input.game_id) {
            continue;
        }
        match_predictions.push(MatchPrediction {
            game_id: input.game_id.clone(),
            home_score: input.home_score.clamp(0, u8::MAX as i32) as u8,
            away_score: input.away_score.clamp(0, u8::MAX as i32) as u8,
            locked: lock,
        });
    }

    // Drop the existing standings prediction for this group, then re-add it
    // when a new one was supplied.
    let mut standings_predictions: Vec<DomainStandingsPrediction> = player
        .standings_predictions
        .iter()
        .filter(|s| s.group_id != group_id)
        .cloned()
        .collect();
    if let Some(input) = standings {
        standings_predictions.push(DomainStandingsPrediction {
            group_id: group_id.to_owned(),
            ordering: input.ordering.clone(),
            draw_order: input.draw_order.clone(),
            locked: lock,
        });
    }

    DomainPlayer {
        match_predictions,
        standings_predictions,
        ..player.clone()
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Save or lock a whole group's predictions in one call (`API.md` §5–6).
    /// Optimistic concurrency on `Player::version`; retries once on conflict.
    async fn submit_group(
        &self,
        ctx: &Context<'_>,
        group_id: String,
        predictions: Vec<MatchPredictionInput>,
        standings: Option<StandingsInput>,
        lock: bool,
    ) -> async_graphql::Result<Player> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);

        let tournament = repo
            .get_tournament()
            .await?
            .ok_or_else(|| async_graphql::Error::new("no tournament loaded"))?;
        let game_ids: Vec<String> = tournament
            .games_in(&group_id)
            .iter()
            .map(|g| g.id.clone())
            .collect();
        if game_ids.is_empty() {
            return Err(async_graphql::Error::new(format!(
                "group `{group_id}` has no games"
            )));
        }

        // First attempt uses the player from the auth context; a retry
        // re-reads the current player state after a version conflict.
        let mut current = viewer.clone();
        for attempt in 0..2 {
            let next = apply_group_predictions(
                &current,
                &group_id,
                &game_ids,
                &predictions,
                standings.as_ref(),
                lock,
            );
            match repo.put_player(&next).await {
                Ok(()) => return Ok(Player::from(&next)),
                Err(e) if attempt == 0 => {
                    tracing::warn!("submit_group conflict, retrying: {e}");
                    current = repo
                        .get_player(&viewer.id)
                        .await?
                        .ok_or_else(|| async_graphql::Error::new("player vanished"))?;
                }
                Err(e) => return Err(async_graphql::Error::new(e.to_string())),
            }
        }
        unreachable!("loop returns on both attempts")
    }

    /// Create a pool owned by the current player.
    async fn create_pool(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let pool = DomainPool {
            id,
            name,
            owner: viewer.id.clone(),
            members: vec![viewer.id.clone()],
        };
        repo(ctx).put_pool(&pool).await?;
        Ok(Pool::from(&pool))
    }

    /// Update a pool's name and/or members. Only the owner may update.
    async fn update_pool(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: Option<String>,
        members: Option<Vec<String>>,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let mut pool = repo
            .list_pools()
            .await?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
        if pool.owner != viewer.id {
            return Err(async_graphql::Error::new(
                "only the pool owner may update it",
            ));
        }
        if let Some(name) = name {
            pool.name = name;
        }
        if let Some(members) = members {
            pool.members = members;
        }
        repo.put_pool(&pool).await?;
        Ok(Pool::from(&pool))
    }

    /// Update the current player's profile (nick / full name).
    async fn update_profile(
        &self,
        ctx: &Context<'_>,
        nick: Option<String>,
        full_name: Option<String>,
    ) -> async_graphql::Result<Player> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        // Re-read for the freshest version before writing.
        let mut player = repo
            .get_player(&viewer.id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("player not found"))?;
        if let Some(nick) = nick {
            player.nick = nick;
        }
        if let Some(full_name) = full_name {
            player.full_name = full_name;
        }
        // `version` is left as read — see `apply_group_predictions`.
        repo.put_player(&player).await?;
        Ok(Player::from(&player))
    }

    /// Record a referral invitation: the invitee's `referrer` is set to the
    /// current player. The invitee must already exist (dev stub).
    async fn invite(&self, ctx: &Context<'_>, invitee_id: String) -> async_graphql::Result<bool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let mut invitee = repo
            .get_player(&invitee_id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("invitee not found"))?;
        invitee.referrer = Some(viewer.id.clone());
        repo.put_player(&invitee).await?;
        Ok(true)
    }

    /// Admin: enter (or correct) a result, then run the post-result recompute.
    /// `advancer` is the team id that progresses on a knockout draw.
    async fn enter_result(
        &self,
        ctx: &Context<'_>,
        game_id: String,
        home_score: i32,
        away_score: i32,
        #[allow(unused_variables)] advancer: Option<String>,
        lock: bool,
    ) -> async_graphql::Result<bool> {
        let admin = CurrentPlayer::require_admin(ctx)?;
        let repo = repo(ctx);

        // Re-read the result user for the freshest version.
        let mut result_user = repo
            .get_player(&admin.id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("result user not found"))?;

        let new_prediction = MatchPrediction {
            game_id: game_id.clone(),
            home_score: home_score.clamp(0, u8::MAX as i32) as u8,
            away_score: away_score.clamp(0, u8::MAX as i32) as u8,
            locked: lock,
        };
        result_user
            .match_predictions
            .retain(|p| p.game_id != game_id);
        result_user.match_predictions.push(new_prediction);
        repo.put_player(&result_user).await?;

        // Wholesale recompute: scoreboard + bracket resolution.
        recompute(repo.as_ref())
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(true)
    }

    /// Admin: set the site-wide banner message.
    async fn set_motd(&self, ctx: &Context<'_>, text: String) -> async_graphql::Result<bool> {
        CurrentPlayer::require_admin(ctx)?;
        repo(ctx).put_motd(&domain::Motd { text }).await?;
        Ok(true)
    }
}
