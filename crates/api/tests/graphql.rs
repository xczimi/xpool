//! Integration tests driving the GraphQL schema against an in-memory
//! repository — no DynamoDB. Covers submitGroup draft/lock, tips visibility,
//! enterResult → scoreboard recompute, and auth-required queries.

mod common;

use async_graphql::Variables;
use chrono::Duration;
use common::*;
use serde_json::json;

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
        "p": [{ "gameId": GAME_1, "homeScore": 1, "awayScore": 0 }],
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

#[tokio::test]
async fn set_motd_requires_admin_and_persists() {
    let repo = seeded_repo(Duration::hours(24)).await;
    // Non-admin rejected.
    let vars = Variables::from_json(json!({ "t": "hello" }));
    let bad = run(
        &repo,
        r#"mutation($t: String!) { setMotd(text: $t) }"#,
        vars.clone(),
        Some(ALICE),
    )
    .await;
    assert!(!bad.errors.is_empty());

    // Admin succeeds.
    let ok = run(
        &repo,
        r#"mutation($t: String!) { setMotd(text: $t) }"#,
        vars,
        Some(RESULT_ID),
    )
    .await;
    assert!(ok.errors.is_empty(), "{:?}", ok.errors);
    assert_eq!(repo.get_motd().await.unwrap().unwrap().text, "hello");
}
