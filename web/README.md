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
npm run dev      # dev server, proxies /api to :3000
npm run build    # tsc -b && vite build
npm run lint     # eslint
```

The build does **not** require the backend to be running.

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
(results / banner / teams / players sub-routes).

Notable behaviours:

- **Dev auth** — there is no real login yet. The auth bar offers a player
  picker (sourced from the public scoreboard) plus a free-text id field.
  The chosen id is stored in `localStorage` and sent as `X-Dev-Player` on
  every GraphQL request. "Log out" clears it; a visitor sends no header.
- **My Tips** — a group-level form (API.md §6). All matches in a leaf group
  are edited together; **Save draft** / **Lock** submits the whole group via
  `submitGroup`. Locked predictions (or a passed deadline) render read-only.
  Predicted standings are computed client-side from the draft scores; tied
  teams can be reordered (the manual `draw_order`).
- **Smart polling** (API.md §7) — Today and Scoreboard poll every 30s only
  while a match is *result-pending* (kickoff + buffer passed, no locked
  result loaded). Otherwise no polling.
- **All Tips** — renders exactly what `tips(groupId)` returns; the API does
  the hidden-until-locked filtering, so a `visible: false` tip shows as
  "hidden".

## GraphQL assumptions

Task P5 (the `api` crate) is still a stub, so the schema below is the
**frontend's assumed contract**, derived from `crates/domain/src/model.rs` and
`API.md`. The API and frontend will be reconciled when P5 lands. Field names
to verify:

- **Enums** are assumed `SCREAMING_SNAKE_CASE` (`GROUP_STAGE`, `R32`, …;
  `LOCK_TOGETHER`). async-graphql's default rename — adjust `Round` /
  `LockMode` in `src/graphql/types.ts` if it differs.
- **`GroupGame`** — the domain `GroupChildren` enum is flattened to two list
  fields, `childGroupIds` and `gameIds` (one is empty). `deadline` is exposed
  as a computed ISO field (`Tournament::deadline` in the domain crate).
- **`SingleGame.result`** — assumed a nullable `MatchResult` `{ homeScore,
  awayScore, locked }`, sourced from the result user's `MatchPrediction` for
  that game. If the API exposes the result user differently, only
  `src/graphql/queries.ts` + `types.ts` need changing.
- **`me`** — assumed to carry `isAdmin` and `isResultUser` booleans plus the
  player's `matchPredictions` / `standingsPredictions`.
- **`scoreboard`** — assumed to return `entries` (with `nick`, `total`,
  per-round breakdown) and a `multipliers` list for display. `pool` argument
  is `ID` (null = the implicit "everyone" root pool).
- **`tips`** — each `PlayerTip` is assumed to carry a `visible` boolean; the
  API still applies UC-9 visibility server-side.
- **`submitGroup`** — assumed signature
  `submitGroup(groupId, predictions: [PredictionInput!]!, standings: StandingsInput, lock: Boolean!)`.
  `API.md §5` lists `(groupId, predictions[], lock)`; the frontend adds an
  optional `standings` argument (`{ ordering, drawOrder }`) because a group's
  `StandingsPrediction` must ride along with the group submission. This is the
  one place the frontend extends the documented signature — flagged for P5.
- **`invite`** — assumed `invite(input: { email, nick, fullName })`.
- **`updateProfile`** — assumed `updateProfile(input: { nick, fullName,
  email, password? })`.
- **`enterResult`** — assumed
  `enterResult(gameId, homeScore, awayScore, lock)`.

## Incomplete / deferred

- Admin **Teams** and **Players** are read-only listings. Team metadata
  editing and the dedicated `players` admin query are not built — they need
  mutations P5 has not defined yet. Players are listed from scoreboard
  entries as a stand-in.
- Pool creation/management UI is not built (`createPool` / `updatePool`
  documents exist but pool membership management is out of scope per
  `DATA_MODEL.md` §8). The Scoreboard pool selector reads existing pools.
- No automated tests — pure helpers in `src/lib/` are structured to be unit
  testable, but a test runner was not added under P6.
