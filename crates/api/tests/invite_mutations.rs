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

#[tokio::test]
async fn claim_invite_creates_player_with_referrer() {
    unsafe {
        std::env::set_var("INVITE_CODE_SECRET", "test-secret-must-be-32-bytes-long");
    }
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, common::ALICE, "alice@dev.invalid").await;

    // Alice creates an invite.
    let create_body = r#"{"query":"mutation { createInvite(pool: null) { code } }"}"#;
    let res = common::query_as(&app, common::ALICE, create_body).await;
    let code = res["data"]["createInvite"]["code"].as_str().expect("code").to_string();

    // A fresh visitor (no matching Identity) authenticates with verified email
    // "newbie@example.com" and claims with that code, supplying nick / full name.
    let token = api::auth::local_issuer::mint_for_test("auth0|newbie", "newbie@example.com");
    let claim_body = format!(
        r#"{{"query":"mutation {{ claimInvite(code: \"{code}\", nick: \"Newbie\", fullName: \"New B.\") {{ player {{ id nick }} }} }}"}}"#,
    );
    let res = common::query_with_bearer(&app, &token, &claim_body).await;
    assert!(res.get("errors").is_none(), "claim should succeed: {res:?}");
    let player_id = res["data"]["claimInvite"]["player"]["id"].as_str().expect("player id");

    let player = repo.get_player(player_id).await.unwrap().expect("player exists");
    assert_eq!(player.nick, "Newbie");
    assert_eq!(player.referrer.as_deref(), Some(common::ALICE));
}

#[tokio::test]
async fn claim_invite_for_existing_person_does_not_duplicate() {
    unsafe {
        std::env::set_var("INVITE_CODE_SECRET", "test-secret-must-be-32-bytes-long");
    }
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, common::ALICE, "alice@dev.invalid").await;
    // BOB exists in the seeded fixture too, with an Identity wired.
    common::seed_identity_for(&repo, common::BOB, "bob@dev.invalid").await;

    // Alice creates an invite.
    let create_body = r#"{"query":"mutation { createInvite(pool: null) { code } }"}"#;
    let res = common::query_as(&app, common::ALICE, create_body).await;
    let code = res["data"]["createInvite"]["code"].as_str().expect("code").to_string();

    // Bob (already in the system) opens the link. The result should NOT create a new Player.
    let claim_body = format!(
        r#"{{"query":"mutation {{ claimInvite(code: \"{code}\", nick: \"X\", fullName: \"Y\") {{ player {{ id }} }} }}"}}"#,
    );
    let res = common::query_as(&app, common::BOB, &claim_body).await;
    let player_id = res["data"]["claimInvite"]["player"]["id"].as_str().expect("player id");
    assert_eq!(player_id, common::BOB);
    // Nick was NOT changed to "X" — Bob keeps his existing profile.
    let bob = repo.get_player(common::BOB).await.unwrap().expect("bob exists");
    assert_ne!(bob.nick, "X");
}
