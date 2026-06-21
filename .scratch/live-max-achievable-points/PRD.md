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
