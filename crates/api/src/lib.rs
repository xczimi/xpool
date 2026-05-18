//! xpool API crate — an axum + async-graphql server (`API.md`).
//!
//! The GraphQL layer is a thin adapter: coarse load → expose graph → glue to
//! the pure `domain`/`fwc26` functions. The auth seam is a dev stub
//! (`X-Dev-Player` header → `CurrentPlayer` in context).

pub mod auth;
pub mod clock;
pub mod gql;
pub mod recompute;
pub mod router;

use std::sync::Arc;
use storage::Repository;

/// Build the axum app from a repository: schema + router. `cors` enables
/// permissive CORS for local dev. Used by both `main.rs` and the tests.
pub fn build_app(repo: Arc<dyn Repository>, cors: bool) -> axum::Router {
    let schema = gql::build_schema(repo.clone());
    router::build_router(schema, repo, cors)
}
