//! Tests for `Repository::find_identities_by_verified_email`.
//! The in-memory tests run unconditionally; the DynamoDB tests are gated
//! behind `DYNAMO_TEST=1` (same pattern as `tests/dynamo.rs`).

use domain::Identity;
use storage::{InMemoryRepository, Repository};

// ── in-memory tests ───────────────────────────────────────────────────────────

#[tokio::test]
async fn find_by_verified_email_returns_all_matches_for_one_person() {
    let repo = InMemoryRepository::default();
    repo.put_identity(&Identity {
        id: "i1".into(),
        provider: "email".into(),
        provider_id: "ada@example.com".into(),
        person_id: "p1".into(),
        verified_email: Some("ada@example.com".into()),
    })
    .await
    .unwrap();
    repo.put_identity(&Identity {
        id: "i2".into(),
        provider: "google".into(),
        provider_id: "g-123".into(),
        person_id: "p1".into(),
        verified_email: Some("ada@example.com".into()),
    })
    .await
    .unwrap();
    repo.put_identity(&Identity {
        id: "i3".into(),
        provider: "email".into(),
        provider_id: "other@example.com".into(),
        person_id: "p2".into(),
        verified_email: Some("other@example.com".into()),
    })
    .await
    .unwrap();

    let hits = repo
        .find_identities_by_verified_email("ada@example.com")
        .await
        .unwrap();
    assert_eq!(hits.len(), 2);
    let person_ids: std::collections::HashSet<_> =
        hits.iter().map(|i| i.person_id.clone()).collect();
    assert_eq!(
        person_ids,
        ["p1".to_string()].into_iter().collect::<std::collections::HashSet<_>>()
    );
}

#[tokio::test]
async fn find_by_verified_email_skips_identities_with_no_email() {
    let repo = InMemoryRepository::default();
    repo.put_identity(&Identity {
        id: "i1".into(),
        provider: "phone".into(),
        provider_id: "+15555550100".into(),
        person_id: "p1".into(),
        verified_email: None,
    })
    .await
    .unwrap();
    let hits = repo
        .find_identities_by_verified_email("ada@example.com")
        .await
        .unwrap();
    assert!(hits.is_empty());
}

#[tokio::test]
async fn find_by_verified_email_returns_empty_for_unknown() {
    let repo = InMemoryRepository::default();
    let hits = repo
        .find_identities_by_verified_email("nobody@example.com")
        .await
        .unwrap();
    assert!(hits.is_empty());
}

// ── DynamoDB tests (gated behind DYNAMO_TEST=1) ───────────────────────────────

/// Returns `true` if DynamoDB tests should run.
fn dynamo_enabled() -> bool {
    std::env::var("DYNAMO_TEST").as_deref() == Ok("1")
}

#[tokio::test]
async fn dynamo_find_by_verified_email_returns_all_matches() {
    if !dynamo_enabled() {
        return;
    }
    use storage::DynamoRepository;

    std::env::set_var(
        "XPOOL_TABLE",
        std::env::var("XPOOL_TABLE").unwrap_or_else(|_| "xpool-test".to_owned()),
    );
    std::env::set_var(
        "CURRENT_TOURNAMENT_ID",
        std::env::var("CURRENT_TOURNAMENT_ID").unwrap_or_else(|_| "test".to_owned()),
    );

    let repo = DynamoRepository::from_env().await.expect("build repo");
    repo.ensure_table().await.expect("ensure_table");

    let pid = std::process::id();
    let email = format!("dynamo-lookup-{pid}@example.com");

    repo.put_identity(&Identity {
        id: format!("dynlookup-i1-{pid}"),
        provider: "email".into(),
        provider_id: email.clone(),
        person_id: format!("dl-p1-{pid}"),
        verified_email: Some(email.clone()),
    })
    .await
    .unwrap();
    repo.put_identity(&Identity {
        id: format!("dynlookup-i2-{pid}"),
        provider: "google".into(),
        provider_id: format!("g-dl-{pid}"),
        person_id: format!("dl-p1-{pid}"),
        verified_email: Some(email.clone()),
    })
    .await
    .unwrap();
    repo.put_identity(&Identity {
        id: format!("dynlookup-i3-{pid}"),
        provider: "email".into(),
        provider_id: format!("other-dl-{pid}@example.com"),
        person_id: format!("dl-p2-{pid}"),
        verified_email: Some(format!("other-dl-{pid}@example.com")),
    })
    .await
    .unwrap();

    let hits = repo
        .find_identities_by_verified_email(&email)
        .await
        .unwrap();
    assert_eq!(
        hits.len(),
        2,
        "expected 2 hits for {email}, got {}: {:?}",
        hits.len(),
        hits
    );
    let person_ids: std::collections::HashSet<_> =
        hits.iter().map(|i| i.person_id.clone()).collect();
    assert_eq!(
        person_ids,
        [format!("dl-p1-{pid}")]
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
    );
}

#[tokio::test]
async fn dynamo_find_by_verified_email_returns_empty_for_none() {
    if !dynamo_enabled() {
        return;
    }
    use storage::DynamoRepository;

    std::env::set_var(
        "XPOOL_TABLE",
        std::env::var("XPOOL_TABLE").unwrap_or_else(|_| "xpool-test".to_owned()),
    );
    std::env::set_var(
        "CURRENT_TOURNAMENT_ID",
        std::env::var("CURRENT_TOURNAMENT_ID").unwrap_or_else(|_| "test".to_owned()),
    );

    let repo = DynamoRepository::from_env().await.expect("build repo");
    repo.ensure_table().await.expect("ensure_table");

    let pid = std::process::id();
    let hits = repo
        .find_identities_by_verified_email(&format!("nobody-dl-{pid}@example.com"))
        .await
        .unwrap();
    assert!(
        hits.is_empty(),
        "expected no hits for unknown email, got {}: {:?}",
        hits.len(),
        hits
    );
}
