# Group-standings bonus must be monotonic (award only at group completion)

Status: done — shipped on backlog-parallel-build (fix(domain) b55c4e7), merged to master 2026-06-27
Area: domain scoring (locked contract) + .specs/SCORING.md

## Problem

The group-standings bonus is awarded **provisionally** from the group's first kickoff:
`standings_score` gates on the group deadline + locked predictions and scores
`pairs_correct` against `rank_group` of results-so-far. As more group results land the
provisional ranking shifts, so `pairs_correct` — and the awarded bonus — can **decrease**.
A player's committed points then drop over time, which is wrong.

## Decision (Peter, 2026-06-27)

- **Award the group-standings bonus ONLY when the group is COMPLETE** — every game in the
  leaf group has an official (result-user) result entered. Until then the bonus is 0.
- **Complete = all group games have official results** (same definition the points-trajectory
  already uses, so scoreboard total and trajectory endpoint reconcile exactly).
- **Scope: standings bonus only.** Match points are already monotonic (fixed once a result is
  entered); the committed scoreboard counts only final entered results, so nothing else drops.

## Consequence (intended)

On data with incomplete groups, affected players' scoreboard totals drop to their settled
value (e.g. a top player ~198 → ~194), now matching the trajectory. Those points weren't
earned yet. Going forward, points never decrease.

## Where

`crates/domain/src/scoring.rs::standings_score` (completion gate) → flows to `score_leaf_group`
→ `score_tournament` → materialised scoreboard (`crates/api/src/recompute.rs`), the standings
resolver, and the points-timeline. Update `.specs/SCORING.md`. TDD a monotonicity test.
