# Head-to-head view comparing two players

Status: needs-triage
Area: web (+ possibly api)

## Idea

A head-to-head view that compares **two players** side by side: their points,
position, prediction-by-prediction agreement/disagreement, and where each gained
or lost ground over the tournament.

## Motivation

Pools are social and rivalries are the fun part — "am I beating my brother-in-
law?". A flat scoreboard answers the global ranking but not the personal duel.
A focused two-player comparison makes the rivalry legible: who's ahead, by how
much, and exactly which matches decided it.

## Sketch

- Pick two players (from the scoreboard or a picker) → `/h2h/:a/:b` style route
  (prefer clean aliases over UUIDs, per [[prefer-clean-url-aliases-over-uuids]]).
- Show: current points + positions, total delta, and a per-match breakdown where
  their predictions/points differ.
- Respect tip-visibility gating for any not-yet-revealable predictions.
- Could share rows/components with the match prediction list and scoreboard.
- Natural companion to [[player-points-timeline-chart]] (plot both players'
  trajectories on one chart).

## Resolved decisions (2026-06-27 grill)

- **Two players only** this round: `/h2h/:a/:b` with clean aliases (not UUIDs, per [[prefer-clean-url-aliases-over-uuids]]).
- **Data:** reuse existing per-player scoreboard data client-side where possible; add a resolver only if the data shape forces it.
- **Entry points — player-centric (revised 2026-06-27 after review):** the comparison is
  anchored to a player page, not a generic scoreboard picker. On `/me` it reads "Compare me
  with…"; on `/player/:playerId` the same picker works from THAT player's POV ("Compare
  <nick> with…"). The Scoreboard "pick two" picker is removed.
- **Pool-scoped** comparison.
- Show points + positions, total delta, and a per-match breakdown where predictions/points differ. Respect tip-gating.
- Companion to [[player-points-timeline-chart]] (overlay both players' trajectories).
- Cluster: `cluster/player-analytics` (Wave 1). New page files + owns `App.tsx` routing.
