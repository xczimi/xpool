# Timeline chart of player points and position

Status: needs-triage
Area: web (+ possibly api)

## Idea

A timeline chart that plots a player's **points and/or league position over
time** across the tournament — a line that rises with each match and shows how
the standing moved match by match (or day by day).

## Motivation

The scoreboard is a snapshot; a timeline tells the story. "When did I overtake
them? Which match was my big jump? When did I slip?" A trajectory chart makes the
arc of the tournament visible and is far more engaging than a static total. It
pairs naturally with the [[head-to-head-players]] view (overlay two lines) and
the [[timeline-schedule]] date-ordered framing.

## Sketch

- Per-player line chart: x = match order / date, y = cumulative points (and/or
  position rank, possibly a second series).
- Overlay support: plot several players (or both sides of a head-to-head) on one
  chart.
- Needs per-match cumulative history — either derive client-side from already
  scored tips, or a small resolver returning the running totals.
- Honour the server-authoritative clock for date bucketing (no `Date.now()`
  branching); reuse a lightweight charting approach consistent with the app.

## Resolved decisions (2026-06-27 grill)

- **Hand-rolled SVG** (`<polyline>`, ~5–10KB) — no charting library. Fits the project's
  lean / no-deps philosophy and the existing hand-rolled SVG pattern (`BrandIcon.tsx`).
- **Cumulative points trajectory** first; position/rank trajectory deferred.
- **x-axis by round** (matches `SCOREBOARD_QUERY` stage granularity); calendar-date deferred.
- **Compute client-side** from the existing `SCOREBOARD_QUERY` stages (running totals) —
  **no new resolver**.
- **Overlay support** so [[head-to-head-players]] can plot both players on one chart.
- Honour the server-authoritative clock — **no `Date.now()`**.
- Cluster: `cluster/player-analytics` (Wave 1).
