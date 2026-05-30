mod common;

use api::auth::invite_code::decode_invite;

#[tokio::test]
async fn create_invite_returns_a_signed_link() {
    unsafe {
        std::env::set_var("INVITE_CODE_SECRET", "test-secret-must-be-32-bytes-long");
    }
    let (app, repo) = common::test_app_with_local_auth().await;
    // auth_header(ALICE) mints a JWT with email "alice@dev.invalid" —
    // seed_identity_for must use the same email so the seam resolves to a Player.
    common::seed_identity_for(&repo, common::ALICE, "alice@dev.invalid").await;

    let body = r#"{"query":"mutation { createInvite(pool: null) { code link } }"}"#;
    let res = common::query_as(&app, common::ALICE, body).await;
    let code = res["data"]["createInvite"]["code"]
        .as_str()
        .unwrap()
        .to_string();
    let link = res["data"]["createInvite"]["link"].as_str().unwrap();
    assert!(link.ends_with(&code), "link must end with the opaque code");
    let payload =
        decode_invite(b"test-secret-must-be-32-bytes-long", &code).unwrap();
    assert_eq!(payload.referrer, common::ALICE);
    assert!(payload.pool.is_none());
}
