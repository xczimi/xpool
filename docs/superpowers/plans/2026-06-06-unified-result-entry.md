# Unified Result Entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the result user enter official results through the normal My Tips
form (no separate admin screen), counting them in scoring the moment they're
entered — by making the result side of scoring use the same `effective_locked`
rule as predictions.

**Architecture:** Official results are the result user's `MatchPrediction`s.
Kickoff/deadline implicitly locks an entered result exactly as it locks a
prediction (`effective_locked = locked || (now > deadline && complete)`), so the
scoring/display rules become symmetric with **no `is_result_user` branch**. The
only real special case is on the write path: the result user is exempt from
`submitGroup`'s deadline rejection and their save triggers the wholesale
scoreboard/bracket recompute (calculate-on-write). The standalone `/admin`
Results screen and the `enterResult`/`unlockResult` mutations are removed.

**Tech Stack:** Rust (axum + async-graphql, domain crate), React + TypeScript +
Vite + urql, Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-06-06-unified-result-entry-design.md`

---

## Task 0: Baseline

**Files:** none (verification only)

- [ ] **Step 1: Build the worktree and confirm a green baseline**

Run:
```bash
cargo build --workspace
cargo test --workspace
cd web && npm install && npm run build && npm run lint && cd ..
```
Expected: builds succeed; tests pass. If anything fails before any change, STOP
and report — do not start on a red baseline.

---

## Task 1: Domain — result side uses `effective_locked` (symmetric)

**Files:**
- Modify: `crates/domain/src/scoring.rs` (per-match guard ~442-446; standings guard ~468-472)
- Test: `crates/domain/tests/scoring.rs` (replace `score_tournament_unlocked_result_scores_zero` ~768-789)
- Docs: `.specs/SCORING.md` (~26-27), `.specs/SCENARIOS.md` (SCORE-13 ~546)

- [ ] **Step 1: Replace the result-locking test with the symmetric pair**

In `crates/domain/tests/scoring.rs`, delete `score_tournament_unlocked_result_scores_zero`
(the whole `#[test]` fn at ~767-789) and add these two in its place:

```rust
/// Symmetric rule: an unlocked result scores zero *before* the deadline
/// (kickoff has not happened, so it is not yet effective-locked).
#[test]
fn score_tournament_unlocked_result_before_deadline_scores_zero() {
    let c = default_config();
    let t = make_tournament_single_group("g", Round::GroupStage, "m1", "A", "B");
    // Game kickoff (= deadline) is 2026-06-01 12:00 (from make_single_game).
    let now = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(); // before

    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );
    // result NOT locked, and now is before the deadline → not effective-locked.
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, false)],
        vec![make_sp("g", vec!["A", "B"], false)],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    assert_eq!(scores.get(&Round::GroupStage).copied().unwrap_or(0), 0);
}

/// Symmetric rule: an unlocked result *after* the deadline counts, exactly like
/// an unlocked-but-complete prediction (`score_tournament_auto_locked_after_deadline`).
#[test]
fn score_tournament_unlocked_result_after_deadline_scores() {
    let c = default_config();
    let t = make_tournament_single_group("g", Round::GroupStage, "m1", "A", "B");
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap(); // after deadline

    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );
    // result NOT explicitly locked, but now > deadline & complete → counts.
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, false)],
        vec![make_sp("g", vec!["A", "B"], false)],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    // 4 match + 1 standings = 5 (GroupStage ×1).
    assert_eq!(scores.get(&Round::GroupStage).copied().unwrap_or(0), 5);
}
```

- [ ] **Step 2: Run the new tests — the "after" case must fail**

Run: `cargo test -p domain score_tournament_unlocked_result`
Expected: `..._before_deadline_scores_zero` PASSES (old rule already returns 0),
`..._after_deadline_scores` FAILS (asserts 5, old code returns 0 because the
result is unlocked).

- [ ] **Step 3: Change the result-side guards in `scoring.rs`**

In `score_leaf_group`, replace the per-match result guard (~442-446):

```rust
        if let (Some(pred_mp), Some(result_mp)) = (pred_mp, result_mp) {
            // Result must be locked (not just effective-locked) per spec §1
            if !result_mp.locked {
                continue;
            }
```
with:
```rust
        if let (Some(pred_mp), Some(result_mp)) = (pred_mp, result_mp) {
            // Result must be effective-locked — the SAME rule as a prediction.
            // Kickoff/deadline implicitly locks an entered result (results are
            // entered post-match), so no explicit-lock requirement and no
            // result-user special case (unified result entry).
            let r_locked = effective_locked(result_mp.locked, now, deadline, true);
            if !r_locked {
                continue;
            }
```

And replace the standings result guard (~468-472):

```rust
            // Result standings must be locked
            if !result_sp.locked {
                return raw; // no bonus
            }
```
with:
```rust
            // Result standings must be effective-locked — same rule as the
            // predicted standings below.
            let r_sp_locked = effective_locked(
                result_sp.locked,
                now,
                deadline,
                !result_sp.ordering.is_empty(),
            );
            if !r_sp_locked {
                return raw; // no bonus
            }
```

- [ ] **Step 4: Run domain tests**

Run: `cargo test -p domain`
Expected: PASS, including both new tests and the existing
`score_tournament_unlocked_prediction_scores_zero` /
`score_tournament_auto_locked_after_deadline`.

- [ ] **Step 5: Update the specs**

In `.specs/SCORING.md` ~26-27, change:
> A prediction contributes only when **effective-locked** (`DATA_MODEL.md` §7);
> a result counts only when locked. Unlocked → 0.

to:
> A prediction *and* a result contribute only when **effective-locked**
> (`DATA_MODEL.md` §7) — the same rule for both: `locked || (now > deadline &&
> complete)`. Because official results are entered after the match (past the
> deadline), an entered result is effective-locked immediately; explicit
> `locked` is a player-only early-reveal flag, never a scoring gate.

In `.specs/SCENARIOS.md`, find SCORE-13 (~546) and reword the "result" half from
"a result that is not effective-locked" so it reads symmetrically: an unlocked
prediction *or* result scores zero only **before** the deadline; after the
deadline a complete entered result counts. Update the `Tests:` line to name
`score_tournament_unlocked_result_before_deadline_scores_zero` and
`score_tournament_unlocked_result_after_deadline_scores`.

- [ ] **Step 6: Commit**

```bash
git add crates/domain/src/scoring.rs crates/domain/tests/scoring.rs .specs/SCORING.md .specs/SCENARIOS.md
git commit -m "feat(domain): score results by effective-lock, symmetric with predictions"
```

---

## Task 2: API read-gates — display entered results, not only locked

**Files:**
- Modify: `crates/api/src/gql/query.rs` (`tournament` ~38-48, `perfects` ~228, `results` ~243-256)
- Test: `crates/api/tests/graphql.rs` (replace `results_returns_only_locked_result_user_predictions` ~745-771)

- [ ] **Step 1: Rewrite the results-query test to expect entered (not only locked)**

In `crates/api/tests/graphql.rs`, replace `results_returns_only_locked_result_user_predictions`
(~745-771) with:

```rust
#[tokio::test]
async fn results_returns_entered_result_user_predictions() {
    let repo = seeded_repo(Duration::hours(-2)).await;
    // Result user has one locked and one unlocked entered result.
    {
        let mut result = repo.get_player(RESULT_ID).await.unwrap().unwrap();
        result.match_predictions.push(locked_pred(GAME_1, 2, 1)); // locked
        let mut draft = locked_pred(GAME_2, 0, 0);
        draft.locked = false; // unlocked, but still an entered official result
        result.match_predictions.push(draft);
        repo.put_player(&result).await.unwrap();
    }
    let resp = run(
        &repo,
        "{ results { gameId homeScore awayScore locked } }",
        Variables::default(),
        None,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let results = d["results"].as_array().unwrap();
    assert_eq!(results.len(), 2, "both entered results are returned");
}
```

- [ ] **Step 2: Run it — expect failure**

Run: `cargo test -p api results_returns_entered_result_user_predictions`
Expected: FAIL (asserts 2, current resolver filters to locked → returns 1).

- [ ] **Step 3: Relax the three read-gates in `query.rs`**

In the `tournament` resolver (~41-48), remove the `.locked` filter so the
official-results set is every entered result:

```rust
            .map(|r| {
                r.match_predictions
                    .iter()
                    .filter(|p| p.locked)            // <-- DELETE this line
                    .map(|p| p.game_id.clone())
                    .collect()
            })
```
becomes:
```rust
            .map(|r| {
                r.match_predictions
                    .iter()
                    .map(|p| p.game_id.clone())
                    .collect()
            })
```

In the `perfects` resolver (~228), change:
```rust
                    if result.locked && is_perfect(prediction, result, &config) {
```
to:
```rust
                    if is_perfect(prediction, result, &config) {
```

In the `results` resolver (~248-254), remove the `.locked` filter and update the
doc comment (~241-242 "result user's *locked* match predictions" → "*entered*"):
```rust
            .map(|r| {
                r.match_predictions
                    .iter()
                    .filter(|p| p.locked)            // <-- DELETE this line
                    .map(MatchPrediction::from)
                    .collect()
            })
```

- [ ] **Step 4: Run the API test suite for the read paths**

Run: `cargo test -p api`
Expected: the new test passes; `results_is_empty_when_no_results_entered` still
passes (no result user predictions → empty). Some `enter_result*` tests still
exist and pass at this point (removed in Task 5).

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/gql/query.rs crates/api/tests/graphql.rs
git commit -m "feat(api): surface entered results (not only locked) in read paths"
```

---

## Task 3: Backend — `submitGroup` result-user deadline exemption + recompute on write

**Files:**
- Modify: `crates/api/src/gql/mutation.rs` (`submit_group` deadline check ~226-232; loop `Ok` arm ~278-281)
- Test: `crates/api/tests/graphql.rs` (add after the existing submitGroup tests, ~330)

- [ ] **Step 1: Add the result-user submitGroup tests**

In `crates/api/tests/graphql.rs`, add (after `submit_group_lock_succeeds_with_all_games`):

```rust
// ── result user enters results via submitGroup (unified result entry) ────────

#[tokio::test]
async fn submit_group_as_result_user_allowed_after_deadline_and_recomputes() {
    // Group A kicked off 2h ago — its deadline has passed for everyone.
    let repo = seeded_repo(Duration::hours(-2)).await;
    // Alice predicts M1 = 2-1 (locked) → an official 2-1 is a perfect (4 pts).
    {
        let mut alice = repo.get_player(ALICE).await.unwrap().unwrap();
        alice.match_predictions.push(locked_pred(GAME_1, 2, 1));
        repo.put_player(&alice).await.unwrap();
    }
    // The result user submits the official Group A results as an ordinary
    // (unlocked) draft — allowed despite the passed deadline.
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [
            { "gameId": GAME_1, "homeScore": 2, "awayScore": 1 },
            { "gameId": GAME_2, "homeScore": 0, "awayScore": 0 }
        ],
        "lock": false
    }));
    let resp = run(&repo, SUBMIT, vars, Some(RESULT_ID)).await;
    assert!(resp.errors.is_empty(), "result user may submit post-deadline: {:?}", resp.errors);

    // The save recomputed the scoreboard on write: Alice scored 4.
    let board = repo.get_scoreboard().await.unwrap().expect("scoreboard written");
    let total: i64 = board.entries.get(ALICE).expect("Alice on scoreboard").values().sum();
    assert_eq!(total, 4, "exact 2-1 official result = 4 points");
    assert!(!board.entries.contains_key(RESULT_ID), "result user not scored against itself");
}

#[tokio::test]
async fn submit_group_as_result_user_can_recorrect_an_unlocked_result() {
    let repo = seeded_repo(Duration::hours(-2)).await;
    {
        let mut alice = repo.get_player(ALICE).await.unwrap().unwrap();
        alice.match_predictions.push(locked_pred(GAME_1, 3, 0));
        repo.put_player(&alice).await.unwrap();
    }
    let entry = |h, a| {
        Variables::from_json(json!({
            "g": GROUP_A,
            "p": [{ "gameId": GAME_1, "homeScore": h, "awayScore": a }],
            "lock": false
        }))
    };
    // First (wrong) entry, then a correction — both accepted, no unlock step.
    assert!(run(&repo, SUBMIT, entry(0, 0), Some(RESULT_ID)).await.errors.is_empty());
    let resp = run(&repo, SUBMIT, entry(3, 0), Some(RESULT_ID)).await;
    assert!(resp.errors.is_empty(), "correction accepted: {:?}", resp.errors);

    // Scoreboard reflects the corrected 3-0 (Alice predicted 3-0 → perfect = 4).
    let board = repo.get_scoreboard().await.unwrap().expect("scoreboard written");
    let total: i64 = board.entries.get(ALICE).unwrap().values().sum();
    assert_eq!(total, 4);
}
```

Keep `submit_group_rejected_after_deadline` (it proves a *regular* player is
still blocked) unchanged.

- [ ] **Step 2: Run — expect failure**

Run: `cargo test -p api submit_group_as_result_user`
Expected: FAIL — both error out on the deadline check (`deadline has passed`).

- [ ] **Step 3: Exempt the result user from the deadline check**

In `submit_group`, wrap the deadline check (~226-232) in a non-result-user guard:

```rust
        // Issue 01 — the group's deadline is final: no edits once it passes.
        // Issue 27 — the boundary is strict `>`. The result user is exempt:
        // official results are entered *after* the match (unified result entry).
        if !viewer.is_result_user {
            if let Some(deadline) = tournament.deadline(&group_id) {
                if now(ctx) > deadline {
                    return Err(async_graphql::Error::new(format!(
                        "group `{group_id}` deadline has passed; predictions are final"
                    )));
                }
            }
        }
```

- [ ] **Step 4: Recompute on the result user's save**

In the retry loop, replace the `Ok` arm (~279):
```rust
                Ok(()) => return Ok(Player::from(&next)),
```
with:
```rust
                Ok(()) => {
                    // The result user's predictions ARE the official results, so
                    // a save recomputes the materialised scoreboard + bracket on
                    // write. Best-effort: a failure is logged, not fatal (the
                    // `recompute` mutation self-heals) — matching enter_result.
                    if viewer.is_result_user {
                        if let Err(e) = recompute(repo.as_ref(), now(ctx)).await {
                            tracing::error!(
                                "recompute after result-user submit_group failed: {e}"
                            );
                        }
                    }
                    return Ok(Player::from(&next));
                }
```

(`recompute` is already imported at the top of `mutation.rs`; `repo` is
`&Arc<dyn Repository>`, so `repo.as_ref()` yields the `&dyn Repository` recompute
expects — same call shape as `enter_result`.)

- [ ] **Step 5: Run — expect pass**

Run: `cargo test -p api submit_group`
Expected: PASS for the two new tests and all existing submitGroup tests
(`submit_group_rejected_after_deadline` still rejects a *regular* player).

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/gql/mutation.rs crates/api/tests/graphql.rs
git commit -m "feat(api): result user enters results via submitGroup (deadline-exempt, recompute on write)"
```

---

## Task 4: Backend — advancer + scoreboard coverage via `submitGroup`

This proves the two behaviours the soon-to-be-deleted `enterResult` tests
covered — drawn-knockout advancer resolution and a public scoreboard read — now
work through `submitGroup`.

**Files:**
- Test: `crates/api/tests/graphql.rs` (add a standings-carrying SUBMIT const + tests; rewrite `scoreboard_query_reflects_recompute`)

- [ ] **Step 1: Add a standings-carrying submit mutation + advancer test**

In `crates/api/tests/graphql.rs`, add near the top-level consts:

```rust
const SUBMIT_WITH_STANDINGS: &str = r#"
mutation($g: ID!, $p: [MatchPredictionInput!]!, $s: StandingsInput, $lock: Boolean!) {
  submitGroup(groupId: $g, predictions: $p, standings: $s, lock: $lock) {
    id
  }
}"#;
```

And add the test:

```rust
#[tokio::test]
async fn submit_group_result_user_resolves_drawn_knockout_advancer() {
    let repo = seeded_repo_with_knockout(Duration::hours(-2)).await;

    // Group A official results: MEX wins M1 3-0, KOR wins M2 1-0 → 1A=MEX, 2A=KOR.
    let ga = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [
            { "gameId": GAME_1, "homeScore": 3, "awayScore": 0 },
            { "gameId": GAME_2, "homeScore": 1, "awayScore": 0 }
        ],
        "lock": false
    }));
    assert!(run(&repo, SUBMIT, ga, Some(RESULT_ID)).await.errors.is_empty());

    // GAME_KO ends level 1-1; KOR advances — expressed as the standings ordering
    // [KOR, MEX] for the one-match knockout group (the draw-order UI's output).
    let ko = Variables::from_json(json!({
        "g": GROUP_KO,
        "p": [{ "gameId": GAME_KO, "homeScore": 1, "awayScore": 1 }],
        "s": { "ordering": ["KOR", "MEX"], "drawOrder": [] },
        "lock": false
    }));
    let resp = run(&repo, SUBMIT_WITH_STANDINGS, ko, Some(RESULT_ID)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);

    // Bracket resolved on write: the downstream R16 home slot is the advancer.
    let t = repo.get_tournament().await.unwrap().unwrap();
    let next = t.games.get(GAME_KO_NEXT).unwrap();
    assert_eq!(
        next.home.team_id.as_deref(),
        Some("KOR"),
        "the drawn knockout advanced KOR, not the home team MEX"
    );
}
```

- [ ] **Step 2: Rewrite `scoreboard_query_reflects_recompute` to use submitGroup**

Replace the body of `scoreboard_query_reflects_recompute` (~535-564) so it enters
the result through `submitGroup` instead of `ENTER_RESULT`:

```rust
#[tokio::test]
async fn scoreboard_query_reflects_recompute() {
    let repo = seeded_repo(Duration::hours(-2)).await;
    {
        let mut alice = repo.get_player(ALICE).await.unwrap().unwrap();
        alice.match_predictions.push(locked_pred(GAME_1, 1, 0));
        repo.put_player(&alice).await.unwrap();
    }
    // Result user enters M1 = 3-0 → Alice (1-0) gets outcome+away-exact = 3 pts.
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 3, "awayScore": 0 }],
        "lock": false
    }));
    run(&repo, SUBMIT, vars, Some(RESULT_ID)).await;

    let resp = run(&repo, "{ scoreboard { playerId total } }", Variables::default(), None).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let alice_row = d["scoreboard"].as_array().unwrap().iter()
        .find(|r| r["playerId"] == "alice").unwrap().clone();
    assert_eq!(alice_row["total"], 3);
}
```

- [ ] **Step 3: Run — expect pass**

Run: `cargo test -p api submit_group_result_user_resolves_drawn_knockout_advancer scoreboard_query_reflects_recompute`
Expected: PASS (submitGroup already persists `standings`; Task 3 added the
recompute that resolves the bracket).

- [ ] **Step 4: Commit**

```bash
git add crates/api/tests/graphql.rs
git commit -m "test(api): advancer + scoreboard via submitGroup result entry"
```

---

## Task 5: Backend — remove `enterResult` / `unlockResult`

**Files:**
- Modify: `crates/api/src/gql/mutation.rs` (delete `enter_result` ~466-576 and `unlock_result` ~578-605; fix module doc ~1-6)
- Modify: `crates/api/src/gql/types.rs` (delete `ResultEntered` + its doc ~338-352)
- Modify: `crates/api/src/recompute.rs` (doc comment mentioning `enter_result`)
- Modify: `crates/api/tests/graphql.rs` (delete dead tests, consts, helper)

- [ ] **Step 1: Delete the dead tests, consts, and helper in `graphql.rs`**

Delete these items entirely:
- consts `ENTER_RESULT` (~478-484), `UNLOCK_RESULT` (~485-489), `ENTER_RESULT_ADVANCER` (~681-686)
- tests: `enter_result_rejects_out_of_range_score`, `enter_result_requires_admin`,
  `enter_result_recomputes_scoreboard`, `enter_result_returns_recompute_pending_false_on_success`,
  `enter_result_rejects_a_locked_result`, `enter_result_allows_correcting_an_unlocked_result`,
  `unlock_result_flips_the_locked_flag`, `unlock_result_requires_admin`,
  `enter_result_advancer_resolves_a_drawn_knockout_to_that_team`,
  `enter_result_rejects_an_advancer_not_in_the_match`
- helper `enter_group_a_results` (~688-697)

Keep `RECOMPUTE` (const ~490), `recompute_mutation_runs_for_an_admin`, and
`recompute_mutation_requires_admin` — the `recompute` mutation stays.

> Note: `enter_result_rejects_an_advancer_not_in_the_match` asserted server-side
> validation that the advancer is one of the match's two teams. `submitGroup`
> does not re-validate the standings `ordering` against the game, so that
> server-side guard is dropped — the draw-order UI only ever offers the two
> teams. This is an accepted reduction (see spec Risks).

- [ ] **Step 2: Delete the mutations and the `ResultEntered` type**

In `crates/api/src/gql/mutation.rs`, delete the entire `enter_result` async fn
(~466-576) and the entire `unlock_result` async fn (~578-605). Update the module
doc (~1-6) to drop the `enterResult` sentence:

```rust
//! The GraphQL mutation root (`API.md` §5).
//!
//! `submitGroup` saves/locks a whole group's predictions onto the player item
//! with optimistic concurrency (retry once on conflict). When the result user
//! submits, their predictions are the official results, so the save triggers
//! the wholesale post-result recompute. `recompute` re-runs it on demand.
```

In `crates/api/src/gql/types.rs`, delete the `ResultEntered` struct and its
doc comment (~338-352).

- [ ] **Step 3: Fix the `recompute.rs` doc comment**

In `crates/api/src/recompute.rs`, change the opening doc line that reads "After
`enter_result` mutates the result user's predictions, the whole ..." to "After
the result user's `submitGroup` mutates their predictions, the whole ...".

- [ ] **Step 4: Build and test the crate**

Run: `cargo test -p api`
Expected: PASS. If the compiler flags an unused import (e.g. `MatchPrediction`
in `mutation.rs` if it was only used by `enter_result`), remove it. Re-run.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/gql/mutation.rs crates/api/src/gql/types.rs crates/api/src/recompute.rs crates/api/tests/graphql.rs
git commit -m "refactor(api): remove enterResult/unlockResult — unified via submitGroup"
```

---

## Task 6: Frontend — My Tips form editable for the result user

**Files:**
- Modify: `web/src/pages/mytips/GroupTipForm.tsx` (~106-111, ~208)

- [ ] **Step 1: Make `readOnly` and per-match locking exempt the result user**

Replace the read-only derivation (~106-111):

```tsx
  // The deadline has passed → the whole group is read-only (UC-7).
  const deadlinePassed = group.deadlinePassed
  const groupLocked =
    deadlinePassed ||
    (games.length > 0 && games.every((g) => matches[g.id]?.locked))
  const readOnly = groupLocked
```
with:
```tsx
  // The result user enters official results and is never locked out — not by
  // the deadline (results arrive after kickoff) nor by a prior lock (they can
  // always re-correct). For everyone else the deadline freezes the group (UC-7).
  const isResultUser = me.isResultUser
  const deadlinePassed = group.deadlinePassed
  const groupLocked =
    deadlinePassed ||
    (games.length > 0 && games.every((g) => matches[g.id]?.locked))
  const readOnly = groupLocked && !isResultUser
```

Replace the per-row lock derivation (~208):
```tsx
            const matchLocked = readOnly || m.locked
```
with:
```tsx
            const matchLocked = readOnly || (m.locked && !isResultUser)
```

- [ ] **Step 2: Type-check, build, lint**

Run: `cd web && npm run build && npm run lint && cd ..`
Expected: PASS. (`me.isResultUser` already exists on the `Player` type and is
fetched by `ME_QUERY`.)

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/mytips/GroupTipForm.tsx
git commit -m "feat(web): result user can enter results from My Tips after the deadline"
```

---

## Task 7: Frontend — remove the `/admin` Results screen

**Files:**
- Modify: `web/src/pages/AdminPage.tsx` (remove the Results tab/route/import; redirect index to teams)
- Delete: `web/src/pages/admin/AdminResults.tsx`
- Modify: `web/src/graphql/queries.ts` (delete `ENTER_RESULT_MUTATION`, `UNLOCK_RESULT_MUTATION`, `RECOMPUTE_MUTATION`)
- Modify: `web/src/graphql/types.ts` (delete `ResultEntered` ~152)
- Modify: `web/src/i18n/strings.ts` (delete keys only used by AdminResults)

- [ ] **Step 1: Drop Results from `AdminPage.tsx`**

- Remove the import line `import { AdminResults } from './admin/AdminResults'`.
- Remove the tab `<AdminTab to="results" label={t('adminResults')} />`.
- Remove the route `<Route path="results" element={<AdminResults />} />`.
- Change the index redirect `<Route index element={<Navigate to="results" replace />} />`
  to `<Route index element={<Navigate to="teams" replace />} />`.

- [ ] **Step 2: Delete the component and its GraphQL/types**

```bash
git rm web/src/pages/admin/AdminResults.tsx
```
In `web/src/graphql/queries.ts`, delete the `ENTER_RESULT_MUTATION` (~181-200),
`UNLOCK_RESULT_MUTATION` (~202-207), and `RECOMPUTE_MUTATION` (~209-213) exports.
In `web/src/graphql/types.ts`, delete the `ResultEntered` interface (~152).

- [ ] **Step 3: Remove the now-orphaned i18n keys**

Confirm each candidate key is referenced only by `strings.ts` (i.e. no remaining
component uses it):
```bash
for k in adminResults resultUnlocked recomputePendingNotice recompute recomputeDone recomputeFailed enterResult unlockResult; do
  echo "== $k =="; rg -n "\b$k\b" web/src --glob '!web/src/i18n/strings.ts'
done
```
Delete from `web/src/i18n/strings.ts` (both the English and Hungarian blocks)
every key above that prints **no** match outside `strings.ts`. Leave any key that
is still referenced elsewhere (e.g. a shared `recompute` label, if used).

- [ ] **Step 4: Type-check, build, lint**

Run: `cd web && npm run build && npm run lint && cd ..`
Expected: PASS with no unused-import or missing-key errors. Fix any TS error the
deletions surface (e.g. a leftover import of a removed constant), then re-run.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/AdminPage.tsx web/src/graphql/queries.ts web/src/graphql/types.ts web/src/i18n/strings.ts
git commit -m "refactor(web): remove standalone /admin Results screen (folded into My Tips)"
```

---

## Task 8: E2E — rewrite the admin-result flow to use My Tips

The existing `web/e2e/admin-scoreboard.spec.ts` drives the **old `/admin` Results
screen** (`expect(page.locator('h3')).toContainText('Results entry')`, the
`Enter result` button). That UI is gone, so the spec must be rewritten to enter
results through My Tips. The clock is switched mid-test via the auth-bar dev-clock
presets (the mechanism `web/e2e/dev-clock-presets.spec.ts` already uses): a player
locks Group A at the `before` preset (deadline future), the result user enters
results at the `after` preset (deadline passed).

**Files:**
- Rename + rewrite: `web/e2e/admin-scoreboard.spec.ts` → `web/e2e/result-entry.spec.ts`

> Dev-stub auth: the dev login `.auth-bar` is the default when `VITE_AUTH0_*` are
> unset (it is — no `web/.env.local` is needed; existing dev-login specs prove
> this). If your shell exports `VITE_AUTH0_*`, blank them via `web/.env.local` or
> the auth bar will be hidden and `devLogin` will fail.

- [ ] **Step 1: Replace the spec file**

```bash
git mv web/e2e/admin-scoreboard.spec.ts web/e2e/result-entry.spec.ts
```
Then overwrite `web/e2e/result-entry.spec.ts` with:

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Unified result entry. A player locks Group A predictions before the deadline;
 * the result user then enters the official results through *My Tips* (not a
 * separate admin screen) after kickoff, and the scoreboard credits the player.
 * Exercises submitGroup (player lock) → submitGroup (result user, deadline-exempt)
 * → recompute → scoreboard — every wire hop the build check cannot reach.
 */

const GAME = 'M1' // Group A's earliest kickoff = the group's deadline.

/** Pick a game + phase in the auth-bar dev clock; it applies and reloads. */
async function setPreset(page: Page, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(GAME)
  await expect(selects.nth(1)).toBeEnabled()
  await selects.nth(1).selectOption(phase)
}

/** Open Group A in My Tips. */
async function openGroupA(page: Page) {
  await page.getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await page.locator('.round-tabs button', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: /^Group A$/ }).click()
  await expect(page.locator('.tip-form h3')).toContainText('Group A')
}

/** The match-prediction rows of the active tip form. */
function matchRows(page: Page) {
  return page.locator('.tip-form table.data-table').first().locator('tbody tr')
}

/** Fill every match in the active tip form with the given score. */
async function fillAll(page: Page, home: string, away: string) {
  const rows = matchRows(page)
  const count = await rows.count()
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption(home)
    await selects.nth(1).selectOption(away)
  }
}

test('result user enters results via My Tips and the scoreboard updates', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // ── 1. demo-ada locks Group A predictions BEFORE the deadline ──────────────
  await devLogin(page, 'demo-ada')
  await setPreset(page, 'before') // Group A open
  await openGroupA(page)
  await fillAll(page, '2', '1')
  const lockBtn = page.getByRole('button', { name: 'Lock group' })
  await expect(lockBtn).toBeEnabled()
  await lockBtn.click()
  await expect(page.locator('.tip-form .flash-bar')).toContainText('Saved')

  // ── 2. the result user enters official results via My Tips AFTER kickoff ────
  await devLogin(page, 'result-user')
  await setPreset(page, 'after') // Group A deadline passed
  await openGroupA(page)
  // Unlike a regular player (locked out post-deadline), the result user can edit.
  await expect(page.getByRole('button', { name: 'Save draft' })).toBeVisible()
  await fillAll(page, '2', '1') // exact match of ada's prediction → max points
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.tip-form .flash-bar')).toContainText('Saved')

  // ── 3. the scoreboard credits demo-ada non-zero points ─────────────────────
  await page.getByRole('link', { name: 'Scoreboard' }).click()
  await expect(page).toHaveURL(/\/scoreboard$/)
  const adaRow = page
    .locator('table.data-table tbody tr')
    .filter({ hasText: 'ada' })
    .first()
  await expect(adaRow).toBeVisible()
  const totalText = await adaRow.locator('td').last().textContent()
  expect(Number((totalText ?? '0').trim()), 'demo-ada credited').toBeGreaterThan(0)

  // ── 4. there is no /admin Results screen anymore ───────────────────────────
  await page.getByRole('link', { name: 'Admin' }).click()
  await expect(page).toHaveURL(/\/admin/)
  await expect(page.locator('.group-subnav')).not.toContainText('Results entry')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the e2e suite**

Run: `cd web && npm run e2e && cd ..`
Expected: the rewritten spec passes; existing specs stay green. (The suite boots
docker + import + seed + API itself via `web/e2e/global-setup.ts`.) If the
`after` preset still shows Group A as read-only for the result user, the Task 6
frontend change is missing or wrong — fix that, not the test.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/result-entry.spec.ts
git commit -m "test(e2e): result user enters results via My Tips (replaces admin flow)"
```

---

## Task 9: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Workspace gates**

Run:
```bash
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```
Expected: all green; `cargo fmt` leaves no diff (re-stage if it reformats).

- [ ] **Step 2: Web gates**

Run: `cd web && npm run build && npm run lint && npm run e2e && cd ..`
Expected: all green.

- [ ] **Step 3: Confirm the removals are complete**

Run:
```bash
rg -n "enterResult|unlockResult|ResultEntered|AdminResults" crates web --glob '!**/target/**'
```
Expected: no hits except possibly the `recompute` mutation (kept) and historical
spec text. Investigate any unexpected match.

- [ ] **Step 4: Commit any fmt/lint fixups**

```bash
git add -A
git commit -m "chore: fmt + lint fixups for unified result entry" --allow-empty
```

---

## Self-review notes (for the implementer)

- **Spec coverage:** Task 1 = scoring §1; Task 2 = read-gates §2; Task 3 =
  submitGroup write exemption §3; Task 4 covers the advancer/standings path the
  identical form already supports; Tasks 5+7 = remove the admin path §5; Task 6 =
  frontend §4; Task 8 = the spec's E2E test; Task 9 = the spec's test gates.
- **Symmetric rule:** the result side now uses `effective_locked` identically to
  the prediction side — no `is_result_user` branch in `domain` or in the
  `query.rs` read-gates. The only `is_result_user` checks are on the write path
  (`submit_group`): deadline exemption + recompute trigger.
- **Dropped behaviour, on purpose:** explicit `unlockResult`, the
  `recomputePending` surface on result entry, and server-side advancer
  membership validation. All are covered by the spec's accepted risks.
