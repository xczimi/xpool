# 19 — fwc26 group-stage tiebreaker simplified (no H2H, conduct=0, hardcoded map)

Status: done
Severity: MEDIUM
Area: crates/fwc26

## Problem

Three related simplifications in `crates/fwc26/src/lib.rs` that feed bracket
resolution and Annexe C:

1. **No head-to-head** — `compute_standings_for_group` (`lib.rs:393-416`) does
   points → GD → GF → draw_order, omitting `FWC26_RULES.md` §2 Step 1. A wrong
   1st/2nd/3rd ordering feeds directly into R32 slots.
2. **`conduct` hardwired to 0** (`lib.rs:479`) — best-thirds criterion (d) is
   inert; `best_thirds_conduct_tiebreak` passes only because it builds
   `TeamStats` directly, not via the real pipeline.
3. **Hardcoded `groups_str → winner_group` map** (`lib.rs:286-296`, `685-702`)
   — couples bracket resolution to one fixture-list spelling; a different
   letter order silently returns `None`.

## Expected — decision needed

Decide which of these are real gaps to close for FWC26 vs acceptable
simplifications to document. (1) and (2) are correctness limitations; (3) is
fragility. Human call on scope — hence `ready-for-human`.

## Acceptance

- Each item either implemented with a test, or explicitly documented as a known
  limitation in `.specs/`.

## Decision (grilled 2026-05-17)

- **Item 1 (no head-to-head)** — resolved by **consolidation**:
  `fwc26::compute_standings_for_group` delegates to `domain::rank_group`,
  passing the result-user's `MatchPrediction`s as the match results. One
  ranker, one FIFA ladder; issue 12's H2H fix in `domain` then covers bracket
  resolution too. The bespoke fwc26 ranker is removed.
- **Item 2 (conduct = 0)** — **not a bug.** `SCORING.md` §4 states disciplinary
  conduct is deliberately *not* modelled — it "collapses into the opaque manual
  `draw_order`". Fix = **remove the dead `conduct` criterion** from the
  best-thirds comparison and document that residual ties fall through to
  `draw_order`.
- **Item 3 (hardcoded `groups_str → winner_group` map)** — **keep the map**,
  add a guard test asserting all 8 keys match the `3XXXXX` slots in the
  imported `fwc26.json` fixture, so a fixture drift fails loudly instead of
  silently returning `None`.

## Comments

Done in `crates/fwc26`. Item 1: `compute_standings_for_group` now delegates to
`domain::rank_group` (the single FIFA ladder, incl. the H2H step); the bespoke
no-H2H sort and `RawStats` are deleted. The "all games predicted" gate is kept —
a partially-played group still yields no standings, so `1X`/`2X` slots stay
`None` (existing partial-results tests still pass). Item 2: the inert `conduct`
criterion and the `TeamStats.conduct` field are removed; `best_thirds` doc/code
now states residual ties fall through to the caller's `draw_order`. Item 3: the
two hardcoded `groups_str → winner_group` matches are consolidated into one
public `BEST_THIRD_SLOTS` const + `winner_group_for_slot` helper; new
`tests/fixture_guard_tests.rs` pins all 8 keys to `tournaments/fwc26.json`.
Verified: `cargo test -p fwc26` green (24 tests), `cargo clippy -p fwc26
--tests -- -D warnings` clean, `cargo build --workspace` clean.
