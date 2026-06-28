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
- **Overlay support** so [[head-to-head-players]] can plot both players on one chart.
- Honour the server-authoritative clock — **no `Date.now()`**.
- Cluster: `cluster/player-analytics` (Wave 1).

## REVISION (2026-06-27, after review — flat-line fix + scoreboard chart)

The first build used an **x-axis by round computed client-side** from `SCOREBOARD_QUERY`
stages. During the group stage all points sit in the single `GROUP_STAGE` bucket, so the
cumulative line was **flat for everyone**. Revised:

- **x-axis is GAME-BY-GAME (chronological by kickoff)** so the line climbs as each match is
  scored — supersedes the by-round axis.
- **New pool-scoped resolver** `pointsTimeline(pool)` returns each player's per-game
  cumulative points over resulted games up to the server "now" (computed via a pure,
  unit-tested domain helper; resolver does no domain logic). Supersedes the
  "no resolver / client-side by round" decision (which caused the flat line).
- **Player page:** the page-owner's trajectory ONLY (a single line — no other players
  unless it's an H2H comparison); the player's **current standing stays prominent** ("around
  now"). 
- **Scoreboard:** a new **all-pool-members trajectory** overlay (one line per player in the
  selected pool).
- **H2H:** the two players' game-by-game lines (the surface where the flat line was reported).
- The old by-round reducer (`web/src/lib/cumulativePoints.ts`) is superseded/removed.
