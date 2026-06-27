# Phase 2 — Editable last-resort tiebreak for best-thirds ranking

**Status:** Backlog (deferred from the Phase 1 display work)
**Parent design:** `docs/superpowers/specs/2026-06-27-best-third-placed-teams-table-design.md`
**Labels:** needs-triage

## What

Let a player **manually reorder the third-placed teams as a last-resort
tiebreaker**, exactly like the per-group `draw_order` editor (`SCORING.md` §4
stand-in for FIFA ranking). This is parity with how group standings already let
players break score-equal ties.

## Why it's a real (but rare) feature

Today the cross-group ranking of the 12 thirds, when teams tie on **points →
goal difference → goals-for**, falls back to **group-letter order (A–L)** inside
`fwc26::best_thirds`. There is **no player-controllable cross-group tiebreak**,
and **no domain field stores one**. The per-group `draw_order` only decides who
comes *third within a group* — not the ranking *across* the 12 thirds.

It only ever changes an outcome when a stat-tie straddles the **8th/9th
qualifying boundary** (i.e. the tie decides who advances). Rare, but it's the
one place a player's predicted bracket can currently differ from their intent
with no way to express it.

## Why it was deferred

Larger blast radius than the display:

- **`crates/domain/src/model.rs`** is a *locked contract*; this adds a new
  prediction field (a tournament-level cross-group third-place ordering — e.g.
  `third_place_draw_order: Vec<TeamId>` on `Player`, or a `StandingsPrediction`
  with a sentinel/whole-tournament `group_id`). Ripples through storage.
- **`crates/fwc26::best_thirds`** must take this ordering as the stable-sort
  fallback **instead of** group-letter order.
- **GraphQL mutation** to persist it (mirror `submitGroup` / `StandingsInput`).
- **Frontend** reorder controls (up/down), active **only on tied thirds**
  (matching the group `PredictedStandingsEditor` UX).
- Tests across domain / fwc26 / api / web e2e.

## Open design questions (resolve at Phase 2 brainstorming)

- Storage shape: new `Player` field vs reuse `StandingsPrediction` with a
  reserved tournament-wide id. Keep `model.rs` churn minimal.
- Should reordering be allowed only among *currently-tied* thirds, or a free
  ordering of all 12 that acts purely as a fallback? (Groups only let you move
  tied teams — keep parity.)
- Locking semantics: when does the thirds tiebreak lock? (Group `draw_order`
  locks with the group; the thirds ranking spans all groups.)
- Whether the official result-user also needs an editable thirds tiebreak (admin
  entering the real-world FIFA-ranking outcome) — likely yes, same mechanism.

## Prerequisite

Phase 1 (`thirdPlaceRanking` query + `ThirdPlaceTable`) should land first; the
editor reuses that ranking as its display surface.
