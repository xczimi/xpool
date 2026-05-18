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

/// Build a repository scoped to the shared test tournament id and the shared
/// `xpool-test` table. Most tests use this — they isolate themselves by using
/// unique player/pool/person ids, so they can share one table safely.
///
/// Tests that mutate the table itself (e.g. `delete_table`) must NOT use this;
/// they should call [`unique_table_repo`] for a private table.
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

/// Build a repository backed by a freshly-named table, so a test that creates
/// or deletes the table itself cannot interfere with tests running in
/// parallel against the shared `xpool-test` table.
async fn unique_table_repo(suffix: &str) -> DynamoRepository {
    let base = test_repo().await;
    DynamoRepository {
        client: base.client.clone(),
        table: format!("xpool-test-{suffix}-{}", std::process::id()),
        tournament_id: base.tournament_id.clone(),
    }
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
    // The repository owns the version counter: a first write of a version-0
    // player is stored at version 1.
    assert_eq!(got.version, 1);
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

/// `list_players` must paginate: a single DynamoDB `Query` returns at most
/// 1 MB, so a player set larger than that spans multiple pages. Using a
/// dedicated tournament namespace isolates the query from other tests. Each
/// player carries a large `referrer` padding so ~60 players exceed the 1 MB
/// page limit; if the pagination loop were missing, `list_players` would
/// return a truncated list and the assertion would fail.
#[tokio::test]
async fn dynamo_list_players_paginates_past_one_page() {
    if !dynamo_enabled() {
        return;
    }
    let base = test_repo().await;
    // A repository scoped to a unique tournament namespace.
    let repo = DynamoRepository {
        client: base.client.clone(),
        table: base.table.clone(),
        tournament_id: format!("paginate-{}", std::process::id()),
    };

    // ~20 KB of padding per player; 60 players ≈ 1.2 MB > one 1 MB page.
    let padding = "x".repeat(20_000);
    let count = 60;
    for i in 0..count {
        let mut p = make_player(&format!("page-{i:03}"), 0);
        p.referrer = Some(padding.clone());
        repo.put_player(&p).await.unwrap();
    }

    let players = repo.list_players().await.unwrap();
    assert_eq!(
        players.len(),
        count,
        "list_players truncated the result — Query pagination loop did not run"
    );
}

/// `list_players` queries a single `<t>#PLAYER` partition, so it must return
/// only the players of its own tournament namespace — never another
/// tournament's players, and never Persons/Identities/Tournaments/Pools.
#[tokio::test]
async fn dynamo_list_players_is_namespace_isolated() {
    if !dynamo_enabled() {
        return;
    }
    let base = test_repo().await;
    let pid = std::process::id();

    let repo_a = DynamoRepository {
        client: base.client.clone(),
        table: base.table.clone(),
        tournament_id: format!("iso-a-{pid}"),
    };
    let repo_b = DynamoRepository {
        client: base.client.clone(),
        table: base.table.clone(),
        tournament_id: format!("iso-b-{pid}"),
    };

    repo_a.put_player(&make_player("a-only", 0)).await.unwrap();
    repo_b.put_player(&make_player("b-only", 0)).await.unwrap();
    // A pool in namespace A shares the tournament prefix but a different
    // partition — it must not leak into list_players.
    repo_a
        .put_pool(&Pool {
            id: "a-pool".to_owned(),
            name: "A Pool".to_owned(),
            owner: "a-only".to_owned(),
            members: vec!["a-only".to_owned()],
            join_code: format!("ISOCODE{pid}"),
        })
        .await
        .unwrap();

    let a_players = repo_a.list_players().await.unwrap();
    let a_ids: Vec<_> = a_players.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        a_ids,
        vec!["a-only"],
        "namespace A's list_players leaked items: {a_ids:?}"
    );

    let b_players = repo_b.list_players().await.unwrap();
    let b_ids: Vec<_> = b_players.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        b_ids,
        vec!["b-only"],
        "namespace B's list_players leaked items: {b_ids:?}"
    );
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

    // Re-read to obtain the repository-bumped version, then update.
    let mut updated = repo.get_player("occ-ok-player").await.unwrap().unwrap();
    assert_eq!(updated.version, 1);
    updated.nick = "updated".to_owned();
    repo.put_player(&updated).await.unwrap();

    let got = repo.get_player("occ-ok-player").await.unwrap().unwrap();
    assert_eq!(got.nick, "updated");
    assert_eq!(got.version, 2);
}

/// Two writers that both read the same base version race: the first wins and
/// the repository bumps the stored version; the second's stale write is
/// rejected with a version-conflict error.
#[tokio::test]
async fn dynamo_player_concurrent_writes_second_conflicts() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    repo.put_player(&make_player("occ-race-player", 0))
        .await
        .unwrap();

    // Both writers read the same base state.
    let base = repo.get_player("occ-race-player").await.unwrap().unwrap();
    let mut writer_a = base.clone();
    writer_a.nick = "from-a".to_owned();
    let mut writer_b = base.clone();
    writer_b.nick = "from-b".to_owned();

    // First write wins.
    repo.put_player(&writer_a).await.unwrap();
    // Second write read a now-stale version — must fail.
    let err = repo.put_player(&writer_b).await;
    assert!(err.is_err(), "expected version-conflict error");
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("ConditionalCheckFailed") || msg.contains("optimistic"),
        "unexpected error: {msg}"
    );

    let got = repo.get_player("occ-race-player").await.unwrap().unwrap();
    assert_eq!(got.nick, "from-a");
}

/// The new-vs-update decision is condition-driven: a single atomic conditional
/// `put_item` (`attribute_not_exists(pk) OR #ver = :v`) replaces the old
/// get-then-branch design. A second insert of an already-existing player id
/// with a fresh version-0 player finds the item present (so
/// `attribute_not_exists(pk)` fails) and a stale version (so `#ver = :v` fails)
/// — both clauses fail and the write is rejected.
#[tokio::test]
async fn dynamo_player_second_insert_of_existing_id_conflicts() {
    if !dynamo_enabled() {
        return;
    }
    let repo = test_repo().await;

    // First insert of a brand-new player succeeds (stored at version 1).
    repo.put_player(&make_player("reinsert-player", 0))
        .await
        .unwrap();

    // A second "insert" of the same id — a fresh version-0 player, as if the
    // caller never read the existing row — must be rejected: the item exists
    // and version 0 does not match the stored version 1.
    let mut second = make_player("reinsert-player", 0);
    second.nick = "second-insert".to_owned();
    let err = repo.put_player(&second).await;
    assert!(
        err.is_err(),
        "second insert of an existing player id must conflict"
    );
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("ConditionalCheckFailed") || msg.contains("optimistic"),
        "unexpected error: {msg}"
    );

    // The original write is intact — the rejected insert did not overwrite it.
    let got = repo.get_player("reinsert-player").await.unwrap().unwrap();
    assert_eq!(got.nick, "nick-reinsert-player");
    assert_eq!(got.version, 1);
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
    // Use a uniquely-named table so deleting it cannot pull the shared
    // `xpool-test` table out from under tests running in parallel.
    let repo = unique_table_repo("delete").await;
    repo.ensure_table().await.unwrap();
    repo.delete_table().await.unwrap();
    // ensure_table must now succeed again — proof the table was gone.
    repo.ensure_table().await.unwrap();
    repo.delete_table().await.unwrap();
}
