# TheSportsDB — Reported Results (assisted official-result entry)

**Status:** design approved, ready for implementation plan
**Date:** 2026-06-14
**Scope:** a shared `sportsdb` integration foundation + assisted official-result
entry (admin pre-fill + confirm). A live provisional-points preview (#2) is
documented as the next consumer of the same foundation but is **not** built here.

---

## 1. Motivation & decomposition

Entering official match results by hand (via the `submitGroup` mutation) is
tedious. [TheSportsDB](https://www.thesportsdb.com) already provides the finished
scores. We want it to **save the admin typing** without ever becoming the
authority on results.

Three related ideas surfaced; they decompose into separate deliverables:

| # | Deliverable | TheSportsDB role | Status |
|---|---|---|---|
| **1** | **Assisted official-result entry** | **core** — fetch finished scores, pre-fill the result-user entry form | **this spec** |
| 2 | Match-detail page with live provisional-points preview | *core (livescore)* — show "points you'd earn if it ended now" | next spec, reuses this foundation |
| 3 | Player-detail page (full prediction view) | *none* — "player" = pool participant, pure xpool data | separate spec |

This spec builds the **shared foundation** (a SportsDB client + the
game↔event mapping + score fetch) and **#1** as its first consumer. #2 then
drops on top with no new SportsDB plumbing.

### Decisions locked during design review

- **Trust model: pre-fill + admin confirms.** SportsDB only populates the
  entry form; the human reviews and submits via the existing `submitGroup`
  path. Auto-*writing* official results is explicitly rejected — a wrong/late
  SportsDB score must never silently corrupt the scoreboard or bracket.
- **Auto-fetch trigger: on-open (client-driven).** Opening a group's
  entry screen auto-fetches reported results for that group's `resultPending`
  games (pre-filled, no click). A background/scheduled fetch with an app-wide
  "pending" badge is a deferred enhancement (see §9).
- **Mapping: reviewed, git-committed.** The `M# → idEvent` link is backfilled
  once into `tournaments/fwc26.json` under human review, consistent with the
  project's "hand-curated, committed tournament definition" philosophy
  (`.specs/DATA_SOURCES.md`).
- **No Rust SDK exists.** crates.io has no TheSportsDB crate (searched
  `thesportsdb` / `sportsdb` / `the-sports-db`; the sports-data crates that
  exist wrap other providers and are mostly stale). We write a thin client —
  structured so it could be **published as an open-source Rust SDK** later
  (see §4).

---

## 2. Rate limits (why this is cheap)

| Tier | Limit | Notes |
|---|---|---|
| Free (key `123`) | 30 req/min | V1 only, no livescore |
| **Premium** (our key) | **100 req/min** | V2 + livescore |
| Business | 120 req/min | — |

The **league-wide** endpoints return everything in one call
(`/schedule/previous/league/4429` = all recently-finished matches;
`/livescore/4429` = all in-progress). So even aggressive polling is single-digit
req/min — rate limits are a **non-constraint**, *provided we always pull
league-wide and filter locally* and never loop per-`idEvent`.

---

## 3. Architecture

A new crate `crates/sportsdb` is the only code that knows about TheSportsDB.
It is a pure I/O boundary: given a key + HTTP client, it exposes typed calls and
returns plain structs. Consumed by both `xtask` (reconcile) and `api`
(reported-results query). `domain` stays SportsDB-agnostic.

```
domain ─▶ fwc26 ─┐
   ├────▶ storage ┼─▶ api ─▶ web
   └────▶ sportsdb┤        (new crate, also used by xtask)
          xtask ──┘
```

### Layer separation (vocabulary)

The naming deliberately keeps frontend workflow words out of the lower layers:

| Layer | Representation | Vocabulary |
|---|---|---|
| `domain` | *(nothing)* | pure — no SportsDB, no "external result" concept |
| `sportsdb` | `Event` (raw wire shape) | upstream's terms (`idEvent`, `intHomeScore`) |
| `api` GraphQL | `ReportedResult` | **provenance**-named, not workflow-named |
| `web` | renders "from SportsDB — confirm" | the "suggestion" framing lives here |

A SportsDB score is neither *predicted* nor *official* — it is a **reported**
real-world result. Whether the UI treats it as a suggestion-to-confirm (#1) or a
live-preview input (#2) is the consumer's choice, not a property of the type.

---

## 4. The `sportsdb` crate

Thin, typed, **V2 only** (header auth). Structured for a possible future
open-source publish: a clean public API, its own error type, **no xpool-specific
types** leaking in, and no dependency on `domain`/`storage`.

```rust
pub struct SportsDb { http: reqwest::Client, key: String }

impl SportsDb {
    /// None when THESPORTSDB_API_KEY is unset/empty.
    pub fn from_env() -> Option<Self>;

    pub async fn season_schedule(&self) -> Result<Vec<Event>, Error>; // /schedule/league/4429/2026
    pub async fn teams(&self)           -> Result<Vec<TeamRow>, Error>; // /list/teams/4429
    pub async fn finished_results(&self)-> Result<Vec<Event>, Error>; // /schedule/previous/league/4429
    // #2 later: pub async fn livescores(&self) -> Result<Vec<Event>, Error>;  // /livescore/4429
}

/// Only the fields xpool uses; the V2 envelope (.schedule/.list/.livescore)
/// is decoded inside the crate so callers never see it.
pub struct Event {
    pub id_event: String,
    pub date_event: String,
    pub id_home_team: String,
    pub id_away_team: String,
    pub int_home_score: Option<i64>,
    pub int_away_score: Option<i64>,
    pub str_status: String,
}
```

- **Key:** env var `THESPORTSDB_API_KEY`, matching the project's existing
  env-injected secret pattern (`CLOUDFRONT_SECRET`, `AUTH0_*`). No runtime SSM
  dependency.
- **League/season constants** (`idLeague 4429`, season `2026`) live in this
  crate (or are passed in) — out of `domain`.
- **Resilience:** ~5s timeout, one retry, errors logged not surfaced. Each
  refresh is 1–2 HTTP calls.

---

## 5. The mapping (`M# ↔ idEvent`)

`domain::model::SingleGame` gains one additive field (mirrors `Team`):

```rust
pub struct SingleGame {
    // ...existing...
    pub external_id: Option<String>,   // TheSportsDB idEvent
}
```

`Option` + serde default → existing data deserializes unchanged. This is the one
touch to the `model.rs` locked contract.

### `xtask reconcile-events` (new subcommand, dev tool)

1. Fetch `season_schedule()` + `teams()` from `sportsdb`.
2. Match each xpool game to a SportsDB event by **(date, home team, away
   team)**, resolving teams via `Team.external_id` (idTeam) once matched —
   id-based, not name-guessing.
3. **Print a proposed `M# → idEvent` table for human review**, then write both
   game `external_id` and team `external_id` into `tournaments/fwc26.json`.
   The human reviews the diff and commits.
4. **Idempotent / re-runnable.** Knockout games (M73+) have no `idEvent` until
   SportsDB publishes them — they stay `null` and fill on a later re-run.
   Ambiguous or unmatched games are **reported, never silently dropped** (loud
   failure, per the importer philosophy).

Solving the fragile date+teams match **once under review** means the runtime
path is a trivial lookup by frozen id — immune to SportsDB renaming a team.

---

## 6. API surface

Admin-only (gated to the result user), reuses the existing write path entirely.

```graphql
reportedResults(groupId: ID!): [ReportedResult!]!

type ReportedResult {
  gameId: ID!
  homeScore: Int!
  awayScore: Int!
  source: String!                 # "thesportsdb"
  sourceStatus: String!           # SportsDB strStatus, e.g. "Match Finished"
  ninetyMinuteUncertain: Boolean! # true for knockout (ET/penalties ambiguity)
  fetchedAt: DateTime!
}
```

**Resolver:** load the group's games → keep those with an `external_id` whose
SportsDB event reports **finished** and that have **no confirmed official result
yet** (the server-clock `resultPending` condition) → map to `ReportedResult`.
Games without a mapping, in-progress, or already entered are simply absent.
If the `sportsdb` client is `None` or errors → returns `[]`.

The `sportsdb` client is injected into the resolver as a trait object so tests
substitute a stub and never hit the network.

**Caching:** a small in-process TTL (~45s) wraps the one league-wide
`finished_results()` call, so opening several group screens / an auto-fetch
followed by a manual refresh don't re-hit SportsDB. In-process is sufficient
(Lambda reuses warm containers; the call is cheap regardless).

### The 90-minute caveat

SportsDB's `intHomeScore/intAwayScore` is the *final* score; for knockout matches
that can include extra time/penalties, but xpool scores knockouts on the
**90-minute** result (`.specs/SCORING.md §5`). The schedule feed doesn't cleanly
separate 90' from ET, so knockout reported results carry
`ninetyMinuteUncertain: true` and the admin must verify/adjust before
submitting. Group-stage matches have no ET, so theirs are exact. This is exactly
why "pre-fill + confirm" is the right trust model rather than auto-write.

---

## 7. Frontend flow

The official write path is **100% unchanged** — SportsDB only populates the form.

1. Admin opens a group's result-entry screen.
2. If that group has `resultPending` games, the SPA **auto-calls
   `reportedResults(groupId)`** and pre-fills the matching score inputs,
   visually marked "from SportsDB — confirm" (and a warning when
   `ninetyMinuteUncertain`). No click required.
3. A manual **"Fetch from SportsDB"** button re-runs the query.
4. Admin reviews and submits via the existing **`submitGroup`** mutation →
   existing **`recompute()`** (scoreboard + knockout resolution).

If SportsDB is down/unconfigured, `reportedResults` returns `[]`, the form stays
empty, and **manual entry works exactly as today** — the feature only ever adds
convenience.

---

## 8. Testing

Per `.specs/TESTING.md` layers; no live network in any test.

- **`sportsdb` crate:** unit-decode recorded V2 JSON fixtures
  (`.schedule`/`.livescore` envelopes) → `Event`; an `httpmock`/`wiremock`
  test covers timeout + one-retry + error→empty degradation.
- **`reconcile-events`:** over a captured schedule fixture, assert the
  `M# → idEvent` matches and that ambiguous/missing games are reported.
- **`api` `reportedResults` resolver:** `InMemoryRepository` + stub `sportsdb`
  trait object — assert finished-only filtering, `external_id` mapping, the
  `resultPending`/already-entered exclusions, knockout `ninetyMinuteUncertain`,
  and `[]` when the client is absent.
- **E2E (Playwright):** stubbed SportsDB endpoint — open a group entry screen
  with a `resultPending` game, assert inputs auto-pre-fill, submit, scoreboard
  updates. Stub keeps the suite hermetic.

---

## 9. Out of scope (explicit)

- Auto-*writing* official results (rejected — stays admin-confirmed).
- Background/scheduled fetch + app-wide "pending" badge (deferred; would add an
  EventBridge schedule, a staging store, and a notification surface — reuses the
  same reported-results logic when built).
- The match-detail page (#2) and player-detail page (#3) themselves.
- Venue/badge image enrichment (`reconcile-events` backfills team `external_id`
  because matching needs it, but pulling images/venues is a separate nicety).
- Runtime SSM reads — Terraform injects the key as an env var.

---

## 10. Deployment delta

One Terraform change: a `data` source reads the existing SSM SecureString
`/xpool/<env>/thesportsdb-api-key` (already populated for dev + prod) and injects
it as the Lambda's `THESPORTSDB_API_KEY` env var. Local dev uses `.env`
(dotenvy's parent-dir walk-up reaches the main checkout's `.env` from a
worktree). The Lambda role already has SSM read granted (`infrastructure/lambda.tf`).

---

## 11. How #2 (live preview) reuses this — for reference, not built here

- Same `sportsdb` crate: add `livescores()` (`/livescore/4429`).
- Same `ReportedResult` type: live matches just carry a non-finished
  `sourceStatus`.
- Feed the live score into the existing **pure** `domain` scoring function to
  show each player's provisional points ("if it ended now") on the match page.

No new SportsDB plumbing — the foundation is the deliverable.
