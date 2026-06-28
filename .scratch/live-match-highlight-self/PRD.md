# Highlight the current player in the live match player list

Status: done — shipped via backlog-parallel-build (cluster/match-page), merged to master 2026-06-27
Area: web

## Idea

On the live match page (`/match/:gameId`), visually highlight the **current
(logged-in) player's** row in the list of players / predictions, so you can find
yourself at a glance without scanning.

## Motivation

During a live match the list of players and their tips can be long. The one row
everyone cares about most is their own. A subtle highlight (background tint,
sticky-to-top, or a "you" marker) makes self-location instant and pairs with the
[[own-player-page-access]] / "always render my own page" work.

## Sketch

- Identify the current player from the existing auth/session context.
- Apply a distinct row style (background tint + maybe a small "you" badge).
- Optionally pin/scroll the player's own row into view on load.
- Consistent with any highlight styling already used elsewhere (scoreboard,
  perfect page) so it reads as the same "this is you" convention.

## Resolved decisions (2026-06-27 grill)

- **Highlight only** (background tint + a small "you" badge). No pin-to-top this round.
- **Reuse** any existing "you" convention/style; if none exists, introduce a minimal one and keep it consistent.
- **Scope** to the match-page player list this round; scoreboard / other lists deferred.
- Pairs with [[match-page-sort-predictions]] so "you" stays findable regardless of sort.
- Cluster: `cluster/match-page` (Wave 1).
