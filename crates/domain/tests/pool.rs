//! Pure pool-operation functions (`SCENARIOS.md` POOL-02, -04, -05, -08,
//! -10, -12). No I/O — join-code generation and persistence are the API's job.

use domain::pool::{self, PoolError};
use domain::{Player, Pool};

fn player(id: &str, is_result_user: bool) -> Player {
    Player {
        id: id.to_owned(),
        person_id: format!("person-{id}"),
        nick: id.to_owned(),
        full_name: id.to_owned(),
        referrer: None,
        is_result_user,
        version: 0,
        match_predictions: vec![],
        standings_predictions: vec![],
    }
}

fn pool(owner: &str, members: &[&str]) -> Pool {
    Pool {
        id: "pool-1".to_owned(),
        name: "Friends".to_owned(),
        owner: owner.to_owned(),
        members: members.iter().map(|m| m.to_string()).collect(),
        join_code: "CODE1234".to_owned(),
    }
}

// ── join (POOL-02, POOL-12) ───────────────────────────────────────────────────

#[test]
fn join_adds_a_new_member() {
    let p = pool("alice", &["alice"]);
    let joined = pool::join(&p, &player("bob", false)).unwrap();
    assert_eq!(joined.members, vec!["alice", "bob"]);
}

#[test]
fn join_is_idempotent_for_an_existing_member() {
    let p = pool("alice", &["alice", "bob"]);
    let joined = pool::join(&p, &player("bob", false)).unwrap();
    assert_eq!(joined.members, vec!["alice", "bob"]);
}

#[test]
fn join_rejects_the_result_user() {
    let p = pool("alice", &["alice"]);
    let err = pool::join(&p, &player("result-user", true)).unwrap_err();
    assert_eq!(err, PoolError::ResultUserExcluded);
}

// ── leave (POOL-05, POOL-10) ──────────────────────────────────────────────────

#[test]
fn leave_removes_a_member() {
    let p = pool("alice", &["alice", "bob"]);
    let left = pool::leave(&p, "bob").unwrap();
    assert_eq!(left.members, vec!["alice"]);
}

#[test]
fn leave_rejects_the_owner() {
    let p = pool("alice", &["alice", "bob"]);
    let err = pool::leave(&p, "alice").unwrap_err();
    assert_eq!(err, PoolError::OwnerCannotLeave);
}

#[test]
fn leave_rejects_a_non_member() {
    let p = pool("alice", &["alice", "bob"]);
    let err = pool::leave(&p, "carol").unwrap_err();
    assert_eq!(err, PoolError::NotAMember);
}

// ── remove_member (POOL-04) ───────────────────────────────────────────────────

#[test]
fn remove_member_drops_a_member_when_requested_by_the_owner() {
    let p = pool("alice", &["alice", "bob"]);
    let updated = pool::remove_member(&p, "alice", "bob").unwrap();
    assert_eq!(updated.members, vec!["alice"]);
}

#[test]
fn remove_member_rejects_a_non_owner_requester() {
    let p = pool("alice", &["alice", "bob", "carol"]);
    let err = pool::remove_member(&p, "bob", "carol").unwrap_err();
    assert_eq!(err, PoolError::NotOwner);
}

#[test]
fn remove_member_rejects_removing_the_owner() {
    let p = pool("alice", &["alice", "bob"]);
    let err = pool::remove_member(&p, "alice", "alice").unwrap_err();
    assert_eq!(err, PoolError::OwnerCannotLeave);
}

// ── rename (POOL-08) ──────────────────────────────────────────────────────────

#[test]
fn rename_changes_the_name_for_the_owner() {
    let p = pool("alice", &["alice"]);
    let renamed = pool::rename(&p, "alice", "Office League".to_owned()).unwrap();
    assert_eq!(renamed.name, "Office League");
}

#[test]
fn rename_rejects_a_non_owner() {
    let p = pool("alice", &["alice", "bob"]);
    let err = pool::rename(&p, "bob", "Hijacked".to_owned()).unwrap_err();
    assert_eq!(err, PoolError::NotOwner);
}

// ── set_join_code (POOL-03) ───────────────────────────────────────────────────

#[test]
fn set_join_code_replaces_the_code_for_the_owner() {
    let p = pool("alice", &["alice"]);
    let rotated = pool::set_join_code(&p, "alice", "NEWCODE9".to_owned()).unwrap();
    assert_eq!(rotated.join_code, "NEWCODE9");
}

#[test]
fn set_join_code_rejects_a_non_owner() {
    let p = pool("alice", &["alice", "bob"]);
    let err = pool::set_join_code(&p, "bob", "NEWCODE9".to_owned()).unwrap_err();
    assert_eq!(err, PoolError::NotOwner);
}
