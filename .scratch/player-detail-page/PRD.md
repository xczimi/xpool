# Player-detail page — one participant's complete view

Status: ready-for-human (implemented on branch worktree-player-detail-page)

## Summary

A dedicated page per **pool participant** (a human predictor — *not* a footballer) giving a complete view of that player: all their match predictions vs official results with points, their standings predictions + bonuses, their perfect tips, and totals/per-round breakdown. This is consumer **#3** from the TheSportsDB feature decomposition — but note **it does not touch SportsDB at all**; it's a read-only aggregation over xpool's own data.

## Why

The scoreboard shows totals; per-group `tips`/`standings` grids show one group at a time. There's no single "everything about player X" view — their whole tournament at a glance. Useful for players reviewing their own run and for the friendly-pool social angle (seeing how someone did, within the visibility rules).

## Background / what to reuse

See the decomposition in `docs/superpowers/specs/2026-06-14-sportsdb-reported-results-design.md` §1 (row 3). This is mostly a **frontend aggregation page** over existing GraphQL — likely no new domain logic, possibly one convenience query.

Existing resolvers to compose (all in `crates/api/src/gql/query.rs`):
- `scoreboard` — totals + per-round (`StageScore`) for the player.
- `tips(groupId)` / `results` — predictions + per-game points + breakdown.
- `standings(groupId)` — group-table bonuses.
- `perfects` — their max-scoring predictions.
- `domain::participation` selectors — participant filtering (exclude result-user / non-participants).

## Critical constraint — tip visibility

Another player's predictions are **not** freely visible: the `tips` resolver enforces mutual-commitment gating (you see someone else's tip for a match only once *both* of you have effective-locked it, or the match has opened). A player-detail page for *another* player MUST apply the same gating — it can't dump all their predictions. Your **own** page shows everything; others' pages show only what's already revealable.

## Scope

- Read-only page, route per participant (e.g. `/player/<id>`).
- Aggregates the above into one view; respects visibility rules.
- "Player" = pool participant; the result-user (official results) is excluded from listings as usual.
- **Out of scope:** SportsDB/live data (that's #2); editing; cross-pool privacy changes.

## Open questions (resolve in a brainstorm before building)

- What exactly belongs on it? (predictions+points; standings; perfects; pools they're in; referral relationship?) — needs scoping.
- Whose pages are viewable, and how do pool-privacy + tip-visibility interact for a non-member viewer?
- New `player(id)` query vs. reusing existing queries client-side.
- Entry points (link from scoreboard rows, All Tips, pools).

## Comments
