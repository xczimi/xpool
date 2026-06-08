# Scenario test-data generator — design

Status: approved (brainstorming complete)
Date: 2026-06-07
Area: `crates/xtask` (generator + seeding) · `crates/api` (dev re-materialize)
Source idea: `.scratch/scenario-test-data-generator/PRD.md`

## Goal

Generate realistic and whacky full-tournament **scenarios** — each one set of
official results plus a body of player predictions — as rich, repeatable test
data, and make the **scoreboard materialise correctly as of any chosen date** so
the tournament can be inspected at several points in time from a single seed.

The thing actually under test is **scoreboard (and bracket) materialisation at a
given `now`** — not just the presence of plausible fixtures.

## Key insight: exploit the result-equals-prediction symmetry

Official results are just the result-user's `match_predictions` /
`standings_predictions` (`is_result_user = true`), and scoring is symmetric
`score(A, B)`. So a generated **outcome-set** — one player's
`(Vec<MatchPrediction>, Vec<StandingsPrediction>)` — can serve as either the
official results or a player's predictions. We generate outcome-sets, then assign
roles: one becomes the result-user, the rest become players.

## Locked decisions

| Decision | Choice |
| --- | --- |
| Where team strength lives | Separate `tournaments/fwc26-rankings.json` (`{team_id: strength}`); the domain `Team` contract is untouched. |
| Scope | 3 scenarios, ~12 players each (6 reused demo players + 5 whacky archetypes + result-user). |
| Whacky archetypes | Fixed roster (named, stable player ids). |
| Persistence | Regenerate from a seed + RNG each run; nothing large checked in. |
| Scenario switching | Re-seed the active `XPOOL_TABLE` (`xtask scenario <id>`), not per-scenario tables. |
| Date peeking | Seed **full** results; re-materialise the scoreboard as-of the dev clock — one seed, any date. |
| Re-materialise home | A dev-only GraphQL mutation (`devRematerialize`); the as-of slice is unified into `recompute`. |
| Picker integration | The SPA dev-clock picker calls `devRematerialize` on every clock change/reset, before reloading. |

## Architecture

```
crates/xtask/src/scenario/
  ranking.rs      load + validate tournaments/fwc26-rankings.json
  policy.rs       ScorelinePolicy trait + realistic & whacky implementations
  engine.rs       forward-simulate a coherent outcome-set from a policy
  scenarios.rs    the 3 fixed scenario definitions + player→policy roster
  mod.rs          wiring; the `xtask scenario <id>` seeding entrypoint
crates/api/src/recompute.rs   gains recompute_as_of (result-user sliced by now)
crates/api/src/...            a dev-only route that re-materialises for X-Dev-Now
```

### 1. Ranking (`ranking.rs`)

Loads `tournaments/fwc26-rankings.json`, a flat `{team_id: strength}` map kept
out of the domain `Team`. Validated on load: every tournament team must have a
strength, or the command errors. Strength is a generator-only signal (relative
ordering + gap drive upset probability); the exact numbers are test data, not
sourced fact.

### 2. Scoreline policies (`policy.rs`)

A small trait isolates the per-game decision:

```rust
struct GameContext { home: TeamId, away: TeamId, home_strength: f64, away_strength: f64, round: Round }
trait ScorelinePolicy { fn score(&mut self, ctx: &GameContext) -> (u8, u8); }
```

Implementations:

- `Realistic { rng, upset_prob }` — scorelines weighted by relative strength; a
  tunable `upset_prob` lets the weaker team win. At `upset_prob = 0` the stronger
  team always wins (used as a deterministic test anchor).
- Fixed whacky roster: `AlwaysHome(1,0)`, `AlwaysDraw(1,1)`, `Chaos(rng)`,
  `Homer(fav_team)`, `Chalk` (higher strength always wins, no upsets).

Each policy is tiny and independently testable.

### 3. The engine (`engine.rs`) — coherence

Pure given a policy (the only randomness lives inside the policy's RNG). It
**forward-simulates** the tournament so every knockout pairing a predictor
produces is derived from that predictor's *own* earlier predicted results — using
the same functions the live app uses, so the set is never internally
inconsistent.

The `Round` order is the only valid order:
`GroupStage → R32 → R16 → QF → SF → ThirdPlace → Final`.

1. **Group stage** — matchups are concrete from tournament data; the policy
   assigns each scoreline.
2. **Standings** — `domain::scoring::rank_group` derives the group ordering from
   those scores (→ `StandingsPrediction`), so standings are consistent with the
   match scores by construction. The engine supplies a deterministic `draw_order`
   (by strength) so ranking is total and `best_thirds` / Annexe-C are
   unambiguous.
3. **Resolve next round** — `fwc26::resolve_bracket(t, predictor_so_far)` fills
   the next knockout round's slots (`"1A"`, `"2C"`, `"3ABCDF"`, `"Winner M73"`,
   `"Loser SF1"`) with concrete teams from the predictor's accumulated results.
4. **Score the resolved round** — the policy scores each now-concrete game; a
   90-minute draw is resolved to an advancer recorded in the one-match group's
   `StandingsPrediction` (the 2-team ordering *is* the ET/penalty advancer).
5. Repeat through the Final.

**Coherence is proven by a round-trip test**: generate the result-user's set →
`resolve_bracket` on it → the resolved bracket reproduces exactly the matchups
the engine walked, and every knockout `MatchPrediction`'s teams equal
`resolve_bracket`'s output. Each predictor is its own self-consistent universe,
so even "always 1-0" yields a legal (if weird) bracket.

### 4. Scenario definitions (`scenarios.rs`)

Three fixed scenario ids, differing in the **result-user's** policy:

- `chalk` — result-user plays `Chalk`; favourites march through.
- `balanced` — result-user plays `Realistic` with a moderate `upset_prob`.
- `chaos` — result-user plays `Chaos`; brackets get wild.

Each scenario maps the player roster to policies: the 6 existing demo players
(`demo-ada` … `demo-dennis`) play `Realistic` (each with its own RNG seed, so
they differ), and the 5 whacky archetypes are added as their own players. All
predictors plus the result-user (~12 entities) join one pool, reusing `seed.rs`'s
Person/Identity/Pool machinery so every player is dev-loginable.

## Time model — seed full, re-materialise as-of `now`

The API reveals stored official results **unconditionally** (`results` resolver
returns every entered result; `result_pending` keys off result *presence*, not
the clock). So the clock alone cannot hide future results on the read side. But
the **scoring engine is already time-gated**: `score_leaf_group` counts a match
only when both prediction and result are `effective_locked` at `now`
(`now > deadline`, the group's earliest kickoff; per-match for one-match knockout
groups).

The gap: a full seed makes *all* of a group's results count the instant the group
locks (first kickoff), including games not yet played — production avoids this
only because later results aren't *entered* yet. The fix reproduces that signal:

> **`recompute_as_of(repo, now)`** = build a result-user view containing only
> match results for games **played as-of `now`** (`now > kickoff +
> result_buffer(round)`, the inverse of `result_pending`), then run the normal
> recompute.

This single slice makes both derived structures correct from one full seed:

- **Scoreboard** — unplayed games' results are absent → not counted → correct
  per-match accumulation, even within a group.
- **Bracket** — `resolve_bracket` runs on the sliced result-user, so the knockout
  bracket resolves *progressively* (R32 fills only once the feeding groups are
  played), instead of showing the full bracket from day one.

For real production entries the slice is a no-op (unplayed games have no entered
result), so it is **unified into `recompute`** rather than duplicated.

### Inspection loop

1. `xtask scenario <id>` — seed the full scenario into `XPOOL_TABLE` (results
   present with `locked = false`; player predictions full).
2. Move the dev clock to date `T` — via the SPA dev-clock picker, or by setting
   `X-Dev-Now` / `XPOOL_NOW` directly.
3. `devRematerialize` rebuilds scoreboard + bracket as-of `T`. From the picker
   this is automatic; from a script it's one mutation call.
4. Sweep `T` to watch the tournament evolve. **One seed, any date — no re-seed.**

### Dev-clock picker hook

`web/src/components/DevClock.tsx` already does `setDevNow(iso)` →
`location.reload()`, and `client.ts` attaches the stored `X-Dev-Now` to every
GraphQL request. The hook: after `setDevNow` (and in the reset handler), call the
`devRematerialize` mutation — which now carries the new `X-Dev-Now`, so the
server re-materialises as-of the just-picked instant — then `await` it and
reload. Failures are swallowed (a prod build without the dev mutation still
reloads cleanly). Result: moving the clock and re-materialising the board are a
single user action, which is the whole inspection loop.

## API / CLI surface

- **`xtask scenario <scenario-id>`** — new subcommand alongside `import` / `seed`
  / `drop-table`. Generates and seeds the named scenario (full outcome-sets).
  Idempotent (fixed entity ids), so switching scenarios overwrites cleanly.
- **`devRematerialize` GraphQL mutation** — calls `recompute` using the
  request's resolved `now`. Gated to the dev stub / dev builds, consistent with
  `X-Dev-Now` and `dev_login`. Chosen over a REST route so the SPA's existing
  urql client carries the dev-clock header for free. Returns a trivial payload
  (e.g. `Boolean`); scripts/curl can call it via the GraphQL endpoint.
- **SPA dev-clock picker** (`web/src/components/DevClock.tsx`) — calls
  `devRematerialize` on every clock change and on reset (see "Dev-clock picker
  hook").

## Determinism

RNG seeded from a stable hash of `(scenario_id, player_id)`. A scenario + player
reproduces byte-for-byte; the same scenario is stable across date-peeks and test
runs. Adds the `rand` crate to `xtask`. (No reliance on wall-clock / process
entropy anywhere in generation.)

## Testing (TDD)

- **Policies** in isolation: `AlwaysHome` emits 1-0; `Chalk` never lets the lower
  strength win; `Realistic` at `upset_prob = 0` always favours the stronger team;
  the same seed reproduces identical output across two runs.
- **Engine** with a deterministic stub policy → a complete, bracket-consistent
  outcome-set spanning all rounds.
- **Coherence round-trip**: generate result-user set → `resolve_bracket` →
  matchups and per-game teams match the engine's walk (per scenario, and per
  whacky archetype).
- **`recompute_as_of` time-slice**: at a date mid-group, only played matches
  score and the bracket is partially resolved; at tournament end the full board
  matches a non-sliced recompute. A no-op-for-production test: with results only
  for played games, slicing changes nothing.
- **Seed integration**: `xtask scenario <id>` populates result-user + ~12 players
  + pool, all dev-loginable (extends the existing `seed.rs` resolution test).
- **`devRematerialize` mutation**: a dev-clock `now` rebuilds the stored
  scoreboard; the mutation is gated off when the dev stub is disabled.
- **E2E (Playwright)**: seed a scenario, move the dev-clock picker across two
  dates, and assert the scoreboard changes accordingly (the inspection loop is
  the deliverable, so it gets an end-to-end check, not just unit coverage).

## Open implementation details (resolved during planning)

- **Where the recompute glue lives so both the dev route and seed-time
  materialisation can reach it.** `recompute` currently sits in `crates/api`,
  which `xtask` does not depend on. Default: keep it in `crates/api`, the dev
  route is the sole materialiser, and a freshly seeded board is empty until the
  route is hit once. If seed-time materialisation is wanted, lift the recompute
  glue (it needs only `domain` + `fwc26` + `storage`) into a small shared spot
  both crates depend on. Decide in the plan.
- Exact strength values in `fwc26-rankings.json` (plausible relative ordering for
  the 48 FWC26 teams).
- Whether the 3 scenarios share one pool or get a pool each (default: one pool).

## Alternatives considered (rejected)

- **Standalone `crates/scenario` crate** — the PRD calls for an `xtask`
  subcommand reusing the seeding machinery, and `xtask` already depends on
  `domain` + `fwc26`; a bounded module beside `seed.rs` is the lighter fit.
- **`--as-of` slicing at seed time (re-seed per date)** — works, but makes the
  main use case (inspect the same scenario at several dates) a re-seed loop.
  Superseded by seed-full + `recompute_as_of`, which peeks any date by moving the
  clock alone.
- **Per-request clock-relative scoreboard (no materialisation)** — would let a
  bare `X-Dev-Now` flip drive everything, but abandons the materialised-scoreboard
  design; far outside a test-data generator.
- **Adding a ranking field to the domain `Team`** — pollutes a locked contract
  with a test-data concern that ripples across crates.

## Related

- `.scratch/scenario-test-data-generator/PRD.md` — the originating idea.
- `.scratch/dev-deploy-clock-and-auth/` — wants the dev clock available on the
  dev deployment, which this leans on for date-peeking.
- `.specs/SCENARIOS.md` — hand-authored behaviour catalogue; generated scenarios
  complement, not replace, it.
