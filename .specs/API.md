# xpool — API & Frontend Contract

The **decided** API contract and frontend-interaction design, settled in design
review. Authoritative.

See [`DATA_MODEL.md`](./DATA_MODEL.md) (entities, storage),
[`SCORING.md`](./SCORING.md) (the engine the resolvers call), and
[`REWRITE_USE_CASES.md`](./REWRITE_USE_CASES.md) (screens & journeys).

---

## 1. Shape

- **Backend:** a Rust **axum** app, portable — runs as a plain local HTTP server
  and wraps into Lambda via `lambda_http`.
- **Frontend:** a React + Vite **SPA** (single-page application) — a static
  bundle on S3 + CloudFront.
- **Transport:** CloudFront routes `/*` → S3 (the SPA), `/api/*` → the Lambda.
- **API style:** **GraphQL** — one `/api/graphql` endpoint.

## 2. Why GraphQL

The backend publishes the whole domain graph; the frontend selects exactly what
each screen needs. This was chosen over typed RPC to keep **frontend iteration
free** — a new screen or feature is usually a new query, not a backend change.

The usual GraphQL costs are defused by decisions already made:
- **N+1 queries** — neutralised by the coarse data model: a request loads the
  coarse items once, resolvers read from memory. No per-field I/O, no `DataLoader`.
- **Query abuse** — the only client is xpool's own SPA behind the auth seam; a
  simple query-depth cap is enough.
- **Edge caching** — not needed: the hot read (scoreboard) is materialised and
  the client caches.

## 3. Resolver discipline — keep them minimal

- Most GraphQL types are **`#[derive(SimpleObject)]`** on the domain structs —
  zero resolver code.
- Hand-written resolvers (`#[Object]`) **only for computed fields** — a player's
  score, derived standings, visibility-filtered tips.
- Resolvers do **no I/O**: the query root loads the coarse items
  (`<t>#TOURNAMENT`, players, `<t>#SCOREBOARD`, pools) once; nested resolvers
  read from memory.
- Resolvers contain **no domain logic** — they call the pure `domain` scoring
  functions and the `fwc26` module. Glue only.

The GraphQL layer is a thin adapter: coarse load → expose graph → glue to the
pure domain.

Library: `async-graphql` (code-first — the schema is generated from Rust types).

## 4. Queries — coarse, mirroring storage

Queries mirror the coarse storage items, so each is near-zero assembly:

| Query | Returns |
|---|---|
| `now` | the server's resolved request clock (`DateTime<Utc>`) — the clock seam the SPA renders time-dependent state against |
| `tournament` | the `<t>#TOURNAMENT` structure (tree, matches, teams) with time-derived flags (see below) |
| `scoreboard(pool?)` | the materialised `<t>#SCOREBOARD`, filtered to a pool |
| `me` | the current player + their predictions |
| `pools` | the player's pools |
| `tips(groupId)` | all players' *visible* predictions for a group (computed — visibility-filtered) |
| `perfects` | perfect predictions |

The `tournament` resolver also loads the player list to find the result user's locked
match predictions; these are used to derive `resultPending` on each game.

Time-derived flags exposed on tournament sub-types (all computed against the request `now`):

- **`Group.deadlinePassed`** — `true` once the group's earliest-kickoff deadline is in the past.
- **`Game.resultPending`** — `true` when the estimated match end (kickoff + round buffer) has passed but no locked official result exists yet; drives smart polling.
- **`Game.withinTodayWindow`** — `true` when the game falls within the ±2-day Today window (UC-11).

The SPA renders, not computes, these flags — they are derived server-side against the
authoritative request clock so the client has no time-handling logic.

The SPA caches these (urql normalised cache) and composes the ~11 screens
client-side. Most screens are a render of one or two cached queries.

## 5. Mutations — coarse, group-grained

| Mutation | Effect |
|---|---|
| `submitGroup(groupId, predictions[], lock)` | save/lock a whole group's predictions in one call |
| `createPool`, `updatePool` | pools |
| `updateProfile` | profile |
| `invite` | referral invitation |
| *(admin set)* | results entry, banner, etc. |

`submitGroup` matches UC-5 ("save submits a whole group together") and the
coarse player item (rewritten whole). A per-match lock is a `submitGroup` with
one match flagged. Optimistic concurrency: the player item's `version`
attribute is checked on write; the loser retries.

## 6. The draft → locked editing interaction

The My Tips screen is a **group-level form**: the player edits all matches in a
group, then an explicit **Save draft** / **Lock** action submits the whole
group in a **single `submitGroup` mutation**. Optimistic cache update; no
full-page reload. No per-keystroke saving.

Preserved from `REWRITE_USE_CASES.md`:
- **draft → locked** — a prediction scores 0 and is editable until locked;
  locking is final for the player (auto-lock of complete drafts at the
  deadline — see [`DATA_MODEL.md`](./DATA_MODEL.md) §7).
- **hidden-until-locked** — `tips` returns a player's prediction to others only
  once it is effective-locked or the match kicked off.

## 7. Live updates — smart polling

No push (WebSockets/SSE) — it would break the pay-per-short-request cost model
(API Gateway WebSocket API, or Lambdas held open). Results are admin-entered
and slow; polling is indistinguishable from live.

The SPA polls scoreboard/today data **only when a match is *result-pending***:

- *result-pending* = the match's estimated end has passed (kickoff + a fixed
  buffer — ~1h45 for 90 minutes, longer for knockout to allow ET/penalties)
  **and** its result is not yet locked in the loaded data.
- No result-pending match → **no polling**; the data is static, one fetch on
  load suffices.

Self-limiting: polling switches on shortly after kickoffs and off once the
admin has entered results. Idle cost stays at zero.

## 8. Auth in the contract

The edge verifies a Bearer JWT (multi-issuer: Auth0 + a local RS256 issuer for
dev/tests), resolves `Identity → Person → Player`, and places a three-state
`CurrentPlayer` (`Visitor` / `AuthenticatedUnclaimed` / `Player`) in the
GraphQL context. Resolvers read it from context and never re-authenticate.
The dev mechanism is a local-issuer JWT — `LOCAL_AUTH_ISSUER` toggles trust;
a `POST /api/dev/login` endpoint mints tokens. Validation is in-app, layered
behind the existing `cloudfront_auth` middleware. See
[`docs/superpowers/specs/2026-05-30-auth-design.md`](../docs/superpowers/specs/2026-05-30-auth-design.md)
§§2–3.

## 9. Open / deferred

- The full GraphQL schema (every type and field) — falls out of `DATA_MODEL.md`;
  written during implementation, not pre-specified here.
- The what-if / player-vs-player feature surface (`SCORING.md` §11).
