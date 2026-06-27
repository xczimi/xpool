# Best-thirds knockout placement fix (parts A–D) — design

**Date:** 2026-06-27
**Status:** Design approved; ready for implementation plan.
**Supersedes the fix direction in:** `.scratch/best-thirds-table/bracket-placement-bug-FINDINGS.md`
**Builds on:** Phase 1 (best-thirds display table), now **merged to master** (commit `224466d`).

---

## 1. Problem

`fwc26` resolves the Annexe C mapping (which group-winner faces which best third in
the R32) as soon as **8** third-placed teams are known, not when **all 12** groups are
final. Because the top-8 selection genuinely depends on all 12 thirds, resolving early
can place the **wrong** team into a knockout slot. The resolved teams are **persisted**
(the post-result hook writes `TeamSlot.team_id` back onto the stored tournament), so a
premature/wrong placement is durable and shown to everyone until corrected.

The bug has already fired in prod: all 8 best-third R32 slots are resolved off the 9
complete groups (A–I) even though groups J/K/L are not final, and 11 locked predictions
sit on those matches (M74/M77/M79/M81). See the FINDINGS doc §3 for the full prod
analysis.

Only the best-third (`"3...."`) slots are affected — `"1X"`/`"2X"` (group winner/runner-up)
resolve from a single group and are correct as soon as that one group is final.

## 2. The architectural spine: two notions of group standings

The fix hinges on separating two things the current code conflates:

| | **Final standings** (exists) | **Provisional standings** (new) |
|---|---|---|
| Function | `compute_group_standings` / `compute_standings_for_group` | new partial-tolerant helper |
| Includes | only groups where **every** game has a result (the `collect::<Option<Vec<_>>>()?` short-circuit at `lib.rs:481`) | **all 12** groups, ranked from whatever results exist so far |
| Built via | `rank_group` with **all** predictions required | `rank_group` with the **available** predictions filtered in |
| Drives | **placement** (B) + the **all-12-final** gate | the **display table** (A) |

Key consequence: because `compute_group_standings` already contains **only complete
groups**, **"all 12 groups final" is exactly `group_standings.len() == 12`**. `fwc26`
needs nothing from the api crate's `group_complete` helper.

`rank_group` (`domain/scoring.rs:451`) collects teams from the game slots (concrete from
the start) and ranks from whatever predictions are passed, so calling it with only the
played games yields a provisional order — including a 3rd-placed team — even for a
partially-played or unplayed group.

## 3. Part B — Placement gate fix (`crates/fwc26/src/lib.rs`)

Replace `qualifying_set.len() == 8` at **both** sites with an all-12-final check
(`group_standings.len() == 12`):

- `lib.rs:318` — `ResolutionContext::build` (the placement path).
- `lib.rs:180` — `third_place_ranking` (the display pairing path; see Part A).

Effect: the `"3...."` best-third slots resolve to `None` until every group A–L is final.
`"1X"`/`"2X"` slots keep resolving per-complete-group as today. This also fixes the
persisted-data path with no extra code — the next recompute (when a J/K/L result lands)
re-nulls the premature placements because the gate is no longer satisfied.

## 4. Part A — Display all 12 provisionally (`fwc26` + `gql`)

Rewrite `third_place_ranking` to use **provisional** standings:

1. For each group A–L, compute provisional standings from the available results and take
   the 3rd-placed team. Always emit a row (decision: **always show the standings 3rd**,
   even for an all-tied / unplayed group — the row shows the positional 3rd).
2. Rank the 12 provisional thirds across groups (same criteria as `best_thirds`: points →
   GD → GF → group-letter stable fallback).
3. Flag the **provisional** top-8 (`qualifies = rank ≤ 8`) and assign `rank`. These are
   shown during the group stage, clearly provisional.
4. Compute the Annexe C pairing (`faces_winner_group` / `faces_game`) **only when all 12
   groups are final** (the same gate as B). Until then they are `None`.

`third_place_ranking` returns the 12 rows plus an **all-12-final** flag (e.g. a small
struct `{ rows, all_groups_final }`) so the resolver can set `complete` correctly.

### GraphQL contract (`query.rs:811`)

`complete` changes from `entries.len() == 12` (now always 12, hence meaningless) to the
**all-12-final** flag returned by `fwc26`. Entries are always 12; `rank` / `qualifies`
are provisional; `faces_*` are null until `complete`.

## 5. Part C — Prediction gating (`crates/api/src/gql/mutation.rs` + web)

Uniform per-slot rule:

> A **knockout-round** match (any `Round` other than `GroupStage`) accepts a prediction
> only when **both** its home and away `TeamSlot.team_id` are concretely placed (`Some`).

- **Server:** enforce in the submit/lock path (`mutation.rs` ~244–272). Reject a
  prediction for a knockout match with an unresolved slot, with a clear error.
- **Web:** disable the prediction input for knockout matches whose slots aren't both
  resolved; show an explanatory state (teams not yet determined).

This composes with B automatically: best-third matches stay blocked until all 12 groups
are final (their slots are `None` per B); `1X`/`2X` matches open as soon as their groups
complete. Because a team is placed only after all groups are final and predictions are
only accepted after placement, a placed team never changes afterward — so there is
nothing to "unlock later" going forward, and **no runtime unlock-on-change mechanism is
needed** (decision: dropped, relying on B+C).

## 6. Part D — One-time prod cleanup (one idempotent `xtask` command)

A dedicated `xtask` subcommand operating on the live table, run once at deploy. Two
logically separate concerns inside one command:

1. **Re-resolve (re-null):** force `resolve_bracket` with the fixed code and persist, so
   the 8 premature best-third slots revert to `None` at deploy time (rather than waiting
   for the next J/K/L result).
2. **Unlock:** unlock any locked prediction on a knockout match whose slot is now
   unresolved (the 11 identified on M74/M77/M79/M81). The criterion is **structural**
   ("locked prediction on a knockout match with an unresolved slot"), not a hardcoded
   list — so the command is idempotent and safe to re-run.

**Verify at deploy:** unlocking only helps before a match's kickoff (knockout deadline =
match kickoff). Confirm the affected R32 matches' kickoffs are still in the future against
the live clock when the command runs (they are R32, after the group stage — expected fine).

## 7. Testing

- **`fwc26` unit:** 8 complete groups → **no** best-third placement (slots `None`); all 12
  complete → placement resolves; provisional 3rd emitted for an incomplete / unplayed
  group; provisional top-8 flagged before all 12 final; `faces_*` null until all 12 final.
- **`api`:** prediction gating rejects a knockout submit with an unresolved slot; accepts
  once both slots placed; `complete` flag reflects all-12-final, not `entries.len()`.
- **web/e2e:** best-thirds table renders 12 provisional rows; knockout prediction input is
  disabled while slots are unresolved and enabled once placed.
- **Cleanup fixture:** `snapshots/prod-snapshot.json` — verify the 8 slots re-null and the
  11 locked predictions unlock; re-running the command is a no-op.

## 8. Branch / delivery

Phase 1 is merged, so this work branches fresh from `master`. Code changes (`crates/*`,
`web/`) go on a branch/worktree per the working agreement; this design doc and the
implementation plan are documentation and may land on `master` directly.

Suggested sequencing: **B** (the core gate fix, smallest, fixes the live bug) → **A**
(provisional display + `complete` contract) → **C** (prediction gating) → **D** (one-time
cleanup command). B+the natural recompute already stop *new* premature placements; D
cleans the existing prod mess at deploy.

## 9. Decisions captured (from brainstorming)

- **Scope:** plan all four parts (A–D).
- **Rule C scope:** per-slot, uniform — predict a knockout match once both teams are placed
  (not best-third-only, not all-knockout-blanket).
- **Cleanup mechanism:** one idempotent `xtask` command that **forces** the re-resolve and
  then unlocks; structural unlock criterion.
- **Runtime safety net:** dropped — rely on B+C.
- **Provisional 3rd display:** always show the current-standings 3rd (all 12 rows).
- **Provisional highlight:** show provisional rank + top-8 `qualifies`; gate only the
  Annexe C pairing (`faces_*`) on all-12-final.
