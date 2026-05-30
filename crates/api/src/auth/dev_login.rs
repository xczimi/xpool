//! `POST /api/dev/login` — mints a local-issuer JWT for a seeded player.
//! Gated behind the `dev_auth` Cargo feature (off in `--features lambda`
//! production builds) AND the `LOCAL_AUTH_ISSUER` env var (off in prod
//! config). Belt-and-suspenders.

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

#[derive(Deserialize)]
pub struct DevLoginRequest {
    pub player: String,
}

#[derive(Serialize)]
pub struct DevLoginResponse {
    pub token: String,
}

pub async fn dev_login(
    State(state): State<DevLoginState>,
    Json(req): Json<DevLoginRequest>,
) -> Response {
    // Verify the player exists. The dev-login endpoint is the only path
    // where pre-existing seeded players are looked up by id — Auth0 doesn't
    // get this shortcut.
    let player = match state.repo.get_player(&req.player).await {
        Ok(Some(p)) => p,
        _ => return (StatusCode::NOT_FOUND, "unknown player").into_response(),
    };
    // Synthesize a verified email from the player id so the resolver
    // (Task 13) can find the corresponding Identity row.
    let email = format!("{}@dev.invalid", player.id);
    let token = mint_for_test(&player.id, &email);
    (StatusCode::OK, Json(DevLoginResponse { token })).into_response()
}
