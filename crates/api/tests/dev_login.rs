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

#[tokio::test]
async fn dev_login_with_sub_and_email_mints_a_token_for_an_arbitrary_identity() {
    let (app, _repo) = common::test_app_with_local_auth().await;
    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"sub":"auth0|stranger","email":"stranger@example.com"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = v["token"].as_str().unwrap();
    let claims = api::auth::local_issuer::verify_local(token).unwrap();
    assert_eq!(claims.sub, "auth0|stranger");
    assert_eq!(claims.email.as_deref(), Some("stranger@example.com"));
}

#[tokio::test]
async fn dev_login_rejects_a_request_with_neither_or_both_modes() {
    let (app, _repo) = common::test_app_with_local_auth().await;

    // empty body
    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // both modes
    let (app, _repo) = common::test_app_with_local_auth().await;
    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"player":"alice","sub":"s","email":"e@x"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
