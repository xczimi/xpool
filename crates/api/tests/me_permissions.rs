//! The `me` viewer exposes `mayCreatePool`, computed from the same referral
//! rule (`domain::pool::may_create_pool`) that gates the `createPool` mutation:
//! the result user and its direct referrals may create pools; everyone else
//! cannot ("restricted creation, open inviting").

mod common;

use common::{ALICE, BOB};

const ME_MAY_CREATE: &str = r#"{"query":"query { me { ... on Player { mayCreatePool } } }"}"#;

#[tokio::test]
async fn me_reports_may_create_pool_for_a_result_user_referral() {
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, ALICE, "alice@dev.invalid").await;

    let res = common::query_as(&app, ALICE, ME_MAY_CREATE).await;
    assert!(res.get("errors").is_none(), "query errored: {res:?}");
    assert_eq!(
        res["data"]["me"]["mayCreatePool"],
        serde_json::json!(true),
        "ALICE is referred by the result-user → may create pools"
    );
}

#[tokio::test]
async fn me_denies_may_create_pool_for_a_plain_joiner() {
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, BOB, "bob@dev.invalid").await;

    let res = common::query_as(&app, BOB, ME_MAY_CREATE).await;
    assert!(res.get("errors").is_none(), "query errored: {res:?}");
    assert_eq!(
        res["data"]["me"]["mayCreatePool"],
        serde_json::json!(false),
        "BOB has no result-user referrer → may not create pools"
    );
}
