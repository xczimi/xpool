//! xpool API crate — an axum + async-graphql server (`API.md`).
//!
//! The GraphQL layer is a thin adapter: coarse load → expose graph → glue to
//! the pure `domain`/`fwc26` functions. The auth seam is a dev stub
//! (`X-Dev-Player` header → `CurrentPlayer` in context).

pub mod auth;
pub mod clock;
pub mod cloudfront_auth;
pub mod gql;
pub mod recompute;
pub mod router;
pub mod timeflags;

use std::sync::Arc;
use storage::Repository;

/// Build the axum app from a repository: schema + router.
///
/// `cors` enables permissive CORS for local dev. `cloudfront_secret`, when
/// `Some(_)`, attaches the [`cloudfront_auth`] layer that requires every
/// request to carry a matching `X-CloudFront-Secret` header; `None` skips
/// that layer entirely (the local-dev path). See `cloudfront_auth.rs` for
/// the why.
pub fn build_app(
    repo: Arc<dyn Repository>,
    cors: bool,
    cloudfront_secret: Option<String>,
) -> axum::Router {
    let schema = gql::build_schema(repo.clone());
    router::build_router(schema, repo, cors, cloudfront_secret)
}
