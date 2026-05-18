# Reliable SDLC — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the test suite reliable — isolate the e2e database per run, and make time-dependent behaviour deterministic by giving the API a controllable clock and the SPA server-derived time flags.

**Architecture:** The e2e suite gets a fresh, uniquely-named DynamoDB table per run. The API resolves "now" per request (`X-Dev-Now` header → `XPOOL_NOW` env → real clock), mirroring the existing `X-Dev-Player` dev stub. Resolvers stop calling `Utc::now()` and read `now` from the GraphQL context. The API exposes time-derived flags (`deadlinePassed`, `resultPending`, `withinTodayWindow`); the SPA renders them instead of computing time from `Date.now()`.

**Tech Stack:** Rust (axum, async-graphql, aws-sdk-dynamodb), React + Vite + urql, Playwright, DynamoDB Local.

**Spec:** [`.specs/TESTING.md`](../../../.specs/TESTING.md). Read it first.

---

## Phase 1 — e2e database isolation (fresh table per run)

### Task 1: `DynamoRepository::delete_table`

**Files:**
- Modify: `crates/storage/src/dynamo.rs` (add a method near `ensure_table`, ~line 86–116)
- Test: `crates/storage/tests/dynamo.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/storage/tests/dynamo.rs` (gated like the others):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DYNAMO_TEST=1 DYNAMO_ENDPOINT=http://localhost:8000 cargo test -p storage --test dynamo dynamo_delete_table_removes_it`
Expected: FAIL — `no method named delete_table`.
(Requires DynamoDB Local: `docker compose up -d`.)

- [ ] **Step 3: Implement `delete_table`**

In `crates/storage/src/dynamo.rs`, after `ensure_table` (it ends ~line 116), add:

```rust
    /// Delete the table. Used by e2e teardown. Idempotent — a missing table
    /// is treated as success.
    pub async fn delete_table(&self) -> anyhow::Result<()> {
        match self
            .client
            .delete_table()
            .table_name(&self.table)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if e.as_service_error().is_some_and(|se| se.is_resource_not_found_exception()) => {
                Ok(())
            }
            Err(e) => Err(e).context("delete_table failed"),
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `DYNAMO_TEST=1 DYNAMO_ENDPOINT=http://localhost:8000 cargo test -p storage --test dynamo dynamo_delete_table_removes_it`
Expected: PASS. Also run `cargo test -p storage` (ungated) — still green.

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/dynamo.rs crates/storage/tests/dynamo.rs
git commit -m "feat: add DynamoRepository::delete_table for e2e teardown"
```

### Task 2: `xtask drop-table` subcommand

**Files:**
- Modify: `crates/xtask/src/main.rs` (the clap `Command` enum + the match arm)

- [ ] **Step 1: Add the subcommand**

In `crates/xtask/src/main.rs`, add a `DropTable` variant to the subcommand enum (alongside `Import` / `Seed`) with a doc comment `/// Drop the DynamoDB table named by XPOOL_TABLE (e2e teardown).`, and a match arm:

```rust
        Command::DropTable => {
            let repo = DynamoRepository::from_env().await?;
            repo.delete_table().await?;
            println!("dropped table {}", std::env::var("XPOOL_TABLE").unwrap_or_default());
        }
```

(`main.rs` already builds a `DynamoRepository::from_env()` for the other commands — match the existing style; do **not** call `ensure_table()` for this command.)

- [ ] **Step 2: Verify it builds and runs**

Run: `cargo run -p xtask -- drop-table` (with `docker compose up -d` and `DYNAMO_ENDPOINT` set, `XPOOL_TABLE=throwaway`).
Expected: prints `dropped table throwaway`, exit 0. Run again — still exit 0 (idempotent).

- [ ] **Step 3: Commit**

```bash
git add crates/xtask/src/main.rs
git commit -m "feat: add xtask drop-table subcommand"
```

### Task 3: e2e uses a fresh table per run

**Files:**
- Modify: `web/scripts/e2e-stack.sh`
- Modify: `web/scripts/e2e-teardown.sh`

- [ ] **Step 1: Generate a unique table in `e2e-stack.sh`**

In `web/scripts/e2e-stack.sh`, after the `AWS_*` exports (~line 30), add:

```bash
# A fresh table per run — isolates this run from every previous run.
# DynamoDB Local is in-memory and the container is long-lived, so the table
# name must be unique; teardown drops it.
export XPOOL_TABLE="xpool-e2e-$(date +%s)"
TABLE_FILE="$REPO_ROOT/web/.e2e-table"
echo "$XPOOL_TABLE" > "$TABLE_FILE"
log "using fresh table $XPOOL_TABLE"
```

`import`, `seed`, and the API binary all inherit `XPOOL_TABLE` from the
environment (they read it via `DynamoRepository::from_env`) — no other change
to the script is needed.

- [ ] **Step 2: Drop the table in `e2e-teardown.sh`**

In `web/scripts/e2e-teardown.sh`, after the API is stopped, add:

```bash
TABLE_FILE="$REPO_ROOT/web/.e2e-table"
if [ -f "$TABLE_FILE" ]; then
  XPOOL_TABLE="$(cat "$TABLE_FILE")"
  export XPOOL_TABLE
  echo "[e2e-teardown] dropping table $XPOOL_TABLE"
  cargo run -q -p xtask -- drop-table || true
  rm -f "$TABLE_FILE"
fi
```

Use the same `cargo()`/`mise exec` wrapper and `REPO_ROOT`/`DYNAMO_ENDPOINT`/`AWS_*`
setup the script already uses for the API; if `e2e-teardown.sh` does not set
those, copy the relevant `export` lines from `e2e-stack.sh`.

- [ ] **Step 3: Add `.e2e-table` to gitignore**

Append `web/.e2e-table` to `.gitignore` (next to the existing `web/.e2e-api.pid` / `.e2e-api.log` entries; add those too if absent).

- [ ] **Step 4: Verify isolation**

Run `cd web && npm run e2e` **twice in a row**.
Expected: both runs are fully green (16 tests). Previously, pool/prediction
state from run 1 leaked into run 2; now each run has its own table.

- [ ] **Step 5: Commit**

```bash
git add web/scripts/e2e-stack.sh web/scripts/e2e-teardown.sh .gitignore
git commit -m "test: give the e2e suite a fresh DynamoDB table per run"
```

---

## Phase 2 — API clock seam

### Task 4: `resolve_now` clock resolution

**Files:**
- Create: `crates/api/src/clock.rs`
- Modify: `crates/api/src/lib.rs` (add `pub mod clock;`)
- Test: `crates/api/src/clock.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

Create `crates/api/src/clock.rs`:

```rust
//! The request clock seam (`.specs/TESTING.md` §3.2).
//!
//! Every request resolves a single `now`, in priority order:
//!   1. the `X-Dev-Now` header   (per-request override — dev/test stub)
//!   2. the `XPOOL_NOW` env var   (process-wide default — dev/test stub)
//!   3. `Utc::now()`              (production)
//!
//! `X-Dev-Now` / `XPOOL_NOW` are dev stubs with the same exposure as
//! `X-Dev-Player` — they must be gated off before any real deployment.

use chrono::{DateTime, Utc};

/// `now`, placed in the GraphQL context. Resolvers read it via [`now`].
#[derive(Clone, Copy, Debug)]
pub struct RequestNow(pub DateTime<Utc>);

/// Parse an RFC3339 instant; `None` if it does not parse.
fn parse_instant(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s.trim())
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Resolve `now` from an optional header value and env value. Pure — the
/// real header/env reads happen in the caller, so this is fully testable.
pub fn resolve_now_from(
    header: Option<&str>,
    env: Option<&str>,
    real_now: DateTime<Utc>,
) -> DateTime<Utc> {
    header
        .and_then(parse_instant)
        .or_else(|| env.and_then(parse_instant))
        .unwrap_or(real_now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn header_wins_over_env_and_real() {
        let got = resolve_now_from(
            Some("2026-06-20T12:00:00Z"),
            Some("2026-07-01T00:00:00Z"),
            t("2026-05-17T00:00:00Z"),
        );
        assert_eq!(got, t("2026-06-20T12:00:00Z"));
    }

    #[test]
    fn env_used_when_no_header() {
        let got = resolve_now_from(None, Some("2026-07-01T00:00:00Z"), t("2026-05-17T00:00:00Z"));
        assert_eq!(got, t("2026-07-01T00:00:00Z"));
    }

    #[test]
    fn real_now_used_when_nothing_set() {
        let real = t("2026-05-17T00:00:00Z");
        assert_eq!(resolve_now_from(None, None, real), real);
    }

    #[test]
    fn unparseable_header_falls_through_to_env() {
        let got = resolve_now_from(Some("not-a-date"), Some("2026-07-01T00:00:00Z"), t("2026-05-17T00:00:00Z"));
        assert_eq!(got, t("2026-07-01T00:00:00Z"));
    }

    #[test]
    fn unparseable_everything_falls_through_to_real() {
        let real = t("2026-05-17T00:00:00Z");
        assert_eq!(resolve_now_from(Some("xxx"), Some("yyy"), real), real);
    }
}
```

Add `pub mod clock;` to `crates/api/src/lib.rs` next to the other `pub mod` lines.

- [ ] **Step 2: Run test to verify it fails, then passes**

Run: `cargo test -p api --lib clock`
Expected: the module compiles and all 5 tests PASS immediately — this is acceptable here because the production code (`resolve_now_from`) and its tests are written together in one pure module; the tests still demonstrate each branch. If any fails, fix `resolve_now_from`.

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/clock.rs crates/api/src/lib.rs
git commit -m "feat: add the request clock seam (resolve_now_from)"
```

### Task 5: wire `now` through the router and resolvers

**Files:**
- Modify: `crates/api/src/clock.rs` (add the header/env wrapper)
- Modify: `crates/api/src/router.rs` (inject `RequestNow` into context)
- Modify: `crates/api/src/gql/query.rs` (`tips` reads ctx `now`)
- Modify: `crates/api/src/recompute.rs` (`recompute` takes `now`)
- Modify: `crates/api/src/gql/mutation.rs` (`enter_result` passes ctx `now`)
- Test: `crates/api/tests/graphql.rs`

- [ ] **Step 1: Add the wrapper to `clock.rs`**

Append to `crates/api/src/clock.rs`:

```rust
use axum::http::HeaderMap;

/// Resolve `now` for a real request: `X-Dev-Now` header, then `XPOOL_NOW`
/// env, then the real clock.
pub fn resolve_now(headers: &HeaderMap) -> DateTime<Utc> {
    let header = headers.get("x-dev-now").and_then(|v| v.to_str().ok());
    let env = std::env::var("XPOOL_NOW").ok();
    resolve_now_from(header, env.as_deref(), Utc::now())
}
```

- [ ] **Step 2: Inject `RequestNow` in the router**

In `crates/api/src/router.rs`, in `graphql_handler`, after the `current` line:

```rust
    let current = resolve_current_player(state.repo.as_ref(), &headers).await;
    let now = crate::clock::RequestNow(crate::clock::resolve_now(&headers));
    let req = req.into_inner().data(current).data(now);
    state.schema.execute(req).await.into()
```

- [ ] **Step 3: Add a context helper and use it in `tips`**

In `crates/api/src/gql/query.rs`, add near the `repo` helper:

```rust
/// The request's `now` (the clock seam — `.specs/TESTING.md` §3.2).
fn now(ctx: &Context<'_>) -> chrono::DateTime<chrono::Utc> {
    ctx.data_unchecked::<crate::clock::RequestNow>().0
}
```

In the `tips` resolver, replace `let now = Utc::now();` with `let now = now(ctx);`.

- [ ] **Step 4: `recompute` takes `now`**

In `crates/api/src/recompute.rs`: change the signature to
`pub async fn recompute(repo: &dyn Repository, now: DateTime<Utc>) -> anyhow::Result<()>`,
remove `use chrono::Utc;` (add `use chrono::{DateTime, Utc};` only if `Utc` is still
referenced — it is not after this change, so import `DateTime` alone), and delete
the `let now = Utc::now();` line (the parameter replaces it).

In `crates/api/src/gql/mutation.rs`, the `enter_result` resolver calls
`recompute(repo.as_ref())` — change it to `recompute(repo.as_ref(), now(ctx)?)`.
`enter_result` is in `mutation.rs` which has its own `repo` helper; add a `now`
helper there too (copy the one from `query.rs`), and it is **not** fallible —
use `recompute(repo.as_ref(), now(ctx))`.

- [ ] **Step 5: Write the failing test**

Add to `crates/api/tests/graphql.rs`. The `run` helper (in `common/mod.rs`)
must forward an `X-Dev-Now` value into the request context. First extend it:
in `common/mod.rs`, find `run(...)` — it builds an async-graphql `Request` and
`.data(...)`s the `CurrentPlayer`. Add a sibling helper:

```rust
/// Like `run`, but also injects a fixed request `now`.
pub async fn run_at(
    repo: &Arc<dyn Repository>,
    query: &str,
    vars: Variables,
    player: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> async_graphql::Response {
    // ... identical to `run`, but the built Request also gets
    // `.data(api::clock::RequestNow(now))`
}
```

(Copy `run`'s body; add `.data(api::clock::RequestNow(now))` to the `Request`.
Also make `run` itself inject `RequestNow(Utc::now())` so existing tests still
have a `now` in context — otherwise `ctx.data_unchecked::<RequestNow>()` panics.)

Then the new test:

```rust
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

    // Viewed by ALICE *before* kickoff → Bob's draft is hidden.
    let before = run_at(
        &repo,
        r#"query($g: ID!){ tips(groupId:$g){ playerId prediction{ homeScore } } }"#,
        Variables::from_json(json!({ "g": GROUP_A })),
        Some(ALICE),
        Utc::now(),
    )
    .await;
    let bob_before = find_tip(&before, BOB, GAME_1);
    assert!(bob_before["prediction"].is_null(), "hidden before kickoff");

    // Viewed with the clock advanced past kickoff → Bob's draft is revealed.
    let after = run_at(
        &repo,
        r#"query($g: ID!){ tips(groupId:$g){ playerId prediction{ homeScore } } }"#,
        Variables::from_json(json!({ "g": GROUP_A })),
        Some(ALICE),
        Utc::now() + Duration::hours(48),
    )
    .await;
    let bob_after = find_tip(&after, BOB, GAME_1);
    assert_eq!(bob_after["prediction"]["homeScore"], json!(1), "revealed after kickoff");
}
```

Add a small `find_tip(resp, player_id, game_id) -> serde_json::Value` helper at
the bottom of `graphql.rs` that pulls one tip out of `data(resp)["tips"]`.

- [ ] **Step 6: Run, verify fail then pass**

Run: `cargo test -p api --test graphql tips_visibility_uses_the_request_clock`
Expected: FAIL first (the test asserts behaviour driven by the new `run_at` clock);
after Steps 1–4 it PASSES. Then `cargo test --workspace` — all green.

- [ ] **Step 7: Commit**

```bash
git add crates/api
git commit -m "feat: resolve request now from the clock seam, not Utc::now()"
```

---

## Phase 3 — server-derived time flags

### Task 6: `timeflags` pure helpers

**Files:**
- Create: `crates/api/src/timeflags.rs`
- Modify: `crates/api/src/lib.rs` (`pub mod timeflags;`)
- Test: `crates/api/src/timeflags.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the module with tests**

Create `crates/api/src/timeflags.rs`:

```rust
//! Pure time-derived flags the API exposes so the SPA renders, not computes,
//! time-dependent state (`.specs/TESTING.md` §3.3).

use chrono::{DateTime, Duration, Utc};
use domain::Round;

/// ±2-day half-window for the "Today / Fresh" screen (UC-11).
const TODAY_WINDOW: Duration = Duration::days(2);

/// Buffer after kickoff before a result is expected: a 90-minute match needs
/// ~1h45; a knockout match may run to extra time / penalties (`API.md` §7).
pub fn result_buffer(round: Round) -> Duration {
    match round {
        Round::GroupStage => Duration::minutes(105),
        _ => Duration::minutes(150),
    }
}

/// A group's deadline has passed.
pub fn deadline_passed(deadline: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    deadline.is_some_and(|d| now > d)
}

/// A match is result-pending: its estimated end has passed and no locked
/// official result exists yet — this is what drives smart polling.
pub fn result_pending(
    kickoff: DateTime<Utc>,
    round: Round,
    has_locked_result: bool,
    now: DateTime<Utc>,
) -> bool {
    !has_locked_result && now > kickoff + result_buffer(round)
}

/// A match falls within the ±2-day Today window.
pub fn within_today_window(kickoff: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    (kickoff - now).abs() <= TODAY_WINDOW
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn deadline_passed_is_false_before_and_true_after() {
        let d = Some(t("2026-06-20T12:00:00Z"));
        assert!(!deadline_passed(d, t("2026-06-20T11:00:00Z")));
        assert!(deadline_passed(d, t("2026-06-20T13:00:00Z")));
        assert!(!deadline_passed(None, t("2026-06-20T13:00:00Z")));
    }

    #[test]
    fn result_pending_true_after_buffer_when_no_result() {
        let ko = t("2026-06-20T18:00:00Z");
        // Group buffer = 105 min → pending at 20:00, not at 19:00.
        assert!(!result_pending(ko, Round::GroupStage, false, t("2026-06-20T19:00:00Z")));
        assert!(result_pending(ko, Round::GroupStage, false, t("2026-06-20T20:00:00Z")));
    }

    #[test]
    fn result_pending_false_once_a_result_is_locked() {
        let ko = t("2026-06-20T18:00:00Z");
        assert!(!result_pending(ko, Round::GroupStage, true, t("2026-06-20T23:00:00Z")));
    }

    #[test]
    fn knockout_uses_the_longer_buffer() {
        let ko = t("2026-07-10T18:00:00Z");
        // 150-min buffer → not pending at 20:00, pending at 21:00.
        assert!(!result_pending(ko, Round::QF, false, t("2026-07-10T20:00:00Z")));
        assert!(result_pending(ko, Round::QF, false, t("2026-07-10T21:00:00Z")));
    }

    #[test]
    fn within_today_window_spans_two_days_either_side() {
        let now = t("2026-06-20T12:00:00Z");
        assert!(within_today_window(t("2026-06-21T12:00:00Z"), now));
        assert!(within_today_window(t("2026-06-19T12:00:00Z"), now));
        assert!(!within_today_window(t("2026-06-23T13:00:00Z"), now));
    }
}
```

Add `pub mod timeflags;` to `crates/api/src/lib.rs`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p api --lib timeflags`
Expected: 5 tests PASS. If a buffer/window boundary test fails, fix the constant.

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/timeflags.rs crates/api/src/lib.rs
git commit -m "feat: add pure timeflags helpers (deadline/result-pending/today)"
```

### Task 7: expose the flags on the GraphQL types

**Files:**
- Modify: `crates/api/src/gql/types.rs` (`Game`, `Group`, `Tournament` builders)
- Modify: `crates/api/src/gql/query.rs` (`tournament` resolver; new `now` field)
- Test: `crates/api/tests/graphql.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/api/tests/graphql.rs`:

```rust
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

    // Clock = real now → deadline not passed, nothing result-pending,
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

    // Clock = 10 days later → deadline passed, results pending, games are
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
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p api --test graphql tournament_exposes_time_flags_against_the_request_clock`
Expected: FAIL — unknown fields `now`, `deadlinePassed`, `resultPending`, `withinTodayWindow`.

- [ ] **Step 3: Add the fields to the GraphQL types**

In `crates/api/src/gql/types.rs`:

Add to `struct Game`: `pub result_pending: bool,` and `pub within_today_window: bool,`.
Add to `struct Group`: `pub deadline_passed: bool,`.

Replace `impl From<&domain::SingleGame> for Game` with a builder (the flags
need `now`, the game's `Round`, and whether a locked result exists):

```rust
impl Game {
    fn build(
        g: &domain::SingleGame,
        round: domain::Round,
        now: chrono::DateTime<chrono::Utc>,
        locked_result_game_ids: &std::collections::HashSet<String>,
    ) -> Self {
        Game {
            id: g.id.clone(),
            kickoff: g.kickoff,
            venue: g.venue.clone(),
            group_id: g.group_id.clone(),
            home: (&g.home).into(),
            away: (&g.away).into(),
            result_pending: crate::timeflags::result_pending(
                g.kickoff, round, locked_result_game_ids.contains(&g.id), now,
            ),
            within_today_window: crate::timeflags::within_today_window(g.kickoff, now),
        }
    }
}
```

Change `Group::from_node` to `Group::build(g, tournament, now)` and set
`deadline_passed: crate::timeflags::deadline_passed(tournament.deadline(&g.id), now)`
(keep the existing `deadline` field).

Change `impl From<&domain::Tournament> for Tournament` to a builder
`Tournament::build(t, now, locked_result_game_ids)` that maps groups via
`Group::build(g, t, now)` and games via `Game::build(g, round_of(g), now, ids)`,
where `round_of` looks up `t.groups[&g.group_id].round`.

- [ ] **Step 4: Update the `tournament` resolver and add `now`**

In `crates/api/src/gql/query.rs`, rewrite the `tournament` resolver:

```rust
    /// The `<t>#TOURNAMENT` structure with time-derived flags (`TESTING.md` §3.3).
    async fn tournament(&self, ctx: &Context<'_>) -> async_graphql::Result<Option<Tournament>> {
        let repo = repo(ctx);
        let Some(t) = repo.get_tournament().await? else {
            return Ok(None);
        };
        // Locked official results = the result user's locked match predictions.
        let players = repo.list_players().await?;
        let locked: std::collections::HashSet<String> = players
            .iter()
            .find(|p| p.is_result_user)
            .map(|r| {
                r.match_predictions
                    .iter()
                    .filter(|p| p.locked)
                    .map(|p| p.game_id.clone())
                    .collect()
            })
            .unwrap_or_default();
        Ok(Some(Tournament::build(&t, now(ctx), &locked)))
    }

    /// The request's resolved `now` — the server clock the SPA renders against.
    async fn now(&self, ctx: &Context<'_>) -> chrono::DateTime<chrono::Utc> {
        now(ctx)
    }
```

`tournament` becomes slightly less coarse (one extra `list_players`) — that is
acceptable and noted in `API.md`.

- [ ] **Step 5: Run, verify pass**

Run: `cargo test -p api --test graphql` then `cargo test --workspace`.
Expected: all green, including the new test. Run `cargo clippy -p api`.

- [ ] **Step 6: Update `API.md`**

In `.specs/API.md` §4, note the `now` query and that `tournament` now loads
players to derive `resultPending`. Add `deadlinePassed` / `resultPending` /
`withinTodayWindow` to the type description.

- [ ] **Step 7: Commit**

```bash
git add crates/api .specs/API.md
git commit -m "feat: expose server-derived time flags on the GraphQL schema"
```

---

## Phase 4 — frontend renders flags, drops `Date.now()`

### Task 8: GraphQL documents and types

**Files:**
- Modify: `web/src/graphql/queries.ts`
- Modify: `web/src/graphql/types.ts`

- [ ] **Step 1: Update `TOURNAMENT_QUERY`**

In `web/src/graphql/queries.ts`, `TOURNAMENT_QUERY`: add `deadlinePassed` to the
`groups { ... }` selection, add `resultPending withinTodayWindow` to the
`games { ... }` selection, and add a top-level `now` field beside `tournament`.

- [ ] **Step 2: Update the types**

In `web/src/graphql/types.ts`: add `deadlinePassed: boolean` to `Group`, and
`resultPending: boolean` + `withinTodayWindow: boolean` to `SingleGame`.

- [ ] **Step 3: Verify build**

Run: `cd web && npm run build`
Expected: PASS (the new fields are optional to consumers until Task 9).

- [ ] **Step 4: Commit**

```bash
git add web/src/graphql/queries.ts web/src/graphql/types.ts
git commit -m "feat: query server-derived time flags from the SPA"
```

### Task 9: consume flags, remove `Date.now()` logic

**Files:**
- Modify: `web/src/lib/polling.ts`
- Modify: `web/src/pages/TodayPage.tsx`
- Modify: `web/src/pages/mytips/GroupTipForm.tsx`
- Modify: `web/src/pages/ScoreboardPage.tsx` and any other `pollIntervalMs` caller

- [ ] **Step 1: Simplify `polling.ts`**

Replace the whole body of `web/src/lib/polling.ts` with:

```ts
import type { SingleGame } from '../graphql/types'

/**
 * Poll only while at least one loaded match is result-pending. Whether a
 * match is result-pending is decided by the server (`Game.resultPending`,
 * `.specs/TESTING.md` §3.3) — the SPA no longer computes time.
 */
export function pollIntervalMs(games: SingleGame[]): number {
  return games.some((g) => g.resultPending) ? 30_000 : 0
}
```

`bufferFor` and `isResultPending` are deleted (the logic now lives in
`crates/api/src/timeflags.rs`, covered by Task 6's tests).

- [ ] **Step 2: Update `pollIntervalMs` call sites**

In `web/src/pages/TodayPage.tsx` and `web/src/pages/ScoreboardPage.tsx` (and any
other caller — grep `pollIntervalMs`), change the call to `pollIntervalMs(games)`
— drop the `resultGameIds` set and the `now` argument. Remove now-unused
`useMemo`/`Set` plumbing that only fed the old signature.

- [ ] **Step 3: `TodayPage` filters on the flag**

In `web/src/pages/TodayPage.tsx`: delete the `const [now] = useState(() => Date.now())`
line and the `WINDOW_MS` constant; change the `.filter(...)` to
`.filter((g) => g.withinTodayWindow)`.

- [ ] **Step 4: `GroupTipForm` reads `deadlinePassed`**

In `web/src/pages/mytips/GroupTipForm.tsx`: delete `const [now] = useState(() => Date.now())`;
replace the `deadlinePassed` computation with `const deadlinePassed = group.deadlinePassed`.
(`group` is the `Group` passed in — it now carries the flag.)

- [ ] **Step 5: Verify**

Run: `cd web && npm run build && npm run lint`
Expected: PASS. `grep -rn "Date.now()" web/src` should return only formatting
uses (e2e unique-name generation aside) — no behavioural ones.

- [ ] **Step 6: Commit**

```bash
git add web/src/lib/polling.ts web/src/pages
git commit -m "refactor: SPA renders server time flags, no Date.now() logic"
```

---

## Phase 5 — dev clock control

### Task 10: `X-Dev-Now` header + auth-bar clock picker

**Files:**
- Modify: `web/src/auth/devAuth.ts`
- Modify: `web/src/graphql/client.ts`
- Modify: `web/src/components/AuthBar.tsx`
- Modify: `web/src/i18n/strings.ts`

- [ ] **Step 1: Persist the dev clock**

In `web/src/auth/devAuth.ts`, add (mirroring `getDevPlayerId`):

```ts
const NOW_KEY = 'xpool.devNow'

/** The dev clock override (RFC3339), or null for the real clock. */
export function getDevNow(): string | null {
  try {
    return localStorage.getItem(NOW_KEY)
  } catch {
    return null
  }
}

export function setDevNow(iso: string): void {
  try {
    localStorage.setItem(NOW_KEY, iso)
  } catch {
    /* ignore */
  }
}

export function clearDevNow(): void {
  try {
    localStorage.removeItem(NOW_KEY)
  } catch {
    /* ignore */
  }
}
```

- [ ] **Step 2: Send `X-Dev-Now`**

In `web/src/graphql/client.ts`, in the `fetchOptions` function, after the
`X-Dev-Player` block:

```ts
      const devNow = getDevNow()
      if (devNow) {
        headers['X-Dev-Now'] = devNow
      }
```

Add `getDevNow` to the `import` from `../auth/devAuth`.

- [ ] **Step 3: Add the clock control to the auth bar**

In `web/src/components/AuthBar.tsx`, render a `<input type="datetime-local">`
(plus a "real time" reset button) inside the `.auth-bar`, in **both** the
logged-in and visitor branches — extract a small `<DevClock />` component in
the same file to avoid duplication:

```tsx
function DevClock() {
  const { t } = useI18n()
  const current = getDevNow()
  // datetime-local wants 'YYYY-MM-DDTHH:mm'; store/send full RFC3339 (UTC).
  const value = current ? current.slice(0, 16) : ''
  const onChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.value) {
      setDevNow(new Date(e.target.value).toISOString())
    } else {
      clearDevNow()
    }
    location.reload() // simplest correct cache reset for the new clock
  }
  return (
    <span className="dev-clock">
      <label>
        {t('devClock')}
        <input type="datetime-local" value={value} onChange={onChange} />
      </label>
      {current && (
        <button type="button" onClick={() => { clearDevNow(); location.reload() }}>
          {t('devClockReset')}
        </button>
      )}
    </span>
  )
}
```

Import `getDevNow, setDevNow, clearDevNow` from `../auth/devAuth`. Render
`<DevClock />` in the auth bar.

- [ ] **Step 4: i18n strings**

In `web/src/i18n/strings.ts`, add to the `en` block and the `hu` block:
`devClock: 'Dev clock'` / `devClock: 'Dev óra'`, and
`devClockReset: 'real time'` / `devClockReset: 'valós idő'`.

- [ ] **Step 5: Verify**

Run: `cd web && npm run build && npm run lint`
Expected: PASS. Manually (or in Task 11's spec): set the clock, observe the
schedule/today/scoreboard change.

- [ ] **Step 6: Commit**

```bash
git add web/src
git commit -m "feat: dev clock control sending the X-Dev-Now header"
```

---

## Phase 6 — e2e time + time-dependent spec

### Task 11: pin the e2e clock and add a time spec

**Files:**
- Modify: `web/scripts/e2e-stack.sh`
- Create: `web/e2e/time.spec.ts`

- [ ] **Step 1: Default the e2e clock into the tournament window**

In `web/scripts/e2e-stack.sh`, near the other `export`s, add:

```bash
# Default the API clock to mid-tournament so the seeded fixture is "live".
# Individual tests override per-request via the dev clock (X-Dev-Now).
export XPOOL_NOW="${XPOOL_NOW:-2026-06-20T12:00:00Z}"
log "API clock (XPOOL_NOW) = $XPOOL_NOW"
```

- [ ] **Step 2: Write the time spec**

Create `web/e2e/time.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * The dev clock (X-Dev-Now) is the server-authoritative test clock
 * (.specs/TESTING.md §3). Moving it changes time-dependent screens — proof
 * the whole clock seam works end to end.
 */

/** Set the dev clock via localStorage before the app loads. */
async function setDevClock(page: import('@playwright/test').Page, iso: string) {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, iso)
}

test('Today is empty well before the tournament, populated during it', async ({
  page,
}) => {
  const net = watchNetwork(page)

  // Clock far before any match → Today window catches nothing.
  await setDevClock(page, '2026-01-01T12:00:00Z')
  await page.goto('/today')
  await expect(page.getByText('No matches near now.')).toBeVisible()
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()

  // Clock during the group stage → Today shows matches.
  await setDevClock(page, '2026-06-20T12:00:00Z')
  await page.goto('/today')
  await expect(page.getByText('No matches near now.')).toHaveCount(0)
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

(If the seeded fwc26 fixture has no match within ±2 days of `2026-06-20`,
adjust the second instant to a date that does — verify against
`tournaments/fwc26.json` kickoff times. The first instant just needs to be
> 2 days from any match.)

- [ ] **Step 3: Run the e2e suite**

Run: `cd web && npm run e2e`
Expected: all specs green, including `time.spec.ts`. Run it **twice** — still
green both times (Phase 1 isolation holds).

- [ ] **Step 4: Commit**

```bash
git add web/scripts/e2e-stack.sh web/e2e/time.spec.ts
git commit -m "test: pin the e2e clock and add a time-dependent e2e spec"
```

### Task 12: reconcile docs and final verification

**Files:**
- Modify: `.specs/SCENARIOS.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update `SCENARIOS.md` test links**

In `.specs/SCENARIOS.md`, fill the `Tests:` field for time-dependent scenarios
now covered: PRED-07/08/09/10 (clock-driven API tests), BROWSE-04 (Today —
`time.spec.ts`), and note the `timeflags` unit tests. Remove stale `Tests: —`
where a test now exists.

- [ ] **Step 2: Note the clock + e2e isolation in `CLAUDE.md`**

In `CLAUDE.md`, under "Running locally" / testing notes, add one line: the API
clock is overridable via `XPOOL_NOW` / `X-Dev-Now` and e2e uses a fresh table
per run — pointer to `.specs/TESTING.md`.

- [ ] **Step 3: Full verification**

Run, and confirm each is green:
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cd web && npm run build && npm run lint`
- `cd web && npm run e2e` (twice)

- [ ] **Step 4: Commit**

```bash
git add .specs/SCENARIOS.md CLAUDE.md
git commit -m "docs: reconcile SCENARIOS.md + CLAUDE.md with the clock seam"
```

---

## Self-review notes

- **Spec coverage:** `TESTING.md` §2 isolation → Phase 1. §3.2 API clock →
  Phase 2. §3.3 derived flags → Phase 3 (API) + Phase 4 (SPA). §3.4 dev clock →
  Phase 5. e2e clock → Phase 6. §5 security (gating the dev stubs) is
  deliberately **out of scope** — it is tracked with the Auth0 work
  (`TESTING.md` §6); this plan adds the stub, the auth plan gates it.
- **Type consistency:** `RequestNow` (clock.rs) — constructed in `router.rs`
  and `common/mod.rs`, read by `now(ctx)` in `query.rs`/`mutation.rs`.
  `Tournament::build` / `Group::build` / `Game::build` — defined in Task 7,
  used only there. `pollIntervalMs(games)` — new 1-arg signature defined in
  Task 9 Step 1, all call sites updated in Step 2.
- **Pre-existing clippy debt:** `crates/domain/tests/scoring.rs` has 5
  `useless use of vec!` warnings unrelated to this plan; Task 12's
  `clippy -D warnings` gate will trip on them. Fix them in Task 12 Step 3 as a
  trivial drive-by (`vec![...]` → `[...]`) or they block the gate.
