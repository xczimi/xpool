# TheSportsDB Reported Results — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the admin pre-fill official match results from TheSportsDB (review + confirm), built on a shared `sportsdb` integration crate and a committed `M# → idEvent` mapping.

**Architecture:** A new pure-I/O `crates/sportsdb` (typed V2 client + pure JSON decoders) is consumed by `xtask reconcile-events` (one-time, reviewed backfill of `idEvent` into `tournaments/fwc26.json`) and by the `api` `reportedResults(groupId)` query (admin-gated, returns finished scores for `resultPending` games). The SPA auto-fetches reported results when the result user opens a group's entry screen and pre-fills the existing `submitGroup` form — the official write path is unchanged.

**Tech Stack:** Rust (async-graphql, reqwest, async-trait, serde, clap), React + urql + TypeScript, Playwright, Terraform.

**Design doc:** `docs/superpowers/specs/2026-06-14-sportsdb-reported-results-design.md`

---

## File structure

| File | Responsibility |
|---|---|
| `crates/domain/src/model.rs` | add `SingleGame.external_id: Option<String>` |
| `crates/xtask/src/dto.rs` | read `external_id` from game JSON |
| `crates/sportsdb/Cargo.toml`, `src/lib.rs` | crate root, re-exports |
| `crates/sportsdb/src/model.rs` | `Event` struct (field subset) |
| `crates/sportsdb/src/decode.rs` | pure V2-envelope decoders (unit-tested) |
| `crates/sportsdb/src/client.rs` | `SportsDb` reqwest client (`from_env`, fetches) |
| `crates/xtask/src/reconcile.rs` | pure `reconcile()` matcher + report |
| `crates/xtask/src/main.rs` | `ReconcileEvents` subcommand wiring |
| `crates/api/src/reported.rs` | `ReportedResultSource` trait + `SportsDbSource`/`NullSource`/`CachingSource` |
| `crates/api/src/gql/types.rs` | `ReportedResult` output type |
| `crates/api/src/gql/query.rs` | `reportedResults(groupId)` resolver |
| `crates/api/src/gql/mod.rs`, `src/lib.rs` | thread the source into the schema |
| `web/src/graphql/queries.ts`, `types.ts` | `reportedResults` query + types |
| `web/src/pages/MyTipsPage.tsx`, `mytips/GroupTipForm.tsx` | auto-fetch + pre-fill seed |
| `web/e2e/reported-results.spec.ts` | stubbed-SportsDB E2E |
| `infrastructure/lambda.tf` | inject key from SSM as `THESPORTSDB_API_KEY` env |
| `.env.example` | document the local key var |

---

## Phase 1 — Domain & mapping field

### Task 1: Add `external_id` to `SingleGame`

**Files:**
- Modify: `crates/domain/src/model.rs:35-43`
- Modify: `crates/xtask/src/dto.rs:53-61` and `:130-146`
- Test: `crates/domain/src/model.rs` (inline `#[cfg(test)]`), `crates/xtask/tests/` (existing import test path)

- [ ] **Step 1: Write the failing test (domain serde default)**

Add to the bottom of `crates/domain/src/model.rs` (create a `#[cfg(test)] mod tests` block if none exists at the file end):

```rust
#[cfg(test)]
mod external_id_tests {
    use super::*;

    #[test]
    fn single_game_external_id_defaults_to_none_when_absent() {
        // Old data (no external_id key) must still deserialize.
        let json = r#"{
            "id": "M1",
            "kickoff": "2026-06-11T19:00:00Z",
            "venue": null,
            "group_id": "A",
            "home": { "team_id": "MEX", "description": "A1" },
            "away": { "team_id": "RSA", "description": "A2" }
        }"#;
        let g: SingleGame = serde_json::from_str(json).unwrap();
        assert_eq!(g.external_id, None);
    }

    #[test]
    fn single_game_external_id_round_trips() {
        let g = SingleGame {
            id: "M1".into(),
            kickoff: "2026-06-11T19:00:00Z".parse().unwrap(),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("MEX".into()), description: "A1".into() },
            away: TeamSlot { team_id: Some("RSA".into()), description: "A2".into() },
            external_id: Some("2461106".into()),
        };
        let s = serde_json::to_string(&g).unwrap();
        let back: SingleGame = serde_json::from_str(&s).unwrap();
        assert_eq!(back.external_id, Some("2461106".into()));
    }
}
```

`crates/domain/Cargo.toml` needs `serde_json` as a dev-dep — check `[dev-dependencies]`; if absent add `serde_json = "1"`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p domain external_id`
Expected: FAIL to compile — `SingleGame` has no field `external_id`.

- [ ] **Step 3: Add the field with a serde default**

In `crates/domain/src/model.rs`, change `SingleGame`:

```rust
/// One match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleGame {
    pub id: GameId,
    pub kickoff: DateTime<Utc>,
    pub venue: Option<String>,
    pub group_id: GroupId,
    pub home: TeamSlot,
    pub away: TeamSlot,
    /// TheSportsDB `idEvent`, backfilled by `xtask reconcile-events`. `None`
    /// until reconciled (e.g. knockout fixtures not yet published upstream).
    #[serde(default)]
    pub external_id: Option<String>,
}
```

- [ ] **Step 4: Fix the DTO so import reads it**

In `crates/xtask/src/dto.rs`, add to `GameDto` (after `away`, line 60):

```rust
    #[serde(default)]
    pub external_id: Option<String>,
```

And in `into_domain` (the `SingleGame { … }` literal around line 136-143), add the field:

```rust
                    SingleGame {
                        id: g.id,
                        kickoff: g.kickoff,
                        venue: g.venue,
                        group_id: g.group_id,
                        home: g.home.into_domain(),
                        away: g.away.into_domain(),
                        external_id: g.external_id,
                    },
```

- [ ] **Step 5: Run domain + xtask tests to verify they pass**

Run: `cargo test -p domain external_id && cargo test -p xtask`
Expected: PASS. (The existing import test still loads `fwc26.json`, now with the new optional field defaulting to `None`.)

- [ ] **Step 6: Build the whole workspace (other crates construct `SingleGame`)**

Run: `cargo build --workspace 2>&1 | grep -E "error|SingleGame" || echo OK`
Expected: `OK`. If any test fixture builds a `SingleGame { … }` literal, add `external_id: None,` there. Find them with:
Run: `grep -rn "SingleGame {" crates --include=*.rs`
Add `external_id: None,` to each literal that fails to compile.

- [ ] **Step 7: Commit**

```bash
git add crates/domain/src/model.rs crates/xtask/src/dto.rs crates/domain/Cargo.toml
git commit -m "feat(domain): add external_id to SingleGame for SportsDB mapping"
```

---

## Phase 2 — The `sportsdb` crate

### Task 2: Create `crates/sportsdb` with pure decoders

**Files:**
- Create: `crates/sportsdb/Cargo.toml`
- Create: `crates/sportsdb/src/lib.rs`
- Create: `crates/sportsdb/src/model.rs`
- Create: `crates/sportsdb/src/decode.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Register the crate in the workspace**

In the root `Cargo.toml`, add `"crates/sportsdb"` to `members`:

```toml
members = ["crates/domain", "crates/fwc26", "crates/storage", "crates/api", "crates/xtask", "crates/sportsdb"]
```

- [ ] **Step 2: Create the crate manifest**

Create `crates/sportsdb/Cargo.toml`:

```toml
[package]
name = "sportsdb"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }

[dev-dependencies]
tokio = { workspace = true }
```

If `serde_json`/`anyhow`/`tokio` are not in `[workspace.dependencies]`, use explicit versions: `serde_json = "1"`, `anyhow = "1"`, `tokio = { version = "1", features = ["macros", "rt"] }`. Verify with `grep -A20 "\[workspace.dependencies\]" Cargo.toml`.

- [ ] **Step 3: Write the failing decoder test**

Create `crates/sportsdb/src/decode.rs`:

```rust
//! Pure decoders for TheSportsDB V2 JSON envelopes. The V2 API keys its
//! top-level array by *operation* (`schedule`/`livescore`/`list`), not entity
//! (`.specs/THESPORTSDB_API.md` §6). These functions take the raw body and
//! return the field subset xpool uses — no HTTP, fully unit-testable.

use crate::model::Event;
use serde::Deserialize;

#[derive(Deserialize)]
struct ScheduleEnvelope {
    schedule: Option<Vec<RawEvent>>,
}

#[derive(Deserialize)]
struct LivescoreEnvelope {
    livescore: Option<Vec<RawEvent>>,
}

#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "idEvent")]
    id_event: Option<String>,
    #[serde(rename = "dateEvent")]
    date_event: Option<String>,
    #[serde(rename = "idHomeTeam")]
    id_home_team: Option<String>,
    #[serde(rename = "idAwayTeam")]
    id_away_team: Option<String>,
    #[serde(rename = "intHomeScore")]
    int_home_score: Option<serde_json::Value>,
    #[serde(rename = "intAwayScore")]
    int_away_score: Option<serde_json::Value>,
    #[serde(rename = "strStatus")]
    str_status: Option<String>,
}

/// Scores arrive as a string ("2"), a number (2), or null. Normalise to i64.
fn score(v: &Option<serde_json::Value>) -> Option<i64> {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

impl RawEvent {
    fn into_event(self) -> Option<Event> {
        Some(Event {
            id_event: self.id_event?,
            date_event: self.date_event.unwrap_or_default(),
            id_home_team: self.id_home_team.unwrap_or_default(),
            id_away_team: self.id_away_team.unwrap_or_default(),
            int_home_score: score(&self.int_home_score),
            int_away_score: score(&self.int_away_score),
            str_status: self.str_status.unwrap_or_default(),
        })
    }
}

/// Decode a `/schedule/league/...` body into events.
pub fn decode_schedule(body: &str) -> anyhow::Result<Vec<Event>> {
    let env: ScheduleEnvelope = serde_json::from_str(body)?;
    Ok(env.schedule.unwrap_or_default().into_iter().filter_map(RawEvent::into_event).collect())
}

/// Decode a `/livescore/...` body into events.
pub fn decode_livescore(body: &str) -> anyhow::Result<Vec<Event>> {
    let env: LivescoreEnvelope = serde_json::from_str(body)?;
    Ok(env.livescore.unwrap_or_default().into_iter().filter_map(RawEvent::into_event).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_schedule_with_string_and_numeric_scores() {
        let body = r#"{"schedule":[
            {"idEvent":"2461106","dateEvent":"2026-06-15","idHomeTeam":"H","idAwayTeam":"A","intHomeScore":"2","intAwayScore":"1","strStatus":"Match Finished"},
            {"idEvent":"2461112","dateEvent":"2026-06-20","idHomeTeam":"H2","idAwayTeam":"A2","intHomeScore":null,"intAwayScore":null,"strStatus":"Not Started"}
        ]}"#;
        let events = decode_schedule(body).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id_event, "2461106");
        assert_eq!(events[0].int_home_score, Some(2));
        assert_eq!(events[0].int_away_score, Some(1));
        assert_eq!(events[0].str_status, "Match Finished");
        assert_eq!(events[1].int_home_score, None);
    }

    #[test]
    fn decodes_null_schedule_as_empty() {
        assert_eq!(decode_schedule(r#"{"schedule":null}"#).unwrap().len(), 0);
    }

    #[test]
    fn decodes_livescore_envelope() {
        let body = r#"{"livescore":[{"idEvent":"2461106","intHomeScore":2,"intAwayScore":1,"strStatus":"HT"}]}"#;
        let events = decode_livescore(body).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].str_status, "HT");
    }
}
```

- [ ] **Step 4: Create the model and lib root**

Create `crates/sportsdb/src/model.rs`:

```rust
//! The field subset of a TheSportsDB event that xpool consumes.

/// A match event as reported by TheSportsDB. Scores are `None` until played.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub id_event: String,
    pub date_event: String,
    pub id_home_team: String,
    pub id_away_team: String,
    pub int_home_score: Option<i64>,
    pub int_away_score: Option<i64>,
    pub str_status: String,
}

impl Event {
    /// A match TheSportsDB considers played out (final score available).
    /// `strStatus` is free-form upstream; treat the documented finished
    /// markers as final and require both scores present.
    pub fn is_finished(&self) -> bool {
        let s = self.str_status.as_str();
        let finished = matches!(s, "Match Finished" | "FT" | "AET" | "Finished")
            || s.eq_ignore_ascii_case("ft");
        finished && self.int_home_score.is_some() && self.int_away_score.is_some()
    }
}
```

Create `crates/sportsdb/src/lib.rs`:

```rust
//! A thin, typed client for [TheSportsDB](https://www.thesportsdb.com) V2.
//!
//! Pure JSON decoding ([`decode`]) is separated from HTTP ([`client`]) so the
//! envelope handling is unit-testable without a network. Structured so it could
//! later be extracted and published as a standalone open-source Rust SDK:
//! the public surface carries no xpool-specific types and no dependency on the
//! `domain`/`storage` crates.

mod client;
mod decode;
mod model;

pub use client::SportsDb;
pub use decode::{decode_livescore, decode_schedule};
pub use model::Event;
```

Create a stub `crates/sportsdb/src/client.rs` so the crate compiles (filled in Task 3):

```rust
//! HTTP client — see Task 3.
pub struct SportsDb;
```

- [ ] **Step 5: Run the decoder tests**

Run: `cargo test -p sportsdb decode`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/sportsdb Cargo.toml
git commit -m "feat(sportsdb): new crate with pure V2 envelope decoders"
```

### Task 3: Add the `SportsDb` HTTP client

**Files:**
- Modify: `crates/sportsdb/src/client.rs`

- [ ] **Step 1: Write the client**

Replace `crates/sportsdb/src/client.rs` entirely:

```rust
//! HTTP client for TheSportsDB V2 (header auth). League/season are fixed to
//! FIFA World Cup 2026 (`idLeague 4429`, season `2026`,
//! `.specs/THESPORTSDB_API.md` §3). Always pulls *league-wide* endpoints in one
//! call and lets callers filter locally — never loops per-event (§2 rate limits).

use crate::decode::decode_schedule;
use crate::model::Event;
use std::time::Duration;

const BASE: &str = "https://www.thesportsdb.com/api/v2/json";
const LEAGUE_ID: &str = "4429";
const SEASON: &str = "2026";

pub struct SportsDb {
    http: reqwest::Client,
    key: String,
}

impl SportsDb {
    /// Build from `THESPORTSDB_API_KEY`. Returns `None` when unset/empty so
    /// every consumer can degrade gracefully to "no reported results".
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("THESPORTSDB_API_KEY").ok().filter(|k| !k.is_empty())?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        Some(Self { http, key })
    }

    /// Full season schedule (every fixture) — the reconcile source.
    pub async fn season_schedule(&self) -> anyhow::Result<Vec<Event>> {
        let url = format!("{BASE}/schedule/league/{LEAGUE_ID}/{SEASON}");
        decode_schedule(&self.get(&url).await?)
    }

    /// Recently-finished matches — the result-entry source. One league-wide call.
    pub async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        let url = format!("{BASE}/schedule/previous/league/{LEAGUE_ID}");
        decode_schedule(&self.get(&url).await?)
    }

    /// GET with one retry. Errors are returned, never panicked.
    async fn get(&self, url: &str) -> anyhow::Result<String> {
        let mut last_err = None;
        for _ in 0..2 {
            match self.http.get(url).header("X-API-KEY", &self.key).send().await {
                Ok(resp) => return Ok(resp.error_for_status()?.text().await?),
                Err(e) => last_err = Some(e),
            }
        }
        Err(anyhow::anyhow!("sportsdb GET failed: {:?}", last_err))
    }
}
```

(`teams()` and `livescores()` are intentionally omitted — `teams()` is added in Task 5 when reconcile needs it; `livescores()` is future work for #2.)

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p sportsdb`
Expected: builds clean (no tests — HTTP is exercised by the decoders + manual verification; see Step 3).

- [ ] **Step 3: Manual smoke check (optional, needs the key)**

This is a manual verification, not an automated test. With the key in `.env`:
Run: `THESPORTSDB_API_KEY=200769 cargo run -p xtask -- reconcile-events --dry-run` (after Task 5)
Expected: prints a mapping table. Skip if offline.

- [ ] **Step 4: Commit**

```bash
git add crates/sportsdb/src/client.rs
git commit -m "feat(sportsdb): add V2 HTTP client (from_env, schedule, finished)"
```

---

## Phase 3 — `xtask reconcile-events`

### Task 4: Pure reconcile matcher

**Files:**
- Create: `crates/xtask/src/reconcile.rs`
- Modify: `crates/xtask/src/lib.rs` (add `pub mod reconcile;`)
- Modify: `crates/sportsdb/src/client.rs` (add `teams()`), `crates/sportsdb/src/lib.rs` (export `TeamRow`), `crates/sportsdb/src/model.rs` (add `TeamRow`), `crates/sportsdb/src/decode.rs` (add `decode_teams`)
- Modify: `crates/xtask/Cargo.toml` (add `sportsdb` dep)

- [ ] **Step 1: Add `TeamRow` + `decode_teams` to sportsdb**

In `crates/sportsdb/src/model.rs` add:

```rust
/// A team row from `/list/teams/{leagueId}` — id + name for matching.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamRow {
    pub id_team: String,
    pub str_team: String,
}
```

In `crates/sportsdb/src/decode.rs`, add the envelope + decoder + test:

```rust
#[derive(Deserialize)]
struct ListEnvelope {
    list: Option<Vec<RawTeam>>,
}

#[derive(Deserialize)]
struct RawTeam {
    #[serde(rename = "idTeam")]
    id_team: Option<String>,
    #[serde(rename = "strTeam")]
    str_team: Option<String>,
}

/// Decode a `/list/teams/{leagueId}` body into team rows.
pub fn decode_teams(body: &str) -> anyhow::Result<Vec<crate::model::TeamRow>> {
    let env: ListEnvelope = serde_json::from_str(body)?;
    Ok(env
        .list
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| {
            Some(crate::model::TeamRow {
                id_team: t.id_team?,
                str_team: t.str_team.unwrap_or_default(),
            })
        })
        .collect())
}
```

Add to the `decode.rs` tests module:

```rust
    #[test]
    fn decodes_teams_list() {
        let body = r#"{"list":[{"idTeam":"133","strTeam":"Sweden"}]}"#;
        let teams = decode_teams(body).unwrap();
        assert_eq!(teams.len(), 1);
        assert_eq!(teams[0].id_team, "133");
        assert_eq!(teams[0].str_team, "Sweden");
    }
```

Export from `crates/sportsdb/src/lib.rs`:

```rust
pub use decode::{decode_livescore, decode_schedule, decode_teams};
pub use model::{Event, TeamRow};
```

Add `teams()` to `crates/sportsdb/src/client.rs` (`impl SportsDb`, and `use crate::decode::decode_teams;` at the top):

```rust
    /// All teams in the league — the reconcile team-id source.
    pub async fn teams(&self) -> anyhow::Result<Vec<crate::model::TeamRow>> {
        let url = format!("{BASE}/list/teams/{LEAGUE_ID}");
        crate::decode::decode_teams(&self.get(&url).await?)
    }
```

Run: `cargo test -p sportsdb decode` → expect PASS (4 tests).

- [ ] **Step 2: Add `sportsdb` to xtask deps**

In `crates/xtask/Cargo.toml` `[dependencies]` add:

```toml
sportsdb = { path = "../sportsdb" }
```

- [ ] **Step 3: Write the failing reconcile test**

Create `crates/xtask/src/reconcile.rs`:

```rust
//! Pure matcher for `reconcile-events`: align xpool games to TheSportsDB events
//! by (date, home team, away team) and propose `M# → idEvent` mappings for
//! human review. No I/O — the subcommand (main.rs) does the fetching + writing.

use sportsdb::{Event, TeamRow};
use std::collections::HashMap;

/// One proposed mapping row.
#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub game_id: String,
    pub id_event: String,
}

/// The outcome of a reconcile pass — matches plus games we could not align.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub matched: Vec<Match>,
    pub unmatched_games: Vec<String>,
}

/// Match games to events. `team_external_id` maps our team id → SportsDB idTeam
/// (from a prior team reconcile or the committed `external_id`s). A game matches
/// an event when the kickoff date (UTC `YYYY-MM-DD`) and both team ids agree.
pub fn reconcile(
    games: &[(String, String, Option<String>, Option<String>)], // (game_id, date, home_team_id, away_team_id)
    team_external_id: &HashMap<String, String>,
    events: &[Event],
) -> Report {
    // Index events by (date, home idTeam, away idTeam).
    let mut by_key: HashMap<(String, String, String), &Event> = HashMap::new();
    for e in events {
        by_key.insert(
            (e.date_event.clone(), e.id_home_team.clone(), e.id_away_team.clone()),
            e,
        );
    }

    let mut report = Report::default();
    for (game_id, date, home, away) in games {
        let resolved = home
            .as_ref()
            .and_then(|h| team_external_id.get(h))
            .zip(away.as_ref().and_then(|a| team_external_id.get(a)));
        let hit = resolved
            .and_then(|(h, a)| by_key.get(&(date.clone(), h.clone(), a.clone())));
        match hit {
            Some(e) => report.matched.push(Match {
                game_id: game_id.clone(),
                id_event: e.id_event.clone(),
            }),
            None => report.unmatched_games.push(game_id.clone()),
        }
    }
    report
}

/// Resolve `idTeam` for every SportsDB team name we can match by exact name —
/// a helper for first-time team reconcile (case-insensitive exact match).
pub fn team_ids_by_name(rows: &[TeamRow]) -> HashMap<String, String> {
    rows.iter()
        .map(|r| (r.str_team.to_lowercase(), r.id_team.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, date: &str, h: &str, a: &str) -> Event {
        Event {
            id_event: id.into(),
            date_event: date.into(),
            id_home_team: h.into(),
            id_away_team: a.into(),
            int_home_score: None,
            int_away_score: None,
            str_status: "Not Started".into(),
        }
    }

    #[test]
    fn matches_by_date_and_team_ids() {
        let events = vec![ev("2461106", "2026-06-15", "133", "999")];
        let team_ext: HashMap<String, String> =
            [("SWE".to_string(), "133".to_string()), ("TUN".to_string(), "999".to_string())]
                .into_iter()
                .collect();
        let games = vec![(
            "M5".to_string(),
            "2026-06-15".to_string(),
            Some("SWE".to_string()),
            Some("TUN".to_string()),
        )];
        let report = reconcile(&games, &team_ext, &events);
        assert_eq!(report.matched, vec![Match { game_id: "M5".into(), id_event: "2461106".into() }]);
        assert!(report.unmatched_games.is_empty());
    }

    #[test]
    fn reports_unmatched_when_team_id_missing() {
        let events = vec![ev("2461106", "2026-06-15", "133", "999")];
        let team_ext = HashMap::new(); // no team mapping yet
        let games = vec![(
            "M5".to_string(),
            "2026-06-15".to_string(),
            Some("SWE".to_string()),
            Some("TUN".to_string()),
        )];
        let report = reconcile(&games, &team_ext, &events);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched_games, vec!["M5".to_string()]);
    }
}
```

Add `pub mod reconcile;` to `crates/xtask/src/lib.rs`.

- [ ] **Step 4: Run the reconcile tests to verify they pass**

Run: `cargo test -p xtask reconcile`
Expected: PASS (2 tests in reconcile + the decode tests already green).

- [ ] **Step 5: Commit**

```bash
git add crates/sportsdb crates/xtask/src/reconcile.rs crates/xtask/src/lib.rs crates/xtask/Cargo.toml
git commit -m "feat(xtask): pure reconcile matcher for game<->idEvent mapping"
```

### Task 5: Wire the `ReconcileEvents` subcommand

**Files:**
- Modify: `crates/xtask/src/main.rs`

- [ ] **Step 1: Add the subcommand variant**

In `crates/xtask/src/main.rs`, add to `enum Command` (after `FixGroupsGh`):

```rust
    /// Reconcile xpool games against TheSportsDB and print proposed
    /// `M# -> idEvent` mappings. Read-only: prints a table for the human to
    /// paste into `tournaments/fwc26.json`. Requires THESPORTSDB_API_KEY.
    ReconcileEvents,
```

- [ ] **Step 2: Add the match arm**

In the `match cli.command { … }` block, add:

```rust
        Command::ReconcileEvents => {
            let client = sportsdb::SportsDb::from_env()
                .ok_or_else(|| anyhow::anyhow!("THESPORTSDB_API_KEY not set"))?;
            let tournament = repo
                .get_tournament()
                .await?
                .ok_or_else(|| anyhow::anyhow!("no tournament loaded — run `import` first"))?;
            let events = client.season_schedule().await?;
            let team_rows = client.teams().await?;

            // Team external_ids: prefer those already committed on the teams,
            // else fall back to exact-name match against SportsDB.
            let names = xtask::reconcile::team_ids_by_name(&team_rows);
            let mut team_ext = std::collections::HashMap::new();
            for team in tournament.teams.values() {
                if let Some(ext) = &team.external_id {
                    team_ext.insert(team.id.clone(), ext.clone());
                } else if let Some(id) = names.get(&team.name.to_lowercase()) {
                    team_ext.insert(team.id.clone(), id.clone());
                }
            }

            let games: Vec<(String, String, Option<String>, Option<String>)> = tournament
                .games
                .values()
                .map(|g| {
                    (
                        g.id.clone(),
                        g.kickoff.format("%Y-%m-%d").to_string(),
                        g.home.team_id.clone(),
                        g.away.team_id.clone(),
                    )
                })
                .collect();

            let report = xtask::reconcile::reconcile(&games, &team_ext, &events);
            println!("# Proposed game -> idEvent mappings ({} matched):", report.matched.len());
            let mut matched = report.matched;
            matched.sort_by(|a, b| a.game_id.cmp(&b.game_id));
            for m in &matched {
                println!("{}\t{}", m.game_id, m.id_event);
            }
            if !report.unmatched_games.is_empty() {
                let mut un = report.unmatched_games;
                un.sort();
                eprintln!("# Unmatched ({}): {}", un.len(), un.join(", "));
            }
            println!(
                "# Review, then set each game's `external_id` in tournaments/fwc26.json."
            );
        }
```

- [ ] **Step 3: Build to verify it wires up**

Run: `cargo build -p xtask`
Expected: builds clean.

- [ ] **Step 4: Run to confirm graceful failure without a key**

Run: `env -u THESPORTSDB_API_KEY cargo run -p xtask -- reconcile-events`
Expected: exits with error `THESPORTSDB_API_KEY not set` (a clear, loud failure — reconcile is an explicit dev action, per the spec).

- [ ] **Step 5: Commit**

```bash
git add crates/xtask/src/main.rs
git commit -m "feat(xtask): reconcile-events subcommand prints game->idEvent table"
```

---

## Phase 4 — `api` reported-results query

### Task 6: `ReportedResultSource` trait + schema injection

**Files:**
- Create: `crates/api/src/reported.rs`
- Modify: `crates/api/src/lib.rs` (add `pub mod reported;`, construct source in `build_app`)
- Modify: `crates/api/src/gql/mod.rs` (`build_schema` takes the source)
- Modify: `crates/api/Cargo.toml` (add `sportsdb`, `async-trait`)

- [ ] **Step 1: Add deps**

In `crates/api/Cargo.toml` `[dependencies]` add:

```toml
sportsdb = { path = "../sportsdb" }
async-trait.workspace = true
```

- [ ] **Step 2: Write the source trait + impls + a failing test**

Create `crates/api/src/reported.rs`:

```rust
//! The reported-results source seam. The GraphQL resolver depends on this
//! trait (not on `sportsdb` directly) so tests inject a stub and never touch
//! the network. Production wraps the real `sportsdb::SportsDb`; when no API key
//! is configured a `NullSource` returns nothing and result entry degrades to
//! manual (the feature only ever *adds* convenience).

use async_trait::async_trait;
use sportsdb::{Event, SportsDb};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[async_trait]
pub trait ReportedResultSource: Send + Sync {
    /// Recently-finished matches, league-wide. Errors degrade to `Ok(vec![])`
    /// at the call site — see the resolver.
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>>;
}

/// The real source — wraps the SportsDB client.
pub struct SportsDbSource(pub SportsDb);

#[async_trait]
impl ReportedResultSource for SportsDbSource {
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        self.0.finished_results().await
    }
}

/// Used when `THESPORTSDB_API_KEY` is unset — always empty.
pub struct NullSource;

#[async_trait]
impl ReportedResultSource for NullSource {
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        Ok(Vec::new())
    }
}

/// A ~45s in-process TTL cache so opening several group screens (or an
/// auto-fetch followed by a manual refresh) doesn't re-hit SportsDB. In-process
/// is enough — Lambda reuses warm containers and the call is cheap regardless.
pub struct CachingSource<S> {
    inner: S,
    ttl: Duration,
    cache: Mutex<Option<(Instant, Vec<Event>)>>,
}

impl<S> CachingSource<S> {
    pub fn new(inner: S) -> Self {
        Self { inner, ttl: Duration::from_secs(45), cache: Mutex::new(None) }
    }
}

#[async_trait]
impl<S: ReportedResultSource> ReportedResultSource for CachingSource<S> {
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        if let Some((at, events)) = self.cache.lock().unwrap().as_ref() {
            if at.elapsed() < self.ttl {
                return Ok(events.clone());
            }
        }
        let fresh = self.inner.finished_results().await?;
        *self.cache.lock().unwrap() = Some((Instant::now(), fresh.clone()));
        Ok(fresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_source_is_empty() {
        assert!(NullSource.finished_results().await.unwrap().is_empty());
    }
}
```

- [ ] **Step 3: Run the test**

Run: `cargo test -p api null_source_is_empty`
Expected: PASS.

- [ ] **Step 4: Thread the source into `build_schema`**

In `crates/api/src/gql/mod.rs`, change `build_schema`:

```rust
use crate::reported::ReportedResultSource;

pub fn build_schema(
    repo: Arc<dyn Repository>,
    reported: Arc<dyn ReportedResultSource>,
) -> XpoolSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(repo)
        .data(reported)
        .limit_depth(20)
        .finish()
}
```

- [ ] **Step 5: Construct the source in `build_app`**

In `crates/api/src/lib.rs`, add `pub mod reported;` to the module list, then in `build_app` replace the `let schema = …` line:

```rust
    use crate::reported::{CachingSource, NullSource, ReportedResultSource, SportsDbSource};
    let reported: Arc<dyn ReportedResultSource> = match sportsdb::SportsDb::from_env() {
        Some(client) => Arc::new(CachingSource::new(SportsDbSource(client))),
        None => Arc::new(NullSource),
    };
    let schema = gql::build_schema(repo.clone(), reported);
```

- [ ] **Step 6: Build the whole workspace (catch other `build_schema` callers)**

Run: `cargo build --workspace 2>&1 | grep -E "error\[|build_schema" || echo OK`
Expected: any test/bin calling `build_schema(repo)` now needs the second arg. Find them:
Run: `grep -rn "build_schema(" crates/api --include=*.rs`
For each test call site, pass a source, e.g. `gql::build_schema(repo, std::sync::Arc::new(crate::reported::NullSource))`.

- [ ] **Step 7: Run api tests + commit**

Run: `cargo test -p api`
Expected: PASS.

```bash
git add crates/api/src/reported.rs crates/api/src/lib.rs crates/api/src/gql/mod.rs crates/api/Cargo.toml
git commit -m "feat(api): reported-results source seam injected into the schema"
```

### Task 7: `ReportedResult` type + `reportedResults(groupId)` query

**Files:**
- Modify: `crates/api/src/gql/types.rs` (add `ReportedResult`)
- Modify: `crates/api/src/gql/query.rs` (add resolver + a test module)

- [ ] **Step 1: Add the output type**

At the end of `crates/api/src/gql/types.rs`:

```rust
/// A match result as *reported* by an external data source (TheSportsDB). Not
/// an official or predicted result — the SPA presents it as a fill-in to
/// confirm. Provenance-named on purpose (`source`), so #2's live-preview can
/// reuse it with a non-finished `source_status`.
#[derive(SimpleObject, Clone, Debug)]
pub struct ReportedResult {
    pub game_id: String,
    pub home_score: i32,
    pub away_score: i32,
    /// Which source reported it (currently always `"thesportsdb"`).
    pub source: String,
    /// The upstream status string, e.g. `"Match Finished"`.
    pub source_status: String,
    /// True for knockout matches: the upstream final score may include extra
    /// time / penalties, but xpool scores knockouts on the 90-minute result
    /// (`SCORING.md` §5), so the admin must verify before submitting.
    pub ninety_minute_uncertain: bool,
}
```

- [ ] **Step 2: Write the failing resolver test**

Add this concrete test module at the end of `crates/api/src/gql/query.rs`. It mirrors the fixture style in `crates/api/src/recompute.rs` (its `tests` module) and the `Player`/`SingleGame` field sets confirmed there:

```rust
#[cfg(test)]
mod reported_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, Player, Round, SingleGame, Team, TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct StubSource(Vec<Event>);
    #[async_trait]
    impl ReportedResultSource for StubSource {
        async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
            Ok(self.0.clone())
        }
    }

    fn finished(id_event: &str, h: i64, a: i64) -> Event {
        Event {
            id_event: id_event.into(),
            date_event: "2026-06-11".into(),
            id_home_team: "H".into(),
            id_away_team: "A".into(),
            int_home_score: Some(h),
            int_away_score: Some(a),
            str_status: "Match Finished".into(),
        }
    }

    // Result user with NO prediction for M1 -> M1 is result-pending.
    fn result_user() -> Player {
        Player {
            id: "result-user".into(),
            person_id: "p".into(),
            nick: "official".into(),
            full_name: "Official".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    // Group A (GroupStage), one game M1 mapped to idEvent "E1", kickoff in the past.
    async fn repo_with_pending_m1() -> Arc<dyn Repository> {
        let team = |id: &str| Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        };
        let g1 = SingleGame {
            id: "M1".into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 19, 0, 0).unwrap(),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("AAA".into()), description: "A1".into() },
            away: TeamSlot { team_id: Some("BBB".into()), description: "A2".into() },
            external_id: Some("E1".into()),
        };
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into()]),
        };
        let t = Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), g1)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        repo.put_player(&result_user()).await.unwrap();
        Arc::new(repo)
    }

    #[tokio::test]
    async fn maps_finished_event_to_pending_game_for_result_user() {
        let repo = repo_with_pending_m1().await;
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![finished("E1", 2, 1)]));
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(
            r#"{ reportedResults(groupId:"A"){ gameId homeScore awayScore source sourceStatus ninetyMinuteUncertain } }"#,
        )
        .data(CurrentPlayer::Player(Box::new(result_user())))
        // kickoff 19:00 + 105min buffer -> pending after 20:45; noon next day is pending.
        .data(crate::clock::RequestNow("2026-06-12T12:00:00Z".parse().unwrap()));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        let json = resp.data.into_json().unwrap();
        let row = &json["reportedResults"][0];
        assert_eq!(row["gameId"], "M1");
        assert_eq!(row["homeScore"], 2);
        assert_eq!(row["awayScore"], 1);
        assert_eq!(row["source"], "thesportsdb");
        assert_eq!(row["ninetyMinuteUncertain"], false);
    }

    #[tokio::test]
    async fn non_result_user_is_rejected() {
        let repo = repo_with_pending_m1().await;
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![finished("E1", 2, 1)]));
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(r#"{ reportedResults(groupId:"A"){ gameId } }"#)
            .data(CurrentPlayer::Visitor)
            .data(crate::clock::RequestNow("2026-06-12T12:00:00Z".parse().unwrap()));
        let resp = schema.execute(req).await;
        assert!(!resp.errors.is_empty());
    }
}
```

> If `Player`/`SingleGame` field names drift from the above, reconcile against the live `recompute.rs` `tests::fixture()` — it is the canonical fixture this mirrors.

- [ ] **Step 3: Run to confirm it fails (resolver missing)**

Run: `cargo test -p api reported_results_maps`
Expected: FAIL — `reportedResults` is not a field.

- [ ] **Step 4: Implement the resolver**

In `crates/api/src/gql/query.rs`, add inside `impl QueryRoot`:

```rust
    /// External (TheSportsDB) reported results for a group's result-pending
    /// games — the admin pre-fill source. Admin-only (the result user). Returns
    /// only finished, mapped, not-yet-entered games; `[]` if the source is
    /// absent or errors (manual entry is never blocked).
    async fn reported_results(
        &self,
        ctx: &Context<'_>,
        group_id: String,
    ) -> async_graphql::Result<Vec<ReportedResult>> {
        // Gate: only the result user (the official-results admin).
        let viewer = CurrentPlayer::require(ctx)?;
        if !viewer.is_result_user {
            return Err(async_graphql::Error::new("not authorized"));
        }

        let repo = repo(ctx);
        let now = now(ctx);
        let Some(tournament) = repo.get_tournament().await? else {
            return Ok(Vec::new());
        };
        let players = repo.list_players().await?;
        let entered: std::collections::HashSet<String> = players
            .iter()
            .find(|p| p.is_result_user)
            .map(|r| r.match_predictions.iter().map(|p| p.game_id.clone()).collect())
            .unwrap_or_default();

        // Games in this group that are result-pending and have an idEvent.
        // event idEvent -> (game_id, round) for O(1) join below.
        let mut by_event: std::collections::HashMap<String, (String, domain::Round)> =
            std::collections::HashMap::new();
        for game in tournament.games_in(&group_id) {
            let round = tournament
                .groups
                .get(&game.group_id)
                .map(|g| g.round)
                .unwrap_or(domain::Round::GroupStage);
            let pending = crate::timeflags::result_pending(
                game.kickoff,
                round,
                entered.contains(&game.id),
                now,
            );
            if pending {
                if let Some(ext) = &game.external_id {
                    by_event.insert(ext.clone(), (game.id.clone(), round));
                }
            }
        }
        if by_event.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch reported results; any error degrades to empty.
        let source = ctx.data_unchecked::<Arc<dyn crate::reported::ReportedResultSource>>();
        let events = source.finished_results().await.unwrap_or_default();

        let mut out = Vec::new();
        for e in events {
            if !e.is_finished() {
                continue;
            }
            if let Some((game_id, round)) = by_event.get(&e.id_event) {
                if let (Some(h), Some(a)) = (e.int_home_score, e.int_away_score) {
                    out.push(ReportedResult {
                        game_id: game_id.clone(),
                        home_score: h as i32,
                        away_score: a as i32,
                        source: "thesportsdb".to_string(),
                        source_status: e.str_status.clone(),
                        ninety_minute_uncertain: *round != domain::Round::GroupStage,
                    });
                }
            }
        }
        out.sort_by(|a, b| a.game_id.cmp(&b.game_id));
        Ok(out)
    }
```

Ensure `Arc` is in scope (the file already has `use std::sync::Arc;`).

- [ ] **Step 5: Run the resolver test to verify it passes**

Run: `cargo test -p api reported`
Expected: PASS (the resolver test + `null_source_is_empty`).

- [ ] **Step 6: Full api test + clippy**

Run: `cargo test -p api && cargo clippy -p api -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/gql/types.rs crates/api/src/gql/query.rs
git commit -m "feat(api): reportedResults(groupId) query for admin result pre-fill"
```

---

## Phase 5 — Web pre-fill

### Task 8: `reportedResults` query + auto-fetch + seed

**Files:**
- Modify: `web/src/graphql/queries.ts` (add query)
- Modify: `web/src/graphql/types.ts` (add `ReportedResult`)
- Modify: `web/src/pages/MyTipsPage.tsx` (auto-fetch when result user)
- Modify: `web/src/pages/mytips/GroupTipForm.tsx` (seed from reported)

- [ ] **Step 1: Add the query + type**

In `web/src/graphql/queries.ts` (mirror an existing `gql`-tagged export like `RESULTS_QUERY`):

```ts
export const REPORTED_RESULTS_QUERY = `
  query ReportedResults($groupId: String!) {
    reportedResults(groupId: $groupId) {
      gameId
      homeScore
      awayScore
      source
      sourceStatus
      ninetyMinuteUncertain
    }
  }
`
```

In `web/src/graphql/types.ts` add:

```ts
export interface ReportedResult {
  gameId: string
  homeScore: number
  awayScore: number
  source: string
  sourceStatus: string
  ninetyMinuteUncertain: boolean
}
```

- [ ] **Step 2: Auto-fetch in MyTipsPage when the viewer is the result user**

In `web/src/pages/MyTipsPage.tsx`:

Add to the imports from `../graphql/queries`: `REPORTED_RESULTS_QUERY`. Add to the type imports: `ReportedResult`.

After the `standingsResult` query (around line 110), add:

```tsx
  // The result user (official-results admin) gets SportsDB pre-fill: when they
  // open a group with result-pending games, auto-fetch reported scores. The
  // query is admin-gated server-side and returns [] when SportsDB is absent, so
  // this is a no-op for everyone else / when unconfigured.
  const isResultUser = meRaw?.__typename === 'Player' && meRaw.isResultUser
  const [reportedResult] = useQuery<{ reportedResults: ReportedResult[] }>({
    query: REPORTED_RESULTS_QUERY,
    variables: { groupId: tipsGroupId },
    pause: !label || !tipsGroupId || !isResultUser,
  })
  const reportedByGame = useMemo(() => {
    const map = new Map<string, ReportedResult>()
    for (const r of reportedResult.data?.reportedResults ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [reportedResult.data])
```

Pass it into `GroupTipForm` (in the `shownGroups.map`, add the prop):

```tsx
              reported={reportedByGame}
```

- [ ] **Step 3: Seed the form from reported scores (only when no prediction exists)**

In `web/src/pages/mytips/GroupTipForm.tsx`:

Add to the imports: `ReportedResult` from `../../graphql/types`.

Add to the props type (after `results`):

```tsx
  /** SportsDB reported scores to pre-fill empty inputs (result user only). */
  reported?: Map<string, ReportedResult>
```

Add `reported` to the destructured params.

Change `initialMatches` (lines 100-111) so a reported score seeds an otherwise-empty input:

```tsx
  const initialMatches = useMemo(() => {
    const map: Record<string, DraftMatch> = {}
    for (const game of games) {
      const existing = me.matchPredictions.find((p) => p.gameId === game.id)
      const fill = !existing ? reported?.get(game.id) : undefined
      map[game.id] = {
        homeScore: existing
          ? String(existing.homeScore)
          : fill
            ? String(fill.homeScore)
            : '',
        awayScore: existing
          ? String(existing.awayScore)
          : fill
            ? String(fill.awayScore)
            : '',
        locked: existing?.locked ?? false,
      }
    }
    return map
  }, [games, me, reported])
```

Because `initialMatches` now depends on `reported` (which arrives after the first render), the `useState(initialMatches)` seed won't pick up a late fetch. Re-seed when reported scores land for a still-empty input — add this effect after the `useState` declarations (after line 123):

```tsx
  // Pull in reported pre-fills that arrived after mount, without clobbering any
  // value the admin has already typed.
  useEffect(() => {
    setMatches((prev) => {
      let changed = false
      const next = { ...prev }
      for (const game of games) {
        const r = reported?.get(game.id)
        const cur = prev[game.id]
        if (r && cur && cur.homeScore === '' && cur.awayScore === '' && !cur.locked) {
          next[game.id] = { homeScore: String(r.homeScore), awayScore: String(r.awayScore), locked: false }
          changed = true
        }
      }
      return changed ? next : prev
    })
  }, [reported, games])
```

Add `useEffect` to the React import at the top: `import { useEffect, useMemo, useState } from 'react'`.

- [ ] **Step 4: Typecheck + lint + build**

Run: `cd web && npm run build && npm run lint`
Expected: `tsc -b` passes, eslint clean.

- [ ] **Step 5: Commit**

```bash
git add web/src/graphql/queries.ts web/src/graphql/types.ts web/src/pages/MyTipsPage.tsx web/src/pages/mytips/GroupTipForm.tsx
git commit -m "feat(web): pre-fill result-entry form from SportsDB for the result user"
```

---

## Phase 6 — E2E & deployment

### Task 9: E2E — stubbed SportsDB pre-fill

**Files:**
- Create: `web/e2e/reported-results.spec.ts`

> The E2E stack boots the live API (`e2e/global-setup.ts`). Since the API calls
> TheSportsDB over HTTP, the hermetic approach is to **seed an official-results
> gap and stub at the network boundary**. Two options — pick the one matching
> the suite's existing patterns (check `grep -rn "route\|XPOOL_NOW\|X-Dev-Now" web/e2e`):
> (a) Playwright `page.route` only stubs browser requests, NOT the API's server-side
> fetch — so it will NOT intercept SportsDB. Therefore use (b): set the API's
> `THESPORTSDB_API_KEY` to empty in the e2e env so `reportedResults` returns `[]`,
> and assert the *absence* of pre-fill + that manual entry still works; OR run a
> tiny local stub server and point the e2e API's base URL at it (requires making
> `BASE` overridable via env — out of scope here).

- [ ] **Step 1: Write the E2E spec (degradation path — always hermetic)**

Create `web/e2e/reported-results.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { devLogin } from './helpers' // reuse the suite's dev-login helper; adjust import to the real path

// With no THESPORTSDB_API_KEY in the e2e env, reportedResults returns [] and the
// result user's entry form must still work entirely by hand. This locks in the
// "manual entry is never blocked" guarantee.
test('result user can enter results manually when SportsDB is unconfigured', async ({ page }) => {
  await devLogin(page, 'result-user')
  await page.goto('/my-tips')
  // Pick the Group Stage, a group with a played match (set the dev clock so a
  // game is result-pending). Assert the score selects are present and empty,
  // then enter + lock a result and see it persist.
  // ... mirror the existing my-tips e2e spec's selectors ...
  await expect(page.locator('.tip-form')).toBeVisible()
})
```

> NOTE TO IMPLEMENTER: model this spec on the existing my-tips / dev-login E2E
> (find it: `ls web/e2e/*.spec.ts` and reuse its login + clock helpers and
> selectors). The positive pre-fill path (asserting inputs auto-populate from a
> stub) requires the e2e API to point at a local stub server; if you make
> `sportsdb`'s `BASE` overridable by env, add that assertion. Otherwise this
> degradation test is the hermetic guarantee.

- [ ] **Step 2: Run the targeted E2E**

Run: `cd web && npm run e2e -- reported-results`
Expected: PASS. (Per `e2e-needs-dev-stub-auth` memory: ensure `web/.env.local` blanks `VITE_AUTH0_*` so dev-login works.)

- [ ] **Step 3: Commit**

```bash
git add web/e2e/reported-results.spec.ts
git commit -m "test(e2e): result entry works hermetically without SportsDB key"
```

### Task 10: Deploy wiring — inject the key from SSM

**Files:**
- Modify: `infrastructure/lambda.tf`
- Modify: `.env.example`

- [ ] **Step 1: Read the existing Lambda env block**

Run: `grep -n "environment\|variables\|CLOUDFRONT_SECRET\|aws_ssm_parameter" infrastructure/lambda.tf`
This shows the `environment { variables = { … } }` block and the existing `thesportsdb_key` SSM resource (in `ssm.tf`).

- [ ] **Step 2: Add a data source + env var**

In `infrastructure/lambda.tf`, read the SecureString and inject it. Add near the top:

```hcl
data "aws_ssm_parameter" "thesportsdb_key" {
  name            = aws_ssm_parameter.thesportsdb_key.name
  with_decryption = true
}
```

Then inside the Lambda function's `environment { variables = { … } }` map add:

```hcl
      THESPORTSDB_API_KEY = data.aws_ssm_parameter.thesportsdb_key.value
```

(The Lambda role already has SSM read — `lambda.tf:68`.) If the project prefers not to render the secret into Lambda config plaintext, the alternative is runtime SSM read in `from_env`; the spec chose env injection for consistency with `CLOUDFRONT_SECRET`/`AUTH0_*`.

- [ ] **Step 3: Validate Terraform**

Run: `cd infrastructure && terraform validate`
Expected: `Success! The configuration is valid.` (Do NOT `terraform apply` — deployment is the maintainer's explicit action.)

- [ ] **Step 4: Document the local var**

In `.env.example`, add:

```sh
# TheSportsDB premium API key (V2). Local dev only — prod/dev read it from SSM.
THESPORTSDB_API_KEY=
```

- [ ] **Step 5: Commit**

```bash
git add infrastructure/lambda.tf .env.example
git commit -m "chore(infra): inject THESPORTSDB_API_KEY into Lambda from SSM"
```

---

## Final verification

- [ ] **Workspace green:** `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`
- [ ] **Web green:** `cd web && npm run build && npm run lint`
- [ ] **E2E green:** `cd web && npm run e2e -- reported-results`
- [ ] **Manual smoke (optional, needs key):** add `THESPORTSDB_API_KEY=200769` to `.env`, run `cargo run -p xtask -- reconcile-events`, review the `M# → idEvent` table, paste into `tournaments/fwc26.json`, re-import, then as `result-user` open a result-pending group in My Tips and confirm the form pre-fills.

---

## Notes for the implementer

- **Don't modify `submitGroup` or `recompute`** — the official write path is unchanged; this feature only *populates the form*.
- **Pre-fill never clobbers** an existing or typed-in score (the `cur.homeScore === '' && !cur.locked` guard).
- **Knockout 90′ ambiguity** is surfaced (`ninetyMinuteUncertain`) but not auto-resolved — the admin confirms.
- **`sportsdb` stays publishable** — no `domain`/`storage`/xpool types in its public API; keep it that way.
- **Rate limits are a non-issue** as long as every fetch is league-wide (`finished_results`/`season_schedule`) and never a per-`idEvent` loop.
