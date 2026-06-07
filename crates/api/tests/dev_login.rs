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
        .body(Body::from(
            r#"{"sub":"auth0|stranger","email":"stranger@example.com"}"#,
        ))
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
async fn dev_login_for_result_user_uses_configured_email_and_resolves() {
    // The result-user's Identity is seeded at RESULT_USER_EMAIL (configurable),
    // not the `{id}@dev.invalid` scheme used for demo players. dev-login must
    // mint the token at that same email, otherwise `me` resolves to an
    // UnclaimedViewer and the admin can't log in via the dev picker.
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    std::env::set_var("RESULT_USER_EMAIL", "admin@example.com");
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, common::RESULT_ID, "admin@example.com").await;

    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"player":"{}"}}"#,
            common::RESULT_ID
        )))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let token = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let claims = api::auth::local_issuer::verify_local(&token).unwrap();
    assert_eq!(claims.email.as_deref(), Some("admin@example.com"));

    // The token must resolve to the admin Player — not an UnclaimedViewer.
    let me = common::query_with_bearer(
        &app,
        &token,
        r#"{"query":"{ me { __typename ... on Player { id isResultUser } } }"}"#,
    )
    .await;
    assert_eq!(me["data"]["me"]["__typename"], "Player");
    assert_eq!(me["data"]["me"]["id"], common::RESULT_ID);
    assert_eq!(me["data"]["me"]["isResultUser"], true);

    std::env::remove_var("RESULT_USER_EMAIL");
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
