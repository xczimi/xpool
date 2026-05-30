//! The auth seam (`docs/superpowers/specs/2026-05-30-auth-design.md`).
//!
//! Bearer-JWT verification, multi-issuer (Auth0 + local). Three-state
//! `CurrentPlayer`. Identity → Person → Player resolution. The
//! `X-Dev-Player` header is gone — local dev mints local-issuer JWTs via
//! the dev-login endpoint instead (one auth code path).

pub mod local_issuer;

// Filled in by later tasks:
//   pub mod jwt;
//   pub mod auth0_jwks;
//   pub mod resolution;
//   pub mod seam;
//   pub mod invite_code;
//   #[cfg(feature = "dev_auth")] pub mod dev_login;

// CurrentPlayer moves here in Task 4. Re-exported at the module root for
// callers that used `crate::auth::CurrentPlayer` previously.

use async_graphql::Context;
use domain::Player;

/// The viewer of a request, placed in the GraphQL context.
///
/// `Visitor` — no `X-Dev-Player` header; unauthenticated.
/// `Authenticated` — a resolved `Player`.
#[derive(Clone, Debug)]
pub enum CurrentPlayer {
    Visitor,
    Authenticated(Box<Player>),
}

impl CurrentPlayer {
    /// The authenticated player, or a GraphQL auth error for a visitor.
    pub fn require<'a>(ctx: &'a Context<'_>) -> async_graphql::Result<&'a Player> {
        match ctx.data_unchecked::<CurrentPlayer>() {
            CurrentPlayer::Authenticated(p) => Ok(p),
            CurrentPlayer::Visitor => Err(async_graphql::Error::new(
                "authentication required: send an X-Dev-Player header",
            )),
        }
    }

    /// The authenticated player only if they are the result user (admin).
    pub fn require_admin<'a>(ctx: &'a Context<'_>) -> async_graphql::Result<&'a Player> {
        let player = Self::require(ctx)?;
        if !player.is_result_user {
            return Err(async_graphql::Error::new(
                "admin privileges required (result user only)",
            ));
        }
        Ok(player)
    }
}
