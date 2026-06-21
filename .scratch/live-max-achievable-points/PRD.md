# Max achievable points per player during live matches

Status: needs-triage
Area: web (+ api/domain scoring)

## Idea

While matches are **live**, show each player not just their current points but
their **maximum still-achievable points** — the ceiling if every in-progress
match ended exactly as they predicted from here.

## Motivation

Mid-match, the scoreboard total is provisional. "Can I still catch the leader?"
is the gripping question during live play. A max-achievable column turns the
scoreboard into a live race rather than a static tally, and builds on the
[[sportsdb-live-preview]] live-score work.

## Sketch

- For each live match, compute the best score a player could still earn given
  the current live score and their prediction (some outcomes may already be
  unreachable — e.g. they predicted a draw but it's 2–0 late).
- Player ceiling = locked/settled points + best-case live points + not-yet-
  started predicted points.
- Reuse the pure `domain` scoring engine for the "what would this prediction
  score against result X" calc; the hard part is enumerating still-reachable
  results per live match.
- Surface as a column / secondary number on `ScoreboardPage`, clearly marked
  provisional.

## Open questions

- How to bound "still reachable" — any future result, or constrained by current
  live score + time remaining (we likely only know the score, not minute)?
- Server-computed (new resolver) vs client-side from live scores + predictions?
- Show only during live windows, or always (= current + remaining fixtures)?

## Resolved decisions (grilled 2026-06-21; re-scoped 2026-06-21)

**Correction (re-scope):** the first cut computed a per-player *tournament-total*
ceiling and showed it as a "Max" column on the **Scoreboard**. That was the
wrong place. The ceiling people care about mid-match is **per-match**, not a
running total: *"can my tip for THIS live game still score the max?"* So the
feature now lives on the **live match page (`/match/:gameId`)**, as a per-player
still-reachable ceiling on each tip row — NOT a scoreboard total column. The
scoreboard renders exactly as it did before this feature.

- **Ceiling = best mathematically-reachable score for the one live match.** For
  a live match at H–A, enumerate finals >= the live score (goals only go up) and
  take the max points the prediction could still earn via `domain::score_match`.
  Honest about partially-lost outcomes (e.g. predicted 1–0 but it's 0–2). The
  pure primitive is `domain::scoring::max_reachable_score(prediction, live,
  config, multiplier)` (unchanged — verified by its own unit tests).
- **Per-match, per-player.** Surfaced as `Tip.maxReachable` on the `match`
  resolver's `rows`. `None` unless the match is live (a live *provisional*
  score) **and** the tip is visible. The all-tips grid (`tips`) never shows it;
  a not-yet-started or official-final match returns `null`.
- **Shown only while the match is live**, as a small secondary "max ≤ N"
  indicator beside the points cell, clearly marked provisional; otherwise the
  match page looks as today. NO `Date.now()` — the live state is server-derived
  (`actual.provisional`).
- **Computed server-side in the `match` resolver**, reusing the live score it
  already fetches via the SportsDB source. Client can't get live scores
  directly. Reuses the pure `domain::max_reachable_score`.
