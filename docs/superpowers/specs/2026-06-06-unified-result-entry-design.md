# Unified result entry — design

**Date:** 2026-06-06
**Status:** approved

## Goal

The result user (the official/admin `Player` with `is_result_user = true`, whose
`MatchPrediction`s *are* the official results) enters results through the **same
My Tips form players use to enter predictions** — no separate admin Results
screen.

The key realisation: **the prediction deadline already implicitly locks an
entered result**, the same way it locks a prediction (`effective_locked = locked
|| (now > deadline && complete)`). That `deadline` is the **group's deadline —
the earliest kickoff in the group, not each game's own kickoff** (`Tournament::
deadline`); it applies uniformly to every game in the group. Because official
results are always entered *after* the match (hence past that deadline), a result
is effective-locked the moment it is entered — so the scoring/display rules
become **symmetric** between predictions and results, with **no `is_result_user`
special case**. The only genuine special-casing is on the **write** path:

1. **Time** — the result user is not blocked by a group's deadline (they enter
   results *after* matches conclude, i.e. after the deadline has passed).
2. **Recompute on write** — a save by the result user triggers the wholesale
   scoreboard/bracket recompute (their predictions *are* the official results).

Scores are calculated **on write** (when a result is entered/updated), not on
read — this is the existing materialised-scoreboard model
(`crates/api/src/recompute.rs`), which the unified entry path simply triggers.

This supersedes the divergence between spec **ADMIN-04** ("enter a result through
My Tips") and the implementation, which had grown a separate `/admin` Results
screen + `enterResult` mutation.

## Background — why this is needed

The result user currently *cannot* enter results through My Tips, and the
dedicated `/admin` path is a parallel implementation. Two mechanisms blocked the
unified flow, and a third made entered results invisible to scoring:

- `submitGroup` rejects any save once `now > deadline` (`mutation.rs:226`),
  applied to **all** callers including the result user.
- `GroupTipForm` forces `readOnly` when `deadlinePassed` (`GroupTipForm.tsx:106`),
  again without exempting the result user.
- Scoring counts a match only when the **result is explicitly locked**
  (`scoring.rs:444`, `if !result_mp.locked { continue; }`; `SCORING.md:27`
  "a result counts only when locked. Unlocked → 0"). So even a result entered
  via the admin path scored nothing until locked.

The result-user design's whole point was to avoid a separate admin results UI —
official results are just a player's predictions, with a deadline exemption.

## What the identical form already covers (no new machinery)

The existing `GroupTipForm` + `submitGroup` path is already sufficient for the
result user, because:

- **Per-match knockout entry/scoring** — each knockout match is its own
  one-match `LockPerMatch` group (`fwc26.json`), so entering one result is
  naturally per-match.
- **Knockout draw advancer** — knockout groups have `carries_standings = true`;
  the form's draw-order control (`GroupTipForm.tsx:240`) reorders the two teams,
  setting `ordering[0]`, which `fwc26::determine_winner_loser` reads as the
  advancer when a knockout match ends level.
- **Group standings + tie-breaks** — the same draw-order control records the
  official standings/`draw_order` for the group-stage standings bonus.
- **Calculate-on-write** — `recompute()` already materialises the scoreboard +
  bracket wholesale; we trigger it on the result user's save.

## Design

### 1. Scoring (domain) — results are effective-locked, symmetric with predictions

In `score_leaf_group` (`crates/domain/src/scoring.rs`), the **result side** uses
`effective_locked` exactly as the prediction side already does — the group
deadline (earliest kickoff in the group, not each game's own kickoff) implicitly
locks an entered result. **No `is_result_user` branch.**

```rust
// Per-match — was: if !result_mp.locked { continue; }
let r_locked = effective_locked(result_mp.locked, now, deadline, true);
if !r_locked { continue; }

// Standings bonus — was: if !result_sp.locked { return raw; }
let r_sp_locked =
    effective_locked(result_sp.locked, now, deadline, !result_sp.ordering.is_empty());
if !r_sp_locked { return raw; }
```

`effective_locked(locked, now, deadline, complete) = locked || (now > deadline &&
complete)`. Because results are entered after the match (after the group
deadline), `now > deadline` holds, so an entered result counts immediately — no
explicit lock needed. The rule is now identical for both prediction and result.

Spec/test follow-on:

- `SCORING.md:26-27` — change "a result counts only when locked. Unlocked → 0"
  to: a result counts when **effective-locked**, the same rule as a prediction
  (kickoff/deadline implicitly locks it).
- `SCENARIOS.md` **SCORE-13** — reframe to the symmetric rule: an unlocked
  prediction *or* result scores zero **only before the deadline**; after the
  deadline a complete, entered result counts.
- Domain test `score_tournament_unlocked_result_scores_zero` — rename/rework so
  `now` is **before** the deadline (asserts 0), and add a sibling asserting an
  unlocked result **after** the deadline **scores** (mirrors the existing
  `score_tournament_auto_locked_after_deadline` for predictions).

### 2. Read-gates (API) — display results once they're in, not once locked

Three resolvers in `crates/api/src/gql/query.rs` currently treat a result as
official only when its raw `locked` flag is set. With results counting in scoring
the moment they're entered (post-kickoff), these must match, or an entered result
would update the scoreboard yet still show as "result pending" with a `—` score.
Relax each to **presence** (the result user only ever enters a result after
kickoff, which is exactly the effective-lock condition):

- `tournament` resolver (`query.rs:38-48`) — the official-results set that drives
  the `resultPending` / "result in" flags: drop `.filter(|p| p.locked)`, use all
  of the result user's entered match predictions.
- `results` query (`query.rs:243-256`) — drop `.filter(|p| p.locked)`; return the
  result user's entered match predictions. Update the doc comment ("locked" → "entered").
- `perfects` resolver (`query.rs:226-235`) — change `if result.locked &&
  is_perfect(..)` to `if is_perfect(..)`.

Note the read-gates key on **presence** while scoring keys on `effective_locked`
(presence **and** `now > deadline`). They coincide by assumption — the result
user only enters a result after kickoff (past the group deadline) — not by an
enforced lower bound on the write path. If a result were entered *before* the
group deadline, the UI would show it "in" while the scoreboard scored it 0 until
the deadline passed; this is operator-error-only and self-corrects, so we accept
the presence-based gates rather than thread `now`/`deadline` through each resolver.

(The `tips` reveal logic at `query.rs:182-193` is about *player* predictions, not
results — `is_result_user` players are skipped — so it is unchanged.)

### 3. Backend — `submitGroup` result-user write exemption

In `crates/api/src/gql/mutation.rs::submit_group`, when `viewer.is_result_user`:

- **Skip the deadline check** (`mutation.rs:226`) — the result user must save
  results *after* the deadline. This is the one genuine special case.
- **After a successful persist, auto-fire `recompute(repo, now)`** — calculate on
  write. Best-effort/non-fatal, matching today's `enter_result` philosophy (a
  recompute failure is logged, not surfaced as a mutation error; the scoreboard
  self-heals on the next entry or the internal `recompute` mutation). Regular
  players never trigger recompute.

Unchanged: validation and optimistic-concurrency retry. The result user saves
with `lock=false` (the normal "Save draft" path), so PRED-03 lock-completeness is
not triggered and the "already-locked prediction cannot change" guard never fires
— results stay unlocked and freely re-editable, so *correcting mistakes* needs no
special code.

### 4. Frontend — identical My Tips form for the result user

In `web/src/pages/mytips/GroupTipForm.tsx` (and `MyTipsPage.tsx`), when
`me.isResultUser`:

- `deadlinePassed` no longer forces `readOnly` (`GroupTipForm.tsx:106-111`) — the
  form stays editable after kickoff, and entries remain editable (re-correction).
- Everything else is byte-for-byte the same form: score selects, the
  draw-order/standings control, and the Save/Lock buttons. "Save draft" is all
  the result user needs — the saved results are effective-locked (post-kickoff)
  and recompute makes them count and display.

### 5. Remove the redundant admin results path

- Delete the `/admin` **Results** tab + route from `web/src/pages/AdminPage.tsx`.
- Delete `web/src/pages/admin/AdminResults.tsx` and its result-entry i18n
  strings (`adminResults`, the manual-recompute notices).
- Delete the `enterResult` / `unlockResult` mutations (now redundant; their
  per-game lock, advancer, and recompute responsibilities are covered by the
  unified `submitGroup` path). Keep the internal `recompute()` function and the
  `recompute` mutation as an ops self-heal.
- **Keep** the `/admin` **Teams** and **Players** management screens untouched.

## Consequence: per-match scoreboard updates everywhere

Because results score the moment they're entered (recompute on write, no lock
required), the scoreboard updates **per-match** in every round — including the
group stage. The earlier concern that `LockTogether` group-stage groups would
only score after a whole-group lock is **moot**: locking is no longer a
prerequisite for a result to count.

## Note: what explicit locking is for

Explicit `locked` exists for one reason (carried over from the legacy app) —
**a player can lock predictions before the deadline to let rivals see them
early** (the reveal rule at `query.rs:182-193`:
a prediction is visible to others when `locked || now >= kickoff || deadline
passed`). It was never meant as a scoring gate. This design keeps that
player-side reveal mechanism exactly as-is, and stops *results* from depending on
it — results are entered post-kickoff and are effective-locked automatically, so
the result user never needs to press Lock.

## Non-goals / explicitly unchanged

- Player prediction semantics: deadline gating, `effective_locked` auto-counting
  after the deadline, PRED-03 group-lock completeness, and the early-reveal-on-lock
  behaviour for players.
- The server-authoritative clock model (`X-Dev-Now` → `XPOOL_NOW` → real clock).
- Teams/Players admin screens.
- The recompute architecture itself (still wholesale, materialised, on write).

## Risks

- **Scoring is a locked contract.** `scoring.rs` / `model.rs` are depended on
  across crates; the change is small (result side now mirrors the prediction side
  via `effective_locked`) but must land with updated `SCORING.md` §1,
  `SCENARIOS.md` SCORE-13, and domain tests in the same change. The symmetry
  preserves what-if/relative scoring: every baseline (official or not) counts a
  result/prediction by the same effective-lock rule.
- **Read-gates must move with scoring.** The three `query.rs` resolvers in §2 are
  the ones that make an entered result *visible*; if any is missed, the scoreboard
  moves but the UI shows "result pending". They are listed explicitly so none is
  forgotten.
- **Drafts score immediately after kickoff.** A mistyped result hits the
  scoreboard on save (it is effective-locked once entered post-kickoff). Accepted:
  results are entered post-match as facts, and the result user can always re-edit
  (which re-triggers recompute).
- **Removing `enterResult`/`unlockResult`** is a public GraphQL surface change;
  confirm no other consumer (web only uses them in the deleted AdminResults).

## Testing

- **Domain unit** (`crates/domain/tests/scoring.rs`): the result user's *unlocked*
  result awards points; an *unlocked prediction* still scores zero (regression).
- **API**: `submitGroup` as the result user past a group's deadline succeeds and
  triggers recompute; as a regular player past the deadline still errors.
- **E2E** (Playwright; needs the dev-stub-auth `web/.env.local` blanking
  `VITE_AUTH0_*`): result user logs in → My Tips → enters a group-stage result
  with `X-Dev-Now` set after kickoff → the player scoreboard reflects the new
  score immediately; assert there is no `/admin` **Results** route.
- **Regression**: a regular player is still deadline-locked (read-only) on My
  Tips after the deadline.
