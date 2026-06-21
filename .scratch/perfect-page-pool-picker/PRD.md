# Pool picker on the Perfect page

Status: needs-triage
Area: web

## Idea

Add a **pool picker** to the Perfect page, mirroring the one already on the
Scoreboard and All Tips pages, so perfects can be scoped to a chosen pool.

## Motivation

The scoreboard and All Tips gained a pool selector (commits `ca9ea6f`,
`8ad0d94`); the Perfect page should be consistent — viewing "perfect tips" makes
most sense scoped to the pool you're competing in. Inconsistent scoping across
pages is confusing.

## Sketch

- Reuse the existing pool-picker component/pattern from `ScoreboardPage` /
  `AllTipsPage` on `web/src/pages/PerfectPage.tsx`.
- Scope the perfects query/listing to the selected pool.
- Keep selection consistent with the other pages (shared state / same default).

## Open questions

- Share the selected-pool state across pages (sticky) or per-page?
- Does the perfects resolver already accept a pool arg, or is one needed?

## Resolved decisions (grilled 2026-06-21)

- **Pool state is sticky & shared across pages** (not per-page). Introduce a
  shared pool-selection primitive (React context backed by localStorage) and
  migrate ScoreboardPage + AllTipsPage onto it; PerfectPage reads the same
  sticky selection. Default remains 'your first pool' on first load.
- The perfects resolver does **not** yet accept a pool arg — add a `pool: ID`
  argument to the `perfects` query + resolver (mirror the scoreboard/tips
  pool-membership filtering already in query.rs).
- Cross-cluster note: the sticky pool context is consumed by match-page
  prediction-stats and live max-achievable too; this cluster owns building it.
