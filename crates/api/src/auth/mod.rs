//! The auth seam (`docs/superpowers/specs/2026-05-30-auth-design.md`).
//!
//! Bearer-JWT verification, multi-issuer (Auth0 + local). Three-state
//! `CurrentPlayer`. Identity → Person → Player resolution. The
//! `X-Dev-Player` header is gone — local dev mints local-issuer JWTs via
//! the dev-login endpoint instead (one auth code path).

pub mod auth0_jwks;
pub mod jwt;
pub mod local_issuer;
pub mod resolution;
pub mod seam;

#[cfg(feature = "dev_auth")]
pub mod dev_login;
pub mod invite_code;

// CurrentPlayer moves here in Task 4. Re-exported at the module root for
// callers that used `crate::auth::CurrentPlayer` previously.

use async_graphql::Context;
use domain::Player;

/// The verified-identity claims-set the seam extracts from a JWT, *before*
/// any Identity/Person lookup. Carried through the AUTH-06 unclaimed state
/// so the claim flow can act on it.
#[derive(Clone, Debug)]
pub struct VerifiedIdentity {
    /// "email" | "sms" | "google" | "dev"
    pub connection: String,
    /// The original `sub` from the JWT (Auth0 connection-specific or local).
    pub sub: String,
    pub verified_email: Option<String>,
    pub verified_phone: Option<String>,
}

/// The viewer of a request, placed in the GraphQL context.
#[derive(Clone, Debug)]
pub enum CurrentPlayer {
    /// No / invalid token.
    Visitor,
    /// Valid token, verified contact, but no `Person`/`Player` (AUTH-06).
    AuthenticatedUnclaimed(VerifiedIdentity),
    /// Resolved `Player` (including the result-user).
    Player(Box<Player>),
}

impl CurrentPlayer {
    /// The authenticated player, or a GraphQL auth error otherwise.
    pub fn require<'a>(ctx: &'a Context<'_>) -> async_graphql::Result<&'a Player> {
        match ctx.data_unchecked::<CurrentPlayer>() {
            CurrentPlayer::Player(p) => Ok(p),
            CurrentPlayer::AuthenticatedUnclaimed(_) => {
                Err(async_graphql::Error::new("invitation required"))
            }
            CurrentPlayer::Visitor => Err(async_graphql::Error::new("authentication required")),
        }
    }

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
