# Unified result entry — design

**Date:** 2026-06-06
**Status:** approved

## Goal

The result user (the official/admin `Player` with `is_result_user = true`, whose
`MatchPrediction`s *are* the official results) enters results through the **same
My Tips form players use to enter predictions** — no separate admin Results
screen. The only special-casing is:

1. **Time** — the result user is not blocked by a group's deadline (they enter
   results *after* matches conclude, i.e. after the deadline has passed).
2. **No-lock scoring** — the result user's results award points to players the
   moment they're entered; they do **not** need to be explicitly locked first.

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

### 1. Scoring (domain) — official results count by presence, not lock

In `score_leaf_group` (`crates/domain/src/scoring.rs`), relax the **result-side**
lock guard so the official result user's entries count immediately, while
non-official baselines still require locked data (preserving the symmetric
what-if / player-vs-player semantics in `SCORING.md` §1):

```rust
// Per-match (was: if !result_mp.locked { continue; })
if !result.is_result_user && !result_mp.locked { continue; }

// Standings bonus (was: if !result_sp.locked { return raw; })
if !result.is_result_user && !result_sp.locked { return raw; }
```

The **prediction** side is unchanged — players still need `effective_locked`
(`locked || (now > deadline && complete)`). `result.is_result_user` is available
on the `result: &Player` argument.

Spec/test follow-on:

- `SCORING.md:27` — change "a result counts only when locked. Unlocked → 0" to:
  a result by the **result user** counts by presence; any *other* baseline still
  requires locking. Predictions are unaffected.
- `SCENARIOS.md` **SCORE-13** ("Unlocked predictions and results score zero") —
  split: an unlocked *prediction* still scores zero; an unlocked *result by the
  result user* now scores.
- Domain test `score_tournament_unlocked_result_scores_zero` — flip to assert the
  result user's unlocked result **does** score; keep the unlocked-prediction case.

### 2. Backend — `submitGroup` result-user exemptions

In `crates/api/src/gql/mutation.rs::submit_group`, when `viewer.is_result_user`:

- **Skip the deadline check** (`mutation.rs:226`) — exemption: *time*.
- **After a successful persist, auto-fire `recompute(repo, now)`** — calculate
  on write. Best-effort/non-fatal, matching today's `enter_result` philosophy
  (a recompute failure is logged, not surfaced as a mutation error; the scoreboard
  self-heals on the next entry or via the internal `recompute`). Regular players
  never trigger recompute.

Unchanged: validation, optimistic-concurrency retry, and PRED-03
lock-completeness (only relevant when the result user chooses to lock — see
below). The "already-locked prediction cannot change" guard does not block the
result user in practice because results stay unlocked and remain editable;
should the result user explicitly lock a result and later need to correct it,
the guard is also skipped for `is_result_user` (exemption: *correcting mistakes*).

### 3. Frontend — identical My Tips form for the result user

In `web/src/pages/mytips/GroupTipForm.tsx` (and `MyTipsPage.tsx`), when
`me.isResultUser`:

- `deadlinePassed` no longer forces `readOnly` (`GroupTipForm.tsx:106`) — the
  form stays editable after kickoff.
- Locked entries remain editable for the result user (correcting mistakes).
- Everything else is byte-for-byte the same form: score selects, the
  draw-order/standings control, and the lock button. The lock button stays for
  parity but is **optional** for the result user — it no longer gates scoring.

### 4. Remove the redundant admin results path

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

## Non-goals / explicitly unchanged

- Player prediction semantics: deadline gating, `effective_locked` auto-counting
  after the deadline, and PRED-03 group-lock completeness for players.
- The server-authoritative clock model (`X-Dev-Now` → `XPOOL_NOW` → real clock).
- Teams/Players admin screens.
- The recompute architecture itself (still wholesale, materialised, on write).

## Risks

- **Scoring is a locked contract.** `scoring.rs` / `model.rs` are depended on
  across crates; the change is small (two guard conditions) but must land with
  updated `SCORING.md` §1, `SCENARIOS.md` SCORE-13, and domain tests in the same
  change. The `is_result_user` exemption deliberately preserves what-if/relative
  scoring (a non-official baseline still needs locked data).
- **Drafts score immediately.** With no lock prerequisite, a mistyped result hits
  the scoreboard on save. Accepted: results are entered post-match as facts, and
  the result user can always re-edit (which re-triggers recompute).
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
