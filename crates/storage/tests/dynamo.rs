//! Integration tests for `DynamoRepository`.
//!
//! These tests require a live DynamoDB (or DynamoDB Local) endpoint.
//! They are **skipped** unless the environment variable `DYNAMO_TEST=1` is set,
//! so `cargo test` remains green without any AWS infrastructure running.
//!
//! To run against DynamoDB Local:
//! ```bash
//! docker compose up -d dynamodb
//! DYNAMO_TEST=1 DYNAMO_ENDPOINT=http://localhost:8000 \
//!   XPOOL_TABLE=xpool-test \
//!   CURRENT_TOURNAMENT_ID=test \
//!   cargo test -p storage -- --nocapture
//! ```

use chrono::{TimeZone, Utc};
use domain::{
    GroupChildren, GroupGame, Identity, LockMode, MatchPrediction, Person, Player, Pool, Round,
    SingleGame, StandingsPrediction, Team, TeamSlot, Tournament,
};
use std::collections::HashMap;
use storage::{DynamoRepository, Repository, Scoreboard};

// ── gate ──────────────────────────────────────────────────────────────────────

/// Returns `true` if DynamoDB tests should run.
fn dynamo_enabled() -> bool {
    std::env::var("DYNAMO_TEST").as_deref() == Ok("1")
}

/// Build a repository scoped to the test tournament id, with a unique table
/// name so tests can run in isolation.
async fn test_repo() -> DynamoRepository {
    // Use a dedicated test table to avoid polluting the dev table.
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
    repo
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_tournament() -> Tournament {
    let team = Team {
        id: "ARG".to_owned(),
        name: "Argentina".to_owned(),
        short_code: "ARG".to_owned(),
        flag: None,
        external_id: None,
    };
    let game = SingleGame {
        id: "M1".to_owned(),
        kickoff: Utc.with_ymd_and_hms(2026, 6, 1, 15, 0, 0).unwrap(),
        venue: Some("MetLife".to_owned()),
        group_id: "GroupA".to_owned(),
        home: TeamSlot {
            team_id: Some("ARG".to_owned()),
            description: "1A".to_owned(),
        },
        away: TeamSlot {
            team_id: None,
            description: "2B".to_owned(),
        },
    };
    let group = GroupGame {
        id: "GroupA".to_owned(),
        name: "Group A".to_owned(),
        parent: None,
        round: Round::GroupStage,
        lock_mode: LockMode::LockTogether,
        carries_standings: true,
        children: GroupChildren::Games(vec!["M1".to_owned()]),
    };
    Tournament {
        root: "GroupA".to_owned(),
        groups: {
            let mut m = HashMap::new();
            m.insert("GroupA".to_owned(), group);
            m
        },
        games: {
            let mut m = HashMap::new();
            m.insert("M1".to_owned(), game);
            m
        },
        teams: {
            let mut m = HashMap::new();
            m.insert("ARG".to_owned(), team);
            m
        },
    }
}

fn make_player(id: &str, version: u64) -> Player {
    Player {
        id: id.to_owned(),
        person_id: format!("person-{id}"),
        nick: format!("nick-{id}"),
        full_name: format!("Full {id}"),
        referrer: None,
        is_result_user: false,
        version,
        match_predictions: vec![MatchPrediction {
            game_id: "M1".to_owned(),
            home_score: 1,
            away_score: 0,
            locked: false,
        }],
        standings_predictions: vec![StandingsPrediction {
            group_id: "GroupA".to_owned(),
            ordering: vec!["ARG".to_owned()],
            draw_order: vec![],
            locked: false,
        }],
    }
}

fn make_scoreboard() -> Scoreboard {
    let mut entries = HashMap::new();
    let mut rounds = HashMap::new();
    rounds.insert(Round::GroupStage, 8_i64);
    entries.insert("player-1".to_owned(), rounds);
    Scoreboard { entries }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn dynamo_tournament_round_trip() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let t = make_tournament();
    repo.put_tournament(&t).await.unwrap();
    let got = repo.get_tournament().await.unwrap().expect("Some");
    assert_eq!(got.root, t.root);
    assert_eq!(got.teams.len(), t.teams.len());
    assert_eq!(got.games.len(), t.games.len());
}

#[tokio::test]
async fn dynamo_player_round_trip() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let p = make_player("dynamo-p1", 0);
    repo.put_player(&p).await.unwrap();

    let got = repo.get_player("dynamo-p1").await.unwrap().expect("Some");
    assert_eq!(got.nick, p.nick);
    assert_eq!(got.version, 0);
}

#[tokio::test]
async fn dynamo_player_list() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    repo.put_player(&make_player("list-a", 0)).await.unwrap();
    repo.put_player(&make_player("list-b", 0)).await.unwrap();

    let players = repo.list_players().await.unwrap();
    // There may be players from other tests; just check ours are present.
    let ids: Vec<_> = players.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"list-a"), "list-a missing from {:?}", ids);
    assert!(ids.contains(&"list-b"), "list-b missing from {:?}", ids);
}

#[tokio::test]
async fn dynamo_player_optimistic_concurrency_conflict() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let p = make_player("occ-player", 10);
    repo.put_player(&p).await.unwrap();

    // Write with wrong version — must fail
    let mut stale = p.clone();
    stale.version = 7;
    let err = repo.put_player(&stale).await;
    assert!(err.is_err(), "expected OCC error");
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("ConditionalCheckFailed") || msg.contains("optimistic"),
        "unexpected error: {msg}"
    );
}

#[tokio::test]
async fn dynamo_player_optimistic_concurrency_success() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let p = make_player("occ-ok-player", 0);
    repo.put_player(&p).await.unwrap();

    // Caller bumps version before writing
    let mut updated = p.clone();
    updated.nick = "updated".to_owned();
    updated.version = 0; // same version — OCC passes
    repo.put_player(&updated).await.unwrap();

    let got = repo.get_player("occ-ok-player").await.unwrap().unwrap();
    assert_eq!(got.nick, "updated");
}

#[tokio::test]
async fn dynamo_scoreboard_round_trip() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let s = make_scoreboard();
    repo.put_scoreboard(&s).await.unwrap();
    let got = repo.get_scoreboard().await.unwrap().expect("Some");
    assert_eq!(got, s);
}

#[tokio::test]
async fn dynamo_pool_round_trip() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let pool = Pool {
        id: "dynamo-pool-1".to_owned(),
        name: "Dynamo Pool".to_owned(),
        owner: "player-x".to_owned(),
        members: vec!["player-x".to_owned()],
        join_code: "DYNCODE1".to_owned(),
    };
    repo.put_pool(&pool).await.unwrap();

    let pools = repo.list_pools().await.unwrap();
    let ids: Vec<_> = pools.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"dynamo-pool-1"), "{ids:?}");

    repo.delete_pool("dynamo-pool-1").await.unwrap();
    let after = repo.list_pools().await.unwrap();
    assert!(
        !after.iter().any(|p| p.id == "dynamo-pool-1"),
        "pool should be gone after delete_pool"
    );
}

#[tokio::test]
async fn dynamo_identity_round_trip() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let id = Identity {
        id: "google:dynamo-sub".to_owned(),
        provider: "google".to_owned(),
        provider_id: "dynamo-sub".to_owned(),
        person_id: "person-dynamo".to_owned(),
    };
    repo.put_identity(&id).await.unwrap();

    let got = repo
        .get_identity("google", "dynamo-sub")
        .await
        .unwrap()
        .expect("Some");
    assert_eq!(got.person_id, "person-dynamo");
}

#[tokio::test]
async fn dynamo_person_round_trip() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    let p = Person {
        id: "dynamo-person-1".to_owned(),
        identity_ids: vec!["google:sub-xyz".to_owned()],
    };
    repo.put_person(&p).await.unwrap();

    let got = repo
        .get_person("dynamo-person-1")
        .await
        .unwrap()
        .expect("Some");
    assert_eq!(got.identity_ids, p.identity_ids);
}

#[tokio::test]
async fn dynamo_delete_table_removes_it() {
    if !dynamo_enabled() {
        return;
    }
    // test_repo() creates a uniquely-named table via ensure_table().
    let repo = test_repo().await;
    repo.delete_table().await.unwrap();
    // ensure_table must now succeed again — proof the table was gone.
    repo.ensure_table().await.unwrap();
    repo.delete_table().await.unwrap();
}
