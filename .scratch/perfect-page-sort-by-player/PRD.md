# Sort / group the Perfect page by player

Status: needs-triage
Area: web

## Idea

Let the Perfect page be **sorted or grouped by player**, so you can see one
participant's perfect tips together rather than a flat list ordered by match.

## Motivation

"Who has the most perfect tips, and which ones?" is a natural way to read the
Perfect page. Grouping by player turns it into a mini-leaderboard of spot-on
predictions and pairs well with the [[player-detail-page]] (each player's
perfects already appear there).

## Sketch

- A sort/group toggle on `web/src/pages/PerfectPage.tsx`: by **match** (current)
  ⇄ by **player**.
- By-player: section per participant, their perfect tips listed under their name,
  perhaps ordered by perfect-count (most first).
- Reuse existing player links (`/player/:id`) and nick rendering.
- Plays nicely with the [[perfect-page-pool-picker]] scoping.

## Open questions

- Sort (reorder a flat list) or true grouping (sectioned headers)?
- Order players by perfect-count, by standings, or alphabetically?
- Persist the chosen grouping per user?
