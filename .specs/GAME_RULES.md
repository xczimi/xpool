# xpool — Prediction Game Rules

A score-prediction pool for major soccer tournaments (FIFA World Cup, UEFA Euro).
Players predict the exact score of every match and earn points based on accuracy.

This document summarizes the **game rules as implemented in code** (`pool.py`,
`model.py`) and as described to players (`view/rules.html`). Where the two
disagree, it is flagged under [Known discrepancies](#known-discrepancies).

## Overview

- Players predict the exact score of each match by picking home/away goal counts.
- A prediction (a `Result` row) is only scored once the player **locks** it
  (`locked = True`). Unlocked predictions score **0** and stay hidden from other
  players to prevent strategic betting.
- Match scores are evaluated at the end of **90 minutes** (full time, second
  half). Extra time / penalties do not count toward the score prediction.
- The official tournament result is stored under a dedicated "result user"
  (`resultuser`); every other user's predictions are compared against it.

## Tournament structure

The tournament is modeled as a tree of `GroupGame` nodes:

- A `GroupGame` can contain sub-`GroupGame`s (`groupgames()`) and/or leaf
  `SingleGame`s (`singlegames()`).
- A `SingleGame` is one match: `homeTeam` vs `awayTeam` ("home" = team listed
  first), with a kickoff `time` and a final `homeScore`/`awayScore`.
- The group stage and each knockout round are `GroupGame` levels. In the
  knockout stage, **each individual match forms its own group** for ranking
  purposes.

## Scoring a single match — `singlegame_result_point(bet, result)`

For each match, comparing a player's `bet` to the official `result`:

| Condition | Points |
|-----------|--------|
| Exact home-team score guessed correctly | +1 |
| Exact away-team score guessed correctly | +1 |
| Correct outcome — predicted a home **win** and it was a home win | +2 |
| Correct outcome — predicted a **draw** and it was a draw | +2 |
| Correct outcome — predicted a home **loss** and it was a home loss | +2 |
| High-scoring fallback (see below) | +1 |

Maximum per match = **4 points** (2 exact scores + 1 outcome; the three outcome
checks are mutually exclusive).

### The "4-goal rule"

Intended rule (from `rules.html`): if a team scores **at least 4** goals,
predicting *any* count of "at least 4" for that team earns the score point —
you do not need the exact number.

In code this is the `elif` branch of each score check in `pool.py`. See
[Known discrepancies](#known-discrepancies) — the implementation does not match
the stated rule.

## Scoring a group's standings — `groupgame_result_point(bet, result)`

Beyond per-match points, players earn a **bonus** for predicting the relative
order of teams correctly.

- Each player's predictions imply a ranking of the teams in a group
  (`GroupResult.get_ranks()`).
- For **every pair of teams** that competed in the same group / knockout tie,
  the player earns **+1 point** if those two teams finish in the same relative
  order in the player's predicted standings as in the official standings.
- Implemented by `count_orders()`: it builds the set of ordered pairs for the
  player and for the official result, and counts the intersection.
- Average payout is ~1 bonus point per game in the group.

### Group standings & tie-breaking — `GroupResult.get_ranks()`

Standings are computed from the player's own predicted match scores:

1. **Points**: 3 for a win, 1 for a draw, 0 for a loss (`TeamGroupRank.pt`).
2. Teams are first ordered by points only (`TeamGroupPointRank`).
3. If teams are tied on points, tie-break rules are applied using only the
   games played **among the tied teams** (head-to-head): points, then goal
   difference, then goals scored (`TeamGroupRank.__cmp__`).
4. If teams are still tied, the player supplies an explicit `draw_order`
   (a manual ordering of team names) to break it — this models drawing of
   lots / extra time / penalties when a non-matching score is needed to decide
   qualification.

## Stage multipliers — `group_multiplier(game)`

Total points for a game node are multiplied by a stage factor:

- **Group stage**: ×1
- **Knockout rounds**: each successive knockout level (sorted by start time)
  adds +1 to the multiplier — first KO round ×2, next ×3, then ×4, and so on.

`score_group()` recursively sums single-match points + group-standings bonus
for a node, then multiplies the total by that node's `group_multiplier`.

## Scoreboard & "perfects"

- `scoreboard()` — sums every player's `score_group()` for the tournament,
  sorts descending; highest total wins. Cached in memcache.
- `perfects_single()` / `perfects_group()` — finds players who scored a
  "perfect" prediction on a match (default threshold = **4 points**, i.e. exact
  score + correct outcome). Surfaced on the `/perfect` page.

## Deadlines

- Group-stage predictions must be locked **before the first match of the
  group** kicks off.
- In the knockout stage each match is its own group, so deadlines are
  **per match**.
- Deadlines are shown on the "My Bets" page.

## Known discrepancies

Future devs should be aware that code and stated rules do not fully agree:

1. **4-goal rule, wrong field.** In `pool.py`, the away-score `elif` branch
   reads `bet.homeScore > 4 and result.homeScore > 4` — it checks the *home*
   score again instead of the away score. The away-team high-scoring fallback
   is therefore broken.
2. **4-goal rule, off-by-one threshold.** `rules.html` says "at least 4 goals"
   (≥4), but the code uses `> 4` (≥5). As written, the fallback only triggers
   at 5+ goals.
3. **Multiplier text vs. code.** `rules.html` lists Group ×1, 1/4 ×2, 1/2 ×3,
   Final ×4 and omits the 1/8 finals. `group_multiplier()` actually assigns +1
   per knockout level in start-time order, so with a Round of 16 present the
   multipliers shift (1/8 ×2, 1/4 ×3, 1/2 ×4, Final ×5).
4. **Stale tournament copy.** `rules.html` still references the "2012 UEFA Euro
   Cup" (and even "South Africa"); the rules text is not kept in sync with the
   active tournament.

## Key files

| File | Role |
|------|------|
| `pool.py` | Scoring engine: per-match points, group-order bonus, multipliers, scoreboard |
| `model.py` | Datastore models: `LocalUser`, `Team`, `GroupGame`, `SingleGame`, `Result`, `GroupResult`; standings & tie-break logic |
| `view/rules.html` | Player-facing rules page |
| `control.py` | Request handlers (scoreboard, tips, admin, etc.) |
| `fifa*.py`, `uefa*.py` | Per-tournament fixture/data setup |
