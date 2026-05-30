//! The axum router: GraphQL endpoint, playground, health, and the auth
//! seam (`docs/superpowers/specs/2026-05-30-auth-design.md` §2).

use crate::auth::seam::{auth_seam, SeamState};
use crate::auth::CurrentPlayer;
use crate::auth::jwt::TrustList;
use crate::gql::XpoolSchema;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::sync::Arc;
use storage::Repository;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub schema: XpoolSchema,
    pub repo: Arc<dyn Repository>,
}

async fn graphql_handler(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentPlayer>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let now = crate::clock::RequestNow(crate::clock::resolve_now(&headers));
    let req = req.into_inner().data(current).data(now);
    state.schema.execute(req).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new(
        "/api/graphql",
    )))
}

async fn health() -> impl IntoResponse {
    "ok"
}

pub fn build_router(
    schema: XpoolSchema,
    repo: Arc<dyn Repository>,
    cors: bool,
    cloudfront_secret: Option<String>,
) -> Router {
    let state = AppState { schema, repo: repo.clone() };
    let trust = TrustList::from_env();
    let seam_state = SeamState { trust, repo };

    let mut router = Router::new()
        .route(
            "/api/graphql",
            get(graphql_playground).post(graphql_handler),
        )
        .route("/api/health", get(health))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            seam_state,
            auth_seam,
        ));

    if cors {
        router = router.layer(CorsLayer::permissive());
    }
    if let Some(expected) = cloudfront_secret {
        router = router.layer(axum::middleware::from_fn_with_state(
            expected,
            crate::cloudfront_auth::require_cloudfront_secret,
        ));
    }
    router
}
