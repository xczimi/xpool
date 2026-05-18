//! Integration tests driving the GraphQL schema against an in-memory
//! repository — no DynamoDB. Covers submitGroup draft/lock, tips visibility,
//! enterResult → scoreboard recompute, and auth-required queries.

mod common;

use async_graphql::Variables;
use chrono::{Duration, Utc};
use common::*;
use serde_json::json;
use storage::Repository;

// ── Auth-required queries ────────────────────────────────────────────────────

#[tokio::test]
async fn me_requires_authentication() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(&repo, "{ me { id } }", Variables::default(), None).await;
    assert!(!resp.errors.is_empty(), "visitor must get an auth error");
    assert!(resp.errors[0].message.contains("authentication required"));
}

#[tokio::test]
async fn me_returns_player_when_authenticated() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        "{ me { id nick } }",
        Variables::default(),
        Some(ALICE),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    assert_eq!(
        data(&resp),
        json!({ "me": { "id": "alice", "nick": "alice" } })
    );
}

#[tokio::test]
async fn tournament_query_is_public() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        "{ tournament { root games { id } } }",
        Variables::default(),
        None,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    assert_eq!(d["tournament"]["root"], "ROOT");
    assert_eq!(d["tournament"]["games"].as_array().unwrap().len(), 2);
}

// ── submitGroup: draft and lock ──────────────────────────────────────────────

const SUBMIT: &str = r#"
mutation($g: ID!, $p: [MatchPredictionInput!]!, $lock: Boolean!) {
  submitGroup(groupId: $g, predictions: $p, lock: $lock) {
    id version matchPredictions { gameId homeScore awayScore locked }
  }
}"#;

#[tokio::test]
async fn submit_group_saves_draft() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [
            { "gameId": GAME_1, "homeScore": 2, "awayScore": 1 },
            { "gameId": GAME_2, "homeScore": 0, "awayScore": 0 }
        ],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let preds = d["submitGroup"]["matchPredictions"].as_array().unwrap();
    assert_eq!(preds.len(), 2);
    assert!(
        preds.iter().all(|p| p["locked"] == json!(false)),
        "draft is unlocked"
    );

    // Persisted onto the player item.
    let stored = repo.get_player(ALICE).await.unwrap().unwrap();
    assert_eq!(stored.match_predictions.len(), 2);
}

#[tokio::test]
async fn submit_group_locks_predictions() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [
            { "gameId": GAME_1, "homeScore": 3, "awayScore": 0 },
            { "gameId": GAME_2, "homeScore": 1, "awayScore": 1 }
        ],
        "lock": true
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let stored = repo.get_player(ALICE).await.unwrap().unwrap();
    assert!(
        stored.match_predictions.iter().all(|p| p.locked),
        "all locked"
    );
}

#[tokio::test]
async fn submit_group_resolves_version_conflict_with_retry() {
    // The storage OCC guard rejects a write whose `version` does not match
    // what is stored. Here the auth-context snapshot carries a stale
    // `version` (99) while the stored player is at version 0 — the first
    // write fails the guard, `submitGroup` re-reads the real player, and the
    // retry succeeds.
    let repo = seeded_repo(Duration::hours(24)).await;
    let stale_snapshot = {
        let mut p = repo.get_player(ALICE).await.unwrap().unwrap();
        p.version = 99; // stale — does not match stored version 0
        p
    };

    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 1, "awayScore": 0 }],
        "lock": false
    }));
    let resp = run_with_snapshot(&repo, SUBMIT, vars, stale_snapshot).await;
    assert!(
        resp.errors.is_empty(),
        "retry should succeed: {:?}",
        resp.errors
    );

    let stored = repo.get_player(ALICE).await.unwrap().unwrap();
    assert_eq!(
        stored.match_predictions.len(),
        1,
        "the retry persisted the prediction"
    );
}

#[tokio::test]
async fn submit_group_saves_standings() {
    let repo = seeded_repo(Duration::hours(24)).await;
    const SUBMIT_STANDINGS: &str = r#"
mutation($g: ID!, $p: [MatchPredictionInput!]!, $s: StandingsInput, $lock: Boolean!) {
  submitGroup(groupId: $g, predictions: $p, standings: $s, lock: $lock) {
    id standingsPredictions { groupId ordering drawOrder locked }
  }
}"#;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        // lock: true requires every game in the group (PRED-03).
        "p": [
            { "gameId": GAME_1, "homeScore": 1, "awayScore": 0 },
            { "gameId": GAME_2, "homeScore": 2, "awayScore": 2 }
        ],
        "s": { "ordering": ["MEX", "RSA", "KOR", "CZE"], "drawOrder": ["KOR", "CZE"] },
        "lock": true
    }));
    let resp = run(&repo, SUBMIT_STANDINGS, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let standings = d["submitGroup"]["standingsPredictions"].as_array().unwrap();
    assert_eq!(standings.len(), 1);
    assert_eq!(standings[0]["groupId"], GROUP_A);
    assert_eq!(
        standings[0]["ordering"],
        json!(["MEX", "RSA", "KOR", "CZE"])
    );
    assert_eq!(standings[0]["drawOrder"], json!(["KOR", "CZE"]));
    assert_eq!(standings[0]["locked"], json!(true));

    // Persisted onto the player item.
    let stored = repo.get_player(ALICE).await.unwrap().unwrap();
    assert_eq!(stored.standings_predictions.len(), 1);
    assert_eq!(stored.standings_predictions[0].group_id, GROUP_A);
}

#[tokio::test]
async fn submit_group_without_standings_leaves_them_empty() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 1, "awayScore": 0 }],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let stored = repo.get_player(ALICE).await.unwrap().unwrap();
    assert!(stored.standings_predictions.is_empty());
}

#[tokio::test]
async fn submit_group_requires_authentication() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 1, "awayScore": 0 }],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, None).await;
    assert!(!resp.errors.is_empty());
}

// ── Issue 01: submitGroup deadline / locking-is-final enforcement ─────────────

#[tokio::test]
async fn submit_group_rejected_after_deadline() {
    // Group A kicked off 2h ago — its deadline has passed.
    let repo = seeded_repo(Duration::hours(-2)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 1, "awayScore": 0 }],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(!resp.errors.is_empty(), "post-deadline submit must be rejected");
    assert!(
        resp.errors[0].message.contains("deadline"),
        "{:?}",
        resp.errors
    );
}

#[tokio::test]
async fn submit_group_rejected_when_overwriting_a_locked_prediction() {
    let repo = seeded_repo(Duration::hours(24)).await;
    // Alice already has a locked prediction for GAME_1.
    {
        let mut alice = repo.get_player(ALICE).await.unwrap().unwrap();
        alice.match_predictions.push(locked_pred(GAME_1, 2, 1));
        repo.put_player(&alice).await.unwrap();
    }
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [
            { "gameId": GAME_1, "homeScore": 3, "awayScore": 3 },
            { "gameId": GAME_2, "homeScore": 0, "awayScore": 0 }
        ],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(
        !resp.errors.is_empty(),
        "overwriting a locked prediction must be rejected"
    );
    assert!(resp.errors[0].message.contains("locked"), "{:?}", resp.errors);
    // The locked prediction is untouched.
    let stored = repo.get_player(ALICE).await.unwrap().unwrap();
    let g1 = stored.match_prediction(GAME_1).unwrap();
    assert_eq!((g1.home_score, g1.away_score), (2, 1));
}

// ── Issue 06: lock: true requires every game in the group ────────────────────

#[tokio::test]
async fn submit_group_lock_rejected_with_missing_games() {
    let repo = seeded_repo(Duration::hours(24)).await;
    // Group A has GAME_1 and GAME_2; supply only GAME_1 with lock: true.
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 1, "awayScore": 0 }],
        "lock": true
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(
        !resp.errors.is_empty(),
        "partial lock must be rejected (PRED-03)"
    );
}

#[tokio::test]
async fn submit_group_lock_succeeds_with_all_games() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [
            { "gameId": GAME_1, "homeScore": 1, "awayScore": 0 },
            { "gameId": GAME_2, "homeScore": 2, "awayScore": 2 }
        ],
        "lock": true
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
}

// ── Issue 15: out-of-range scores are rejected, not clamped ──────────────────

#[tokio::test]
async fn submit_group_rejects_negative_score() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": -3, "awayScore": 0 }],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(!resp.errors.is_empty(), "negative score must be rejected");
}

#[tokio::test]
async fn submit_group_rejects_oversized_score() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 999, "awayScore": 0 }],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, Some(ALICE)).await;
    assert!(!resp.errors.is_empty(), "oversized score must be rejected");
}

#[tokio::test]
async fn enter_result_rejects_out_of_range_score() {
    let repo = seeded_repo(Duration::hours(-2)).await;
    let vars = Variables::from_json(json!({ "g": GAME_1, "h": -1, "a": 0, "lock": true }));
    let resp = run(&repo, ENTER_RESULT, vars, Some(RESULT_ID)).await;
    assert!(!resp.errors.is_empty(), "negative result score must be rejected");
}

// ── tips: visibility filtering (UC-9 / API.md §6) ────────────────────────────

const TIPS: &str = r#"
query($g: ID!) {
  tips(groupId: $g) {
    playerId gameId prediction { homeScore awayScore }
  }
}"#;

#[tokio::test]
async fn tips_hides_unlocked_predictions_before_kickoff() {
    // Kickoff is in the future, deadline not passed.
    let repo = seeded_repo(Duration::hours(24)).await;
    // Bob has an unlocked prediction.
    {
        let mut bob = repo.get_player(BOB).await.unwrap().unwrap();
        bob.match_predictions.push(locked_pred(GAME_1, 2, 1));
        bob.match_predictions.last_mut().unwrap().locked = false;
        repo.put_player(&bob).await.unwrap();
    }

    // Alice views the tips; Bob's unlocked prediction must be hidden.
    let vars = Variables::from_json(json!({ "g": GROUP_A }));
    let resp = run(&repo, TIPS, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let tips = data(&resp);
    let bob_g1 = tips["tips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["playerId"] == "bob" && t["gameId"] == GAME_1)
        .unwrap()
        .clone();
    assert_eq!(bob_g1["prediction"], json!(null), "hidden from others");
}

#[tokio::test]
async fn tips_reveals_locked_predictions() {
    let repo = seeded_repo(Duration::hours(24)).await;
    {
        let mut bob = repo.get_player(BOB).await.unwrap().unwrap();
        bob.match_predictions.push(locked_pred(GAME_1, 2, 1)); // locked
        repo.put_player(&bob).await.unwrap();
    }
    let vars = Variables::from_json(json!({ "g": GROUP_A }));
    let resp = run(&repo, TIPS, vars, Some(ALICE)).await;
    let tips = data(&resp);
    let bob_g1 = tips["tips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["playerId"] == "bob" && t["gameId"] == GAME_1)
        .unwrap()
        .clone();
    assert_eq!(
        bob_g1["prediction"],
        json!({ "homeScore": 2, "awayScore": 1 })
    );
}

#[tokio::test]
async fn tips_reveals_after_kickoff_even_if_unlocked() {
    // Kickoff is in the past.
    let repo = seeded_repo(Duration::hours(-2)).await;
    {
        let mut bob = repo.get_player(BOB).await.unwrap().unwrap();
        let mut pred = locked_pred(GAME_1, 4, 2);
        pred.locked = false; // unlocked but the match kicked off
        bob.match_predictions.push(pred);
        repo.put_player(&bob).await.unwrap();
    }
    let vars = Variables::from_json(json!({ "g": GROUP_A }));
    let resp = run(&repo, TIPS, vars, Some(ALICE)).await;
    let tips = data(&resp);
    let bob_g1 = tips["tips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["playerId"] == "bob" && t["gameId"] == GAME_1)
        .unwrap()
        .clone();
    assert_eq!(
        bob_g1["prediction"],
        json!({ "homeScore": 4, "awayScore": 2 })
    );
}

#[tokio::test]
async fn tips_always_shows_own_unlocked_prediction() {
    let repo = seeded_repo(Duration::hours(24)).await;
    {
        let mut alice = repo.get_player(ALICE).await.unwrap().unwrap();
        let mut pred = locked_pred(GAME_1, 1, 1);
        pred.locked = false;
        alice.match_predictions.push(pred);
        repo.put_player(&alice).await.unwrap();
    }
    let vars = Variables::from_json(json!({ "g": GROUP_A }));
    let resp = run(&repo, TIPS, vars, Some(ALICE)).await;
    let tips = data(&resp);
    let own = tips["tips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["playerId"] == "alice" && t["gameId"] == GAME_1)
        .unwrap()
        .clone();
    assert_eq!(own["prediction"], json!({ "homeScore": 1, "awayScore": 1 }));
}

// ── enterResult → scoreboard recompute ───────────────────────────────────────

const ENTER_RESULT: &str = r#"
mutation($g: ID!, $h: Int!, $a: Int!, $lock: Boolean!) {
  enterResult(gameId: $g, homeScore: $h, awayScore: $a, lock: $lock)
}"#;

#[tokio::test]
async fn enter_result_requires_admin() {
    let repo = seeded_repo(Duration::hours(-2)).await;
    let vars = Variables::from_json(json!({ "g": GAME_1, "h": 2, "a": 1, "lock": true }));
    let resp = run(&repo, ENTER_RESULT, vars, Some(ALICE)).await;
    assert!(!resp.errors.is_empty(), "non-admin must be rejected");
    assert!(resp.errors[0].message.contains("admin"));
}

#[tokio::test]
async fn enter_result_recomputes_scoreboard() {
    // Past kickoff so a deadline-driven effective-lock applies to Alice's
    // unlocked-but-complete prediction.
    let repo = seeded_repo(Duration::hours(-2)).await;

    // Alice predicts M1 = 2-1 (locked), so a result of 2-1 = a perfect (4 pts).
    {
        let mut alice = repo.get_player(ALICE).await.unwrap().unwrap();
        alice.match_predictions.push(locked_pred(GAME_1, 2, 1));
        repo.put_player(&alice).await.unwrap();
    }

    // Admin enters M1 = 2-1 and locks it.
    let vars = Variables::from_json(json!({ "g": GAME_1, "h": 2, "a": 1, "lock": true }));
    let resp = run(&repo, ENTER_RESULT, vars, Some(RESULT_ID)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);

    // Scoreboard was materialised: Alice scored 4 in the group stage.
    let board = repo
        .get_scoreboard()
        .await
        .unwrap()
        .expect("scoreboard written");
    let alice_scores = board.entries.get(ALICE).expect("Alice on scoreboard");
    let total: i64 = alice_scores.values().sum();
    assert_eq!(total, 4, "exact 2-1 prediction = 4 points");

    // The result user is never scored against itself.
    assert!(!board.entries.contains_key(RESULT_ID));
}

#[tokio::test]
async fn scoreboard_query_reflects_recompute() {
    let repo = seeded_repo(Duration::hours(-2)).await;
    {
        let mut alice = repo.get_player(ALICE).await.unwrap().unwrap();
        alice.match_predictions.push(locked_pred(GAME_1, 1, 0));
        repo.put_player(&alice).await.unwrap();
    }
    // Admin enters a result that gives Alice the correct outcome only (2 pts).
    let vars = Variables::from_json(json!({ "g": GAME_1, "h": 3, "a": 0, "lock": true }));
    run(&repo, ENTER_RESULT, vars, Some(RESULT_ID)).await;

    let resp = run(
        &repo,
        "{ scoreboard { playerId total } }",
        Variables::default(),
        None,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let alice_row = d["scoreboard"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["playerId"] == "alice")
        .unwrap()
        .clone();
    // 1-0 vs 3-0: home exact? no. away exact (0==0)? yes (+1). outcome (home win) yes (+2). = 3.
    assert_eq!(alice_row["total"], 3);
}

// ── results query ────────────────────────────────────────────────────────────

#[tokio::test]
async fn results_returns_only_locked_result_user_predictions() {
    let repo = seeded_repo(Duration::hours(-2)).await;
    // Result user has one locked and one unlocked prediction.
    {
        let mut result = repo.get_player(RESULT_ID).await.unwrap().unwrap();
        result.match_predictions.push(locked_pred(GAME_1, 2, 1)); // locked
        let mut draft = locked_pred(GAME_2, 0, 0);
        draft.locked = false; // unlocked
        result.match_predictions.push(draft);
        repo.put_player(&result).await.unwrap();
    }
    let resp = run(
        &repo,
        "{ results { gameId homeScore awayScore locked } }",
        Variables::default(),
        None,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let results = d["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "only the locked prediction is returned");
    assert_eq!(results[0]["gameId"], GAME_1);
    assert_eq!(results[0]["homeScore"], 2);
    assert_eq!(results[0]["locked"], json!(true));
}

#[tokio::test]
async fn results_is_empty_when_no_results_entered() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(&repo, "{ results { gameId } }", Variables::default(), None).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    assert_eq!(data(&resp), json!({ "results": [] }));
}

// ── players query ────────────────────────────────────────────────────────────

#[tokio::test]
async fn players_query_returns_all_players_including_result_user() {
    // The dev-login picker (web/src/components/AuthBar.tsx) depends on this
    // query; a schema mismatch here is exactly the class of bug E2E exists to
    // catch. The repo is seeded with the result user, Alice, and Bob.
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        "{ players { id nick fullName isResultUser } }",
        Variables::default(),
        None,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);

    let d = data(&resp);
    let players = d["players"].as_array().unwrap();
    assert_eq!(players.len(), 3, "result user + Alice + Bob");

    let ids: Vec<&str> = players
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&ALICE), "Alice listed: {ids:?}");
    assert!(ids.contains(&BOB), "Bob listed: {ids:?}");
    assert!(ids.contains(&RESULT_ID), "result user listed: {ids:?}");

    // The result user is flagged and sorted last (real players first).
    let result_row = players
        .iter()
        .find(|p| p["id"] == RESULT_ID)
        .expect("result user present");
    assert_eq!(result_row["isResultUser"], json!(true));
    assert_eq!(
        players.last().unwrap()["id"],
        json!(RESULT_ID),
        "result user sorts after real players"
    );

    // Real players are not flagged as the result user.
    let alice_row = players.iter().find(|p| p["id"] == ALICE).unwrap();
    assert_eq!(alice_row["isResultUser"], json!(false));
}

// ── tournament: group deadline ───────────────────────────────────────────────

#[tokio::test]
async fn tournament_group_carries_subtree_deadline() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        "{ tournament { groups { id deadline } } }",
        Variables::default(),
        None,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let group_a = d["tournament"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == "A")
        .unwrap()
        .clone();
    assert!(
        group_a["deadline"].is_string(),
        "leaf group has a deadline: {group_a:?}"
    );
}

// ── Pools (SCENARIOS.md §5) ──────────────────────────────────────────────────

const CREATE_POOL: &str = r#"
mutation($id: ID!, $name: String!) {
  createPool(id: $id, name: $name) { id name owner members joinCode }
}"#;

/// Create a pool as `actor` and return its join code.
async fn make_pool(repo: &std::sync::Arc<dyn Repository>, id: &str, actor: &str) -> String {
    let vars = Variables::from_json(json!({ "id": id, "name": "Friends" }));
    let resp = run(repo, CREATE_POOL, vars, Some(actor)).await;
    assert!(resp.errors.is_empty(), "createPool failed: {:?}", resp.errors);
    data(&resp)["createPool"]["joinCode"]
        .as_str()
        .expect("joinCode string")
        .to_owned()
}

#[tokio::test]
async fn create_pool_sets_owner_membership_and_a_join_code() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({ "id": "p1", "name": "Friends" }));
    let resp = run(&repo, CREATE_POOL, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let pool = &data(&resp)["createPool"];
    assert_eq!(pool["owner"], json!(ALICE));
    assert_eq!(pool["members"], json!([ALICE]));
    assert!(
        !pool["joinCode"].as_str().unwrap().is_empty(),
        "a join code is generated"
    );
}

#[tokio::test]
async fn create_pool_rejected_for_the_result_user() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({ "id": "p1", "name": "Friends" }));
    let resp = run(&repo, CREATE_POOL, vars, Some(RESULT_ID)).await;
    assert!(!resp.errors.is_empty(), "result user must be rejected");
}

#[tokio::test]
async fn join_pool_adds_the_caller_via_the_join_code() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let code = make_pool(&repo, "p1", ALICE).await;
    let vars = Variables::from_json(json!({ "code": code }));
    let resp = run(
        &repo,
        r#"mutation($code: String!) { joinPool(joinCode: $code) { members } }"#,
        vars,
        Some(BOB),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    assert_eq!(data(&resp)["joinPool"]["members"], json!([ALICE, BOB]));
}

#[tokio::test]
async fn join_pool_rejects_an_unknown_code() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let vars = Variables::from_json(json!({ "code": "NOSUCHCODE" }));
    let resp = run(
        &repo,
        r#"mutation($code: String!) { joinPool(joinCode: $code) { id } }"#,
        vars,
        Some(BOB),
    )
    .await;
    assert!(!resp.errors.is_empty(), "unknown code must be rejected");
}

#[tokio::test]
async fn leave_pool_removes_the_caller() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let code = make_pool(&repo, "p1", ALICE).await;
    run(
        &repo,
        r#"mutation($code: String!) { joinPool(joinCode: $code) { id } }"#,
        Variables::from_json(json!({ "code": code })),
        Some(BOB),
    )
    .await;
    let resp = run(
        &repo,
        r#"mutation($id: ID!) { leavePool(id: $id) { members } }"#,
        Variables::from_json(json!({ "id": "p1" })),
        Some(BOB),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    assert_eq!(data(&resp)["leavePool"]["members"], json!([ALICE]));
}

#[tokio::test]
async fn leave_pool_rejects_the_owner() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"mutation($id: ID!) { leavePool(id: $id) { id } }"#,
        Variables::from_json(json!({ "id": "p1" })),
        Some(ALICE),
    )
    .await;
    assert!(!resp.errors.is_empty(), "owner cannot leave");
}

#[tokio::test]
async fn remove_member_lets_the_owner_drop_a_member() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let code = make_pool(&repo, "p1", ALICE).await;
    run(
        &repo,
        r#"mutation($code: String!) { joinPool(joinCode: $code) { id } }"#,
        Variables::from_json(json!({ "code": code })),
        Some(BOB),
    )
    .await;
    let resp = run(
        &repo,
        r#"mutation($p: ID!, $m: ID!) { removeMember(poolId: $p, memberId: $m) { members } }"#,
        Variables::from_json(json!({ "p": "p1", "m": BOB })),
        Some(ALICE),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    assert_eq!(data(&resp)["removeMember"]["members"], json!([ALICE]));
}

#[tokio::test]
async fn remove_member_rejected_for_a_non_owner() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let code = make_pool(&repo, "p1", ALICE).await;
    run(
        &repo,
        r#"mutation($code: String!) { joinPool(joinCode: $code) { id } }"#,
        Variables::from_json(json!({ "code": code })),
        Some(BOB),
    )
    .await;
    let resp = run(
        &repo,
        r#"mutation($p: ID!, $m: ID!) { removeMember(poolId: $p, memberId: $m) { id } }"#,
        Variables::from_json(json!({ "p": "p1", "m": ALICE })),
        Some(BOB),
    )
    .await;
    assert!(!resp.errors.is_empty(), "non-owner cannot remove members");
}

#[tokio::test]
async fn rotate_join_code_changes_the_code_for_the_owner() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let code = make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"mutation($id: ID!) { rotateJoinCode(id: $id) { joinCode } }"#,
        Variables::from_json(json!({ "id": "p1" })),
        Some(ALICE),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let new_code = data(&resp)["rotateJoinCode"]["joinCode"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(new_code, code, "the code must change");
    assert!(!new_code.is_empty());
}

#[tokio::test]
async fn rotate_join_code_rejected_for_a_non_owner() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"mutation($id: ID!) { rotateJoinCode(id: $id) { id } }"#,
        Variables::from_json(json!({ "id": "p1" })),
        Some(BOB),
    )
    .await;
    assert!(!resp.errors.is_empty(), "non-owner cannot rotate the code");
}

#[tokio::test]
async fn delete_pool_removes_it_for_the_owner() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"mutation($id: ID!) { deletePool(id: $id) }"#,
        Variables::from_json(json!({ "id": "p1" })),
        Some(ALICE),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    assert_eq!(data(&resp)["deletePool"], json!(true));
    assert!(repo.list_pools().await.unwrap().is_empty(), "pool is gone");
}

#[tokio::test]
async fn delete_pool_rejected_for_a_non_owner() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"mutation($id: ID!) { deletePool(id: $id) }"#,
        Variables::from_json(json!({ "id": "p1" })),
        Some(BOB),
    )
    .await;
    assert!(!resp.errors.is_empty(), "non-owner cannot delete the pool");
    assert_eq!(repo.list_pools().await.unwrap().len(), 1, "pool survives");
}

#[tokio::test]
async fn update_pool_renames_for_the_owner() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"mutation($id: ID!, $n: String!) { updatePool(id: $id, name: $n) { name } }"#,
        Variables::from_json(json!({ "id": "p1", "n": "Office League" })),
        Some(ALICE),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    assert_eq!(data(&resp)["updatePool"]["name"], json!("Office League"));
}

#[tokio::test]
async fn pools_query_returns_only_the_callers_pools() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "alice-pool", ALICE).await;
    make_pool(&repo, "bob-pool", BOB).await;
    let resp = run(&repo, "{ pools { id } }", Variables::default(), Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let ids = d["pools"].as_array().unwrap();
    assert_eq!(ids.len(), 1, "only Alice's own pool");
    assert_eq!(ids[0]["id"], json!("alice-pool"));
}

// ── tips visibility: clock-driven (UC-9 / clock seam) ────────────────────────

#[tokio::test]
async fn tips_visibility_uses_the_request_clock() {
    // A tournament whose only group kicks off 24h in the future.
    let repo = seeded_repo(Duration::hours(24)).await;
    // Bob saved an unlocked draft.
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 1, "awayScore": 0 }],
        "lock": false
    }));
    run(&repo, SUBMIT, vars, Some(BOB)).await;

    // Viewed by ALICE *before* kickoff -> Bob's draft is hidden.
    let before = run_at(
        &repo,
        r#"query($g: ID!){ tips(groupId:$g){ playerId gameId prediction{ homeScore } } }"#,
        Variables::from_json(json!({ "g": GROUP_A })),
        Some(ALICE),
        Utc::now(),
    )
    .await;
    let bob_before = find_tip(&before, BOB, GAME_1);
    assert!(bob_before["prediction"].is_null(), "hidden before kickoff");

    // Viewed with the clock advanced past kickoff -> Bob's draft is revealed.
    let after = run_at(
        &repo,
        r#"query($g: ID!){ tips(groupId:$g){ playerId gameId prediction{ homeScore } } }"#,
        Variables::from_json(json!({ "g": GROUP_A })),
        Some(ALICE),
        Utc::now() + Duration::hours(48),
    )
    .await;
    let bob_after = find_tip(&after, BOB, GAME_1);
    assert_eq!(bob_after["prediction"]["homeScore"], json!(1), "revealed after kickoff");
}

#[tokio::test]
async fn tournament_exposes_time_flags_against_the_request_clock() {
    // Group A kicks off 24h in the future.
    let repo = seeded_repo(Duration::hours(24)).await;
    let q = r#"{
      now
      tournament {
        groups { id deadlinePassed }
        games { id resultPending withinTodayWindow }
      }
    }"#;

    // Clock = real now -> deadline not passed, nothing result-pending,
    // the games (24h out) are within the ±2-day Today window.
    let early = run_at(&repo, q, Variables::default(), None, Utc::now()).await;
    assert!(early.errors.is_empty(), "{:?}", early.errors);
    let d = data(&early);
    let group_a = d["tournament"]["groups"].as_array().unwrap()
        .iter().find(|g| g["id"] == json!(GROUP_A)).unwrap();
    assert_eq!(group_a["deadlinePassed"], json!(false));
    let game = &d["tournament"]["games"].as_array().unwrap()[0];
    assert_eq!(game["resultPending"], json!(false));
    assert_eq!(game["withinTodayWindow"], json!(true));

    // Clock = 10 days later -> deadline passed, results pending, games are
    // now well outside the Today window.
    let late = run_at(&repo, q, Variables::default(), None, Utc::now() + Duration::days(10)).await;
    let d = data(&late);
    let group_a = d["tournament"]["groups"].as_array().unwrap()
        .iter().find(|g| g["id"] == json!(GROUP_A)).unwrap();
    assert_eq!(group_a["deadlinePassed"], json!(true));
    let game = &d["tournament"]["games"].as_array().unwrap()[0];
    assert_eq!(game["resultPending"], json!(true));
    assert_eq!(game["withinTodayWindow"], json!(false));
}

// ── Issue 04: scoreboard(pool) pool-privacy ──────────────────────────────────

#[tokio::test]
async fn scoreboard_pool_filter_rejects_a_non_member() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    // Bob is not a member of p1.
    let resp = run(
        &repo,
        r#"query($p: ID!) { scoreboard(pool: $p) { playerId } }"#,
        Variables::from_json(json!({ "p": "p1" })),
        Some(BOB),
    )
    .await;
    assert!(
        !resp.errors.is_empty(),
        "non-member must not read a pool's scoreboard"
    );
}

#[tokio::test]
async fn scoreboard_pool_filter_rejects_a_visitor() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"query($p: ID!) { scoreboard(pool: $p) { playerId } }"#,
        Variables::from_json(json!({ "p": "p1" })),
        None,
    )
    .await;
    assert!(!resp.errors.is_empty(), "visitor must not read a pool's scoreboard");
}

#[tokio::test]
async fn scoreboard_pool_filter_succeeds_for_a_member() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    let resp = run(
        &repo,
        r#"query($p: ID!) { scoreboard(pool: $p) { playerId } }"#,
        Variables::from_json(json!({ "p": "p1" })),
        Some(ALICE),
    )
    .await;
    assert!(resp.errors.is_empty(), "member may read: {:?}", resp.errors);
}

#[tokio::test]
async fn scoreboard_global_stays_public() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(&repo, "{ scoreboard { playerId } }", Variables::default(), None).await;
    assert!(resp.errors.is_empty(), "global scoreboard is public: {:?}", resp.errors);
}

// ── Issue 05: invite authorization ───────────────────────────────────────────

const INVITE: &str = r#"mutation($id: ID!) { invite(inviteeId: $id) }"#;

#[tokio::test]
async fn invite_rejects_self_target() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        INVITE,
        Variables::from_json(json!({ "id": ALICE })),
        Some(ALICE),
    )
    .await;
    assert!(!resp.errors.is_empty(), "self-invite must be rejected");
}

#[tokio::test]
async fn invite_rejects_an_already_referred_player() {
    let repo = seeded_repo(Duration::hours(24)).await;
    // Alice invites Bob — succeeds.
    let first = run(
        &repo,
        INVITE,
        Variables::from_json(json!({ "id": BOB })),
        Some(ALICE),
    )
    .await;
    assert!(first.errors.is_empty(), "first invite: {:?}", first.errors);
    // The result user tries to re-invite Bob — rejected.
    let second = run(
        &repo,
        INVITE,
        Variables::from_json(json!({ "id": BOB })),
        Some(RESULT_ID),
    )
    .await;
    assert!(
        !second.errors.is_empty(),
        "re-inviting an already-referred player must be rejected"
    );
    // Bob's original referrer is intact.
    let bob = repo.get_player(BOB).await.unwrap().unwrap();
    assert_eq!(bob.referrer.as_deref(), Some(ALICE));
}

// ── Issue 16: create_pool id collision ───────────────────────────────────────

#[tokio::test]
async fn create_pool_rejects_a_duplicate_id() {
    let repo = seeded_repo(Duration::hours(24)).await;
    make_pool(&repo, "p1", ALICE).await;
    // Bob tries to create a pool with the same id.
    let vars = Variables::from_json(json!({ "id": "p1", "name": "Hijack" }));
    let resp = run(&repo, CREATE_POOL, vars, Some(BOB)).await;
    assert!(
        !resp.errors.is_empty(),
        "creating a pool with an existing id must be rejected"
    );
    // The original pool is untouched.
    let pools = repo.list_pools().await.unwrap();
    assert_eq!(pools.len(), 1);
    assert_eq!(pools[0].owner, ALICE);
}

// ── Issue 17: updateProfile validation ───────────────────────────────────────

const UPDATE_PROFILE: &str = r#"
mutation($n: String, $f: String) {
  updateProfile(nick: $n, fullName: $f) { id nick fullName }
}"#;

#[tokio::test]
async fn update_profile_rejects_empty_nick() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        UPDATE_PROFILE,
        Variables::from_json(json!({ "n": "" })),
        Some(ALICE),
    )
    .await;
    assert!(!resp.errors.is_empty(), "empty nick must be rejected");
}

#[tokio::test]
async fn update_profile_rejects_whitespace_nick() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        UPDATE_PROFILE,
        Variables::from_json(json!({ "n": "   " })),
        Some(ALICE),
    )
    .await;
    assert!(!resp.errors.is_empty(), "whitespace-only nick must be rejected");
}

#[tokio::test]
async fn update_profile_rejects_oversized_nick() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        UPDATE_PROFILE,
        Variables::from_json(json!({ "n": "x".repeat(200) })),
        Some(ALICE),
    )
    .await;
    assert!(!resp.errors.is_empty(), "oversized nick must be rejected");
}

#[tokio::test]
async fn update_profile_accepts_a_valid_nick() {
    let repo = seeded_repo(Duration::hours(24)).await;
    let resp = run(
        &repo,
        UPDATE_PROFILE,
        Variables::from_json(json!({ "n": "  Alice2  ", "f": "Alice Anderson" })),
        Some(ALICE),
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    // Stored trimmed.
    assert_eq!(d["updateProfile"]["nick"], json!("Alice2"));
    assert_eq!(d["updateProfile"]["fullName"], json!("Alice Anderson"));
}

/// Find a tip entry from a `tips` query response matching `player_id` and `game_id`.
fn find_tip(resp: &async_graphql::Response, player_id: &str, game_id: &str) -> serde_json::Value {
    data(resp)["tips"]
        .as_array()
        .expect("tips array")
        .iter()
        .find(|t| t["playerId"] == player_id && t["gameId"] == game_id)
        .unwrap_or_else(|| panic!("no tip found for player={player_id} game={game_id}"))
        .clone()
}

