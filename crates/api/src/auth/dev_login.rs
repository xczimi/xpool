//! `POST /api/dev/login` — mints a local-issuer JWT.
//!
//! Two request shapes:
//!  - `{ "player": "<id>" }` — mint for a seeded `Player` (the existing
//!    AuthBar dev-picker flow).
//!  - `{ "sub": "<sub>", "email": "<verified email>" }` — mint for an
//!    arbitrary identity that does NOT yet exist in storage; used by the
//!    invite-link e2e to authenticate a fresh visitor.
//! Either mode requires the `dev_auth` Cargo feature AND
//! `LOCAL_AUTH_ISSUER` env var (belt and suspenders).

use crate::auth::local_issuer::mint_for_test;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
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
    let email = format!("{}@dev.invalid", player.id);
    let token = mint_for_test(&player.id, &email);
    (StatusCode::OK, Json(DevLoginResponse { token })).into_response()
}
