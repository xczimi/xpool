//! The §3 login-resolution algorithm.

mod common;

use api::auth::jwt::VerifiedClaims;
use api::auth::resolution::{identity_key_for, resolve_player};
use api::auth::CurrentPlayer;
use domain::{Identity, Person};

#[test]
fn identity_key_for_email_connection() {
    let claims = VerifiedClaims {
        sub: "auth0|abc".into(),
        verified_email: Some("ada@example.com".into()),
        verified_phone: None,
        connection: "email".into(),
    };
    let (provider, provider_id) = identity_key_for(&claims).unwrap();
    assert_eq!(provider, "email");
    assert_eq!(provider_id, "ada@example.com");
}

#[test]
fn identity_key_for_google_connection() {
    let claims = VerifiedClaims {
        sub: "google-oauth2|123".into(),
        verified_email: Some("ada@example.com".into()),
        verified_phone: None,
        connection: "google".into(),
    };
    let (provider, provider_id) = identity_key_for(&claims).unwrap();
    assert_eq!(provider, "google");
    assert_eq!(provider_id, "google-oauth2|123");
}

#[test]
fn identity_key_for_sms_connection() {
    let claims = VerifiedClaims {
        sub: "sms|xyz".into(),
        verified_email: None,
        verified_phone: Some("+15555550100".into()),
        connection: "sms".into(),
    };
    let (provider, provider_id) = identity_key_for(&claims).unwrap();
    assert_eq!(provider, "phone");
    assert_eq!(provider_id, "+15555550100");
}

#[tokio::test]
async fn resolve_finds_player_via_identity_lookup() {
    let (_, repo) = common::test_app_with_local_auth().await;
    repo.put_identity(&Identity {
        id: "i1".into(),
        provider: "email".into(),
        provider_id: "alice@example.com".into(),
        person_id: common::ALICE.into(),
        verified_email: Some("alice@example.com".into()),
    })
    .await
    .unwrap();
    repo.put_person(&Person {
        id: common::ALICE.into(),
        identity_ids: vec!["i1".into()],
    })
    .await
    .unwrap();
    // The `alice` Player is already seeded by common::test_app, with id == ALICE.

    let claims = VerifiedClaims {
        sub: "anything".into(),
        verified_email: Some("alice@example.com".into()),
        verified_phone: None,
        connection: "email".into(),
    };
    let current = resolve_player(repo.as_ref(), claims).await;
    match current {
        CurrentPlayer::Player(p) => assert_eq!(p.id, common::ALICE),
        other => panic!("expected Player, got {other:?}"),
    }
}

#[tokio::test]
async fn resolve_unknown_email_is_unclaimed() {
    let (_, repo) = common::test_app_with_local_auth().await;
    let claims = VerifiedClaims {
        sub: "auth0|xyz".into(),
        verified_email: Some("stranger@example.com".into()),
        verified_phone: None,
        connection: "email".into(),
    };
    let current = resolve_player(repo.as_ref(), claims).await;
    assert!(matches!(current, CurrentPlayer::AuthenticatedUnclaimed(_)));
}
