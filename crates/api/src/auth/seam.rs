//! The auth seam axum middleware. Runs after `cloudfront_auth`. Extracts a
//! Bearer token; verifies against the trust-list; resolves to a
//! `CurrentPlayer`; places it into request extensions for the GraphQL
//! handler to read.
//!
//! A request with no `Authorization` header is a `Visitor` — no token, no
//! error. An invalid token IS an error (`401`).

use crate::auth::jwt::{verify_token, TrustList, VerifiedClaims};
use crate::auth::{CurrentPlayer, VerifiedIdentity};
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use storage::Repository;

#[derive(Clone)]
pub struct SeamState {
    pub trust: TrustList,
    pub repo: Arc<dyn Repository>,
}

pub async fn auth_seam(
    State(state): State<SeamState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let current = match bearer {
        None => CurrentPlayer::Visitor,
        Some(token) => match verify_token(&state.trust, token).await {
            Err(_) => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
            Ok(claims) => to_current_player(claims, state.repo.as_ref()).await,
        },
    };

    request.extensions_mut().insert(current);
    next.run(request).await
}

async fn to_current_player(claims: VerifiedClaims, repo: &dyn Repository) -> CurrentPlayer {
    // Task 13 replaces this with the full §3 algorithm via
    // `resolution::resolve_player`. Placeholder for now: if a Player whose
    // id equals the JWT's `sub` exists, that's the player; otherwise
    // unclaimed.
    if let Ok(Some(player)) = repo.get_player(&claims.sub).await {
        CurrentPlayer::Player(Box::new(player))
    } else {
        CurrentPlayer::AuthenticatedUnclaimed(VerifiedIdentity {
            connection: claims.connection,
            sub: claims.sub,
            verified_email: claims.verified_email,
            verified_phone: claims.verified_phone,
        })
    }
}
