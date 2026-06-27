# Sortable player predictions on the match page

Status: needs-triage
Area: web

## Idea

Make the list of player predictions on the match page (`/match/:gameId`)
sortable — by player name, by predicted scoreline/outcome, and (once the result
is in) by points earned for this match.

## Motivation

A flat, fixed-order prediction list is hard to read once there are many players.
Sorting lets you answer "who tipped what" questions directly — group everyone who
picked the same scoreline, or see who scored on this match. It complements the
aggregate view in [[match-page-prediction-stats]] (the stats summarise; sorting
lets you drill into the raw rows) and mirrors the sort work on
[[perfect-page-sort-by-player]].

## Sketch

- Clickable column headers on the prediction table: player, prediction, points.
- Client-side sort over the already-loaded, visibility-gated rows.
- Respect tip-visibility gating — only sortable/visible once tips are revealable.
- Optional: remember last sort choice (localStorage), consistent with other
  persisted view prefs.

## Resolved decisions (2026-06-27 grill)

- **Default sort:** current scoreboard position (most useful at a glance); secondary by player name.
- **Persist** the chosen sort in `localStorage` (consistent with other view prefs).
- **Self-highlight integration:** keep the [[live-match-highlight-self]] "you" highlight visible regardless of sort, so the current player stays findable.
- **Columns:** player, prediction, points (points sortable only once the result is in). Client-side sort over the already-loaded, visibility-gated rows; respect tip-gating.
- Cluster: `cluster/match-page` (Wave 1). Owns `MatchPage.tsx` + components.
