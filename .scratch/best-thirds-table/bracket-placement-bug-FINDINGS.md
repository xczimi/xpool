# Best-thirds knockout placement bug — findings & fix direction (handoff)

**Date:** 2026-06-27
**Status:** Investigation complete; NOT yet implemented. For planning in a fresh session.
**Related docs (same folder / docs/superpowers):**
- `2026-06-27-best-third-placed-teams-table-design.md` — Phase 1 display design (already built)
- `../../docs/superpowers/plans/2026-06-27-best-third-placed-teams-table.md` — Phase 1 plan (already executed)
- `phase-2-editable-tiebreak.md` — separate deferred feature (editable last-resort tiebreak)

---

## 1. Where things stand

Phase 1 (the read-only "best third-placed teams" table) is **fully implemented on the git
worktree branch `worktree-best-thirds-table`** (worktree at
`.claude/worktrees/best-thirds-table`). It is **green and reviewed but NOT merged** — left
unmerged on purpose because the bug below changes its behaviour.

Branch commits (oldest→newest), base = `origin/master`:

| SHA | Commit |
|-----|--------|
| d234fa5 | feat(fwc26): third_place_ranking — ranked thirds + Annexe C pairing for display |
| ed68101 | feat(api): thirdPlaceRanking query (official + per-player) |
| 3bfe036 | refactor(api): clarify thirdPlaceRanking constants + cover edge cases |
| b3261a3 | feat(web): THIRD_PLACE_QUERY + ThirdPlaceRanking types |
| 1613734 | feat(web): ThirdPlaceTable presentational component + styles |
| 035a101 | feat(web): show official best-thirds ranking on the Schedule page |
| 2b27b3f | feat(web): show predicted + official best-thirds on My Tips |
| d451fbc | test(web/e2e): best-thirds round-trip on Schedule + My Tips |
| 11037b5 | fix(web): render best-thirds teams via TeamLabel for flag/display consistency |

All Rust tests (`cargo test -p fwc26 -p api`), web `build`/`lint`, and the 2 e2e specs pass.
Visual check done (empty + populated states). Final whole-branch review verdict: "ready to
merge" (only 2 cosmetic minors). **The design/plan/phase-2 docs are on `master`** (commits
08b4da3, 14cc6d5), which are local-only/unpushed and NOT in the worktree branch (it was cut
from `origin/master`).

### What Phase 1 built (relevant to the fix)
- `crates/fwc26/src/lib.rs`: pure `third_place_ranking(t, result) -> Vec<ThirdPlaceRow>` —
  ranks the determinable thirds, flags top 8, attaches Annexe C pairing. **It currently only
  emits rows for groups whose 3rd place is determinable** (every group game has a result), and
  mirrors the same premature `qualifying_set.len() == 8` gating as `resolve_bracket`.
- `crates/api/src/gql/`: `thirdPlaceRanking(player: ID)` query (`null` = official).
- `web/src/components/ThirdPlaceTable.tsx` + mounts on Schedule (`/games`, official) and My
  Tips (`/mytips`, predicted + official side by side).

---

## 2. The bug (found during review)

**Premature best-third placement.** `ResolutionContext::build` resolves the Annexe C mapping as
soon as **8** third-placed teams are known, not when **all 12** groups are final:

`crates/fwc26/src/lib.rs` (~line 318):
```rust
let annexe_c_map = if qualifying_set.len() == 8 {
    annexe_c(&qualifying_set)
} else {
    None
};
```

Because the top-8 selection genuinely depends on **all 12** thirds, resolving at 8-known can
place the **wrong** team into a knockout slot. A later group result can change which thirds
qualify and the entire Annexe C winner→third mapping.

**The resolved teams are persisted (durable), not derived-on-read.** The post-result hook writes
resolved knockout `TeamSlot.team_id`s back onto the stored tournament:

`crates/api/src/recompute.rs` (~lines 110–126): calls `fwc26::resolve_bracket`, then for each
knockout game sets `game.home.team_id` / `game.away.team_id` and `repo.put_tournament(&next)`.
So a premature/wrong placement is written to storage and shown to everyone until corrected.

**Only the best-third (`"3...."`) slots are affected.** `"1X"`/`"2X"` (group winner/runner-up)
resolve from a single group and are correct as soon as that one group is final. The all-12-final
gate must apply **only to the multi-group `"3ABCDF"`-style slots** (`lib.rs` ~365–391).

**Group-finality helper already exists:** `recompute.rs` ~41–46 `group_complete(t, group_id, now)`
(all games in the group played). "All 12 final" = every group A–L is `group_complete`.

### The 8 best-third R32 matches (from `BEST_THIRD_SLOTS`, `fwc26/src/lib.rs` ~40)
`M74` (1E vs 3ABCDF), `M77` (1I vs 3CDFGH), `M79` (1A vs 3CEFHI), `M80` (1L vs 3EHIJK),
`M81` (1D vs 3BEFIJ), `M82` (1G vs 3AEHIJ), `M85` (1B vs 3EFGIJ), `M87` (1K vs 3DEIJL).

---

## 3. Production data analysis (why this matters NOW)

Source: `snapshots/prod-snapshot.json` (pulled via `bin/pull-data`; loaded via `xtask load`).
41 players.

- **2074 of 2092** match predictions are **locked** (34/41 players actively predicting).
- **The bug has already fired in prod.** All 8 best-third R32 slots are already resolved to
  teams **even though the group stage is not complete**:
  - M74→PAR(3ABCDF), M77→SWE(3CDFGH), M79→SCO(3CEFHI), M80→SEN(3EHIJK), M81→BIH(3BEFIJ),
    M82→KOR(3AEHIJ), M85→IRN(3EFGIJ), M87→ECU(3DEIJL).
  - Only **66/72** group games have an official result. The missing 6 are the **final round of
    groups J, K, L** (M67–M72: CRO–GHA, PAN–ENG, COL–POR, COD–UZB, ALG–AUT, JOR–ARG). So J/K/L's
    third-placed teams are **genuinely undetermined**, yet the thirds were placed off the 9
    complete groups (A–I) — exactly the premature-resolution path.
- **11 locked predictions sit on best-third matches** (M74: 3, M77: 3, M79: 2, M81: 3).
  (M80/M82/M85/M87 currently have no locked predictions.) When J/K/L finish, the 12-team ranking
  and Annexe C mapping can shift, so those placed opponents are likely to change — leaving those
  11 locked predictions against the wrong team.
- Other knockout matches with locked predictions: M73 (7), M75 (3), M76 (3), M78 (3), M88 (1) —
  these are `1X`/`2X` slots, NOT best-third-dependent, so not affected by this bug.

**Conclusion:** the unlock concern is real and live, not hypothetical.

---

## 4. Chosen fix direction (Peter's explanation — to be planned next session)

Four parts. The last two together **avoid a complex runtime "unlock-on-team-change" diff
mechanism** by preventing wrong-team predictions from existing in the first place.

### (A) Display — show all 12 groups' third place
The best-thirds table should show **all 12 groups'** current third-placed team (the provisional
3rd from current standings), **not only** groups whose games are all final. `qualifies` / Annexe
C pairing should only be shown once **all 12** groups are final (until then: provisional, no
pairing). Changes `third_place_ranking` to emit a row per group using current standings even when
the group isn't complete, and decouples "show the 3rd" from "place into knockout".

### (B) Placement rule — gate best-third knockout placement on ALL groups final
`resolve_bracket` must only fill the `"3...."` best-third slots once **every** group A–L is
final. Until then those slots stay at their placeholder (`team_id = None`). `"1X"`/`"2X"` slots
keep resolving per-group as today.

### (C) Prediction gating — no knockout predictions until BOTH teams are placed
Do **not** allow a prediction to be entered/saved for a knockout-round match until **both** the
home and away `TeamSlot.team_id` are concretely placed. This prevents blind predictions against
placeholders/wrong teams, and means a placed team (placed only after all groups final, per B)
won't subsequently change — so there is nothing to "unlock later" going forward.
- Enforce server-side in the submit path (`crates/api/src/gql/mutation.rs`) AND disable the input
  in the UI for knockout matches whose slots aren't both resolved.
- **Open question:** does this apply to ALL knockout matches (incl. `1X`/`2X`-only like M73), or
  only the best-third ones? Today early bracket prediction IS allowed (M73 has 7 locked
  predictions). Peter's wording was "knockout round matches" (all). Confirm scope + UX impact
  (this removes the "predict the whole bracket early" experience).

### (D) One-time deployment/migration step — clean up the current prod mess
As a deployment step:
1. **Remove all currently-placed 3rd-place teams from knockout matches** (reset the 8 best-third
   slots' `team_id` back to `None`/placeholder).
2. **Unlock the predictions made on those matches only** (set `locked = false` on the locked
   MatchPredictions for those specific best-third games — the 11 identified above), so those
   players can re-predict once the teams are correctly placed (after all groups final, and once
   both teams are placed per C).

### Relationship to the earlier Q&A
Earlier in the session Peter answered a question about a **runtime** unlock trigger with "Only
real team swaps (A→B)". The refined direction above **supersedes** that: with (B)+(C), placed
teams don't change after the fact, so a general runtime unlock-on-change mechanism is likely
**unnecessary**. Keep (D) as the one-time cleanup. **Open question:** keep a minimal runtime
A→B-swap unlock as a safety net, or rely entirely on the gating rules?

---

## 5. Open questions to resolve when planning

1. **Scope of rule (C):** all knockout matches, or only best-third-dependent ones? Confirm the
   intended loss of "early full-bracket prediction" and whether legacy behaviour mattered.
2. **Deployment step (D) mechanism:** a one-off `xtask` subcommand (operating on the table /
   snapshot) vs. folding the cleanup into the fixed `recompute` (once B ships, recompute would
   naturally re-null the premature placements since not-all-final). If recompute auto-nulls them,
   the only bespoke part is the one-time **unlock** of the 11 predictions. Decide: separate
   migration command vs. guarded recompute behaviour. Ensure idempotency.
3. **Runtime unlock safety net:** keep an A→B-swap unlock in recompute, or drop it (relying on B+C)?
4. **Display semantics (A):** how to compute a provisional 3rd for a group that hasn't finished
   (current standings 3rd) — and what to show for a group with <3 teams decided / not started
   (row with no team? omit? placeholder?).
5. **Interaction with `complete` flag** in `thirdPlaceRanking`: with (A) the table always shows
   12 rows; `complete`/`qualifies`/pairing only meaningful once all 12 final. Re-confirm the GraphQL
   contract (`complete = all 12 groups final`, not `entries.len() == 12`).
6. **Branch strategy:** Phase 1 display branch is unmerged and (A) modifies it. Decide: extend
   `worktree-best-thirds-table` with the corrected behaviour, or merge Phase 1 first then do a
   follow-up branch. Recommendation: fold (A) into the same branch so the display ships correct.
7. **Deadlines:** knockout deadline = match kickoff (`model.rs` ~262, `min(kickoff)`), fixed and
   independent of team resolution. Unlocking only helps before that kickoff — confirm the 11
   affected matches' kickoffs are still in the future at deploy time (they are R32, after group
   stage — should be fine, but verify against the live clock at deploy).

---

## 6. Key code locations (for the planning session)

| Concern | Location |
|---|---|
| Premature Annexe C gate (the bug) | `crates/fwc26/src/lib.rs` ~318 (`qualifying_set.len() == 8`) |
| Best-third slot resolution | `crates/fwc26/src/lib.rs` ~381–391 (`"3...."` via `annexe_c_map`) |
| `1X`/`2X` resolution (unaffected) | `crates/fwc26/src/lib.rs` ~365–379 |
| Display ranking (needs "all 12") | `crates/fwc26/src/lib.rs` `third_place_ranking` ~120–232 |
| `BEST_THIRD_SLOTS` (the 8 matches) | `crates/fwc26/src/lib.rs` ~40 |
| resolve_bracket + persist slots | `crates/api/src/recompute.rs` ~110–126 |
| group_complete helper | `crates/api/src/recompute.rs` ~41–46 |
| Submit/lock path + deadline gate | `crates/api/src/gql/mutation.rs` ~244–272, ~376–387 |
| `MatchPrediction.locked`, `TeamSlot`, `deadline()` | `crates/domain/src/model.rs` ~99–106, ~25–32, ~262 |
| put_player / put_tournament (optimistic version) | `crates/storage/src/lib.rs` |
| snapshot export/load | `crates/xtask/src/export.rs`; `bin/pull-data`, `bin/deploy-data` |
| GraphQL `thirdPlaceRanking` resolver | `crates/api/src/gql/query.rs` (after `match_detail`) |
| Web table component | `web/src/components/ThirdPlaceTable.tsx` |

---

## 7. Suggested next-session entry point

Start by re-confirming the open questions in §5 (esp. #1 scope and #2 deployment mechanism), then
brainstorm → spec → plan the four-part fix (A display-all-12, B placement gate, C prediction gate,
D one-time cleanup). The prod snapshot at `snapshots/prod-snapshot.json` is the fixture of record
for verifying the cleanup (11 locked predictions on M74/M77/M79/M81; 8 slots to re-null).
