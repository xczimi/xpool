# Show points earned per prediction, wherever a player's tip is shown

Status: ready-for-agent
Area: api + web

## Idea

When a player and their tip for a game are displayed, also show the **points
that tip earned** against the official result. Today tips are shown as bare
score predictions; the scoring is invisible until it's aggregated into the
scoreboard total. Surfacing per-prediction points makes scoring legible — which
is exactly what's needed to **validate scenarios and the scoring engine** by eye.

## Motivation

- Validating the generated scenarios (`bin/local-scenario chalk|balanced|chaos`)
  and the scoring rules currently means trusting the scoreboard total with no way
  to see *how* it was earned. Per-game points let you confirm each prediction
  scored what `SCORING.md` says it should.
- For real users it answers the obvious question on every tip: "how many points
  did I get for that one?"

## Current state (what exists today)

- **Scoring engine** (`crates/domain/src/scoring.rs`) already computes everything
  needed, as pure functions:
  - `score_match(p, r, c) -> i64` — per-match points (default max `2*exact +
    outcome` = `2*1 + 2 = 4`); per-side, symmetric 4-goal rule (§3).
  - `is_perfect(p, r, c) -> bool` — scored ≥ `perfect_threshold` (default 4).
  - per-round **multiplier** (`ScoringConfig::multiplier(round)`) — Group Stage
    ×1, later rounds higher; the scoreboard's `stages { points }` are already
    multiplied.
- **API exposes** (`crates/api/src/gql`): `scoreboard { total, stages { round
  points } }` and `perfects { playerId nick gameId }` — both **aggregates**.
  There is **no per-(player, game) points field** anywhere.
- **Where tips are shown** (the consumers that should get points):
  - `web/src/pages/AllTipsPage.tsx` — grid of every player's predictions for a
    round/group (primary target; `TIPS_QUERY` → `tips { playerId nick gameId
    prediction { gameId homeScore awayScore locked } }`).
  - `web/src/pages/MyTipsPage.tsx` (+ `mytips/GroupTipForm.tsx`) — your
    predictions with a "Result" column already shown.
  - `web/src/pages/PerfectPage.tsx` — perfect predictions list.

## Sketch

- **API:** add a `points` field (and likely an `isPerfect` flag) to the per-tip
  GraphQL type the `tips` resolver returns (`Tip` / `StageScore` siblings live in
  `crates/api/src/gql/types.rs`). The query root already loads tournament +
  players + the result user once; the resolver stays pure — for each tip with a
  known result, call `score_match(prediction, result, config)` and apply
  `multiplier(round)`. `null`/omitted when the game has no official result yet.
  - The official result for a game = the **result user's** match prediction for
    that game (the "result user" model). Reuse the same lookup the scoreboard
    recompute uses.
- **Web:** render the points next to each shown tip:
  - AllTips grid cell: prediction `2:1` → append the earned points (e.g. a small
    badge `+4`), with a visual marker for a perfect.
  - MyTips: a points column/affordance beside the existing Result column.
- **Visibility:** points only render once the result is known — gate on the
  existing server-derived time flags (`resultPending` / result present), and the
  tips API already hides predictions until locked. No client clock branching.
- **i18n:** any new labels in `web/src/i18n/strings.ts` (en + hu).

## Open questions (decide in the fresh session)

- Show **multiplied** points (what actually fed the total — recommended, so
  per-game points sum to the stage total) or **raw** points with the round
  multiplier shown separately?
- Show a **breakdown** (exact / outcome / perfect) on hover or just the number?
- Include the **standings bonus** anywhere here, or leave it to the scoreboard
  stages only? (It's per-group, not per-game — likely out of scope.)
- Scope: AllTips + MyTips first; PerfectPage is mostly redundant once AllTips
  marks perfects — fold it in or leave it?

## Acceptance criteria

- A new/extended GraphQL field returns per-(player, game) earned points, computed
  via the existing pure `domain` scoring functions (resolver does no I/O / no new
  domain logic), `null` until the game has a result.
- AllTips and MyTips display the earned points beside each shown prediction, with
  a perfect marker, only after the result is in.
- Points shown are consistent with the scoreboard: summing a player's per-game
  points (+ any standings bonus) per round equals that round's `stages.points`.
- Verified end-to-end against a seeded scenario (e.g. `chalk`) with the dev clock
  advanced past results — an e2e asserts a known prediction shows its expected
  points (frontend work needs E2E).

## Pointers

- Scoring: `crates/domain/src/scoring.rs` (`score_match`, `is_perfect`,
  `multiplier`), `.specs/SCORING.md` (authoritative rules + corrections).
- API tips resolver + types: `crates/api/src/gql/query.rs`,
  `crates/api/src/gql/types.rs`.
- Web: `web/src/pages/AllTipsPage.tsx`, `web/src/pages/MyTipsPage.tsx`,
  `web/src/pages/mytips/GroupTipForm.tsx`, `web/src/graphql/queries.ts`.
- Explore live with a scenario: `bin/local-scenario chalk`, then advance the
  DevClock so games have results.
