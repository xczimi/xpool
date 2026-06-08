# Scenario generator — realistic + whacky results/predictions as test data

Status: done (merge 87a3f14, 2026-06-07) — shipped as `xtask scenario` + `bin/local-scenario`
Area: crates/xtask / test data

## Idea

Generate multiple full tournament **scenarios** — each a set of official results
plus a body of player predictions — to use as rich, repeatable test data. Then
inspect how the game looks across the tournament by peeking at different dates,
for different scenarios.

## Key design insight (exploit the symmetry)

Predictions and results are the **same shape** in the model: official results
are just the result-user's `match_predictions` (`is_result_user = true`), and
scoring is symmetric `score(A, B)`. So a generated "set of match outcomes" can
serve as either a player's predictions **or** as the official results — no
separate buckets, no separate generators. Generate one kind of thing
(an outcome-set), then assign roles: one becomes "results", the rest become
"players' predictions".

## What to generate

- **Realistic outcome-sets** — scorelines weighted by relative team strength
  (FIFA ranking), so stronger teams usually win, but with real variance and a
  tunable **upset probability** so surprising results happen. Drives believable
  group standings and knockout brackets.
- **A few whacky predictors** — fixed-pattern archetypes alongside the realistic
  crowd, e.g.:
  - "always 1-0" (every game 1-0 home)
  - "always a draw" (every game 1-1)
  - "chaos" (uniform random scores)
  - "homer" (one favourite team always wins big)
  - "chalk" (always the higher-ranked team, no upsets)
- **Multiple complete scenarios** — each = one official-result set + N player
  prediction sets — so the whole tournament can be played out and compared.

## Why

A robust scenario body lets you eyeball the real UX — scoreboard, perfect tips,
standings, knockout resolution — at any point in the tournament, and gives
integration/e2e tests realistic fixtures instead of hand-built minimal cases.
(Today `xtask seed` creates the 6 demo players with **empty** predictions —
this fills that gap.)

## Time-awareness (peeking into dates)

Pair with the server-authoritative dev clock (`XPOOL_NOW` env / `X-Dev-Now`
header). A scenario plus a chosen "now" shows the tournament partially played:
past games resulted, upcoming games still open, deadlines approaching. Inspecting
the same scenario at several dates is the main use case.

## How it might plug in

- A new `xtask` subcommand (e.g. `xtask scenario ...`) alongside `import` /
  `seed` / `drop-table` (`crates/xtask/src/main.rs`), reusing the seeding
  machinery in `crates/xtask/src/seed.rs`.
- Knockout outcome generation must respect bracket resolution + advancer logic
  (`crates/fwc26`) and standings/draw-order predictions, not just raw scorelines.
- **Deterministic**: seed the RNG per scenario id so a scenario reproduces
  exactly — essential for tests and for comparing the same scenario across dates.

## Data gap to resolve

Team data has **no FIFA ranking** today (`tournaments/fwc26.json` teams are
`{id,name,short_code,flag:null,external_id:null}`). The realistic generator
needs a strength signal — decide where ranking lives (added to team data, a
separate ranking file, or a hardcoded table) and how it's sourced/updated.

## Open questions

- How many scenarios, and how many players per scenario?
- How to select/switch the active scenario for inspection — env var, a
  per-scenario DynamoDB table (mirroring the per-branch `xpool-<branch>`
  pattern), or seed-on-demand?
- Should whacky archetypes be a fixed roster or parameterised/weighted?
- Do generated scenarios get checked in as fixtures, or regenerated from a seed
  + RNG each time?

## Related

- [[dev-deploy-clock-and-auth]] — inspecting scenarios across dates wants the
  dev clock available (locally and on the dev deployment).
- `.specs/SCENARIOS.md` — the behaviour catalogue; generated scenarios should
  complement, not replace, its hand-authored cases.
