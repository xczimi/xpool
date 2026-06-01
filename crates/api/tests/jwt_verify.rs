//! The multi-issuer verifier dispatches by the trust-list configured in
//! env vars. Auth0 path is stubbed in this test (no network); the local
//! path runs end-to-end.

use api::auth::jwt::{verify_token, TrustList};
use api::auth::local_issuer::mint_for_test;

#[tokio::test]
async fn local_issuer_only_accepts_local_tokens() {
    let trust = TrustList::from_env_for_test(/* local */ true, /* auth0 */ None);
    let token = mint_for_test("demo-ada", "ada@example.com");
    let verified = verify_token(&trust, &token).await.expect("local must verify");
    assert_eq!(verified.sub, "demo-ada");
    assert_eq!(verified.verified_email.as_deref(), Some("ada@example.com"));
}

#[tokio::test]
async fn empty_trustlist_rejects_everything() {
    let trust = TrustList::from_env_for_test(false, None);
    let token = mint_for_test("demo-ada", "ada@example.com");
    assert!(verify_token(&trust, &token).await.is_err());
}

#[tokio::test]
async fn malformed_token_fails_closed() {
    let trust = TrustList::from_env_for_test(true, None);
    assert!(verify_token(&trust, "not.a.jwt").await.is_err());
}
