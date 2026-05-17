//! The auth seam (`API.md` §8, `DATA_MODEL.md` §12).
//!
//! Phase 1 is a **dev stub**: the edge reads the `X-Dev-Player` header,
//! resolves the `Player`, and places a `CurrentPlayer` in the GraphQL context.
//! Resolvers read it from context and never re-authenticate. Swapping to real
//! auth is a change in one place — the header read in `router.rs`.

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
