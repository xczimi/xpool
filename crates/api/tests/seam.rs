//! End-to-end: a request with a valid Bearer JWT for a seeded player
//! resolves to `CurrentPlayer::Player`. No header → `Visitor`. The
//! Identity→Person→Player resolver itself comes in Task 13; this test
//! uses the placeholder seam path (sub == player_id matches the seeded
//! player directly).

mod common;

use api::auth::local_issuer::mint_for_test;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn bearer_token_resolves_to_player() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;

    let token = mint_for_test(common::ALICE, "alice@example.com");
    let req = Request::post("/api/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(r#"{"query":"{ me { id } }"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["data"]["me"]["id"].as_str(), Some(common::ALICE));
}

#[tokio::test]
async fn no_bearer_is_visitor() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;
    let req = Request::post("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ me { id } }"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["errors"][0]["message"].as_str().unwrap().contains("authentication required"));
}
