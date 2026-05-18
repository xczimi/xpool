//! The GraphQL mutation root (`API.md` §5).
//!
//! `submitGroup` saves/locks a whole group's predictions onto the player item
//! with optimistic concurrency (retry once on conflict). The `enterResult`
//! admin mutation requires the result user and triggers the wholesale
//! post-result recompute.

use crate::auth::CurrentPlayer;
use crate::gql::inputs::{validate_score, MatchPredictionInput, StandingsInput};
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

/// The request's `now` (the clock seam — `.specs/TESTING.md` §3.2).
fn now(ctx: &Context<'_>) -> chrono::DateTime<chrono::Utc> {
    ctx.data_unchecked::<crate::clock::RequestNow>().0
}

/// A fresh opaque pool join code — 8 uppercase hex characters.
fn generate_join_code() -> String {
    uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>()
        .to_uppercase()
}

/// Load a pool by id, or a GraphQL "not found" error.
async fn load_pool(repo: &Arc<dyn Repository>, id: &str) -> async_graphql::Result<DomainPool> {
    repo.list_pools()
        .await?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| async_graphql::Error::new("pool not found"))
}

/// Map a domain `PoolError` to a GraphQL error.
fn pool_err(e: domain::pool::PoolError) -> async_graphql::Error {
    async_graphql::Error::new(e.to_string())
}

/// Maximum length of a player's `nick` (shown across the app).
const MAX_NICK_LEN: usize = 40;
/// Maximum length of a player's `full_name`.
const MAX_FULL_NAME_LEN: usize = 120;

/// Validate a free-text profile field: non-empty after trim and within
/// `max_len` characters. Returns the trimmed value, or a GraphQL error.
fn validate_profile_field(
    label: &str,
    value: &str,
    max_len: usize,
) -> async_graphql::Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(async_graphql::Error::new(format!("{label} must not be empty")));
    }
    if trimmed.chars().count() > max_len {
        return Err(async_graphql::Error::new(format!(
            "{label} must be at most {max_len} characters"
        )));
    }
    Ok(trimmed.to_owned())
}

/// Apply a batch of predictions for one group onto a player, returning the
/// next player state. Pure helper.
///
/// Returns a GraphQL error when an input score is out of range (`Issue 15` —
/// scores are rejected, not clamped).
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
) -> async_graphql::Result<DomainPlayer> {
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
            home_score: validate_score("home", input.home_score)?,
            away_score: validate_score("away", input.away_score)?,
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

    Ok(DomainPlayer {
        match_predictions,
        standings_predictions,
        ..player.clone()
    })
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

        // Issue 01 — the group's deadline is final: no edits once it passes.
        if let Some(deadline) = tournament.deadline(&group_id) {
            if now(ctx) >= deadline {
                return Err(async_graphql::Error::new(format!(
                    "group `{group_id}` deadline has passed; predictions are final"
                )));
            }
        }

        // Issue 06 (PRED-03) — a lock must cover every game in the group.
        if lock {
            let supplied: std::collections::HashSet<&str> = predictions
                .iter()
                .filter(|p| game_ids.contains(&p.game_id))
                .map(|p| p.game_id.as_str())
                .collect();
            let missing: Vec<&str> = game_ids
                .iter()
                .map(String::as_str)
                .filter(|id| !supplied.contains(id))
                .collect();
            if !missing.is_empty() {
                return Err(async_graphql::Error::new(format!(
                    "cannot lock group `{group_id}`: missing predictions for {missing:?}"
                )));
            }
        }

        // First attempt uses the player from the auth context; a retry
        // re-reads the current player state after a version conflict.
        let mut current = viewer.clone();
        for attempt in 0..2 {
            // Issue 01 — locking is final for the player: a prediction that is
            // already locked cannot be overwritten.
            if let Some(locked) = current
                .match_predictions
                .iter()
                .find(|p| game_ids.contains(&p.game_id) && p.locked)
            {
                return Err(async_graphql::Error::new(format!(
                    "prediction for `{}` is already locked and cannot be changed",
                    locked.game_id
                )));
            }
            let next = apply_group_predictions(
                &current,
                &group_id,
                &game_ids,
                &predictions,
                standings.as_ref(),
                lock,
            )?;
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

    /// Create a pool owned by the current player (POOL-01). The result user
    /// cannot own a pool (POOL-12).
    async fn create_pool(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        if viewer.is_result_user {
            return Err(async_graphql::Error::new(
                "the result user cannot own a pool",
            ));
        }
        // Issue 16 — reject a client-supplied id that is already taken so a
        // caller cannot clobber another player's pool.
        if repo(ctx).list_pools().await?.iter().any(|p| p.id == id) {
            return Err(async_graphql::Error::new(format!(
                "a pool with id `{id}` already exists"
            )));
        }
        let pool = DomainPool {
            id,
            name,
            owner: viewer.id.clone(),
            members: vec![viewer.id.clone()],
            join_code: generate_join_code(),
        };
        repo(ctx).put_pool(&pool).await?;
        Ok(Pool::from(&pool))
    }

    /// Rename a pool (POOL-08). Owner-only.
    async fn update_pool(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &id).await?;
        let updated = domain::pool::rename(&pool, &viewer.id, name).map_err(pool_err)?;
        repo.put_pool(&updated).await?;
        Ok(Pool::from(&updated))
    }

    /// Join a pool by its join code (POOL-02).
    async fn join_pool(
        &self,
        ctx: &Context<'_>,
        join_code: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = repo
            .list_pools()
            .await?
            .into_iter()
            .find(|p| p.join_code == join_code)
            .ok_or_else(|| async_graphql::Error::new("no pool with that join code"))?;
        let updated = domain::pool::join(&pool, viewer).map_err(pool_err)?;
        repo.put_pool(&updated).await?;
        Ok(Pool::from(&updated))
    }

    /// Leave a pool (POOL-05). The owner cannot leave (POOL-10).
    async fn leave_pool(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &id).await?;
        let updated = domain::pool::leave(&pool, &viewer.id).map_err(pool_err)?;
        repo.put_pool(&updated).await?;
        Ok(Pool::from(&updated))
    }

    /// Remove a member from a pool (POOL-04). Owner-only.
    async fn remove_member(
        &self,
        ctx: &Context<'_>,
        pool_id: String,
        member_id: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &pool_id).await?;
        let updated =
            domain::pool::remove_member(&pool, &viewer.id, &member_id).map_err(pool_err)?;
        repo.put_pool(&updated).await?;
        Ok(Pool::from(&updated))
    }

    /// Rotate a pool's join code (POOL-03). Owner-only.
    async fn rotate_join_code(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &id).await?;
        let updated = domain::pool::set_join_code(&pool, &viewer.id, generate_join_code())
            .map_err(pool_err)?;
        repo.put_pool(&updated).await?;
        Ok(Pool::from(&updated))
    }

    /// Delete a pool (POOL-09). Owner-only.
    async fn delete_pool(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<bool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &id).await?;
        if pool.owner != viewer.id {
            return Err(async_graphql::Error::new("only the pool owner may delete it"));
        }
        repo.delete_pool(&id).await?;
        Ok(true)
    }

    /// Update the current player's profile (nick / full name). Issue 17 —
    /// `nick` / `full_name` are validated: non-empty after trim and within a
    /// sensible length; the trimmed value is what gets stored.
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
            player.nick = validate_profile_field("nick", &nick, MAX_NICK_LEN)?;
        }
        if let Some(full_name) = full_name {
            player.full_name = validate_profile_field("full name", &full_name, MAX_FULL_NAME_LEN)?;
        }
        // `version` is left as read — see `apply_group_predictions`.
        repo.put_player(&player).await?;
        Ok(Player::from(&player))
    }

    /// Record a referral invitation: the invitee's `referrer` is set to the
    /// current player. The invitee must already exist (dev stub).
    ///
    /// Issue 05 — a player cannot invite themselves, and an invitee can only
    /// be referred once (a set `referrer` is never overwritten).
    async fn invite(&self, ctx: &Context<'_>, invitee_id: String) -> async_graphql::Result<bool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        if invitee_id == viewer.id {
            return Err(async_graphql::Error::new("you cannot invite yourself"));
        }
        let invitee = repo
            .get_player(&invitee_id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("invitee not found"))?;
        if invitee.referrer.is_some() {
            return Err(async_graphql::Error::new(
                "this player has already been referred",
            ));
        }
        let updated = DomainPlayer {
            referrer: Some(viewer.id.clone()),
            ..invitee
        };
        repo.put_player(&updated).await?;
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
            home_score: validate_score("home", home_score)?,
            away_score: validate_score("away", away_score)?,
            locked: lock,
        };
        result_user
            .match_predictions
            .retain(|p| p.game_id != game_id);
        result_user.match_predictions.push(new_prediction);
        repo.put_player(&result_user).await?;

        // Wholesale recompute: scoreboard + bracket resolution.
        recompute(repo.as_ref(), now(ctx))
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(true)
    }
}
