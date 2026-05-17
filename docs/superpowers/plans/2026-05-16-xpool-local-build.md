# xpool Local Build — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> or superpowers:executing-plans. Each subsystem is built TDD (red-green-refactor).
> Steps use checkbox (`- [ ]`) syntax.

**Goal:** Build the full xpool soccer-prediction app, runnable entirely locally per `.specs/DEPLOYMENT.md` §4 (docker compose + axum server + Vite SPA + seed importer).

**Architecture:** Rust workspace (5 crates) + React/Vite SPA. `domain` is a pure, I/O-free crate (entities + scoring engine). `fwc26` holds FIFA-WC-26-specific logic (Annexe C, bracket resolution). `storage` is the `Repository` trait with an in-memory fake and a DynamoDB adapter. `api` is an axum + async-graphql server. `xtask` is the seed/import CLI. The SPA is a urql GraphQL client.

**Tech Stack:** Rust 1.95, axum, async-graphql, aws-sdk-dynamodb, tokio, serde; React 18 + Vite + TypeScript + urql + react-router; DynamoDB Local + MailHog via docker compose.

---

## Locked contracts

These are fixed so parallel subagents do not diverge. Subagents MUST NOT change
signatures without updating this plan.

### Crate layout

```
xpool/
  Cargo.toml              # workspace
  crates/domain/          # pure: entities, scoring, standings ladder
  crates/fwc26/           # FWC26 logic: Annexe C, bracket resolution
  crates/storage/         # Repository trait, InMemoryRepository, DynamoRepository
  crates/api/             # axum + async-graphql server + lambda_http wrapper
  crates/xtask/           # import/seed CLI
  tournaments/fwc26.json  # tournament definition (104 matches, 48 teams)
  web/                    # React + Vite SPA
  docker-compose.yml      # DynamoDB Local + MailHog
```

Workspace `Cargo.toml` lists all 5 crates under `crates/*`, `resolver = "2"`.

### `domain` crate — entities (`crates/domain/src/model.rs`)

Single-tournament: no `tournament_id` anywhere. All `id` fields are `String`
newtypes via a `id!` macro or plain `String` aliases. Derive
`Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize`.

```rust
pub type TeamId = String;
pub type GroupId = String;
pub type GameId = String;
pub type PlayerId = String;

pub struct Team { pub id: TeamId, pub name: String, pub short_code: String,
                  pub flag: Option<String>, pub external_id: Option<String> }

pub struct TeamSlot { pub team_id: Option<TeamId>, pub description: String }

pub struct SingleGame { pub id: GameId, pub kickoff: chrono::DateTime<chrono::Utc>,
                        pub venue: Option<String>, pub group_id: GroupId,
                        pub home: TeamSlot, pub away: TeamSlot }

#[derive(Copy)]
pub enum Round { GroupStage, R32, R16, QF, SF, ThirdPlace, Final }

#[derive(Copy)]
pub enum LockMode { LockTogether, LockPerMatch }

pub enum GroupChildren { Groups(Vec<GroupId>), Games(Vec<GameId>) }

pub struct GroupGame { pub id: GroupId, pub name: String,
                       pub parent: Option<GroupId>, pub round: Round,
                       pub lock_mode: LockMode, pub carries_standings: bool,
                       pub children: GroupChildren }

pub struct Tournament { pub root: GroupId,
                        pub groups: std::collections::HashMap<GroupId, GroupGame>,
                        pub games: std::collections::HashMap<GameId, SingleGame>,
                        pub teams: std::collections::HashMap<TeamId, Team> }

pub struct MatchPrediction { pub game_id: GameId, pub home_score: u8,
                             pub away_score: u8, pub locked: bool }

pub struct StandingsPrediction { pub group_id: GroupId, pub ordering: Vec<TeamId>,
                                 pub draw_order: Vec<TeamId>, pub locked: bool }

pub struct Player { pub id: PlayerId, pub person_id: String, pub nick: String,
                    pub full_name: String, pub referrer: Option<PlayerId>,
                    pub is_result_user: bool, pub version: u64,
                    pub match_predictions: Vec<MatchPrediction>,
                    pub standings_predictions: Vec<StandingsPrediction> }

pub struct Person { pub id: String, pub identity_ids: Vec<String> }
pub struct Identity { pub id: String, pub provider: String,
                      pub provider_id: String, pub person_id: String }
pub struct Pool { pub id: String, pub name: String, pub owner: PlayerId,
                  pub members: Vec<PlayerId> }
pub struct Motd { pub text: String }
```

### `domain` crate — scoring (`crates/domain/src/scoring.rs`)

```rust
pub struct ScoringConfig {
    pub exact_score_point: i64,        // 1
    pub outcome_point: i64,            // 2
    pub high_scoring_threshold: u8,    // 4
    pub standings_pair_point: i64,     // 1
    pub perfect_threshold: i64,        // 4
    pub multiplier: fn(Round) -> i64,  // Group 1,R32 2,R16 3,QF 4,SF 5,ThirdPlace 5,Final 6
}
impl Default for ScoringConfig { /* seeded values */ }

/// Per-match: P vs R, both 90-minute scores. Max 4.
pub fn score_match(p: &MatchPrediction, r: &MatchPrediction, c: &ScoringConfig) -> i64;

/// True iff `score_match == perfect_threshold`.
pub fn is_perfect(p: &MatchPrediction, r: &MatchPrediction, c: &ScoringConfig) -> bool;

/// effective-locked: locked OR (now > deadline AND prediction complete).
pub fn effective_locked(locked: bool, now: DateTime<Utc>, deadline: DateTime<Utc>,
                        complete: bool) -> bool;

/// Standings ladder (SCORING.md §4) — rank teams from predicted scores.
pub fn rank_group(group: &GroupGame, games: &[&SingleGame],
                  predictions: &[&MatchPrediction], draw_order: &[TeamId]) -> Vec<TeamId>;

/// Standings bonus: pairs correctly ordered vs result ordering.
pub fn standings_bonus(predicted: &[TeamId], official: &[TeamId], c: &ScoringConfig) -> i64;

/// Whole-tournament score for one prediction-set vs a baseline.
/// Returns per-stage breakdown: Round -> points (with multipliers applied).
pub fn score_tournament(t: &Tournament, prediction: &Player, result: &Player,
                        now: DateTime<Utc>, c: &ScoringConfig)
    -> std::collections::HashMap<Round, i64>;
```

### `storage` crate — Repository trait (`crates/storage/src/lib.rs`)

```rust
#[async_trait::async_trait]
pub trait Repository: Send + Sync {
    async fn get_tournament(&self) -> anyhow::Result<Option<Tournament>>;
    async fn put_tournament(&self, t: &Tournament) -> anyhow::Result<()>;
    async fn get_player(&self, id: &str) -> anyhow::Result<Option<Player>>;
    async fn list_players(&self) -> anyhow::Result<Vec<Player>>;
    /// Optimistic concurrency: fails if stored version != player.version.
    async fn put_player(&self, p: &Player) -> anyhow::Result<()>;
    async fn get_scoreboard(&self) -> anyhow::Result<Option<Scoreboard>>;
    async fn put_scoreboard(&self, s: &Scoreboard) -> anyhow::Result<()>;
    async fn list_pools(&self) -> anyhow::Result<Vec<Pool>>;
    async fn put_pool(&self, p: &Pool) -> anyhow::Result<()>;
    async fn get_motd(&self) -> anyhow::Result<Option<Motd>>;
    async fn put_motd(&self, m: &Motd) -> anyhow::Result<()>;
    async fn get_identity(&self, provider: &str, provider_id: &str)
        -> anyhow::Result<Option<Identity>>;
    async fn put_identity(&self, i: &Identity) -> anyhow::Result<()>;
    async fn get_person(&self, id: &str) -> anyhow::Result<Option<Person>>;
    async fn put_person(&self, p: &Person) -> anyhow::Result<()>;
}

pub struct Scoreboard { pub entries: HashMap<PlayerId, HashMap<Round, i64>> }
```

`InMemoryRepository` (Mutex-wrapped HashMaps) for tests. `DynamoRepository`
wraps `aws_sdk_dynamodb::Client`, single table `xpool`, key zones per
`DATA_MODEL.md` §9, tournament-id prefix from `CURRENT_TOURNAMENT_ID` env var.

### `fwc26` crate (`crates/fwc26/src/lib.rs`)

```rust
/// Annexe C: 495-row table, embedded as a const data file.
pub fn annexe_c(qualifying_third_groups: &BTreeSet<char>)
    -> Option<HashMap<char, char>>;  // 1A..1L winner-group -> 3X third-group

/// Rank the 12 third-placed teams (FWC26_RULES §3); return top 8 group letters.
pub fn best_thirds(standings: &HashMap<char, Vec<TeamStats>>) -> Vec<char>;

/// Resolve every knockout TeamSlot description to a concrete team given
/// current official results. Pure. Undeterminable slots stay None.
pub fn resolve_bracket(t: &Tournament, result: &Player) -> HashMap<GameId, (Option<TeamId>, Option<TeamId>)>;
```

### GraphQL schema (`api` crate) — per `API.md`

Queries: `tournament`, `scoreboard(pool: ID)`, `me`, `pools`, `tips(groupId: ID!)`,
`perfects`. Mutations: `submitGroup(groupId, predictions[], lock)`, `createPool`,
`updatePool`, `updateProfile`, `invite`, plus admin: `enterResult`, `setMotd`.
Endpoint `POST /api/graphql`. Auth stub: `X-Dev-Player` header → `CurrentPlayer`
in context. `GET /api/health` returns 200.

### `fwc26.json` schema (`tournaments/fwc26.json`)

```json
{
  "tournament_id": "fwc26",
  "teams": [{"id","name","short_code","flag","external_id"}],
  "groups": [{"id","name","parent","round","lock_mode","carries_standings",
              "children": {"groups":[...]} | {"games":[...]}}],
  "games": [{"id","kickoff","venue","group_id",
             "home":{"team_id"?,"description"},"away":{"team_id"?,"description"}}]
}
```

48 teams, 12 group-stage groups (A–L) + knockout one-match groups, 104 games
(M1–M104). Group games carry real team ids; knockout games carry placeholder
descriptions (`2A`, `3ABCDF`, `Winner SF 1`).

---

## Subsystem tasks

Each is built TDD by a dedicated subagent. Order reflects dependencies:
P1–P4 parallel; P5 after P1–P4; P6–P7 parallel with anything.

- [ ] **P0 — Workspace skeleton.** Root `Cargo.toml`, empty crate skeletons that
  compile, `mise.toml`, `.gitignore`, `docker-compose.yml` (DynamoDB Local on
  8000, MailHog on 1025/8025). Owner: orchestrator (this session).

- [ ] **P1 — `domain` crate.** Entities + scoring engine + standings ladder.
  Pure, no I/O. Thorough unit suite incl. SCORING.md §10 regression tests
  (per-side 4-goal rule, threshold ≥4, explicit multiplier table) and §4 ladder
  edge cases. Specs: `DATA_MODEL.md`, `SCORING.md`, `GAME_RULES.md`.

- [ ] **P2 — `fwc26` crate.** Annexe C lookup (embed the 495-row table from
  `FWC26_RULES.md` §5 as a generated data file), third-placed ranking (§3),
  bracket resolution. Pure. Depends on `domain` types only. Specs:
  `FWC26_RULES.md`, `DATA_SOURCES.md` §5.

- [ ] **P3 — `storage` crate.** `Repository` trait, `InMemoryRepository`,
  `DynamoRepository`. Integration tests run `InMemoryRepository`; Dynamo tests
  gated behind a `DYNAMO_TEST` env (DynamoDB Local). Specs: `DATA_MODEL.md` §9.

- [ ] **P4 — `tournaments/fwc26.json`.** Generate from `FWC26_RULES.md` (group
  structure, M1–M104 pairings, knockout placeholders) and the FotMob ICS
  (kickoff times). 48 teams, 104 games, 12+knockout groups. Validate counts.

- [ ] **P5 — `api` + `xtask`.** `xtask import <file>` reads `fwc26.json` →
  `Repository::put_tournament`. `api` is axum + async-graphql exposing the §4/§5
  contract, post-result hook (scoreboard recompute + bracket resolution),
  auth stub, `lambda_http` wrapper behind a `lambda` feature. Depends P1–P4.

- [ ] **P6 — `web` SPA.** React + Vite + TS, urql client, react-router, the 11
  screens from `REWRITE_USE_CASES.md` §4, i18n en/hu, smart polling, the
  draft→locked group form. Talks to `/api/graphql`.

- [ ] **P7 — Local dev wiring.** `docker-compose.yml`, a `README` dev section,
  `xtask seed` for demo players + a result user, Vite proxy `/api` → `:3000`.

## Definition of done

`docker compose up` + `cargo run -p xtask -- import tournaments/fwc26.json` +
`cargo run -p api` + `cd web && npm run dev` yields a working app: browse
schedule/scoreboard, log in via dev stub, enter & lock predictions, admin enters
results, scoreboard updates. `cargo test` and `cargo clippy` green; `npm run
build` green.
