//! Tests for the invite/referral table (`Repository::put_invite` /
//! `get_invite` / `list_invites_by_pool` / `list_invites_by_invited_by` /
//! `revoke_invite`).
//!
//! The in-memory tests run unconditionally; the DynamoDB tests are gated behind
//! `DYNAMO_TEST=1` (same pattern as `tests/invite_usage.rs`).

use chrono::{TimeZone, Utc};
use domain::Invite;
use storage::{InMemoryRepository, Repository};

fn invite(code: &str, pool: &str, invited_by: &str) -> Invite {
    Invite {
        code: code.to_owned(),
        pool_id: pool.to_owned(),
        invited_by: invited_by.to_owned(),
        created_at: Utc.with_ymd_and_hms(2026, 6, 7, 12, 0, 0).unwrap(),
        expires_at: None,
        revoked: false,
    }
}

// ── in-memory tests ───────────────────────────────────────────────────────────

#[tokio::test]
async fn put_then_get_round_trips_an_invite() {
    let repo = InMemoryRepository::default();
    let inv = invite("AD9XK3P7QT", "southsiders", "ada");
    repo.put_invite(&inv).await.unwrap();
    assert_eq!(repo.get_invite("AD9XK3P7QT").await.unwrap(), Some(inv));
}

#[tokio::test]
async fn get_unknown_code_is_none() {
    let repo = InMemoryRepository::default();
    assert_eq!(repo.get_invite("NOPE").await.unwrap(), None);
}

#[tokio::test]
async fn list_invites_by_pool_filters_to_that_pool() {
    let repo = InMemoryRepository::default();
    repo.put_invite(&invite("AAA", "southsiders", "ada"))
        .await
        .unwrap();
    repo.put_invite(&invite("BBB", "southsiders", "alan"))
        .await
        .unwrap();
    repo.put_invite(&invite("CCC", "workfriends", "grace"))
        .await
        .unwrap();

    let mut codes: Vec<String> = repo
        .list_invites_by_pool("southsiders")
        .await
        .unwrap()
        .into_iter()
        .map(|i| i.code)
        .collect();
    codes.sort();
    assert_eq!(codes, vec!["AAA".to_owned(), "BBB".to_owned()]);
}

#[tokio::test]
async fn list_invites_by_invited_by_filters_to_that_player() {
    let repo = InMemoryRepository::default();
    repo.put_invite(&invite("AAA", "southsiders", "ada"))
        .await
        .unwrap();
    repo.put_invite(&invite("BBB", "workfriends", "ada"))
        .await
        .unwrap();
    repo.put_invite(&invite("CCC", "southsiders", "alan"))
        .await
        .unwrap();

    let mut codes: Vec<String> = repo
        .list_invites_by_invited_by("ada")
        .await
        .unwrap()
        .into_iter()
        .map(|i| i.code)
        .collect();
    codes.sort();
    assert_eq!(codes, vec!["AAA".to_owned(), "BBB".to_owned()]);
}

#[tokio::test]
async fn revoke_invite_marks_it_revoked() {
    let repo = InMemoryRepository::default();
    repo.put_invite(&invite("AD9XK3P7QT", "southsiders", "ada"))
        .await
        .unwrap();
    repo.revoke_invite("AD9XK3P7QT").await.unwrap();

    let stored = repo.get_invite("AD9XK3P7QT").await.unwrap().unwrap();
    assert!(stored.revoked, "invite should be flagged revoked");
}

#[tokio::test]
async fn revoke_unknown_invite_is_a_no_op() {
    let repo = InMemoryRepository::default();
    // Must not error when the code does not exist.
    repo.revoke_invite("GHOST").await.unwrap();
    assert_eq!(repo.get_invite("GHOST").await.unwrap(), None);
}

// ── DynamoDB tests (gated behind DYNAMO_TEST=1) ───────────────────────────────

fn dynamo_enabled() -> bool {
    std::env::var("DYNAMO_TEST").as_deref() == Ok("1")
}

async fn dynamo_repo() -> storage::DynamoRepository {
    std::env::set_var(
        "XPOOL_TABLE",
        std::env::var("XPOOL_TABLE").unwrap_or_else(|_| "xpool-test".to_owned()),
    );
    std::env::set_var(
        "CURRENT_TOURNAMENT_ID",
        std::env::var("CURRENT_TOURNAMENT_ID").unwrap_or_else(|_| "test".to_owned()),
    );
    let repo = storage::DynamoRepository::from_env().await.expect("build repo");
    repo.ensure_table().await.expect("ensure_table");
    repo
}

#[tokio::test]
async fn dynamo_put_get_and_revoke_round_trip() {
    if !dynamo_enabled() {
        return;
    }
    let repo = dynamo_repo().await;
    let pid = std::process::id();
    let code = format!("dyntest-inv-{pid}");
    let pool = format!("dyntest-pool-{pid}");

    repo.put_invite(&invite(&code, &pool, "ada")).await.unwrap();
    let got = repo.get_invite(&code).await.unwrap().expect("present");
    assert_eq!(got.pool_id, pool);
    assert!(!got.revoked);

    repo.revoke_invite(&code).await.unwrap();
    assert!(repo.get_invite(&code).await.unwrap().unwrap().revoked);
}

#[tokio::test]
async fn dynamo_list_invites_by_pool() {
    if !dynamo_enabled() {
        return;
    }
    let repo = dynamo_repo().await;
    let pid = std::process::id();
    let pool = format!("dyntest-listpool-{pid}");
    let a = format!("dyntest-la-{pid}");
    let b = format!("dyntest-lb-{pid}");

    repo.put_invite(&invite(&a, &pool, "ada")).await.unwrap();
    repo.put_invite(&invite(&b, &pool, "alan")).await.unwrap();

    let found: Vec<String> = repo
        .list_invites_by_pool(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|i| i.code)
        .collect();
    assert!(found.contains(&a) && found.contains(&b));
}
