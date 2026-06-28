# xpool — Scoring Engine

The rewrite's **decided** scoring design, settled in design review. Authoritative.

[`GAME_RULES.md`](./GAME_RULES.md) remains the *legacy* rules-and-bugs reference;
this document is the *rewrite's* scoring authority and records which legacy
discrepancies it resolves ([§10](#10-resolved-legacy-discrepancies)).
See also [`DATA_MODEL.md`](./DATA_MODEL.md) for entities and storage.

---

## 1. Shape

The engine is **pure functions**, no I/O:

```
score(prediction, result, config: ScoringConfig) -> points
```

`prediction` and `result` are both prediction-sets (the official result is the
"result user" — see `DATA_MODEL.md` §5), so scoring is **symmetric**: any
prediction-set may be the baseline. The official scoreboard uses the result
user; player-vs-player and "what-if" scoring reuse the same function with a
different baseline.

A prediction *and* a result contribute only when **effective-locked**
(`DATA_MODEL.md` §7) — the same rule for both: `locked || (now > deadline &&
complete)`. Because official results are entered after the match (past the
deadline), an entered result is effective-locked immediately; explicit
`locked` is a player-only early-reveal flag, never a scoring gate.

"Effective-locked" is decided **per `MatchPrediction`**, never per group. The
`complete` term in `effective_locked` is read per-match: a `MatchPrediction`
always carries both `u8` scores, so it is always complete. In an unlocked
`LockTogether` group, each game a player *did* predict auto-counts on its own
after the deadline; games the player left unpredicted have no `MatchPrediction`
and simply score 0. There is no group-level "all matches predicted" gate on
this auto-count path. (An explicit *lock* of a whole group is a separate bulk
action, and that path may require all games — see `DATA_MODEL.md` §7.)

## 2. `ScoringConfig`

A struct of **centralized source-code constants**, passed into the engine. Not
a stored entity, no admin UI — tuned before launch and redeployed. (Moving it
into storage for runtime editing is a possible later upgrade; not now.)

| Field | Default | Meaning |
|---|---|---|
| `exact_score_point` | 1 | per side, for an exact score |
| `outcome_point` | 2 | for the correct W/D/L outcome |
| `high_scoring_threshold` | 4 | the "4-goal rule" threshold (≥) |
| `standings_pair_point` | 1 | per correctly-ordered team pair |
| `perfect_threshold` | 4 | points needed for a "perfect" |
| `multiplier[round]` | Group 1, R32 2, R16 3, QF 4, SF 5, ThirdPlace 5, Final 6 | per-round stage multiplier |

Defaults are *seeded* values — the admin's call before launch, not locked rules.

## 3. Per-match scoring

Per `SingleGame`, comparing prediction `P` to result `R` (both **90-minute**
scores — see [§5](#5-the-90-minute-rule)):

- **Home side** — earn `exact_score_point` iff
  `P.home == R.home` **OR** (`P.home ≥ threshold` **AND** `R.home ≥ threshold`).
- **Away side** — earn `exact_score_point` iff
  `P.away == R.away` **OR** (`P.away ≥ threshold` **AND** `R.away ≥ threshold`).
- **Outcome** — earn `outcome_point` iff the predicted W/D/L equals the actual
  W/D/L (`sign(P.home − P.away) == sign(R.home − R.away)`).

Maximum per match = `1 + 1 + 2 = 4`.

The OR clause is the **"4-goal rule"**: a side scoring `≥ threshold` is matched
by *any* prediction of `≥ threshold` for that side — the exact count isn't
needed. The clause is per-side and symmetric (fixes the legacy bug where the
away check read the home field).

## 4. Standings bonus

Beyond per-match points, a per-group bonus rewards predicting the **relative
order** of teams.

**Predicted standings** are computed from the player's own predicted match
scores, ranked by this ladder — applied in order, stopping when the tie breaks:

1. Points — 3 win / 1 draw / 0 loss.
2. Head-to-head among the tied teams: points, then goal difference, then goals.
3. All-matches goal difference.
4. All-matches goals scored.
5. The player's manual **`draw_order`**.

The ladder covers only **score-derivable** criteria. Everything below — real
disciplinary conduct, FIFA ranking/coefficient, drawing of lots — is *not*
modelled by the engine; it collapses into the opaque manual `draw_order`. This
keeps the engine generic and unaffected if a tournament changes its non-score
tiebreakers.

**Bonus** — for every pair of teams in a group, award `standings_pair_point` if
the pair's relative order in the player's predicted standings matches the
official standings (the result user's standings, computed by the same ladder).
A 4-team group → up to 6 pairs; a knockout one-match group → 1 pair.

**Completion gate (monotonicity).** The standings bonus is awarded **only once
the group is complete** — i.e. every game in the leaf group has an official
(result-user) result entered and effective-locked. Until the group is complete
the bonus is `None` / 0. Rationale: the provisional official ranking shifts as
results land, so a bonus credited from partial results could *decrease* later;
a player's committed points must never go down. Gating on completion makes the
bonus final-and-stable (monotonic) and reconciles the materialised scoreboard
exactly with the points-timeline, which already settles a group's bonus at its
last resulted game. This **supersedes** any earlier wording that described the
bonus as awarded provisionally from the group's first kickoff. (Match points are
already monotonic — fixed once a result is entered — so only the standings bonus
needed this gate.) Enforced in `domain::scoring::standings_score`, so it flows to
the scoreboard, the `standings` resolver, and the timeline alike.

## 5. The 90-minute rule

Match scores are evaluated at **full time / 90 minutes**. Extra time and
penalties do **not** count toward the per-match score.

A knockout result is therefore two facts:

- the **90-minute score** — what [§3](#3-per-match-scoring) is judged against;
- the **advancer** — who progressed, possibly via ET/penalties.

The advancer rides on the one-match group's `StandingsPrediction` (the 2-team
ordering). It composes with the [§4](#4-standings-bonus) ladder: a *decisive*
predicted 90-minute score derives the advancer automatically; a predicted
*draw* requires the player to set the advancer explicitly via `draw_order`.

> Ingestion note: feeds (TheSportsDB, FotMob) report a knockout match's *final*
> score, often inclusive of ET. The model needs the **90-minute** score —
> automated ingestion can't be trusted for ET matches; a human confirms. Belongs
> in [`DATA_SOURCES.md`](./DATA_SOURCES.md).

## 6. Stage multipliers

A node's total = (Σ per-match points + standings bonus for that node) ×
`multiplier[round]`. Summed recursively over the `GroupGame` tree.

Multipliers are an **explicit per-round table** in `ScoringConfig` — never
derived from start-time order (that derivation was the cause of the legacy's
multiplier drift). The 3rd-place playoff is its own round, defaulted to the SF
multiplier — a real knockout match, but not crowned equal to the Final.

## 7. "Perfect"

A match prediction is a **perfect** when it scored `perfect_threshold` points
(the maximum). Defined purely as "scored the max" — *not* "exact scoreline" —
so a maximum reached via the 4-goal rule still counts. Drives the `/perfect`
page.

## 8. Scoreboard

Materialized as a single **`<t>#SCOREBOARD`** item — `playerId → {stage →
score}` (per-stage breakdown serves UC-8's overall *and* per-stage views).

- Recomputed **wholesale** on the events that change results: an admin locks or
  edits a result, or edits a prediction post-deadline. A single item, full
  rebuild — no per-node cache, no invalidation cascade.
- Every scoreboard — global or any pool — is served by **one `GetItem`** of this
  item, then filter to pool members + sort.
- The on-demand `score()` path still exists (it performs the recompute, and
  powers what-if / player-vs-player scoring); the materialized item is a cached
  output of it, not a second algorithm.

## 9. Engine placement & testing

- Pure, I/O-free; lives in a `domain`-style crate, no AWS or DynamoDB deps.
- Highest-risk code in the project — covered by a thorough unit suite,
  including explicit regression tests for the resolved discrepancies in §10 and
  the §4 ladder's edge cases.

## 10. Resolved legacy discrepancies

From `GAME_RULES.md` "Known discrepancies":

| # | Legacy | Resolution |
|---|---|---|
| 1 | 4-goal rule's away check read the *home* field | Fixed — §3's per-side symmetric definition |
| 2 | 4-goal threshold `> 4` (≥5) vs stated ≥4 | Threshold = **≥4**; `high_scoring_threshold` config, default 4 |
| 3 | Multipliers derived from start-time order, drift when bracket changes | Explicit per-round table in `ScoringConfig` (§6) |
| 4 | Stale player-facing rules copy | A UI concern, not the engine — out of scope here |

## 11. Open / deferred

- **What-if / player-vs-player scoring** — the symmetric engine supports it; the
  feature surface (where it appears, who can run it) is not yet designed.
- **Runtime-editable `ScoringConfig`** — deferred; constants for now.
