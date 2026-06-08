//! Invite mutations over the stored invite table: `createInvite` (mint/reuse a
//! pool-bound reusable code), `claimInvite` (the not-yet-a-Player front door —
//! lazy-create + join + record `invited_by`), and the AUTH-13 link path.

mod common;

use common::{ALICE, BOB};

/// ALICE (an admin) creates pool `p1` and mints her invite into it. Returns the
/// nested `PREFIX-SUFFIX` invite code. The app must be `test_app_with_local_auth`.
async fn alice_pool_and_invite(
    app: &axum::Router,
    repo: &std::sync::Arc<dyn storage::Repository>,
) -> String {
    common::seed_identity_for(repo, ALICE, "alice@dev.invalid").await;
    let create_pool =
        r#"{"query":"mutation { createPool(id: \"p1\", name: \"Friends\") { id prefix } }"}"#;
    let res = common::query_as(app, ALICE, create_pool).await;
    assert!(res.get("errors").is_none(), "createPool failed: {res:?}");

    let invite = r#"{"query":"mutation { createInvite(pool: \"p1\") { code link } }"}"#;
    let res = common::query_as(app, ALICE, invite).await;
    assert!(res.get("errors").is_none(), "createInvite failed: {res:?}");
    res["data"]["createInvite"]["code"]
        .as_str()
        .expect("code")
        .to_string()
}

#[tokio::test]
async fn create_invite_returns_a_nested_link() {
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, ALICE, "alice@dev.invalid").await;
    let create_pool =
        r#"{"query":"mutation { createPool(id: \"p1\", name: \"Friends\") { id prefix } }"}"#;
    let res = common::query_as(&app, ALICE, create_pool).await;
    let prefix = res["data"]["createPool"]["prefix"]
        .as_str()
        .expect("prefix")
        .to_string();

    let invite = r#"{"query":"mutation { createInvite(pool: \"p1\") { code link } }"}"#;
    let res = common::query_as(&app, ALICE, invite).await;
    let code = res["data"]["createInvite"]["code"].as_str().unwrap();
    let link = res["data"]["createInvite"]["link"].as_str().unwrap();
    assert!(
        code.starts_with(&format!("{prefix}-")),
        "code `{code}` must be the nested PREFIX-SUFFIX form"
    );
    assert!(link.ends_with(code), "link must end with the nested code");
}

#[tokio::test]
async fn create_invite_is_reused_not_duplicated() {
    let (app, repo) = common::test_app_with_local_auth().await;
    let code1 = alice_pool_and_invite(&app, &repo).await;
    // A second createInvite into the same pool returns the same code.
    let invite = r#"{"query":"mutation { createInvite(pool: \"p1\") { code } }"}"#;
    let res = common::query_as(&app, ALICE, invite).await;
    let code2 = res["data"]["createInvite"]["code"].as_str().unwrap();
    assert_eq!(code1, code2, "reusable per-member: same code returned");
}

#[tokio::test]
async fn claim_invite_creates_player_with_invited_by_referrer() {
    let (app, repo) = common::test_app_with_local_auth().await;
    let code = alice_pool_and_invite(&app, &repo).await;

    // A fresh visitor (no matching Identity) authenticates and claims the code.
    let token = api::auth::local_issuer::mint_for_test("auth0|newbie", "newbie@example.com");
    let claim_body = format!(
        r#"{{"query":"mutation {{ claimInvite(code: \"{code}\", nick: \"Newbie\", fullName: \"New B.\") {{ player {{ id nick }} }} }}"}}"#,
    );
    let res = common::query_with_bearer(&app, &token, &claim_body).await;
    assert!(res.get("errors").is_none(), "claim should succeed: {res:?}");
    let player_id = res["data"]["claimInvite"]["player"]["id"]
        .as_str()
        .expect("player id");

    let player = repo
        .get_player(player_id)
        .await
        .unwrap()
        .expect("player exists");
    assert_eq!(player.nick, "Newbie");
    // referrer = the invite's invited_by (ALICE), recorded at join time.
    assert_eq!(player.referrer.as_deref(), Some(ALICE));
    // …and they landed in the invite's pool.
    let pool = repo.list_pools().await.unwrap().pop().expect("pool");
    assert!(
        pool.members.iter().any(|m| m == player_id),
        "claimer must be a member of the invite's pool"
    );
}

#[tokio::test]
async fn claim_invite_for_existing_person_does_not_duplicate() {
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, BOB, "bob@dev.invalid").await;
    let code = alice_pool_and_invite(&app, &repo).await;

    // Bob (already a Player) opens the link — no new Player is created.
    let claim_body = format!(
        r#"{{"query":"mutation {{ claimInvite(code: \"{code}\", nick: \"X\", fullName: \"Y\") {{ player {{ id }} }} }}"}}"#,
    );
    let res = common::query_as(&app, BOB, &claim_body).await;
    let player_id = res["data"]["claimInvite"]["player"]["id"]
        .as_str()
        .expect("player id");
    assert_eq!(player_id, BOB);
    let bob = repo.get_player(BOB).await.unwrap().expect("bob exists");
    assert_ne!(bob.nick, "X", "Bob keeps his existing profile");
    // Bob had no referrer; accepting the invite records ALICE as his referrer.
    assert_eq!(bob.referrer.as_deref(), Some(ALICE));
}

#[tokio::test]
async fn reusable_invite_accepts_a_second_distinct_claimer() {
    let (app, repo) = common::test_app_with_local_auth().await;
    let code = alice_pool_and_invite(&app, &repo).await;

    let claim = |nick: &str| {
        format!(
            r#"{{"query":"mutation {{ claimInvite(code: \"{code}\", nick: \"{nick}\", fullName: \"F.\") {{ player {{ id }} }} }}"}}"#,
        )
    };

    let token1 = api::auth::local_issuer::mint_for_test("auth0|first", "first@example.com");
    let res1 = common::query_with_bearer(&app, &token1, &claim("First")).await;
    assert!(res1.get("errors").is_none(), "first claim: {res1:?}");

    // A second, different user claims the SAME code — reusable, so it succeeds.
    let token2 = api::auth::local_issuer::mint_for_test("auth0|second", "second@example.com");
    let res2 = common::query_with_bearer(&app, &token2, &claim("Second")).await;
    assert!(
        res2.get("errors").is_none(),
        "reusable code: second claim should also succeed: {res2:?}"
    );
}

#[tokio::test]
async fn me_for_unclaimed_with_email_match_signals_link_path() {
    let (app, repo) = common::test_app_with_local_auth().await;
    // Wire alice's existing Person to a Google identity with the same
    // verified email a stranger is about to log in with.
    repo.put_person(&domain::Person {
        id: ALICE.into(),
        identity_ids: vec!["i-alice-google".into()],
    })
    .await
    .unwrap();
    repo.put_identity(&domain::Identity {
        id: "i-alice-google".into(),
        provider: "google".into(),
        provider_id: "google-oauth2|alice".into(),
        person_id: ALICE.into(),
        verified_email: Some("alice@example.com".into()),
    })
    .await
    .unwrap();

    // Someone logs in via passwordless email to alice@example.com (sub
    // differs from any existing Identity row's provider_id, so no direct
    // lookup hit).
    let token =
        api::auth::local_issuer::mint_for_test("auth0|new-sub-for-alice", "alice@example.com");
    let body = r#"{"query":"{ me { __typename ... on UnclaimedViewer { linkCandidate { personId provider } } } }"}"#;
    let res = common::query_with_bearer(&app, &token, body).await;
    assert!(
        res.get("errors").is_none()
            || res["errors"]
                .as_array()
                .map(|e| e.is_empty())
                .unwrap_or(true),
        "me query should not error: {res:?}"
    );
    assert_eq!(res["data"]["me"]["__typename"], "UnclaimedViewer");
    let candidate = &res["data"]["me"]["linkCandidate"];
    assert_eq!(candidate["personId"], ALICE);
    assert_eq!(candidate["provider"], "google");
}

#[tokio::test]
async fn confirm_link_attaches_a_new_identity_to_an_existing_person() {
    let (app, repo) = common::test_app_with_local_auth().await;
    repo.put_person(&domain::Person {
        id: ALICE.into(),
        identity_ids: vec!["i-alice-google".into()],
    })
    .await
    .unwrap();
    repo.put_identity(&domain::Identity {
        id: "i-alice-google".into(),
        provider: "google".into(),
        provider_id: "google-oauth2|alice".into(),
        person_id: ALICE.into(),
        verified_email: Some("alice@example.com".into()),
    })
    .await
    .unwrap();

    let token =
        api::auth::local_issuer::mint_for_test("auth0|new-sub-for-alice", "alice@example.com");
    let body = format!(
        r#"{{"query":"mutation {{ confirmLink(personId: \"{ALICE}\") {{ player {{ id }} }} }}"}}"#,
    );
    let res = common::query_with_bearer(&app, &token, &body).await;
    assert!(
        res.get("errors").is_none()
            || res["errors"]
                .as_array()
                .map(|e| e.is_empty())
                .unwrap_or(true),
        "confirmLink should succeed: {res:?}"
    );
    assert_eq!(res["data"]["confirmLink"]["player"]["id"], ALICE);

    let person = repo.get_person(ALICE).await.unwrap().expect("person");
    assert_eq!(person.identity_ids.len(), 2);
}
