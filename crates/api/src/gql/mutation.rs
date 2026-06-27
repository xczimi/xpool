//! The GraphQL mutation root (`API.md` §5).
//!
//! `submitGroup` saves/locks a whole group's predictions onto the player item
//! with optimistic concurrency (retry once on conflict). When the result user
//! submits, their predictions are the official results, so the save triggers
//! the wholesale post-result recompute. `recompute` re-runs it on demand.

use crate::auth::CurrentPlayer;
use crate::gql::inputs::{validate_score, MatchPredictionInput, StandingsInput};
use crate::gql::types::*;
use crate::recompute::recompute;
use async_graphql::{Context, Object, SimpleObject};
use domain::invite::{normalize_suffix, parse_code, slugify, CodeInput};
use domain::{
    Invite, MatchPrediction, Player as DomainPlayer, Pool as DomainPool, Round,
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

/// Whether the dev stub is enabled, given the resolved `LOCAL_AUTH_ISSUER`
/// value. Pure — testable without touching the process environment.
fn dev_stub_enabled_from(local_auth_issuer: Option<&str>) -> bool {
    local_auth_issuer.is_some_and(|v| !v.is_empty())
}

/// Whether the dev stub is enabled (same gate as the dev-login route / clock
/// override — the `LOCAL_AUTH_ISSUER` env var, absent in production).
fn dev_stub_enabled() -> bool {
    dev_stub_enabled_from(std::env::var("LOCAL_AUTH_ISSUER").ok().as_deref())
}

/// A fresh high-entropy invite suffix — 10 Crockford-base32 chars (~50 bits).
/// Entropy comes from a v4 UUID's random bytes (no extra `rand` dependency).
fn generate_invite_code() -> String {
    let bytes = *uuid::Uuid::new_v4().as_bytes();
    domain::invite::encode_suffix(&bytes[..domain::invite::SUFFIX_LEN])
}

/// A unique-per-pool cosmetic prefix label from the pool name: a 5-char slug
/// plus a 2-char disambiguator, regenerated until it collides with no existing
/// pool prefix (case-insensitive). Falls back to a pure code if the name has no
/// alphanumerics.
fn unique_prefix(existing: &[DomainPool], name: &str) -> String {
    let base = {
        let s = slugify(name, 5);
        if s.is_empty() {
            "POOL".to_owned()
        } else {
            s
        }
    };
    loop {
        let bytes = *uuid::Uuid::new_v4().as_bytes();
        let disambiguator = domain::invite::encode_suffix(&bytes[..2]);
        let candidate = format!("{base}{disambiguator}");
        if !existing
            .iter()
            .any(|p| p.prefix.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
}

/// Mint and persist a fresh reusable invite row for `invited_by` into `pool_id`.
async fn mint_invite(
    repo: &dyn Repository,
    pool_id: &str,
    invited_by: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> async_graphql::Result<Invite> {
    let invite = Invite {
        code: generate_invite_code(),
        pool_id: pool_id.to_owned(),
        invited_by: invited_by.to_owned(),
        created_at: now,
        expires_at: None,
        revoked: false,
    };
    repo.put_invite(&invite).await?;
    Ok(invite)
}

/// The full shareable URL for a nested `PREFIX-SUFFIX` invite code.
fn invite_link(prefix: &str, code: &str) -> String {
    let origin =
        std::env::var("XPOOL_PUBLIC_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_owned());
    format!("{origin}/invite/{prefix}-{code}")
}

/// Resolve a typed/pasted invite code to its stored row (lenient: full
/// `PREFIX-SUFFIX`, bare suffix, or bare prefix → the pool's owner invite). The
/// suffix is authoritative; a mismatched prefix is advisory (warn, resolve by
/// suffix). Rejects revoked or expired invites against `now`.
async fn resolve_invite(
    repo: &dyn Repository,
    raw: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> async_graphql::Result<Invite> {
    let parsed = parse_code(raw).ok_or_else(|| async_graphql::Error::new("empty invite code"))?;
    let invite = match parsed {
        CodeInput::PrefixAndSuffix { prefix, suffix } => {
            let code = normalize_suffix(&suffix);
            let inv = repo
                .get_invite(&code)
                .await?
                .ok_or_else(|| async_graphql::Error::new("no invite matches that code"))?;
            // Advisory: warn if the typed prefix disagrees with the pool's, but
            // resolve by the (authoritative) suffix regardless.
            if let Some(pool) = repo
                .list_pools()
                .await?
                .into_iter()
                .find(|p| p.id == inv.pool_id)
            {
                if !prefix.eq_ignore_ascii_case(&pool.prefix) {
                    tracing::warn!(
                        "invite prefix `{prefix}` != pool prefix `{}`; resolving by suffix",
                        pool.prefix
                    );
                }
            }
            inv
        }
        CodeInput::Bare(token) => {
            // A bare token is either a suffix (the key) or a pool prefix.
            let code = normalize_suffix(&token);
            if let Some(inv) = repo.get_invite(&code).await? {
                inv
            } else {
                let pool = repo
                    .list_pools()
                    .await?
                    .into_iter()
                    .find(|p| p.prefix.eq_ignore_ascii_case(&token))
                    .ok_or_else(|| {
                        async_graphql::Error::new("no invite or pool matches that code")
                    })?;
                repo.list_invites_by_pool(&pool.id)
                    .await?
                    .into_iter()
                    .find(|i| i.invited_by == pool.owner && !i.revoked)
                    .ok_or_else(|| {
                        async_graphql::Error::new("that pool has no active owner invite")
                    })?
            }
        }
    };
    if invite.revoked {
        return Err(async_graphql::Error::new("this invite has been revoked"));
    }
    if let Some(expires_at) = invite.expires_at {
        if expires_at < now {
            return Err(async_graphql::Error::new("this invite has expired"));
        }
    }
    Ok(invite)
}

/// Set `player_id`'s `referrer` to `invited_by` when it is currently unset and
/// the two differ (no self-referral). Re-reads for a fresh version. Idempotent.
async fn set_referrer_if_unset(
    repo: &dyn Repository,
    player_id: &str,
    invited_by: &str,
) -> async_graphql::Result<()> {
    if player_id == invited_by {
        return Ok(());
    }
    let player = repo
        .get_player(player_id)
        .await?
        .ok_or_else(|| async_graphql::Error::new("player not found"))?;
    if player.referrer.is_none() {
        let updated = DomainPlayer {
            referrer: Some(invited_by.to_owned()),
            ..player
        };
        repo.put_player(&updated).await?;
    }
    Ok(())
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
        return Err(async_graphql::Error::new(format!(
            "{label} must not be empty"
        )));
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

/// Returned by `createInvite` — the opaque code and the full shareable URL.
#[derive(SimpleObject)]
pub struct InviteLink {
    /// The raw opaque token (for programmatic use / testing).
    pub code: String,
    /// The full `<origin>/invite/<code>` URL the inviter shares.
    pub link: String,
}

/// Returned by `claimInvite` — the resolved or newly-created player.
#[derive(SimpleObject)]
pub struct ClaimResult {
    pub player: PlayerSummary,
}

/// Derive the `(provider, provider_id)` key for storing an Identity row,
/// based on the unclaimed session's verified contact — mirrors the
/// `resolution::identity_key_for` logic but operates on `VerifiedIdentity`.
fn identity_key_for_unclaimed(u: &crate::auth::VerifiedIdentity) -> Option<(String, String)> {
    let claims = crate::auth::jwt::VerifiedClaims {
        sub: u.sub.clone(),
        verified_email: u.verified_email.clone(),
        verified_phone: u.verified_phone.clone(),
        connection: u.connection.clone(),
    };
    crate::auth::resolution::identity_key_for(&claims)
}

/// Add `player_id` to a pool by id. No-op if already a member.
async fn add_to_pool(
    repo: &dyn Repository,
    player_id: &str,
    pool_id: &str,
) -> async_graphql::Result<()> {
    let pools = repo.list_pools().await?;
    let mut pool = pools
        .into_iter()
        .find(|p| p.id == pool_id)
        .ok_or_else(|| async_graphql::Error::new(format!("pool not found: {pool_id}")))?;
    if !pool.members.iter().any(|m| m == player_id) {
        pool.members.push(player_id.to_owned());
        repo.put_pool(&pool).await?;
    }
    Ok(())
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
        // Issue 27 — the boundary is strict `>`. The result user is exempt:
        // official results are entered *after* the match (unified result entry).
        if !viewer.is_result_user {
            if let Some(deadline) = tournament.deadline(&group_id) {
                if now(ctx) > deadline {
                    return Err(async_graphql::Error::new(format!(
                        "group `{group_id}` deadline has passed; predictions are final"
                    )));
                }
            }
        }

        // Best-thirds fix (Part C) — a knockout-round match accepts a prediction
        // only once BOTH its team slots are concretely placed. Best-third slots
        // stay `None` until all 12 groups are final (see `resolve_bracket`), so
        // this blocks blind predictions against not-yet-known opponents. Group
        // stage games always carry concrete team ids, so they are unaffected.
        let is_knockout = tournament
            .groups
            .get(&group_id)
            .is_some_and(|g| g.round != Round::GroupStage);
        if is_knockout {
            if let Some(unresolved) = tournament
                .games_in(&group_id)
                .iter()
                .find(|g| g.home.team_id.is_none() || g.away.team_id.is_none())
            {
                return Err(async_graphql::Error::new(format!(
                    "match `{}` teams are not yet determined; predictions open once both teams are placed",
                    unresolved.id
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
                Ok(()) => {
                    // The result user's predictions ARE the official results, so
                    // a save recomputes the materialised scoreboard + bracket on
                    // write. Best-effort: a failure is logged, not fatal (the
                    // `recompute` mutation self-heals).
                    if viewer.is_result_user {
                        if let Err(e) = recompute(repo.as_ref(), now(ctx)).await {
                            tracing::error!("recompute after result-user submit_group failed: {e}");
                        }
                    }
                    return Ok(Player::from(&next));
                }
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

    /// Create a pool owned by the current player (POOL-01). Restricted
    /// creation: the result-user (the referral-graph root) and the admins it
    /// referred directly may create pools (`may_create_pool`). The result user
    /// owns as a transient bootstrapper but is never a member (POOL-12); it hands
    /// the pool over once a real player has joined. The owner's invite row (the
    /// pool link) is minted on creation.
    async fn create_pool(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let ruid = crate::gql::result_user_id(repo.as_ref()).await?;
        if !domain::pool::may_create_pool(viewer, &ruid) {
            return Err(async_graphql::Error::new(
                "you are not allowed to create pools",
            ));
        }
        // Issue 16 — reject a client-supplied id that is already taken so a
        // caller cannot clobber another player's pool.
        let pools = repo.list_pools().await?;
        if pools.iter().any(|p| p.id == id) {
            return Err(async_graphql::Error::new(format!(
                "a pool with id `{id}` already exists"
            )));
        }
        let pool = DomainPool {
            id,
            name: name.clone(),
            owner: viewer.id.clone(),
            // The result user owns the pool it bootstraps but is never a member
            // (POOL-12) — it must stay out of standings/scoring. A normal admin
            // owner joins their own pool as usual.
            members: if viewer.is_result_user {
                vec![]
            } else {
                vec![viewer.id.clone()]
            },
            prefix: unique_prefix(&pools, &name),
        };
        repo.put_pool(&pool).await?;
        // Mint the owner's invite — the pool link (bare prefix resolves to it).
        mint_invite(repo.as_ref(), &pool.id, &viewer.id, now(ctx)).await?;
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

    /// Join a pool by accepting an invite code (POOL-02). For an
    /// already-identified player: resolve the code (lenient — full link, bare
    /// suffix, or bare prefix), add to the pool, and record `invited_by` as the
    /// player's `referrer` if it is not already set.
    async fn join(&self, ctx: &Context<'_>, code: String) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let invite = resolve_invite(repo.as_ref(), &code, now(ctx)).await?;
        let pool = load_pool(repo, &invite.pool_id).await?;
        let updated = domain::pool::join(&pool, viewer).map_err(pool_err)?;
        repo.put_pool(&updated).await?;
        set_referrer_if_unset(repo.as_ref(), &viewer.id, &invite.invited_by).await?;
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

    /// Hand a pool over to one of its members (ownership transfer). Owner-only;
    /// the new owner must already be a member. This is the bootstrap exit for the
    /// result user, which owns a pool transiently then detaches by handing it to
    /// a real member (POOL-12).
    async fn transfer_ownership(
        &self,
        ctx: &Context<'_>,
        pool_id: String,
        new_owner: String,
    ) -> async_graphql::Result<Pool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &pool_id).await?;
        let updated =
            domain::pool::transfer_ownership(&pool, &viewer.id, &new_owner).map_err(pool_err)?;
        repo.put_pool(&updated).await?;
        Ok(Pool::from(&updated))
    }

    /// Delete a pool (POOL-09). Owner-only.
    async fn delete_pool(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<bool> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &id).await?;
        if pool.owner != viewer.id {
            return Err(async_graphql::Error::new(
                "only the pool owner may delete it",
            ));
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

    /// Admin: re-run the wholesale post-result recompute on demand (Issue 18).
    /// Idempotent — fully repairs a scoreboard/bracket left stale by an earlier
    /// failed recompute.
    async fn recompute(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        CurrentPlayer::require_admin(ctx)?;
        let repo = repo(ctx);
        recompute(repo.as_ref(), now(ctx)).await.map_err(|e| {
            tracing::error!("recompute mutation failed: {e}");
            async_graphql::Error::new("recompute failed; please retry")
        })?;
        Ok(true)
    }

    /// Dev-only: re-materialise the scoreboard + bracket as-of the request
    /// clock (`X-Dev-Now`). Unlike the admin `recompute`, this is gated on the
    /// dev stub rather than admin, so the dev-clock picker can call it as any
    /// logged-in dev player. Returns an error when the stub is disabled.
    async fn dev_rematerialize(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        if !dev_stub_enabled() {
            return Err(async_graphql::Error::new("dev rematerialize is disabled"));
        }
        let repo = repo(ctx);
        recompute(repo.as_ref(), now(ctx)).await.map_err(|e| {
            tracing::error!("dev_rematerialize failed: {e}");
            async_graphql::Error::new("rematerialize failed; please retry")
        })?;
        Ok(true)
    }

    /// Accept an invite when the viewer is **not yet a Player** — the front
    /// door to identity. Resolves the code via the stored invite table, then
    /// establishes the player (the dev stand-in for Auth0 signup: lazy
    /// Person+Player+Identity creation, or AUTH-12 reuse if the verified email
    /// already maps to an existing Person), records `invited_by` as the
    /// player's `referrer`, and joins the invite's pool.
    ///
    /// An already-resolved `Player` is handled too (AUTH-12 shortcut: just join
    /// the pool, set referrer if unset, return) — but the simpler [`join`]
    /// mutation is preferred for that case.
    async fn claim_invite(
        &self,
        ctx: &Context<'_>,
        code: String,
        nick: String,
        full_name: String,
    ) -> async_graphql::Result<ClaimResult> {
        let viewer = ctx.data_unchecked::<crate::auth::CurrentPlayer>();
        let repo = repo(ctx);
        let invite = resolve_invite(repo.as_ref(), &code, now(ctx)).await?;
        let pool_id = invite.pool_id.clone();
        let invited_by = invite.invited_by.clone();

        // Already a resolved Player → AUTH-12: join the pool, set referrer, return.
        if let crate::auth::CurrentPlayer::Player(p) = viewer {
            add_to_pool(repo.as_ref(), &p.id, &pool_id).await?;
            set_referrer_if_unset(repo.as_ref(), &p.id, &invited_by).await?;
            return Ok(ClaimResult {
                player: PlayerSummary::from(p.as_ref()),
            });
        }

        let unclaimed = match viewer {
            crate::auth::CurrentPlayer::AuthenticatedUnclaimed(u) => u.clone(),
            crate::auth::CurrentPlayer::Visitor => {
                return Err(async_graphql::Error::new("authentication required"));
            }
            crate::auth::CurrentPlayer::Player(_) => unreachable!(),
        };

        // AUTH-12 by verified email: if the email is already in the system,
        // resolve to that existing player without creating a duplicate.
        if let Some(email) = &unclaimed.verified_email {
            let hits = repo.find_identities_by_verified_email(email).await?;
            if let Some(identity) = hits.into_iter().next() {
                // Look the player up by person id (matches `Player.person_id`,
                // distinct from `Player.id`) — `get_player` would miss and we'd
                // wrongly create a duplicate player. Mirrors `auth::resolution`.
                if let Some(player) = repo.get_player_by_person(&identity.person_id).await? {
                    add_to_pool(repo.as_ref(), &player.id, &pool_id).await?;
                    set_referrer_if_unset(repo.as_ref(), &player.id, &invited_by).await?;
                    return Ok(ClaimResult {
                        player: PlayerSummary::from(&player),
                    });
                }
            }
        }

        // Lazy creation: build Person + Player + Identity from the unclaimed session.
        let (provider, provider_id) = identity_key_for_unclaimed(&unclaimed).ok_or_else(|| {
            async_graphql::Error::new("no usable verified contact on the auth session")
        })?;
        let nick = validate_profile_field("nick", &nick, MAX_NICK_LEN)?;
        let full_name = validate_profile_field("full name", &full_name, MAX_FULL_NAME_LEN)?;
        let person_id = uuid::Uuid::new_v4().to_string();
        let identity = domain::Identity {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            provider_id,
            person_id: person_id.clone(),
            verified_email: unclaimed.verified_email.clone(),
        };
        let person = domain::Person {
            id: person_id.clone(),
            identity_ids: vec![identity.id.clone()],
        };
        let player = DomainPlayer {
            id: person_id.clone(),
            person_id: person_id.clone(),
            nick: nick.clone(),
            full_name,
            referrer: Some(invited_by.clone()),
            is_result_user: false,
            version: 0,
            match_predictions: Vec::new(),
            standings_predictions: Vec::new(),
        };
        repo.put_identity(&identity).await?;
        repo.put_person(&person).await?;
        repo.put_player(&player).await?;
        add_to_pool(repo.as_ref(), &player.id, &pool_id).await?;
        Ok(ClaimResult {
            player: PlayerSummary::from(&player),
        })
    }

    /// Attach a new Identity to an existing Person by confirming that the
    /// current session's verified email matches one already in the system via
    /// a different provider (AUTH-13 cross-provider linking).
    ///
    /// The caller must be `AuthenticatedUnclaimed` (i.e. `me` returns an
    /// `UnclaimedViewer` with a `linkCandidate`). The `personId` must be the
    /// one surfaced in `linkCandidate` — a defense-in-depth check verifies the
    /// verified email still belongs to that Person before writing.
    async fn confirm_link(
        &self,
        ctx: &Context<'_>,
        person_id: String,
    ) -> async_graphql::Result<ClaimResult> {
        let viewer = ctx.data_unchecked::<crate::auth::CurrentPlayer>();
        let unclaimed = match viewer {
            crate::auth::CurrentPlayer::AuthenticatedUnclaimed(u) => u.clone(),
            _ => return Err(async_graphql::Error::new("not in a link-prompt state")),
        };
        let repo = repo(ctx);

        // Defense in depth: verify the verified email actually matches a Person
        // via some existing Identity row.
        let email = unclaimed
            .verified_email
            .as_deref()
            .ok_or_else(|| async_graphql::Error::new("no verified email on the auth session"))?;
        let hits = repo.find_identities_by_verified_email(email).await?;
        if !hits.iter().any(|i| i.person_id == person_id) {
            return Err(async_graphql::Error::new(
                "verified email does not belong to that Person",
            ));
        }

        let (provider, provider_id) = identity_key_for_unclaimed(&unclaimed)
            .ok_or_else(|| async_graphql::Error::new("no usable contact"))?;
        let identity = domain::Identity {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            provider_id,
            person_id: person_id.clone(),
            verified_email: unclaimed.verified_email.clone(),
        };
        repo.put_identity(&identity).await?;

        let mut person = repo
            .get_person(&person_id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("person not found"))?;
        person.identity_ids.push(identity.id.clone());
        repo.put_person(&person).await?;

        let player = repo
            .get_player(&person_id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("player not found"))?;
        Ok(ClaimResult {
            player: PlayerSummary::from(&player),
        })
    }

    /// Share your invite into a pool you belong to. Every invite is pool-bound;
    /// the code is reusable (one per member per pool), so a second call returns
    /// your existing active invite rather than minting a duplicate. The returned
    /// `code` is the nested `PREFIX-SUFFIX` form and `link` is the full URL.
    async fn create_invite(
        &self,
        ctx: &Context<'_>,
        pool: String,
    ) -> async_graphql::Result<InviteLink> {
        let me = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let pool = load_pool(repo, &pool).await?;
        if pool.owner != me.id && !pool.members.iter().any(|m| m == &me.id) {
            return Err(async_graphql::Error::new(
                "join the pool before sharing an invite to it",
            ));
        }
        // Reusable per-member: reuse an existing active invite if present.
        let existing = repo
            .list_invites_by_invited_by(&me.id)
            .await?
            .into_iter()
            .find(|i| i.pool_id == pool.id && !i.revoked);
        let invite = match existing {
            Some(i) => i,
            None => mint_invite(repo.as_ref(), &pool.id, &me.id, now(ctx)).await?,
        };
        Ok(InviteLink {
            code: format!("{}-{}", pool.prefix, invite.code),
            link: invite_link(&pool.prefix, &invite.code),
        })
    }

    /// Revoke one of your invites (the reusable-code off-switch — POOL-03's
    /// rotation is revoke + re-mint via [`create_invite`]). Resolves the code
    /// and revokes it only if it is yours.
    async fn revoke_invite(&self, ctx: &Context<'_>, code: String) -> async_graphql::Result<bool> {
        let me = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let invite = resolve_invite(repo.as_ref(), &code, now(ctx)).await?;
        if invite.invited_by != me.id {
            return Err(async_graphql::Error::new(
                "that invite is not yours to revoke",
            ));
        }
        repo.revoke_invite(&invite.code).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod dev_rematerialize_tests {
    use super::*;

    #[test]
    fn dev_gate_keys_off_local_auth_issuer() {
        // Pure gate — no process-env mutation, safe under parallel tests.
        assert!(!dev_stub_enabled_from(None));
        assert!(!dev_stub_enabled_from(Some("")));
        assert!(dev_stub_enabled_from(Some("local-dev")));
    }
}

#[cfg(test)]
mod submit_group_tests {
    use crate::auth::CurrentPlayer;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, Player, Round, SingleGame, Team, TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct NoSource;
    #[async_trait]
    impl crate::reported::ReportedResultSource for NoSource {
        async fn lookup_events(&self, _ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(vec![])
        }
    }

    fn team(id: &str) -> Team {
        Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        }
    }

    fn normal_player(id: &str) -> Player {
        Player {
            id: id.into(),
            person_id: format!("p-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: Vec::new(),
            standings_predictions: Vec::new(),
        }
    }

    /// Execute a GraphQL request and return the raw response so tests can
    /// inspect `.errors` as well as `.data`.
    async fn exec_raw(
        repo: InMemoryRepository,
        viewer: Player,
        now: chrono::DateTime<Utc>,
        query: &str,
    ) -> async_graphql::Response {
        let repo: Arc<dyn Repository> = Arc::new(repo);
        let source: Arc<dyn crate::reported::ReportedResultSource> = Arc::new(NoSource);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(CurrentPlayer::Player(Box::new(viewer)))
            .data(crate::clock::RequestNow(now));
        schema.execute(req).await
    }

    #[tokio::test]
    async fn knockout_submit_blocked_when_slot_unplaced() {
        // R32 one-match group "r32-m74": game "M74" has one unresolved home slot
        // (team_id: None) simulating a best-third placeholder not yet determined.
        let game = SingleGame {
            id: "M74".into(),
            kickoff: Utc.with_ymd_and_hms(2026, 7, 1, 18, 0, 0).unwrap(),
            venue: None,
            group_id: "r32-m74".into(),
            home: TeamSlot {
                team_id: None, // unresolved — e.g. best third of A/B/C/D/E/F
                description: "3ABCDEF".into(),
            },
            away: TeamSlot {
                team_id: Some("NED".into()),
                description: "2C".into(),
            },
            external_id: None,
        };
        let group = GroupGame {
            id: "r32-m74".into(),
            name: "Round of 32 — match 74".into(),
            parent: None,
            round: Round::R32,
            lock_mode: LockMode::LockPerMatch,
            carries_standings: false,
            children: GroupChildren::Games(vec!["M74".into()]),
        };
        let t = Tournament {
            root: "r32-m74".into(),
            groups: HashMap::from([("r32-m74".to_string(), group)]),
            games: HashMap::from([("M74".to_string(), game)]),
            teams: HashMap::from([("NED".to_string(), team("NED"))]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        let alice = normal_player("alice");
        repo.put_player(&alice).await.unwrap();

        // Clock well before kickoff so the deadline gate doesn't fire first.
        let now = Utc.with_ymd_and_hms(2026, 6, 27, 12, 0, 0).unwrap();
        let resp = exec_raw(
            repo,
            alice,
            now,
            r#"mutation { submitGroup(groupId: "r32-m74", predictions: [{ gameId: "M74", homeScore: 1, awayScore: 0 }], lock: false) { id } }"#,
        )
        .await;

        assert!(
            !resp.errors.is_empty(),
            "expected error: knockout submit must be blocked when a slot is unplaced"
        );
        let msg = resp.errors[0].message.as_str();
        assert!(
            msg.contains("not yet determined"),
            "expected 'not yet determined' in error message, got: {msg:?}"
        );
    }
}
