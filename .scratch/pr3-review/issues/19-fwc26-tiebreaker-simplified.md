# 19 — fwc26 group-stage tiebreaker simplified (no H2H, conduct=0, hardcoded map)

Status: ready-for-agent
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
