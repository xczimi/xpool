mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn dev_login_returns_a_local_issuer_jwt() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;

    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"player":"{}"}}"#, common::ALICE)))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = v["token"].as_str().unwrap();
    assert!(!token.is_empty());
}

#[tokio::test]
async fn dev_login_rejects_unknown_player() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;
    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"player":"nobody"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
