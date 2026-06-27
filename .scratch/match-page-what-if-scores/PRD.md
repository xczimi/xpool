# What-if scoreline columns on the live match page

Status: needs-triage
Area: web (+ possibly domain scoring)

## Idea

On the live match page (`/match/:gameId`), add "what if" columns that show, for
each in-progress match, what every player's score **would become** under a
hypothetical next result — e.g. a "if home scores" column and an "if away
scores" column (or the two candidate scorelines). Let people see how the next
goal would reshuffle the standings before it happens.

## Motivation

Mid-match, the gripping question is "what does the next goal do to me / to the
leader?". A what-if column turns the live page into a live race simulator. It is
the dynamic sibling of [[live-max-achievable-points]]: max-achievable assumes
current predictions hold to the final whistle, whereas what-if perturbs the
live scoreline by one goal and re-scores on the spot.

## Sketch

- For the selected live match, take the current live score and compute two (or
  more) hypothetical scorelines: home +1, away +1.
- Re-run the scoring for that match against each player's prediction under each
  hypothetical, and show the resulting point delta / new total per player.
- Respect tip-visibility gating — only show this once tips are revealable
  (deadline passed / kickoff), same gate as [[match-page-prediction-stats]].
- Likely client-side: reuse the already-loaded, gated tip rows and the existing
  scoring rules; no new resolver if the scoring logic can run in the browser.

## Resolved decisions (2026-06-27 grill)

- **±1 goal each side only:** two columns, "if home scores" / "if away scores". No arbitrary score picker this round.
- **Show both** absolute new total and delta vs current, with the delta emphasised.
- **Pool scope:** the sticky selected pool (consistent with [[perfect-page-pool-picker]]).
- **Client-side:** reuse the already-loaded, gated tip rows and run the existing scoring rules in the browser — **no new resolver**.
- Respect tip-visibility gating (only once revealable; same gate as [[match-page-prediction-stats]]).
- Cluster: `cluster/match-page` (Wave 1).
