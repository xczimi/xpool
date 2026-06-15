//! Shared test fixtures: an in-memory repository preloaded with a tiny
//! two-game tournament, a result user, and two demo players.

use async_graphql::{Request, Variables};
use chrono::{Duration, Utc};
use domain::{
    GroupChildren, GroupGame, Identity, LockMode, MatchPrediction, Person, Player, Round,
    SingleGame, Team, TeamSlot, Tournament,
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
        external_id: None,
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
        external_id: None,
    };
    t.games.insert(
        GAME_KO.to_owned(),
        mk_ko_game(GAME_KO, GROUP_KO, "1A", "2A"),
    );
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
        mk_ko_group(
            GROUP_KO_NEXT,
            "Round of 16 — Match 1",
            Round::R16,
            GAME_KO_NEXT,
        ),
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
        // Test fixtures key Person/Identity at the bare id (see `seed_identity_for`
        // and the `person_id: ALICE` assertions), so the player's person_id must
        // match — otherwise login resolution (by `Player.person_id`) can't find it.
        person_id: id.to_owned(),
        nick: id.to_owned(),
        full_name: id.to_owned(),
        // ALICE is referred by the result-user → an "admin" who may create pools
        // (`may_create_pool`). BOB has no referrer, so he's a plain joiner whose
        // referrer gets set when he accepts an invite.
        referrer: (id == ALICE).then(|| RESULT_ID.to_owned()),
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

/// Build an axum router wired to a fresh in-memory repo (tiny tournament +
/// 3 players, games 24 h in the future). Used by HTTP-level tests such as
/// `tests/seam.rs` that need to drive the full request stack.
pub async fn test_app() -> (axum::Router, Arc<dyn Repository>) {
    let repo = seeded_repo(Duration::hours(24)).await;
    let app = api::build_app(repo.clone(), false, None);
    (app, repo)
}

/// Like `test_app`, but sets `LOCAL_AUTH_ISSUER=1` before building the router
/// so the trust-list accepts local-issuer JWTs minted by the helpers below.
/// Call this instead of `test_app()` from any test that uses `query_as` or
/// `query_with_bearer`.
pub async fn test_app_with_local_auth() -> (axum::Router, Arc<dyn Repository>) {
    // SAFETY: single-threaded test harness; the env mutation is visible to
    // `TrustList::from_env()` which is called synchronously inside build_app.
    unsafe {
        std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    }
    test_app().await
}

/// Mint a valid local-issuer Bearer token for `player_id` and return the
/// corresponding `Authorization` header name/value pair.
///
/// The caller must have built their router via `test_app_with_local_auth()`
/// (or otherwise set `LOCAL_AUTH_ISSUER` before constructing the router)
/// so the trust-list accepts the resulting token.
#[allow(dead_code)]
pub fn auth_header(player_id: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    let email = format!("{player_id}@dev.invalid");
    let token = api::auth::local_issuer::mint_for_test(player_id, &email);
    (
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    )
}

/// Drop-in helper: attach a local-issuer Bearer header for `player_id` to a
/// request builder. Intended for one-off requests in tests; prefer `query_as`
/// for full GraphQL round-trips.
#[allow(dead_code)]
pub fn with_player(
    builder: axum::http::request::Builder,
    player_id: &str,
) -> axum::http::request::Builder {
    let (name, value) = auth_header(player_id);
    builder.header(name, value)
}

/// POST a GraphQL body authenticated as `player_id`, return the parsed
/// response JSON. Used by mutation/query tests in Tasks 15-17.
///
/// The `app` must have been built via `test_app_with_local_auth()`.
#[allow(dead_code)]
pub async fn query_as(app: &axum::Router, player_id: &str, body: &str) -> serde_json::Value {
    let (name, value) = auth_header(player_id);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(name, value)
        .body(axum::body::Body::from(body.to_owned()))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
    let bytes = http_body_util::BodyExt::collect(res.into_body())
        .await
        .unwrap()
        .to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Seed an `Identity` + `Person` row for `player_id` so that a local-issuer
/// JWT minted with `mint_for_test(player_id, email)` resolves to that player
/// through the §3 algorithm. The identity is keyed at `("email", email)`.
///
/// Call this after `test_app_with_local_auth()` when a test needs an
/// authenticated HTTP request to land on a specific player.
#[allow(dead_code)]
pub async fn seed_identity_for(repo: &Arc<dyn Repository>, player_id: &str, email: &str) {
    let identity_id = format!("i-{player_id}");
    repo.put_identity(&Identity {
        id: identity_id.clone(),
        provider: "email".to_owned(),
        provider_id: email.to_owned(),
        person_id: player_id.to_owned(),
        verified_email: Some(email.to_owned()),
    })
    .await
    .unwrap();
    repo.put_person(&Person {
        id: player_id.to_owned(),
        identity_ids: vec![identity_id],
    })
    .await
    .unwrap();
}

/// Seed a **Google** Identity (+ its Person) for a player — keyed by the opaque
/// OAuth `sub`, not an e-mail. Mirrors a real federated login (and a pulled prod
/// player), so dev-login must reproduce the google connection to resolve it.
#[allow(dead_code)]
pub async fn seed_google_identity_for(
    repo: &Arc<dyn Repository>,
    player_id: &str,
    sub: &str,
    email: &str,
) {
    let identity_id = format!("i-google-{player_id}");
    repo.put_identity(&Identity {
        id: identity_id.clone(),
        provider: "google".to_owned(),
        provider_id: sub.to_owned(),
        person_id: player_id.to_owned(),
        verified_email: Some(email.to_owned()),
    })
    .await
    .unwrap();
    repo.put_person(&Person {
        id: player_id.to_owned(),
        identity_ids: vec![identity_id],
    })
    .await
    .unwrap();
}

/// Same as `query_as`, but with a pre-minted Bearer token. Used when testing
/// the unclaimed / claim flows for arbitrary subs where `player_id` is not
/// known ahead of time.
///
/// The `app` must have been built via `test_app_with_local_auth()`.
#[allow(dead_code)]
pub async fn query_with_bearer(app: &axum::Router, bearer: &str, body: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {bearer}"),
        )
        .body(axum::body::Body::from(body.to_owned()))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
    let bytes = http_body_util::BodyExt::collect(res.into_body())
        .await
        .unwrap()
        .to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
