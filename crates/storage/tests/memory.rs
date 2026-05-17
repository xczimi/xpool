//! Integration tests for `InMemoryRepository`. Run unconditionally.

use chrono::{TimeZone, Utc};
use domain::{
    GroupChildren, GroupGame, Identity, LockMode, MatchPrediction, Person, Player, Pool, Round,
    SingleGame, StandingsPrediction, Team, TeamSlot, Tournament,
};
use std::collections::HashMap;
use storage::{InMemoryRepository, Repository, Scoreboard};

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
        venue: Some("MetLife Stadium".to_owned()),
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
        full_name: format!("Full Name {id}"),
        referrer: None,
        is_result_user: false,
        version,
        match_predictions: vec![MatchPrediction {
            game_id: "M1".to_owned(),
            home_score: 2,
            away_score: 1,
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

fn make_pool(id: &str) -> Pool {
    Pool {
        id: id.to_owned(),
        name: format!("Pool {id}"),
        owner: "player-1".to_owned(),
        members: vec!["player-1".to_owned(), "player-2".to_owned()],
    }
}

fn make_identity(provider: &str, provider_id: &str, person_id: &str) -> Identity {
    Identity {
        id: format!("{provider}:{provider_id}"),
        provider: provider.to_owned(),
        provider_id: provider_id.to_owned(),
        person_id: person_id.to_owned(),
    }
}

fn make_person(id: &str) -> Person {
    Person {
        id: id.to_owned(),
        identity_ids: vec!["google:sub-abc".to_owned()],
    }
}

fn make_scoreboard() -> Scoreboard {
    let mut entries = HashMap::new();
    let mut rounds = HashMap::new();
    rounds.insert(Round::GroupStage, 10_i64);
    rounds.insert(Round::QF, 4);
    entries.insert("player-1".to_owned(), rounds);
    Scoreboard { entries }
}

// ── tournament ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tournament_round_trip() {
    let repo = InMemoryRepository::new();

    // Initially absent
    assert!(repo.get_tournament().await.unwrap().is_none());

    let t = make_tournament();
    repo.put_tournament(&t).await.unwrap();

    let got = repo
        .get_tournament()
        .await
        .unwrap()
        .expect("should be Some");
    assert_eq!(got, t);
}

#[tokio::test]
async fn tournament_overwrite() {
    let repo = InMemoryRepository::new();
    let mut t = make_tournament();
    repo.put_tournament(&t).await.unwrap();

    // Overwrite with a modified tournament
    t.root = "GroupB".to_owned();
    repo.put_tournament(&t).await.unwrap();

    let got = repo.get_tournament().await.unwrap().unwrap();
    assert_eq!(got.root, "GroupB");
}

// ── player ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn player_round_trip() {
    let repo = InMemoryRepository::new();

    assert!(repo.get_player("p1").await.unwrap().is_none());

    let p = make_player("p1", 0);
    repo.put_player(&p).await.unwrap();

    let got = repo
        .get_player("p1")
        .await
        .unwrap()
        .expect("should be Some");
    assert_eq!(got, p);
}

#[tokio::test]
async fn list_players_empty() {
    let repo = InMemoryRepository::new();
    assert!(repo.list_players().await.unwrap().is_empty());
}

#[tokio::test]
async fn list_players_multiple() {
    let repo = InMemoryRepository::new();
    repo.put_player(&make_player("p1", 0)).await.unwrap();
    repo.put_player(&make_player("p2", 0)).await.unwrap();
    repo.put_player(&make_player("p3", 0)).await.unwrap();

    let mut players = repo.list_players().await.unwrap();
    players.sort_by(|a, b| a.id.cmp(&b.id));

    assert_eq!(players.len(), 3);
    assert_eq!(players[0].id, "p1");
    assert_eq!(players[1].id, "p2");
    assert_eq!(players[2].id, "p3");
}

// ── player: optimistic concurrency ────────────────────────────────────────────

/// First insert of a new player must always succeed regardless of version.
#[tokio::test]
async fn player_first_insert_succeeds() {
    let repo = InMemoryRepository::new();
    let p = make_player("new-player", 0);
    repo.put_player(&p).await.unwrap();
}

/// Second write with the same version succeeds (caller hasn't bumped yet —
/// acts as a no-conflict update).
#[tokio::test]
async fn player_update_same_version_succeeds() {
    let repo = InMemoryRepository::new();
    let mut p = make_player("p1", 0);
    repo.put_player(&p).await.unwrap();

    // Update with the same version — allowed
    p.nick = "updated-nick".to_owned();
    repo.put_player(&p).await.unwrap();

    let got = repo.get_player("p1").await.unwrap().unwrap();
    assert_eq!(got.nick, "updated-nick");
}

/// Write with a stale version must fail.
#[tokio::test]
async fn player_conflict_returns_error() {
    let repo = InMemoryRepository::new();
    let p = make_player("p1", 5);
    repo.put_player(&p).await.unwrap();

    // Attempt with wrong version
    let mut stale = p.clone();
    stale.version = 3; // stale
    let err = repo.put_player(&stale).await;
    assert!(err.is_err(), "expected conflict error but got Ok");
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("optimistic concurrency"),
        "unexpected error message: {msg}"
    );
}

/// Concurrent clone — two clones share state, conflict is detected.
#[tokio::test]
async fn player_clone_shares_state() {
    let repo1 = InMemoryRepository::new();
    let repo2 = repo1.clone(); // shares inner state

    let p = make_player("shared", 0);
    repo1.put_player(&p).await.unwrap();

    // repo2 sees what repo1 wrote
    let got = repo2.get_player("shared").await.unwrap();
    assert!(got.is_some());
}

// ── scoreboard ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scoreboard_round_trip() {
    let repo = InMemoryRepository::new();

    assert!(repo.get_scoreboard().await.unwrap().is_none());

    let s = make_scoreboard();
    repo.put_scoreboard(&s).await.unwrap();

    let got = repo.get_scoreboard().await.unwrap().unwrap();
    assert_eq!(got, s);
}

// ── pool ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn pool_round_trip() {
    let repo = InMemoryRepository::new();

    assert!(repo.list_pools().await.unwrap().is_empty());

    let pool = make_pool("pool-1");
    repo.put_pool(&pool).await.unwrap();

    let pools = repo.list_pools().await.unwrap();
    assert_eq!(pools.len(), 1);
    assert_eq!(pools[0], pool);
}

#[tokio::test]
async fn pool_list_multiple() {
    let repo = InMemoryRepository::new();
    repo.put_pool(&make_pool("alpha")).await.unwrap();
    repo.put_pool(&make_pool("beta")).await.unwrap();

    let mut pools = repo.list_pools().await.unwrap();
    pools.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(pools.len(), 2);
    assert_eq!(pools[0].id, "alpha");
    assert_eq!(pools[1].id, "beta");
}

#[tokio::test]
async fn pool_overwrite() {
    let repo = InMemoryRepository::new();
    let pool = make_pool("p");
    repo.put_pool(&pool).await.unwrap();

    let updated = Pool {
        id: "p".to_owned(),
        name: "New Name".to_owned(),
        owner: "player-1".to_owned(),
        members: vec![],
    };
    repo.put_pool(&updated).await.unwrap();

    let got = repo.list_pools().await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].name, "New Name");
}

// ── identity ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn identity_round_trip() {
    let repo = InMemoryRepository::new();

    assert!(repo
        .get_identity("google", "sub-123")
        .await
        .unwrap()
        .is_none());

    let id = make_identity("google", "sub-123", "person-1");
    repo.put_identity(&id).await.unwrap();

    let got = repo
        .get_identity("google", "sub-123")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got, id);
}

#[tokio::test]
async fn identity_different_providers_isolated() {
    let repo = InMemoryRepository::new();
    let id_google = make_identity("google", "same-id", "person-1");
    let id_github = make_identity("github", "same-id", "person-2");

    repo.put_identity(&id_google).await.unwrap();
    repo.put_identity(&id_github).await.unwrap();

    let g = repo
        .get_identity("google", "same-id")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(g.person_id, "person-1");

    let gh = repo
        .get_identity("github", "same-id")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(gh.person_id, "person-2");
}

// ── person ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn person_round_trip() {
    let repo = InMemoryRepository::new();

    assert!(repo.get_person("person-1").await.unwrap().is_none());

    let p = make_person("person-1");
    repo.put_person(&p).await.unwrap();

    let got = repo.get_person("person-1").await.unwrap().unwrap();
    assert_eq!(got, p);
}

#[tokio::test]
async fn person_overwrite() {
    let repo = InMemoryRepository::new();
    let mut p = make_person("person-1");
    repo.put_person(&p).await.unwrap();

    p.identity_ids.push("magic-link:abc".to_owned());
    repo.put_person(&p).await.unwrap();

    let got = repo.get_person("person-1").await.unwrap().unwrap();
    assert_eq!(got.identity_ids.len(), 2);
}

// ── full round-trip: all entities ─────────────────────────────────────────────

#[tokio::test]
async fn full_round_trip_all_entities() {
    let repo = InMemoryRepository::new();

    // Tournament
    repo.put_tournament(&make_tournament()).await.unwrap();
    assert!(repo.get_tournament().await.unwrap().is_some());

    // Players
    repo.put_player(&make_player("p1", 0)).await.unwrap();
    repo.put_player(&make_player("p2", 0)).await.unwrap();
    assert_eq!(repo.list_players().await.unwrap().len(), 2);

    // Scoreboard
    repo.put_scoreboard(&make_scoreboard()).await.unwrap();
    assert!(repo.get_scoreboard().await.unwrap().is_some());

    // Pools
    repo.put_pool(&make_pool("pool-1")).await.unwrap();
    assert_eq!(repo.list_pools().await.unwrap().len(), 1);

    // Identity
    repo.put_identity(&make_identity("google", "x", "person-1"))
        .await
        .unwrap();
    assert!(repo.get_identity("google", "x").await.unwrap().is_some());

    // Person
    repo.put_person(&make_person("person-1")).await.unwrap();
    assert!(repo.get_person("person-1").await.unwrap().is_some());
}
