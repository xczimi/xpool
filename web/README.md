# xpool — web SPA

The React + Vite + TypeScript single-page app for xpool (plan task **P6**).
Talks to the GraphQL API at `/api/graphql`; Vite proxies `/api` →
`http://localhost:3000` in dev.

## Stack

- **React 19 + TypeScript** (Vite scaffold; the plan said React 18 — the
  current `create vite` template ships React 19, which works with all deps).
- **urql** (`urql` + `graphql`) — GraphQL client with the normalised cache.
- **react-router-dom** — routing (11 screens + `/admin/*` sub-routes).
- **i18n** — a small custom React context (`src/i18n/`), no extra dependency.
  English + Hungarian; add a language by extending `src/i18n/strings.ts`.

## Run

```sh
npm install
npm run dev            # dev server, proxies /api to :3000
npm run build          # tsc -b && vite build
npm run lint           # eslint
npm run test           # Vitest — unit tests for src/lib/ pure logic
npm run test:watch     # Vitest in watch mode
npm run test:coverage  # Vitest with a v8 coverage report (80% threshold on src/lib/)
npm run e2e            # Playwright end-to-end suite (boots the full live stack)
```

The build, lint and unit tests do **not** require the backend to be running.
`npm run e2e` does — see below.

## Unit tests (`npm run test`)

[Vitest](https://vitest.dev) covers the SPA's pure logic layer — `src/lib/`
(`standings.ts`, `rounds.ts`, `format.ts`, `polling.ts`). Tests live next to
the code as `*.test.ts`. `npm run test:coverage` enforces an 80% threshold
over `src/lib/` (React hooks such as `usePolledQuery.ts` are excluded — they
are effect-driven, exercised by the e2e suite). Config: `vitest.config.ts`.

## End-to-end tests (`npm run e2e`)

Playwright drives a real browser against the **full live stack** (DynamoDB
Local + the axum API + this Vite dev server). A `build`-only check cannot catch
SPA↔API integration bugs (a GraphQL schema mismatch, urql sending a query as
GET); these tests do.

**Prerequisites:** Docker, the Rust toolchain (`rustup`), Node, and the
Playwright browser binary:

```sh
npx playwright install chromium   # one-time
```

**What `npm run e2e` does** — the Playwright `globalSetup`
(`e2e/global-setup.ts` → `scripts/e2e-stack.sh`) boots the backend before any
test runs:

1. kills any stale API process on `:3000`;
2. `docker compose up -d` (DynamoDB Local + MailHog);
3. waits for DynamoDB Local;
4. runs `xtask import tournaments/fwc26.json` + `xtask seed` (idempotent —
   re-seeding gives every run a clean, known dataset);
5. builds and starts the API, waits for `GET /api/health`.

Playwright then starts the Vite dev server itself (`webServer`, with
`reuseExistingServer`). `globalTeardown` stops the API; **Docker is left
running** (DynamoDB Local is in-memory and cheap — run `docker compose down`
to stop it).

The specs live in `e2e/`: visitor smoke (every public route renders with no
error view / no failed GraphQL), schedule chronological order, dev login +
auth gating, the My Tips save/persist round-trip, and the admin-result →
scoreboard flow.

## Layout

```
src/
  auth/        dev auth stub — X-Dev-Player header seam (API.md §8)
  i18n/        custom i18n context + en/hu string catalogue
  graphql/     urql client, query/mutation documents, TS schema types
  lib/         pure helpers — standings ladder, smart polling, formatting
  components/  persistent chrome (header, auth bar, nav, layout) + shared UI
  pages/       one file per screen; pages/mytips & pages/admin for sub-trees
```

## Screens (REWRITE_USE_CASES §4)

All 11 screens render and are wired to the API: Home, Today, Schedule, My
Tips, All Tips, Scoreboard, Perfect, Profile, Invite, Rules, Admin
(results / teams / players sub-routes).

Notable behaviours:

- **Dev auth** — there is no real login yet. The auth bar offers a player
  picker sourced from the public `players` query. The chosen id is stored in
  `localStorage` and sent as `X-Dev-Player` on every GraphQL request. "Log
  out" clears it; a visitor sends no header. The urql client is rebuilt on
  every identity change (`graphql/GraphqlProvider.tsx`) so one player's cached
  `me`/`pools`/`tips` is never served to another.
- **My Tips** — a group-level form (API.md §6). All matches in a leaf group
  are edited together; **Save draft** / **Lock** submits the whole group via
  `submitGroup`. Locked predictions (or a passed deadline) render read-only.
  Predicted standings are computed client-side from the draft scores; tied
  teams can be reordered (the manual `draw_order`).
- **Smart polling** (API.md §7) — Today and Scoreboard poll every 30s only
  while a match is *result-pending* (kickoff + buffer passed, no locked
  result loaded). Otherwise no polling.
- **All Tips** — renders exactly what `tips(groupId)` returns; the API does
  the hidden-until-locked filtering, so a tip with a null nested `prediction`
  shows as "hidden".

## GraphQL assumptions

The schema below is the **agreed reconciled contract** — the `api` crate's
`gql` types and `src/graphql/{queries,types}.ts` match it exactly.

- **Enums** are `SCREAMING_SNAKE_CASE` (async-graphql default):
  `Round = GROUP_STAGE|R32|R16|QF|SF|THIRD_PLACE|FINAL`,
  `LockMode = LOCK_TOGETHER|LOCK_PER_MATCH`.
- **`Team`** — `{ id, name, shortCode, flag, externalId }`.
- **`GroupGame`** — the domain `GroupChildren` enum is flattened to two list
  fields, `childGroupIds` and `childGameIds` (one is empty). `deadline` is a
  computed ISO field — the earliest kickoff in the node's subtree.
- **`Game`** — `{ id, kickoff, venue, groupId, home, away }`. There is **no**
  `result` field on `Game`.
- **`results`** — a top-level query `results: [MatchPrediction!]!` returning
  the result user's *locked* match predictions (the official scores). The
  frontend overlays these onto games client-side (Schedule, Today, My Tips
  actual standings, Admin Results).
- **`me` / `Player`** — `{ id, nick, fullName, isResultUser, version,
  matchPredictions, standingsPredictions }`. There is **no** `email` and **no**
  `isAdmin`: the result user *is* the admin, so the Admin screen is gated on
  `isResultUser`.
- **`scoreboard(pool: ID)`** — returns `[ScoreEntry!]!` directly (no wrapper
  object). Each entry is `{ playerId, nick, total, stages: [{ round, points }] }`.
  Stage multipliers are static and hardcoded in `src/lib/rounds.ts`
  (`STAGE_MULTIPLIERS`) for display only.
- **`tips`** — each `Tip` is `{ playerId, nick, gameId, prediction }` with a
  nullable nested `prediction`; the API applies UC-9 visibility server-side.
- **`Pool`** — `{ id, name, owner, members }`.
- **`submitGroup`** — `submitGroup(groupId: ID!, predictions:
  [MatchPredictionInput!]!, standings: StandingsInput, lock: Boolean!): Player`.
  When `standings` (`{ ordering, drawOrder }`) is supplied, the group's
  `StandingsPrediction` is upserted alongside the match predictions.
- **`updateProfile`** — flat args: `updateProfile(nick: String, fullName:
  String): Player`. No email/password.
- **`enterResult`** — `enterResult(gameId, homeScore, awayScore, advancer,
  lock): Boolean!`. `setMotd` likewise returns `Boolean!`. The frontend
  refetches after these boolean mutations.
- **`invite`** — `invite(inviteeId: ID!): Boolean!`. This only records a
  referral link to an **already-existing** player; it does **not** create an
  account. The Invite screen is therefore a simple dev action — "refer an
  existing player by id" — not a sign-up form.

## Incomplete / deferred

- Admin **Teams** and **Players** are read-only listings.
- Pool creation/management UI is not built (`createPool` / `updatePool`
  documents exist but pool membership management is out of scope per
  `DATA_MODEL.md` §8). The Scoreboard pool selector reads existing pools.
- A Playwright **end-to-end suite** (`npm run e2e`, see above) covers the
  cross-stack flows; **Vitest** (`npm run test`, see above) covers the pure
  helpers in `src/lib/`. A React component test runner has not been added.
