//! `POST /api/dev/login` — mints a local-issuer JWT.
//!
//! Two request shapes:
//!  - `{ "player": "<id>" }` — mint for a seeded `Player` (the existing
//!    AuthBar dev-picker flow).
//!  - `{ "sub": "<sub>", "email": "<verified email>" }` — mint for an
//!    arbitrary identity that does NOT yet exist in storage; used by the
//!    invite-link e2e to authenticate a fresh visitor.
//!
//! Either mode requires the `dev_auth` Cargo feature AND
//! `LOCAL_AUTH_ISSUER` env var (belt and suspenders).

use crate::auth::local_issuer::{mint_claims, mint_for_test};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use domain::{Identity, Player};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use storage::Repository;

#[derive(Clone)]
pub struct DevLoginState {
    pub repo: Arc<dyn Repository>,
}

/// All fields optional — caller picks ONE of the two modes:
/// - `player` set → mint for the seeded player (its id becomes the sub,
///   `{id}@dev.invalid` becomes the verified email)
/// - `sub` AND `email` set → mint for an arbitrary identity (no Player
///   lookup; the resolver will return `AuthenticatedUnclaimed`)
#[derive(Deserialize)]
pub struct DevLoginRequest {
    pub player: Option<String>,
    pub sub: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize)]
pub struct DevLoginResponse {
    pub token: String,
}

pub async fn dev_login(
    State(state): State<DevLoginState>,
    Json(req): Json<DevLoginRequest>,
) -> Response {
    match (req.player, req.sub, req.email) {
        (Some(player_id), None, None) => mint_for_seeded_player(&state, &player_id).await,
        (None, Some(sub), Some(email)) => {
            let token = mint_for_test(&sub, &email);
            (StatusCode::OK, Json(DevLoginResponse { token })).into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "expected exactly one of { player } or { sub, email }",
        )
            .into_response(),
    }
}

async fn mint_for_seeded_player(state: &DevLoginState, player_id: &str) -> Response {
    let player = match state.repo.get_player(player_id).await {
        Ok(Some(p)) => p,
        _ => return (StatusCode::NOT_FOUND, "unknown player").into_response(),
    };

    // Mint a token that resolves as this player. Prefer the player's *actual*
    // Identity (so a pulled prod player — Google sub or real-but-anonymised
    // e-mail — logs in correctly), falling back to the seed convention when no
    // identity exists yet.
    let identities = state
        .repo
        .find_identities_by_person(&player.person_id)
        .await
        .unwrap_or_default();
    let token = match pick_identity(identities) {
        Some(identity) => {
            let (sub, email, connection) = claims_for_identity(&identity);
            mint_claims(&sub, email.as_deref(), &connection)
        }
        None => {
            let email = seed_fallback_email(&player);
            mint_for_test(&player.id, &email)
        }
    };
    (StatusCode::OK, Json(DevLoginResponse { token })).into_response()
}

/// Choose which identity to mint for when a player has several. Prefer an
/// `email` identity (the simplest claims), else the first by a stable order, so
/// the choice is deterministic.
fn pick_identity(mut identities: Vec<Identity>) -> Option<Identity> {
    identities.sort_by(|a, b| {
        a.provider
            .cmp(&b.provider)
            .then_with(|| a.provider_id.cmp(&b.provider_id))
    });
    identities
        .iter()
        .find(|i| i.provider == "email")
        .cloned()
        .or_else(|| identities.into_iter().next())
}

/// The `(sub, email, connection)` a local-issuer token must carry to resolve as
/// the owner of `identity`. Mirrors `resolution::identity_key_for`:
/// - `google` → `(provider_id, verified_email, "google")` (keyed by sub)
/// - everything else (e-mail) → `(id, provider_id, "email")` (keyed by e-mail,
///   where the e-mail identity's `provider_id` *is* the address).
fn claims_for_identity(identity: &Identity) -> (String, Option<String>, String) {
    match identity.provider.as_str() {
        "google" => (
            identity.provider_id.clone(),
            identity.verified_email.clone(),
            "google".to_owned(),
        ),
        _ => (
            identity.id.clone(),
            Some(identity.provider_id.clone()),
            "email".to_owned(),
        ),
    }
}

/// Seed-convention e-mail for a player with no stored identity: the result-user
/// at `RESULT_USER_EMAIL` (default `result-user@dev.invalid`), every other demo
/// player at `{id}@dev.invalid`.
fn seed_fallback_email(player: &Player) -> String {
    if player.is_result_user {
        std::env::var("RESULT_USER_EMAIL").unwrap_or_else(|_| "result-user@dev.invalid".into())
    } else {
        format!("{}@dev.invalid", player.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(provider: &str, provider_id: &str, email: Option<&str>) -> Identity {
        Identity {
            id: format!("identity-{provider}-{provider_id}"),
            provider: provider.to_owned(),
            provider_id: provider_id.to_owned(),
            person_id: "p-1".to_owned(),
            verified_email: email.map(str::to_owned),
        }
    }

    #[test]
    fn google_identity_mints_google_connection_keyed_by_sub() {
        let id = identity("google", "google-oauth2|123", Some("ada@dev.invalid"));
        let (sub, email, connection) = claims_for_identity(&id);
        assert_eq!(connection, "google");
        assert_eq!(sub, "google-oauth2|123", "resolver keys google on sub");
        assert_eq!(email.as_deref(), Some("ada@dev.invalid"));
    }

    #[test]
    fn email_identity_mints_email_connection_keyed_by_address() {
        let id = identity("email", "ada@dev.invalid", Some("ada@dev.invalid"));
        let (_sub, email, connection) = claims_for_identity(&id);
        assert_eq!(connection, "email");
        assert_eq!(
            email.as_deref(),
            Some("ada@dev.invalid"),
            "resolver keys email on the address, which is the provider_id"
        );
    }

    #[test]
    fn pick_prefers_email_over_google() {
        let ids = vec![
            identity("google", "google-oauth2|1", Some("ada@dev.invalid")),
            identity("email", "ada@dev.invalid", Some("ada@dev.invalid")),
        ];
        let picked = pick_identity(ids).unwrap();
        assert_eq!(picked.provider, "email");
    }

    #[test]
    fn pick_falls_back_to_first_when_no_email() {
        let ids = vec![identity("google", "google-oauth2|9", Some("g@dev.invalid"))];
        let picked = pick_identity(ids).unwrap();
        assert_eq!(picked.provider, "google");
    }

    #[test]
    fn pick_none_when_empty() {
        assert!(pick_identity(vec![]).is_none());
    }
}
