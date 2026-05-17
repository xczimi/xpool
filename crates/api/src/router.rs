//! The axum router: the GraphQL endpoint, the playground, the health check,
//! and the auth seam (`API.md` §1, §8).

use crate::auth::CurrentPlayer;
use crate::gql::XpoolSchema;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    http::HeaderMap,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::sync::Arc;
use storage::Repository;
use tower_http::cors::CorsLayer;

/// Shared application state for the router.
#[derive(Clone)]
pub struct AppState {
    pub schema: XpoolSchema,
    pub repo: Arc<dyn Repository>,
}

/// Resolve the `CurrentPlayer` from the `X-Dev-Player` header (the dev auth
/// stub). No header → `Visitor`. An unknown player id → `Visitor` as well —
/// resolvers turn that into an auth error where a player is required.
async fn resolve_current_player(repo: &dyn Repository, headers: &HeaderMap) -> CurrentPlayer {
    let player_id = headers
        .get("x-dev-player")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    match player_id {
        Some(id) => match repo.get_player(&id).await {
            Ok(Some(player)) => CurrentPlayer::Authenticated(Box::new(player)),
            _ => CurrentPlayer::Visitor,
        },
        None => CurrentPlayer::Visitor,
    }
}

/// `POST /api/graphql` — execute a GraphQL request with the per-request
/// `CurrentPlayer` injected into the context.
async fn graphql_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let current = resolve_current_player(state.repo.as_ref(), &headers).await;
    let req = req.into_inner().data(current);
    state.schema.execute(req).await.into()
}

/// `GET /api/graphql` — the GraphiQL playground.
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new(
        "/api/graphql",
    )))
}

/// `GET /api/health` — liveness probe.
async fn health() -> impl IntoResponse {
    "ok"
}

/// Build the axum router. `cors` enables permissive CORS for local dev.
pub fn build_router(schema: XpoolSchema, repo: Arc<dyn Repository>, cors: bool) -> Router {
    let state = AppState { schema, repo };

    let mut router = Router::new()
        .route(
            "/api/graphql",
            get(graphql_playground).post(graphql_handler),
        )
        .route("/api/health", get(health))
        .with_state(state);

    if cors {
        router = router.layer(CorsLayer::permissive());
    }
    router
}
