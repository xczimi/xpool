# Best-thirds knockout placement fix (parts A–D) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the best-third R32 slots resolving (and persisting) before all 12 groups are final, show all 12 provisional thirds, block knockout predictions until both teams are placed, and clean up the live prod mess.

**Architecture:** Two distinct notions of group standings. **Final** standings (`compute_group_standings`, exists) already contains *only complete groups*, so `len() == 12` ⇔ "all 12 groups final" — this gates placement (B) and the Annexe C pairing. **Provisional** standings (new helper) rank every group A–L from whatever results exist so far, driving the display table (A). A uniform per-slot rule blocks knockout predictions until both `TeamSlot.team_id`s are placed (C). A one-off idempotent `xtask` command force-re-resolves the bracket and unlocks affected predictions (D).

**Tech Stack:** Rust workspace (`fwc26`, `api`/async-graphql, `xtask`/clap, `storage`), React + TS + Vite (`web`).

**Spec:** `docs/superpowers/specs/2026-06-27-best-thirds-placement-fix-design.md`

**Branch:** Code changes touch `crates/*` and `web/`, so per the working agreement they go on a branch/worktree (e.g. `best-thirds-placement-fix`) cut from `master`, merged locally when green. This plan doc may sit on `master`.

---

## Phase B — Placement gate fix (`crates/fwc26`)

The smallest change; it fixes the live placement bug. After it ships, the next recompute (triggered when a J/K/L result lands) re-nulls the premature slots with no extra code.

### Task B1: Failing test — best-third slot stays `None` until all 12 groups final

**Files:**
- Test: `crates/fwc26/tests/resolve_bracket_tests.rs` (add a test; reuse the file's existing `build_test_tournament`, `group_predictions`, `result_player`, `pred`, `standings_pred` helpers)

- [ ] **Step 1: Write the failing test**

Add at the end of `crates/fwc26/tests/resolve_bracket_tests.rs` (before the final `}` if the tests are in a module; this file's tests are top-level `#[test]` fns, so just append):

```rust
/// Regression (best-thirds placement bug): with only 9 groups complete (A–I),
/// the "3ABCDF" best-third slot must NOT resolve — the top-8 selection depends
/// on all 12 thirds. Mirrors the prod state (J/K/L group games unplayed).
#[test]
fn test_best_third_unresolved_until_all_groups_final() {
    let t = build_test_tournament();

    // Predictions for groups A–I only (9 complete groups); J/K/L have none.
    let mut match_preds = Vec::new();
    let mut standings_preds = Vec::new();
    let mut m = 1u32;
    for letter in 'A'..='I' {
        let ids: Vec<String> = (m..m + 3).map(|n| format!("M{}", n)).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        let (mp, sp) = group_predictions(letter, &id_refs);
        match_preds.extend(mp);
        standings_preds.extend(sp);
        m += 3;
    }

    let result = result_player(match_preds, standings_preds);
    let resolved = resolve_bracket(&t, &result);

    // M74 = 1E vs 3ABCDF. Group E IS complete, so home (1E) resolves, but the
    // best-third away slot must stay None until ALL 12 groups are final.
    let (home, away) = resolved.get("M74").expect("M74 present");
    assert_eq!(home.as_deref(), Some("E1"), "1E resolves (group E complete)");
    assert!(
        away.is_none(),
        "3ABCDF must NOT resolve with only 9 groups final (got {away:?})"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p fwc26 --test resolve_bracket_tests test_best_third_unresolved_until_all_groups_final`
Expected: FAIL — `away` is currently `Some(...)` because the gate resolves at 8 complete groups.

- [ ] **Step 3: (no implementation in this task — fix is B2)**

Skip. This task only locks in the failing test.

- [ ] **Step 4: Commit the failing test**

```bash
git add crates/fwc26/tests/resolve_bracket_tests.rs
git commit -m "test(fwc26): best-third slot must stay None until all 12 groups final"
```

### Task B2: Gate placement on all 12 groups final

**Files:**
- Modify: `crates/fwc26/src/lib.rs` (the gate inside `ResolutionContext::build`, ~line 318)

- [ ] **Step 1: Change the gate**

In `crates/fwc26/src/lib.rs`, inside `ResolutionContext::build`, replace:

```rust
        let annexe_c_map = if qualifying_set.len() == 8 {
            annexe_c(&qualifying_set)
        } else {
            None
        };
```

with:

```rust
        // Best-third placement depends on ALL 12 thirds: the top-8 set can change
        // when a late group finishes. `group_standings` holds only *complete*
        // groups (see `compute_standings_for_group`), so `len() == 12` means every
        // group A–L is final. Resolving earlier persists wrong teams into R32 slots.
        let all_groups_final = group_standings.len() == 12;
        let annexe_c_map = if all_groups_final {
            annexe_c(&qualifying_set)
        } else {
            None
        };
```

- [ ] **Step 2: Run the regression test — verify it passes**

Run: `cargo test -p fwc26 --test resolve_bracket_tests test_best_third_unresolved_until_all_groups_final`
Expected: PASS.

- [ ] **Step 3: Run the whole fwc26 suite — verify nothing broke**

Run: `cargo test -p fwc26`
Expected: PASS. The existing `test_resolve_third_via_annexe_c` provides all 12 groups, so it still resolves; `test_resolve_partial_group_results` already expects `None` for a missing group.

- [ ] **Step 4: Commit**

```bash
git add crates/fwc26/src/lib.rs
git commit -m "fix(fwc26): gate best-third R32 placement on all 12 groups final"
```

---

## Phase A — Display all 12 provisionally (`crates/fwc26` + `crates/api`)

`third_place_ranking` switches to **provisional** standings (all 12 groups, partial-tolerant) and returns an `all_groups_final` flag so the GraphQL `complete` field is meaningful. Annexe C pairing (`faces_*`) is gated on all-12-final.

### Task A1: Failing tests for provisional display

**Files:**
- Test: `crates/fwc26/tests/third_place_ranking_tests.rs` (update the 3 existing tests to the new return type and add 2 new tests; reuse this file's `build_test_tournament`, `all_predictions`, `result_player`)

- [ ] **Step 1: Update the existing three tests to the new return shape**

`third_place_ranking` will return `RankedThirds { rows, all_groups_final }` (defined in A2) instead of `Vec<ThirdPlaceRow>`. Update the three existing tests so each binds `.rows`:

In `ranks_all_twelve_thirds_and_flags_top_eight`, replace `let rows = third_place_ranking(&t, &result);` with:

```rust
    let ranking = third_place_ranking(&t, &result);
    let rows = &ranking.rows;
    assert!(ranking.all_groups_final, "all 12 groups complete");
```

In `attaches_annexe_c_pairing_for_qualifiers`, replace `let rows = third_place_ranking(&t, &result);` with:

```rust
    let ranking = third_place_ranking(&t, &result);
    let rows = &ranking.rows;
```

(Then the existing `rows.iter()...` assertions work unchanged since `rows` is a `&Vec`.)

- [ ] **Step 2: Replace `provisional_when_a_group_is_undecided` with the new semantics**

The old test dropped group L from the tournament. New semantics: the group *exists* but is incomplete, and still gets a provisional row. Replace the whole `provisional_when_a_group_is_undecided` test with:

```rust
#[test]
fn provisional_shows_all_twelve_even_when_a_group_is_incomplete() {
    // All 12 groups exist; only A–K have results. Group L is incomplete.
    let t = build_test_tournament(true);
    let (mp, sp) = all_predictions('K');
    let result = result_player(mp, sp);

    let ranking = third_place_ranking(&t, &result);

    // Provisional table always shows all 12 groups' third.
    assert_eq!(ranking.rows.len(), 12, "all 12 groups get a provisional row");
    assert!(!ranking.all_groups_final, "group L incomplete → not final");
    // Provisional top-8 is still shown (the table's whole point mid-tournament).
    assert_eq!(ranking.rows.iter().filter(|r| r.qualifies).count(), 8);
    assert_eq!(ranking.rows[0].rank, 1);
    assert_eq!(ranking.rows[11].rank, 12);
    // Annexe C pairing is gated until all 12 are final.
    assert!(
        ranking.rows.iter().all(|r| r.faces_game.is_none()),
        "no pairing until all 12 groups final"
    );
}
```

- [ ] **Step 3: Add a test that an unplayed group still yields a positional third**

Append:

```rust
#[test]
fn provisional_third_for_a_group_with_no_results() {
    // Group L exists but has zero predictions → rank_group ranks its teams by
    // the stable fallback, so a positional 3rd is still emitted.
    let t = build_test_tournament(true);
    let (mp, sp) = all_predictions('K'); // L gets nothing
    let result = result_player(mp, sp);

    let ranking = third_place_ranking(&t, &result);

    let l_row = ranking.rows.iter().find(|r| r.group == 'L');
    assert!(l_row.is_some(), "group L still has a provisional row");
    // Its 3rd is one of L's three teams (positional, all tied at 0 points).
    assert!(l_row.unwrap().team_id.starts_with('L'));
    assert_eq!(l_row.unwrap().points, 0, "no results → 0 points");
}
```

- [ ] **Step 4: Run the tests — verify they fail to compile / fail**

Run: `cargo test -p fwc26 --test third_place_ranking_tests`
Expected: FAIL to compile (`third_place_ranking` still returns `Vec`, `RankedThirds` undefined). That is the expected RED.

- [ ] **Step 5: Commit the tests**

```bash
git add crates/fwc26/tests/third_place_ranking_tests.rs
git commit -m "test(fwc26): provisional all-12 third-place ranking + all_groups_final flag"
```

### Task A2: Implement provisional ranking + `RankedThirds`

**Files:**
- Modify: `crates/fwc26/src/lib.rs` — add `RankedThirds`, add `provisional_group_order`, rewrite `third_place_ranking` (~lines 120–215)

- [ ] **Step 1: Add the return struct**

Immediately above the `ThirdPlaceRow` struct (~line 124) add:

```rust
/// The provisional best-third table plus whether the group stage is fully
/// resolved. `all_groups_final` is true only once *every* group A–L is complete;
/// the GraphQL `complete` flag and the Annexe C pairing both gate on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedThirds {
    pub rows: Vec<ThirdPlaceRow>,
    pub all_groups_final: bool,
}
```

- [ ] **Step 2: Add the provisional standings helper**

Add this private fn directly below `third_place_ranking` (after its closing `}`, before `game_for_third_slot`):

```rust
/// Provisional standings order for one group, ranked from whatever results exist
/// so far. Unlike `compute_standings_for_group`, this does NOT require every game
/// to have a result: `rank_group` looks each game's prediction up by id and
/// ignores the ones not yet entered, so a partially-played (or unplayed) group
/// still yields a full positional order. Empty only when the group has no games.
fn provisional_group_order(t: &Tournament, result: &Player, letter: char) -> Vec<TeamId> {
    let Some(group_id) = find_group_id(t, letter) else {
        return Vec::new();
    };
    let Some(group) = t.groups.get(&group_id) else {
        return Vec::new();
    };
    let games = t.games_in(&group_id);
    if games.is_empty() {
        return Vec::new();
    }
    let predictions: Vec<&MatchPrediction> = games
        .iter()
        .filter_map(|g| result.match_prediction(&g.id))
        .collect();
    let draw_order: Vec<TeamId> = result
        .standings_prediction(&group_id)
        .map(|sp| sp.draw_order.clone())
        .unwrap_or_default();
    rank_group(group, &games, &predictions, &draw_order)
}
```

- [ ] **Step 3: Rewrite `third_place_ranking`**

Replace the entire body of `third_place_ranking` (from `pub fn third_place_ranking(t: &Tournament, result: &Player) -> Vec<ThirdPlaceRow> {` through its closing `}`, ~lines 150–215) with:

```rust
pub fn third_place_ranking(t: &Tournament, result: &Player) -> RankedThirds {
    // "All 12 groups final" ⇔ every group is complete. `compute_group_standings`
    // holds only groups whose every game has a result, so its size is the count
    // of final groups.
    let all_groups_final = compute_group_standings(t, result).len() == 12;

    // Provisional third of every group A–L, from current (possibly partial)
    // standings. Always emit the positional 3rd, even before a group is decided.
    let mut thirds: Vec<(usize, char, TeamId, TeamStats)> = Vec::new();
    for (idx, letter) in ('A'..='L').enumerate() {
        let order = provisional_group_order(t, result, letter);
        if order.len() >= 3 {
            let third_id = order[2].clone();
            let stats = compute_team_stats_in_group(t, result, letter, &third_id);
            thirds.push((idx, letter, third_id, stats));
        }
    }

    thirds.sort_by(|a, b| {
        b.3.points
            .cmp(&a.3.points)
            .then_with(|| b.3.goal_diff.cmp(&a.3.goal_diff))
            .then_with(|| b.3.goals_for.cmp(&a.3.goals_for))
            .then_with(|| a.0.cmp(&b.0)) // stable: preserve A–L input order
    });

    // Annexe C pairing is meaningful only once all 12 groups are final (the top-8
    // set can still shift). Until then: provisional rank + qualifies, no pairing.
    let qualifying_set: BTreeSet<char> = thirds.iter().take(8).map(|(_, g, _, _)| *g).collect();
    let annexe_c_map = if all_groups_final {
        annexe_c(&qualifying_set)
    } else {
        None
    };

    let rows = thirds
        .iter()
        .enumerate()
        .map(|(rank0, (_, g, id, s))| {
            let qualifies = rank0 < 8;
            let (faces_winner_group, faces_game) = match (qualifies, &annexe_c_map) {
                (true, Some(annex)) => {
                    let w = annex
                        .iter()
                        .find(|(_, third)| **third == *g)
                        .map(|(w, _)| *w);
                    let game = w.and_then(|w| game_for_third_slot(t, w));
                    (w, game)
                }
                _ => (None, None),
            };
            ThirdPlaceRow {
                group: *g,
                team_id: id.clone(),
                points: s.points,
                goal_diff: s.goal_diff,
                goals_for: s.goals_for,
                rank: rank0 as u32 + 1,
                qualifies,
                faces_winner_group,
                faces_game,
            }
        })
        .collect();

    RankedThirds {
        rows,
        all_groups_final,
    }
}
```

- [ ] **Step 4: Run the fwc26 tests — verify they pass**

Run: `cargo test -p fwc26 --test third_place_ranking_tests`
Expected: PASS (all 5 tests).

- [ ] **Step 5: Run clippy and the full crate**

Run: `cargo test -p fwc26 && cargo clippy -p fwc26 -- -D warnings`
Expected: PASS, no warnings. (`rank_group` is already imported — it is used by `compute_standings_for_group`.)

- [ ] **Step 6: Commit**

```bash
git add crates/fwc26/src/lib.rs
git commit -m "feat(fwc26): provisional all-12 third-place ranking, pairing gated on all-final"
```

### Task A3: GraphQL `complete` reflects all-12-final

**Files:**
- Modify: `crates/api/src/gql/query.rs` — the `third_place_ranking` resolver (~lines 792–812)

- [ ] **Step 1: Use the struct's flag**

In `crates/api/src/gql/query.rs`, replace:

```rust
        let rows = fwc26::third_place_ranking(&t, subject);
```

with:

```rust
        let ranking = fwc26::third_place_ranking(&t, subject);
```

Then change the `rows.iter()` line to `ranking.rows.iter()`, and replace:

```rust
        let complete = entries.len() == 12; // FWC26 has 12 groups (A–L)
        Ok(ThirdPlaceRanking { entries, complete })
```

with:

```rust
        // `complete` ⇔ all 12 groups final (now always 12 entries, so the old
        // `entries.len() == 12` is meaningless). Sourced from fwc26.
        Ok(ThirdPlaceRanking {
            entries,
            complete: ranking.all_groups_final,
        })
```

- [ ] **Step 2: Run the resolver tests — verify they pass**

Run: `cargo test -p api gql::query`
Expected: PASS. The existing tests use a single-group fixture, so `all_groups_final` is `false` (1 ≠ 12) and `entries.len()` stays 1 — assertions hold unchanged.

- [ ] **Step 3: Build the api crate**

Run: `cargo test -p api`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/gql/query.rs
git commit -m "feat(api): thirdPlaceRanking.complete = all 12 groups final"
```

---

## Phase C — Prediction gating (`crates/api` + `web/`)

A knockout-round match accepts a prediction only once **both** its team slots are concretely placed. Composes with B: best-third matches stay blocked until all 12 final; 1X/2X matches open as soon as their groups complete.

### Task C1: Failing api test — reject knockout submit with an unresolved slot

**Files:**
- Test: `crates/api/src/gql/mutation.rs` (its `#[cfg(test)] mod tests`, if present) — OR add a focused test alongside the existing mutation tests. If `mutation.rs` has no test module, add the test to `crates/api/src/gql/query.rs`'s pattern is not appropriate; instead add a `#[cfg(test)] mod gating_tests` at the bottom of `mutation.rs`.

- [ ] **Step 1: Locate the existing mutation test harness**

Run: `grep -n "mod tests\|build_schema\|InMemoryRepository\|async fn exec\|submitGroup" crates/api/src/gql/mutation.rs`
Expected: find whether `mutation.rs` already has a test module + schema-exec helper. If a sibling test harness exists (e.g. in `query.rs` or a `tests` module) reuse its `build_schema` + `exec` pattern shown in `query.rs:948`.

- [ ] **Step 2: Write the failing test**

Add a `#[cfg(test)]` test that: builds a tournament with one R32 one-match group whose game has `home.team_id = None` (placeholder slot, e.g. `slot_placeholder("3ABCDF")`) and `away.team_id = Some(...)`; authenticates as a normal player; calls the `submitGroup` mutation for that knockout group; asserts the response carries an error mentioning teams not determined. Model the schema/exec on `query.rs:948–959` but pass `CurrentPlayer` for a real player (not `Visitor`) and a future-but-pre-kickoff clock. Concretely:

```rust
#[tokio::test]
async fn knockout_submit_blocked_until_both_teams_placed() {
    // tournament: one R32 group "r32-m74" with game M74 = (1E placeholder) vs B1.
    // Use the same Team/SingleGame/GroupGame/Tournament builders as the other
    // mutation tests in this crate (slot with team_id = None for the unplaced side).
    let repo = /* InMemoryRepository with the tournament + a normal player */;
    let data = exec_as_player(
        repo,
        "demo-ada",
        r#"mutation { submitGroup(groupId: "r32-m74", lock: false,
            predictions: [{ gameId: "M74", homeScore: 1, awayScore: 0 }]) { id } }"#,
    )
    .await;
    // submitGroup should error — assert via the response errors, not data.
    assert!(
        data.errors.iter().any(|e| e.message.contains("not yet determined")),
        "expected a 'teams not yet determined' error, got {:?}",
        data.errors
    );
}
```

> Note: the exact builders (`team`, `game`, `slot_placeholder`, `result_player`, `exec`) must match what this crate already uses in its mutation/query tests — reuse them rather than introducing new ones. If the crate's exec helper asserts `errors.is_empty()`, write a variant that returns the raw `Response` so errors are observable.

- [ ] **Step 3: Run — verify it fails**

Run: `cargo test -p api knockout_submit_blocked_until_both_teams_placed`
Expected: FAIL — today `submit_group` has no slot-placement check, so the mutation succeeds (no error).

- [ ] **Step 4: Commit the failing test**

```bash
git add crates/api/src/gql/mutation.rs
git commit -m "test(api): knockout submit must be blocked until both teams placed"
```

### Task C2: Enforce the placement gate in `submit_group`

**Files:**
- Modify: `crates/api/src/gql/mutation.rs` — inside `submit_group`, after the deadline check (~line 387), before the lock-coverage check (~line 389)

- [ ] **Step 1: Add the gate**

In `submit_group`, insert after the closing `}` of the deadline `if !viewer.is_result_user { ... }` block (~line 387):

```rust
        // Best-thirds fix (Part C) — a knockout-round match accepts a prediction
        // only once BOTH its team slots are concretely placed. Best-third slots
        // stay `None` until all 12 groups are final (see `resolve_bracket`), so
        // this blocks blind predictions against not-yet-known opponents. Group
        // stage games always carry concrete team ids, so they are unaffected.
        let is_knockout = tournament
            .groups
            .get(&group_id)
            .is_some_and(|g| g.round != domain::Round::GroupStage);
        if is_knockout {
            if let Some(unresolved) = tournament
                .games_in(&group_id)
                .iter()
                .find(|g| g.home.team_id.is_none() || g.away.team_id.is_none())
            {
                return Err(async_graphql::Error::new(format!(
                    "match `{}` teams are not yet determined; predictions open once both teams are placed",
                    unresolved.id
                )));
            }
        }
```

> If `domain::Round` is not already a valid path in this file, add `use domain::Round;` to the imports and use `g.round != Round::GroupStage`. Verify with: `grep -n "use domain" crates/api/src/gql/mutation.rs`.

- [ ] **Step 2: Run the gating test — verify it passes**

Run: `cargo test -p api knockout_submit_blocked_until_both_teams_placed`
Expected: PASS.

- [ ] **Step 3: Run the whole api suite**

Run: `cargo test -p api`
Expected: PASS. (Existing knockout-prediction tests, if any, use games with placed teams, so they are unaffected. If a pre-existing test submits a knockout group with unplaced slots and expected success, that test encoded the bug — update it to place both teams first.)

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/gql/mutation.rs
git commit -m "feat(api): block knockout predictions until both team slots are placed"
```

### Task C3: Disable the input in the web form for unplaced knockout matches

**Files:**
- Modify: `web/src/pages/mytips/GroupTipForm.tsx` — the `games.map` row render (~lines 278–331)
- Modify (i18n): `web/src/i18n/strings.ts` — add a `teamsNotDetermined` key (en + hu)

- [ ] **Step 1: Add the i18n string**

In `web/src/i18n/strings.ts`, add to both the English and Hungarian maps (match the file's existing shape):

```ts
  teamsNotDetermined: 'Teams not yet determined',
```

and the Hungarian:

```ts
  teamsNotDetermined: 'A csapatok még nem dőltek el',
```

- [ ] **Step 2: Gate the score input on both slots being placed**

In `web/src/pages/mytips/GroupTipForm.tsx`, inside `games.map((game) => { ... })`, after `const matchLocked = ...` (~line 280) add:

```tsx
            const teamsPlaced = !!game.home.teamId && !!game.away.teamId
```

Then change the score cell so an unplaced knockout match shows the notice instead of inputs. Replace the `<td className="score-cell">{matchLocked ? (...) : (...)}</td>` block with:

```tsx
                <td className="score-cell">
                  {!teamsPlaced ? (
                    <span className="hint">{t('teamsNotDetermined')}</span>
                  ) : matchLocked ? (
                    <span className="score-locked">
                      {m.homeScore === '' ? '–' : m.homeScore} :{' '}
                      {m.awayScore === '' ? '–' : m.awayScore}
                    </span>
                  ) : (
                    <>
                      <ScoreInput
                        value={m.homeScore}
                        onChange={(v) => setScore(game.id, 'homeScore', v)}
                      />
                      <span>:</span>
                      <ScoreInput
                        value={m.awayScore}
                        onChange={(v) => setScore(game.id, 'awayScore', v)}
                      />
                    </>
                  )}
                </td>
```

> Verify the web `TeamSlot` field name is `teamId` (camelCase from GraphQL): `grep -n "teamId" web/src/graphql/types.ts`. The `Matchup`/`TeamLabel` components already consume `game.home`/`game.away` with a `teamId` field.

- [ ] **Step 3: Build + lint the web app**

Run: `cd web && npm run build && npm run lint`
Expected: PASS (tsc clean, eslint clean).

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/mytips/GroupTipForm.tsx web/src/i18n/strings.ts
git commit -m "feat(web): disable knockout prediction input until both teams placed"
```

### Task C4: E2E — knockout input disabled until teams placed

**Files:**
- Test: `web/e2e/` (add a spec; model on the existing best-thirds e2e spec added in Phase 1)

- [ ] **Step 1: Find the existing best-thirds e2e spec to model on**

Run: `ls web/e2e && grep -rln "thirdPlace\|third-place\|best.third\|My Tips\|mytips" web/e2e`
Expected: locate the Phase 1 best-thirds spec and the seed/scenario it uses.

- [ ] **Step 2: Write the spec**

Add a spec that seeds a tournament state where a knockout (R32) best-third match has unplaced teams (the default fresh seed, before all groups final), navigates a logged-in player to `/mytips` on the R32 round, and asserts the match row shows the "Teams not yet determined" notice and renders no score `<input>` for that match. Use the e2e suite's dev-login + scenario seeding helpers (the e2e stack boots itself; see `.specs/TESTING.md` §2). Keep it to one focused assertion path.

- [ ] **Step 3: Run the e2e suite**

Run: `cd web && npm run e2e`
Expected: PASS (the new spec + the existing best-thirds specs).

- [ ] **Step 4: Commit**

```bash
git add web/e2e
git commit -m "test(web/e2e): knockout prediction input disabled until teams placed"
```

---

## Phase D — One-time prod cleanup (`crates/xtask`)

One idempotent command, modeled on `crates/xtask/src/migrate_gh.rs`: force-re-resolve the bracket (re-nulls premature best-third slots with the fixed code) and unlock any locked prediction on a knockout match whose slot is now unresolved.

### Task D1: Cleanup module with structural logic + tests

**Files:**
- Create: `crates/xtask/src/cleanup_thirds.rs`
- Modify: `crates/xtask/src/lib.rs` (or `main.rs` module decl) — add `pub mod cleanup_thirds;`

- [ ] **Step 1: Write the module with a generic, testable `run`**

Create `crates/xtask/src/cleanup_thirds.rs`:

```rust
//! One-off cleanup for the best-thirds placement bug.
//!
//! Two logically-separate concerns in one idempotent pass:
//! 1. Force-re-resolve the bracket with the fixed `fwc26::resolve_bracket` and
//!    persist it — premature best-third R32 slots revert to `None` (the gate now
//!    requires all 12 groups final).
//! 2. Unlock any *locked* prediction on a knockout match whose slot is now
//!    unresolved, so those players can re-predict once teams are correctly placed.
//!
//! The unlock criterion is **structural** ("locked prediction on a knockout match
//! with an unresolved team slot"), not a hardcoded match list — so re-running is a
//! no-op once the slots are corrected.
//!
//! Re-resolution uses the result-user's raw predictions. In production the
//! result-user only carries results for games actually played, so this matches the
//! API's as-of-`now` recompute (whose slice is a no-op for real entries).

use anyhow::Context;
use domain::{Player, Round, Tournament};
use std::collections::BTreeSet;
use storage::Repository;

/// Knockout game ids in `t` whose home or away slot is unplaced.
fn unresolved_knockout_games(t: &Tournament) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (id, game) in &t.games {
        let is_knockout = t
            .groups
            .get(&game.group_id)
            .is_some_and(|g| g.round != Round::GroupStage);
        if is_knockout && (game.home.team_id.is_none() || game.away.team_id.is_none()) {
            out.insert(id.clone());
        }
    }
    out
}

/// One prediction the cleanup unlocked (or would unlock in a dry run).
#[derive(Debug, Clone)]
pub struct UnlockRecord {
    pub player_id: String,
    pub nick: String,
    pub game_id: String,
}

/// Outcome of a cleanup run.
#[derive(Debug, Default)]
pub struct CleanupReport {
    pub slots_renulled: usize,
    pub unlocks: Vec<UnlockRecord>,
    pub players_written: usize,
    pub tournament_written: bool,
}

impl CleanupReport {
    pub fn print(&self, applied: bool) {
        let mode = if applied { "APPLIED" } else { "DRY RUN (read-only)" };
        println!("== cleanup-best-thirds — {mode} ==");
        println!("knockout slots re-nulled by re-resolve: {}", self.slots_renulled);
        if self.unlocks.is_empty() {
            println!("locked predictions to unlock: none");
        } else {
            let verb = if applied { "unlocked" } else { "would unlock" };
            println!("locked predictions {verb}: {}", self.unlocks.len());
            for u in &self.unlocks {
                println!("  {} ({}): game {}", u.nick, u.player_id, u.game_id);
            }
        }
        if applied {
            println!(
                "tournament written: {}, players written: {}",
                self.tournament_written, self.players_written
            );
        } else if !self.unlocks.is_empty() || self.slots_renulled > 0 {
            println!("re-run with --apply to write these changes");
        }
    }
}

/// Re-resolve the bracket and write the corrected team slots onto knockout games.
/// Returns the corrected tournament and how many previously-placed knockout slots
/// became `None`.
fn reresolved_tournament(t: &Tournament, result_user: &Player) -> (Tournament, usize) {
    let resolved = fwc26::resolve_bracket(t, result_user);
    let mut next = t.clone();
    let mut renulled = 0usize;
    for (game_id, (home_team, away_team)) in resolved {
        let is_knockout = next
            .games
            .get(&game_id)
            .and_then(|g| next.groups.get(&g.group_id))
            .is_some_and(|grp| grp.round != Round::GroupStage);
        if !is_knockout {
            continue;
        }
        if let Some(game) = next.games.get_mut(&game_id) {
            if game.home.team_id.is_some() && home_team.is_none() {
                renulled += 1;
            }
            if game.away.team_id.is_some() && away_team.is_none() {
                renulled += 1;
            }
            game.home.team_id = home_team;
            game.away.team_id = away_team;
        }
    }
    (next, renulled)
}

/// Run the cleanup. With `apply == false` nothing is written.
pub async fn run<R: Repository>(repo: &R, apply: bool) -> anyhow::Result<CleanupReport> {
    let tournament = repo
        .get_tournament()
        .await?
        .context("no tournament in table — run `xtask import` first")?;
    let players = repo.list_players().await?;
    let result_user = players
        .iter()
        .find(|p| p.is_result_user)
        .context("no result user found — cannot re-resolve")?;

    let (next, renulled) = reresolved_tournament(&tournament, result_user);
    let unresolved = unresolved_knockout_games(&next);

    let mut report = CleanupReport {
        slots_renulled: renulled,
        ..Default::default()
    };

    // Unlock locked predictions on knockout matches now lacking a placed team.
    for p in &players {
        let mut changed = false;
        let new_preds: Vec<_> = p
            .match_predictions
            .iter()
            .map(|mp| {
                if mp.locked && unresolved.contains(&mp.game_id) {
                    report.unlocks.push(UnlockRecord {
                        player_id: p.id.clone(),
                        nick: p.nick.clone(),
                        game_id: mp.game_id.clone(),
                    });
                    changed = true;
                    domain::MatchPrediction {
                        locked: false,
                        ..mp.clone()
                    }
                } else {
                    mp.clone()
                }
            })
            .collect();

        if changed && apply {
            let updated = Player {
                match_predictions: new_preds,
                ..p.clone()
            };
            repo.put_player(&updated).await?;
            report.players_written += 1;
        }
    }

    if apply {
        repo.put_tournament(&next).await?;
        report.tournament_written = true;
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::InMemoryRepository;

    // Build a tournament + result-user fixture where best-third slots are
    // PREMATURELY placed (simulating the pre-fix persisted state) but < 12 groups
    // are complete, plus a player with a locked prediction on that knockout match.
    // Reuse fwc26 test fixture shapes; keep it minimal: one R32 group whose game's
    // away slot is wrongly Some(team) while only a few groups have results.
    // (Construct Tournament/Player directly as the other crates' tests do.)

    #[tokio::test]
    async fn dry_run_writes_nothing_but_reports() {
        // arrange repo with the premature-placement fixture + a locked knockout pred
        // act: run(&repo, false)
        // assert: report.unlocks is non-empty, report.slots_renulled > 0,
        //         and the stored player's prediction is STILL locked (no write).
    }

    #[tokio::test]
    async fn apply_renulls_slots_and_unlocks_then_is_idempotent() {
        // act: run(&repo, true)
        // assert: stored tournament's knockout away slot is now None;
        //         the player's prediction is now unlocked.
        // act again: run(&repo, true) → report.unlocks empty, slots_renulled 0.
    }
}
```

> The two test bodies are stubs to fill with the crate's fixture builders. The cleanup *logic* (`unresolved_knockout_games`, `reresolved_tournament`, the unlock loop) is fully written above — the tests construct a `Tournament`/`Player` and a `InMemoryRepository`, mirroring `crates/fwc26/tests/resolve_bracket_tests.rs` builders. Fill them before moving on; do not leave them empty.

- [ ] **Step 2: Register the module**

Run: `grep -n "pub mod\|mod migrate_gh" crates/xtask/src/lib.rs crates/xtask/src/main.rs`
Then add `pub mod cleanup_thirds;` wherever `migrate_gh` is declared (same file).

- [ ] **Step 3: Run — verify the tests fail until fixtures are filled, then pass**

Run: `cargo test -p xtask cleanup_thirds`
Expected: after filling the fixture bodies, PASS. Also `cargo clippy -p xtask -- -D warnings`.

- [ ] **Step 4: Commit**

```bash
git add crates/xtask/src/cleanup_thirds.rs crates/xtask/src/lib.rs
git commit -m "feat(xtask): cleanup-best-thirds re-resolve + unlock logic (idempotent)"
```

### Task D2: Wire the CLI subcommand

**Files:**
- Modify: `crates/xtask/src/main.rs` — add a `Command` variant + dispatch arm (model on `FixGroupsGh`)

- [ ] **Step 1: Add the command variant**

In the `enum Command` in `crates/xtask/src/main.rs`, add (next to `FixGroupsGh`):

```rust
    /// One-off: clean up the best-thirds placement bug. Re-resolves the bracket
    /// (re-nulls premature best-third R32 slots) and unlocks locked predictions on
    /// knockout matches whose teams are no longer placed. Idempotent; a read-only
    /// report unless `--apply` is given.
    CleanupBestThirds {
        /// Write the changes. Without this flag the command reports only.
        #[arg(long)]
        apply: bool,
    },
```

- [ ] **Step 2: Add the dispatch arm**

In the `match cli.command { ... }` block, add (next to the `FixGroupsGh` arm):

```rust
        Command::CleanupBestThirds { apply } => {
            let report = xtask::cleanup_thirds::run(&repo, apply).await?;
            report.print(apply);
        }
```

- [ ] **Step 3: Build the binary**

Run: `cargo build -p xtask && cargo run -p xtask -- cleanup-best-thirds --help`
Expected: builds; `--help` shows the new subcommand and `--apply` flag.

- [ ] **Step 4: Commit**

```bash
git add crates/xtask/src/main.rs
git commit -m "feat(xtask): cleanup-best-thirds subcommand (dry-run default, --apply)"
```

### Task D3: Dry-run against the prod snapshot (verification, no prod writes)

**Files:** none (operational verification)

- [ ] **Step 1: Load the prod snapshot into a local table**

Per `CLAUDE.md`, bring infra up and load the snapshot:

```bash
docker compose up -d
export DYNAMO_ENDPOINT=http://localhost:8000
export XPOOL_TABLE=xpool-cleanup-check
cargo run -p xtask -- load snapshots/prod-snapshot.json
```

Expected: "loaded ... into `xpool-cleanup-check`".

- [ ] **Step 2: Dry-run the cleanup**

```bash
cargo run -p xtask -- cleanup-best-thirds
```

Expected (per FINDINGS §3): reports re-nulling the 8 best-third R32 slots and unlocking the **11** locked predictions on M74/M77/M79/M81 — and NO writes (dry run).

- [ ] **Step 3: Apply locally + verify idempotency**

```bash
cargo run -p xtask -- cleanup-best-thirds --apply
cargo run -p xtask -- cleanup-best-thirds          # second dry run
```

Expected: first run unlocks 11 + re-nulls 8; the second dry run reports **none** (idempotent).

- [ ] **Step 4: Record the result**

Note the dry-run counts in the PR/commit description (no code change). Do NOT run `--apply` against prod here — production application is a deploy step, gated on Step-of-deploy verification that the affected R32 kickoffs are still in the future (spec §6).

---

## Final verification

- [ ] **Workspace green:** `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`
- [ ] **Web green:** `cd web && npm run build && npm run lint && npm run e2e`
- [ ] **Merge the branch into `master` locally** (solo workflow; PR only if you want the record). Push.
- [ ] **Deploy sequence (operational, post-merge):** deploy code (B+A+C live) → confirm affected R32 kickoffs are still future against the live clock → run `xtask cleanup-best-thirds` dry-run against prod → `--apply`.

---

## Self-review notes (coverage map)

- Spec §3 Part B → Phase B (B1 test, B2 gate).
- Spec §4 Part A → Phase A (provisional helper, `RankedThirds`, rewrite, resolver flag).
- Spec §4 GraphQL contract → Task A3.
- Spec §5 Part C → Phase C (api gate C2, web disable C3, e2e C4).
- Spec §6 Part D → Phase D (force re-resolve + structural unlock, CLI, snapshot dry-run).
- Spec §7 testing → tests in every phase + Final verification.
- Spec §6 "verify kickoffs still future" → Task D3 Step 4 + Final verification deploy sequence.
