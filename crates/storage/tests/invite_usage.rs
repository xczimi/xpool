//! Tests for `Repository::claim_invite_code`.
//! The in-memory tests run unconditionally; the DynamoDB tests are gated
//! behind `DYNAMO_TEST=1` (same pattern as `tests/identity_lookup.rs`).

use storage::{InMemoryRepository, Repository};

// ── in-memory tests ───────────────────────────────────────────────────────────

#[tokio::test]
async fn claim_invite_code_succeeds_once_then_fails() {
    let repo = InMemoryRepository::default();
    assert!(repo.claim_invite_code("abc").await.unwrap());
    assert!(!repo.claim_invite_code("abc").await.unwrap());
}

#[tokio::test]
async fn claim_invite_code_distinguishes_different_codes() {
    let repo = InMemoryRepository::default();
    assert!(repo.claim_invite_code("abc").await.unwrap());
    assert!(repo.claim_invite_code("xyz").await.unwrap());
}

#[tokio::test]
async fn claim_invite_code_fresh_repo_always_succeeds() {
    let repo = InMemoryRepository::default();
    assert!(repo.claim_invite_code("unique-code-1").await.unwrap());
}

// ── DynamoDB tests (gated behind DYNAMO_TEST=1) ───────────────────────────────

/// Returns `true` if DynamoDB tests should run.
fn dynamo_enabled() -> bool {
    std::env::var("DYNAMO_TEST").as_deref() == Ok("1")
}

#[tokio::test]
async fn dynamo_claim_invite_code_succeeds_once_then_fails() {
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
    let code = format!("dyntest-claim-{pid}");

    assert!(
        repo.claim_invite_code(&code).await.unwrap(),
        "first claim should succeed"
    );
    assert!(
        !repo.claim_invite_code(&code).await.unwrap(),
        "second claim of same code should fail"
    );
}

#[tokio::test]
async fn dynamo_claim_invite_code_distinguishes_different_codes() {
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
    let code_a = format!("dyntest-codeA-{pid}");
    let code_b = format!("dyntest-codeB-{pid}");

    assert!(
        repo.claim_invite_code(&code_a).await.unwrap(),
        "first code should be claimable"
    );
    assert!(
        repo.claim_invite_code(&code_b).await.unwrap(),
        "second distinct code should also be claimable"
    );
}
