//! Shared test fixtures: an in-memory repository preloaded with a tiny
//! two-game tournament, a result user, and two demo players.

use async_graphql::{Request, Variables};
use chrono::{Duration, Utc};
use domain::{
    GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame, Team, TeamSlot,
    Tournament,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use storage::{InMemoryRepository, Repository};

/// IDs used across tests.
pub const RESULT_ID: &str = "result-user";
pub const ALICE: &str = "alice";
pub const BOB: &str = "bob";
pub const GROUP_A: &str = "A";
pub const GAME_1: &str = "M1";
pub const GAME_2: &str = "M2";

fn team(id: &str) -> Team {
    Team {
        id: id.to_owned(),
        name: id.to_owned(),
        short_code: id.to_owned(),
        flag: None,
        external_id: None,
    }
}

/// A two-game group-stage tournament. `kickoff_offset` controls how far in
/// the future (or past) the games kick off.
pub fn tiny_tournament(kickoff_offset: Duration) -> Tournament {
    let kickoff = Utc::now() + kickoff_offset;
    let mut teams = HashMap::new();
    for t in ["MEX", "RSA", "KOR", "CZE"] {
        teams.insert(t.to_owned(), team(t));
    }

    let mk_game = |id: &str, home: &str, away: &str| SingleGame {
        id: id.to_owned(),
        kickoff,
        venue: None,
        group_id: GROUP_A.to_owned(),
        home: TeamSlot {
            team_id: Some(home.to_owned()),
            description: format!("{home}-slot"),
        },
        away: TeamSlot {
            team_id: Some(away.to_owned()),
            description: format!("{away}-slot"),
        },
    };
    let mut games = HashMap::new();
    games.insert(GAME_1.to_owned(), mk_game(GAME_1, "MEX", "RSA"));
    games.insert(GAME_2.to_owned(), mk_game(GAME_2, "KOR", "CZE"));

    let mut groups = HashMap::new();
    groups.insert(
        "ROOT".to_owned(),
        GroupGame {
            id: "ROOT".to_owned(),
            name: "Root".to_owned(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: false,
            children: GroupChildren::Groups(vec![GROUP_A.to_owned()]),
        },
    );
    groups.insert(
        GROUP_A.to_owned(),
        GroupGame {
            id: GROUP_A.to_owned(),
            name: "Group A".to_owned(),
            parent: Some("ROOT".to_owned()),
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: false,
            children: GroupChildren::Games(vec![GAME_1.to_owned(), GAME_2.to_owned()]),
        },
    );

    Tournament {
        root: "ROOT".to_owned(),
        groups,
        games,
        teams,
    }
}

/// Knockout game id used by the bracket-resolution tests. The match number
/// must be ≥ 73 for `fwc26` to treat it as a knockout game.
pub const GAME_KO: &str = "M73";
/// The wrapping one-match knockout group for `GAME_KO`.
pub const GROUP_KO: &str = "R32-1";
/// A downstream knockout game that pulls in the winner of `GAME_KO` — used to
/// observe which team the drawn `GAME_KO` advanced.
pub const GAME_KO_NEXT: &str = "M81";
/// The wrapping one-match knockout group for `GAME_KO_NEXT`.
pub const GROUP_KO_NEXT: &str = "R16-1";

/// `tiny_tournament` plus two one-match knockout groups: `GAME_KO` wraps a R32
/// match between the winner ("1A") and runner-up ("2A") of group A, and
/// `GAME_KO_NEXT` wraps a R16 match whose home slot is the winner of
/// `GAME_KO`. Used to exercise the drawn-knockout advancer write path
/// (Issue 24): the team `GAME_KO` advances surfaces as `GAME_KO_NEXT`'s home
/// team after recompute.
pub fn tournament_with_knockout(kickoff_offset: Duration) -> Tournament {
    let kickoff = Utc::now() + kickoff_offset;
    let mut t = tiny_tournament(kickoff_offset);

    let mk_ko_game = |id: &str, group_id: &str, home_desc: &str, away_desc: &str| SingleGame {
        id: id.to_owned(),
        kickoff,
        venue: None,
        group_id: group_id.to_owned(),
        home: TeamSlot {
            team_id: None,
            description: home_desc.to_owned(),
        },
        away: TeamSlot {
            team_id: None,
            description: away_desc.to_owned(),
        },
    };
    t.games
        .insert(GAME_KO.to_owned(), mk_ko_game(GAME_KO, GROUP_KO, "1A", "2A"));
    t.games.insert(
        GAME_KO_NEXT.to_owned(),
        mk_ko_game(GAME_KO_NEXT, GROUP_KO_NEXT, "Winner M73", "Loser M73"),
    );

    let mk_ko_group = |id: &str, name: &str, round: Round, game: &str| GroupGame {
        id: id.to_owned(),
        name: name.to_owned(),
        parent: Some("ROOT".to_owned()),
        round,
        lock_mode: LockMode::LockTogether,
        carries_standings: true,
        children: GroupChildren::Games(vec![game.to_owned()]),
    };
    t.groups.insert(
        GROUP_KO.to_owned(),
        mk_ko_group(GROUP_KO, "Round of 32 — Match 1", Round::R32, GAME_KO),
    );
    t.groups.insert(
        GROUP_KO_NEXT.to_owned(),
        mk_ko_group(GROUP_KO_NEXT, "Round of 16 — Match 1", Round::R16, GAME_KO_NEXT),
    );

    // Make the knockout groups children of ROOT so the tree stays connected.
    if let Some(root) = t.groups.get_mut("ROOT") {
        root.children = GroupChildren::Groups(vec![
            GROUP_A.to_owned(),
            GROUP_KO.to_owned(),
            GROUP_KO_NEXT.to_owned(),
        ]);
    }

    t
}

/// Build an in-memory repo seeded with `tournament_with_knockout` + 3 players.
pub async fn seeded_repo_with_knockout(kickoff_offset: Duration) -> Arc<dyn Repository> {
    let repo = InMemoryRepository::new();
    repo.put_tournament(&tournament_with_knockout(kickoff_offset))
        .await
        .unwrap();
    repo.put_player(&player(RESULT_ID, true)).await.unwrap();
    repo.put_player(&player(ALICE, false)).await.unwrap();
    repo.put_player(&player(BOB, false)).await.unwrap();
    Arc::new(repo)
}

fn player(id: &str, is_result: bool) -> Player {
    Player {
        id: id.to_owned(),
        person_id: format!("person-{id}"),
        nick: id.to_owned(),
        full_name: id.to_owned(),
        referrer: None,
        is_result_user: is_result,
        version: 0,
        match_predictions: Vec::new(),
        standings_predictions: Vec::new(),
    }
}

/// Build an in-memory repo seeded with the tiny tournament + 3 players.
pub async fn seeded_repo(kickoff_offset: Duration) -> Arc<dyn Repository> {
    let repo = InMemoryRepository::new();
    repo.put_tournament(&tiny_tournament(kickoff_offset))
        .await
        .unwrap();
    repo.put_player(&player(RESULT_ID, true)).await.unwrap();
    repo.put_player(&player(ALICE, false)).await.unwrap();
    repo.put_player(&player(BOB, false)).await.unwrap();
    Arc::new(repo)
}

/// A locked match prediction.
pub fn locked_pred(game_id: &str, home: u8, away: u8) -> MatchPrediction {
    MatchPrediction {
        game_id: game_id.to_owned(),
        home_score: home,
        away_score: away,
        locked: true,
    }
}

/// Execute a GraphQL request through the schema, optionally as a player
/// (sets the `CurrentPlayer` context like the router's auth seam would).
pub async fn run(
    repo: &Arc<dyn Repository>,
    query: &str,
    vars: Variables,
    as_player: Option<&str>,
) -> async_graphql::Response {
    run_at(repo, query, vars, as_player, Utc::now()).await
}

/// Like `run`, but with an explicit `now` injected into the GraphQL context.
pub async fn run_at(
    repo: &Arc<dyn Repository>,
    query: &str,
    vars: Variables,
    as_player: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> async_graphql::Response {
    use api::auth::CurrentPlayer;
    let schema = api::gql::build_schema(repo.clone());

    let current = match as_player {
        Some(id) => match repo.get_player(id).await.unwrap() {
            Some(p) => CurrentPlayer::Player(Box::new(p)),
            None => CurrentPlayer::Visitor,
        },
        None => CurrentPlayer::Visitor,
    };
    let req = Request::new(query)
        .variables(vars)
        .data(current)
        .data(api::clock::RequestNow(now));
    schema.execute(req).await
}

/// Like `run`, but the `CurrentPlayer` snapshot is supplied explicitly — used
/// to simulate a stale auth-context snapshot (wrong `version`) so the
/// optimistic-concurrency retry path can be exercised.
pub async fn run_with_snapshot(
    repo: &Arc<dyn Repository>,
    query: &str,
    vars: Variables,
    snapshot: Player,
) -> async_graphql::Response {
    use api::auth::CurrentPlayer;
    let schema = api::gql::build_schema(repo.clone());
    let req = Request::new(query)
        .variables(vars)
        .data(CurrentPlayer::Player(Box::new(snapshot)))
        .data(api::clock::RequestNow(Utc::now()));
    schema.execute(req).await
}

/// Serialise a GraphQL response's `data` to a `serde_json::Value`.
pub fn data(resp: &async_graphql::Response) -> Value {
    serde_json::to_value(&resp.data).unwrap()
}
