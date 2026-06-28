//! Router-level tests for the `X-CloudFront-Secret` middleware.
//!
//! The inline tests in `cloudfront_auth.rs` cover the predicate; these
//! tests exercise the full axum middleware wiring through `build_app`.
//! `/api/health` is the canary — it's a static "ok" handler so we can
//! attribute any failure to the middleware rather than to the GraphQL or
//! repository layer.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use std::sync::Arc;
use storage::{InMemoryRepository, Repository};
use tower::ServiceExt;

fn router(cloudfront_secret: Option<&str>) -> Router {
    let repo: Arc<dyn Repository> = Arc::new(InMemoryRepository::new());
    // `cors = false` keeps the layer stack minimal and avoids hitting CORS
    // preflight branches; we're testing the secret check, not CORS.
    api::build_app(
        repo,
        false,
        cloudfront_secret.map(String::from),
        std::sync::Arc::new(mail::NullSender),
    )
}

async fn status_for(router: Router, secret_header: Option<&str>) -> (StatusCode, String) {
    let mut req = Request::builder().method("GET").uri("/api/health");
    if let Some(s) = secret_header {
        req = req.header("X-CloudFront-Secret", s);
    }
    let response = router
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn local_dev_no_secret_configured_is_open() {
    // env var unset → no middleware attached → request without any
    // X-CloudFront-Secret header still reaches the handler. This is the
    // local-dev behaviour we must preserve.
    let (status, body) = status_for(router(None), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn matching_header_is_let_through() {
    let (status, body) = status_for(router(Some("xyz")), Some("xyz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn mismatched_header_is_forbidden() {
    let (status, _) = status_for(router(Some("xyz")), Some("nope")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_header_is_forbidden() {
    let (status, _) = status_for(router(Some("xyz")), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
