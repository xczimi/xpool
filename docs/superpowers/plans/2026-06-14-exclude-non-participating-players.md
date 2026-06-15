# Exclude non-participating players from listings — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop showing all-zero / "hidden" rows for players who never entered the relevant predictions, by filtering listings on a pure domain predicate ("did this player enter the relevant predictions") applied by the thin GraphQL resolvers.

**Architecture:** A new pure, I/O-free module `crates/domain/src/participation.rs` holds the rule as four selectors (`Player::is_participant`, `participants`, `tippers_in`, `standings_tippers`). The three affected resolvers in `crates/api/src/gql/query.rs` (`scoreboard`, `tips`, `standings`) replace their inline `is_result_user` skips with calls to the matching selector. No `storage`, `recompute`, schema, or frontend changes — materialisation stays complete; filtering is read-side only.

**Tech Stack:** Rust workspace (`cargo`), `async-graphql` resolvers, `InMemoryRepository` for integration tests. Tests run with `cargo test`; lint with `cargo clippy --workspace -- -D warnings`; format with `cargo fmt`.

**Branch:** Work happens on the already-checked-out `exclude-non-participants` branch (code changes never land on `master` directly — see CLAUDE.md "Branch discipline").

---

## Background facts (verified against the codebase)

- `Player` (`crates/domain/src/model.rs:118-130`) has `is_result_user: bool`, `match_predictions: Vec<MatchPrediction>`, `standings_predictions: Vec<StandingsPrediction>`, and the accessors `Player::match_prediction(&self, game_id)` / `Player::standings_prediction(&self, group_id)` (`model.rs:183-195`).
- Type aliases `GameId` / `GroupId` / `PlayerId` are `String` (`model.rs:10-13`); re-exported via `pub use model::*` so `domain::GameId` resolves.
- `crates/domain/src/lib.rs` declares `pub mod {invite, model, pool, scoring}` and re-exports `model::*` + `scoring::*`. The new module is added here.
- `recompute.rs:97-103` inserts a scoreboard entry for **every** non-result player (even all-zero), so the `scoreboard` resolver is where non-participants get dropped at read time.
- `standings_score(...)` already returns `None` for a player with no standings prediction in the group, so the `standings` resolver filter is behaviour-preserving (a superset gate that removes the inline `is_result_user` check and aligns the three resolvers on one pattern) — its integration test is a characterization test.
- Test helpers live in `crates/api/tests/common/mod.rs`: `seeded_repo(offset)` seeds `RESULT_ID` + `ALICE` + `BOB` (none with predictions) into a two-game (`GAME_1`,`GAME_2`) Group A (`GROUP_A`) tournament; `run` / `run_at` execute GraphQL; `data` extracts the JSON; `add_pred` / `locked_pred` add predictions. `pool.rs` is the closest existing pattern for a pure domain logic module; `scoring.rs` is the pattern for an inline `#[cfg(test)] mod tests`.

---

## File Structure

- **Create:** `crates/domain/src/participation.rs` — the four pure selectors + inline unit tests. One responsibility: the participation predicate. ~70 lines + tests.
- **Modify:** `crates/domain/src/lib.rs` — add `pub mod participation;`.
- **Modify:** `crates/api/src/gql/query.rs` — three resolvers (`scoreboard`, `tips`, `standings`) call the selectors instead of inline `is_result_user` skips.
- **Modify:** `crates/api/tests/graphql.rs` — add integration tests for each resolver change.

---

## Task 1: Domain `participation` module (selectors + unit tests)

**Files:**
- Create: `crates/domain/src/participation.rs`
- Modify: `crates/domain/src/lib.rs:6` (add module declaration)

- [ ] **Step 1: Create the module with `todo!()` bodies and the full unit-test module**

Create `crates/domain/src/participation.rs`:

```rust
//! Participation predicates — "did this player enter the relevant predictions"
//! (exclude-non-participating-players design, 2026-06-14).
//!
//! Pure, I/O-free selectors the thin resolvers delegate to, the same way they
//! delegate scoring (`scoring.rs`) and pool rules (`pool.rs`). They answer a
//! domain question (predictions exist), never a presentation one (visibility,
//! points), so the same call returns the same answer for any client — the test
//! that this is domain logic, not view-coupling. The result-user is folded into
//! all of them: it is never a competitor in any listing.

use crate::{GameId, GroupId, Player};

impl Player {
    /// A competing player who has entered at least one prediction.
    /// False for the result-user and for players with no predictions at all.
    pub fn is_participant(&self) -> bool {
        todo!()
    }
}

/// Competitors for global listings (Scoreboard): participants only.
pub fn participants(players: &[Player]) -> Vec<&Player> {
    todo!()
}

/// Players with at least one match prediction among `game_ids` (All Tips).
/// Excludes the result-user.
pub fn tippers_in<'a>(players: &'a [Player], game_ids: &[GameId]) -> Vec<&'a Player> {
    todo!()
}

/// Players with at least one standings prediction among `group_ids`
/// (Standings-bonus grid). Excludes the result-user.
pub fn standings_tippers<'a>(players: &'a [Player], group_ids: &[GroupId]) -> Vec<&'a Player> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MatchPrediction, StandingsPrediction};

    /// Build a player with one match prediction per id in `games` and one
    /// standings prediction per id in `groups`.
    fn mk(id: &str, is_result: bool, games: &[&str], groups: &[&str]) -> Player {
        Player {
            id: id.into(),
            person_id: id.into(),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: is_result,
            version: 0,
            match_predictions: games
                .iter()
                .map(|g| MatchPrediction {
                    game_id: (*g).into(),
                    home_score: 1,
                    away_score: 0,
                    locked: true,
                })
                .collect(),
            standings_predictions: groups
                .iter()
                .map(|g| StandingsPrediction {
                    group_id: (*g).into(),
                    ordering: vec![],
                    draw_order: vec![],
                    locked: true,
                })
                .collect(),
        }
    }

    #[test]
    fn is_participant_truth_table() {
        assert!(
            !mk("ru", true, &["M1"], &["A"]).is_participant(),
            "result-user never participates, even with predictions"
        );
        assert!(
            !mk("empty", false, &[], &[]).is_participant(),
            "no predictions → not a participant"
        );
        assert!(
            mk("matchonly", false, &["M1"], &[]).is_participant(),
            "a single match tip is enough"
        );
        assert!(
            mk("standingsonly", false, &[], &["A"]).is_participant(),
            "a single standings tip is enough"
        );
    }

    #[test]
    fn participants_keeps_only_participating_competitors() {
        let players = vec![
            mk("ru", true, &["M1"], &["A"]),
            mk("empty", false, &[], &[]),
            mk("ada", false, &["M1"], &[]),
        ];
        let got: Vec<&str> = participants(&players).iter().map(|p| p.id.as_str()).collect();
        assert_eq!(got, vec!["ada"]);
    }

    #[test]
    fn participants_handles_empty_input() {
        assert!(participants(&[]).is_empty());
    }

    #[test]
    fn tippers_in_selects_match_tippers_and_excludes_result_user() {
        let players = vec![
            mk("ru", true, &["M1"], &[]),    // result-user → excluded
            mk("ada", false, &["M1"], &[]),  // tipped M1 → in
            mk("alan", false, &["M2"], &[]), // tipped M2, not in [M1] → out
            mk("stand", false, &[], &["A"]), // standings only → out of match grid
        ];
        let got: Vec<&str> = tippers_in(&players, &["M1".to_string()])
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(got, vec!["ada"]);
    }

    #[test]
    fn tippers_in_handles_empty_games() {
        let players = vec![mk("ada", false, &["M1"], &[])];
        assert!(tippers_in(&players, &[]).is_empty());
    }

    #[test]
    fn standings_tippers_selects_standings_tippers_and_excludes_result_user() {
        let players = vec![
            mk("ru", true, &[], &["A"]),     // result-user → excluded
            mk("ada", false, &[], &["A"]),   // standings A → in
            mk("alan", false, &[], &["B"]),  // standings B, not in [A] → out
            mk("match", false, &["M1"], &[]), // match only → out of standings grid
        ];
        let got: Vec<&str> = standings_tippers(&players, &["A".to_string()])
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(got, vec!["ada"]);
    }

    #[test]
    fn standings_tippers_handles_empty_groups() {
        let players = vec![mk("ada", false, &[], &["A"])];
        assert!(standings_tippers(&players, &[]).is_empty());
    }
}
```

Then add the module to `crates/domain/src/lib.rs` — change:

```rust
pub mod invite;
pub mod model;
pub mod pool;
pub mod scoring;
```

to:

```rust
pub mod invite;
pub mod model;
pub mod participation;
pub mod pool;
pub mod scoring;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p domain participation`
Expected: the module compiles, but every test FAILS — panicking with `not yet implemented` from the `todo!()` bodies.

- [ ] **Step 3: Implement the four selectors**

In `crates/domain/src/participation.rs`, replace the four `todo!()` bodies:

```rust
impl Player {
    /// A competing player who has entered at least one prediction.
    /// False for the result-user and for players with no predictions at all.
    pub fn is_participant(&self) -> bool {
        !self.is_result_user
            && (!self.match_predictions.is_empty() || !self.standings_predictions.is_empty())
    }
}

/// Competitors for global listings (Scoreboard): participants only.
pub fn participants(players: &[Player]) -> Vec<&Player> {
    players.iter().filter(|p| p.is_participant()).collect()
}

/// Players with at least one match prediction among `game_ids` (All Tips).
/// Excludes the result-user.
pub fn tippers_in<'a>(players: &'a [Player], game_ids: &[GameId]) -> Vec<&'a Player> {
    players
        .iter()
        .filter(|p| !p.is_result_user)
        .filter(|p| game_ids.iter().any(|g| p.match_prediction(g).is_some()))
        .collect()
}

/// Players with at least one standings prediction among `group_ids`
/// (Standings-bonus grid). Excludes the result-user.
pub fn standings_tippers<'a>(players: &'a [Player], group_ids: &[GroupId]) -> Vec<&'a Player> {
    players
        .iter()
        .filter(|p| !p.is_result_user)
        .filter(|p| group_ids.iter().any(|g| p.standings_prediction(g).is_some()))
        .collect()
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p domain participation`
Expected: all 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/participation.rs crates/domain/src/lib.rs
git commit -m "feat(domain): add participation selectors

Pure is_participant / participants / tippers_in / standings_tippers
selectors lifting the 'entered the relevant predictions' rule out of
inline resolver checks into the domain logic layer."
```

---

## Task 2: Apply `participants` to the `scoreboard` resolver

**Files:**
- Modify: `crates/api/src/gql/query.rs` (the `scoreboard` resolver, ~lines 80-141)
- Test: `crates/api/tests/graphql.rs` (new integration test)

- [ ] **Step 1: Write the failing integration test**

Append to `crates/api/tests/graphql.rs` (after the existing `scoreboard_query_reflects_recompute` test, in the recompute section):

```rust
#[tokio::test]
async fn scoreboard_omits_non_participants_keeps_zero_scorers() {
    // Games kicked off 2h ago. ALICE tipped (0-0, wrong → 0 pts); BOB never
    // tipped. The materialised board scores both (recompute scores everyone),
    // but only the participant ALICE belongs in the listing.
    let repo = seeded_repo(Duration::hours(-2)).await;
    add_pred(&repo, ALICE, GAME_1, 0, 0).await;

    // Result user enters M1 = 2-1 → ALICE scores 0; the submit recomputes.
    let vars = Variables::from_json(json!({
        "g": GROUP_A,
        "p": [{ "gameId": GAME_1, "homeScore": 2, "awayScore": 1 }],
        "lock": false
    }));
    run(&repo, SUBMIT, vars, Some(RESULT_ID)).await;

    let resp = run(
        &repo,
        "{ scoreboard { playerId total } }",
        Variables::default(),
        None,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let rows = d["scoreboard"].as_array().unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r["playerId"].as_str().unwrap()).collect();

    assert!(ids.contains(&ALICE), "participant who scored 0 is kept: {ids:?}");
    assert!(!ids.contains(&BOB), "non-participant is dropped: {ids:?}");
    assert!(!ids.contains(&RESULT_ID), "result user never listed: {ids:?}");

    let alice = rows.iter().find(|r| r["playerId"] == "alice").unwrap();
    assert_eq!(alice["total"], json!(0), "kept with a real 0 total");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p api --test graphql scoreboard_omits_non_participants_keeps_zero_scorers`
Expected: FAIL on `assert!(!ids.contains(&BOB), ...)` — BOB currently appears (recompute gave him an all-zero entry and the resolver does not filter it).

- [ ] **Step 3: Add the participant filter to the resolver**

In `crates/api/src/gql/query.rs`, inside the `scoreboard` resolver, find the block that builds `entries` (just after the `allowed` pool-membership block). It currently reads:

```rust
        let mut entries: Vec<ScoreEntry> = board
            .entries
            .iter()
            .filter(|(pid, _)| allowed.as_ref().is_none_or(|m| m.contains(pid)))
            .map(|(pid, breakdown)| {
```

Replace it with (inserts the participant id-set and one extra `.filter`):

```rust
        // Drop non-participants' all-zero rows. The materialised board scores
        // every player (recompute.rs), but only participants belong in the
        // listing — the same category of rule as excluding the result-user,
        // computed by the pure domain selector.
        let participant_ids: std::collections::HashSet<&str> =
            domain::participation::participants(&players)
                .iter()
                .map(|p| p.id.as_str())
                .collect();

        let mut entries: Vec<ScoreEntry> = board
            .entries
            .iter()
            .filter(|(pid, _)| allowed.as_ref().is_none_or(|m| m.contains(pid)))
            .filter(|(pid, _)| participant_ids.contains(pid.as_str()))
            .map(|(pid, breakdown)| {
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p api --test graphql scoreboard_omits_non_participants_keeps_zero_scorers`
Expected: PASS.

- [ ] **Step 5: Run the existing scoreboard tests to confirm no regression**

Run: `cargo test -p api --test graphql scoreboard`
Expected: all PASS (including `scoreboard_query_reflects_recompute`, whose ALICE has a prediction and stays).

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/gql/query.rs crates/api/tests/graphql.rs
git commit -m "feat(api): scoreboard excludes non-participants

Filter the materialised board to domain::participation::participants —
drops all-zero rows for players who entered no predictions, keeps
participants who tipped but scored 0."
```

---

## Task 3: Apply `tippers_in` to the `tips` resolver

**Files:**
- Modify: `crates/api/src/gql/query.rs` (the `tips` resolver, ~lines 199-279)
- Test: `crates/api/tests/graphql.rs` (two new integration tests)

- [ ] **Step 1: Write the failing integration tests**

Append to `crates/api/tests/graphql.rs` (after the existing tips-visibility tests, near `tips_always_shows_own_unlocked_prediction`). These reuse the existing `TIPS` query const and `add_pred` / `locked_pred` helpers:

```rust
#[tokio::test]
async fn tips_omits_players_with_no_tip_in_the_group() {
    // Games kicked off 2h ago → all tips visible. ALICE tipped M1 (a partial
    // tipper — she skipped M2); BOB tipped nothing in the group.
    let repo = seeded_repo(Duration::hours(-2)).await;
    add_pred(&repo, ALICE, GAME_1, 1, 0).await;

    let vars = Variables::from_json(json!({ "g": GROUP_A }));
    let resp = run(&repo, TIPS, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let rows = d["tips"].as_array().unwrap();

    let players: std::collections::HashSet<&str> =
        rows.iter().map(|t| t["playerId"].as_str().unwrap()).collect();
    assert!(players.contains("alice"), "a tipper is kept: {players:?}");
    assert!(!players.contains("bob"), "a non-tipper is dropped: {players:?}");
    assert!(!players.contains(RESULT_ID), "result user is never listed: {players:?}");

    // The partial tipper is kept across the whole group: ALICE gets a row for
    // both games (M2 renders with an empty prediction — see Non-goals).
    let alice_rows: Vec<&str> = rows
        .iter()
        .filter(|t| t["playerId"] == "alice")
        .map(|t| t["gameId"].as_str().unwrap())
        .collect();
    assert!(alice_rows.contains(&GAME_1), "ALICE row for M1: {alice_rows:?}");
    assert!(alice_rows.contains(&GAME_2), "ALICE row for M2 too: {alice_rows:?}");
}

#[tokio::test]
async fn tips_keeps_a_tipper_whose_prediction_is_still_hidden() {
    // Before kickoff. BOB locked a tip; viewer ALICE has not committed, so BOB's
    // prediction is hidden by mutual commitment — but his ROW must still appear
    // (he participated in this group), not be filtered out as a non-tipper.
    let repo = seeded_repo(Duration::hours(24)).await;
    {
        let mut bob = repo.get_player(BOB).await.unwrap().unwrap();
        bob.match_predictions.push(locked_pred(GAME_1, 2, 1));
        repo.put_player(&bob).await.unwrap();
    }
    let vars = Variables::from_json(json!({ "g": GROUP_A }));
    let resp = run(&repo, TIPS, vars, Some(ALICE)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let bob_g1 = d["tips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["playerId"] == "bob" && t["gameId"] == GAME_1);
    assert!(bob_g1.is_some(), "the hidden tipper's row is still present");
    assert_eq!(
        bob_g1.unwrap()["prediction"],
        json!(null),
        "but the prediction stays hidden until the viewer commits"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p api --test graphql tips_omits_players_with_no_tip_in_the_group`
Expected: FAIL on `assert!(!players.contains("bob"), ...)` — BOB currently gets all-null rows for the group.

(`tips_keeps_a_tipper_whose_prediction_is_still_hidden` passes already — current behaviour keeps the row. It is a regression guard ensuring the new filter does not over-drop a hidden-but-present tipper.)

- [ ] **Step 3: Restrict the resolver loop to `tippers_in`**

In `crates/api/src/gql/query.rs`, inside the `tips` resolver, find:

```rust
        let games = tournament.games_in(&group_id);
        let deadline = tournament.deadline(&group_id);
```

and replace with (add the group's game ids):

```rust
        let games = tournament.games_in(&group_id);
        let game_ids: Vec<domain::GameId> = games.iter().map(|g| g.id.clone()).collect();
        let deadline = tournament.deadline(&group_id);
```

Then find the loop header:

```rust
        let mut tips = Vec::new();
        for player in &players {
            if player.is_result_user {
                continue;
            }
            for game in &games {
```

and replace with (the selector excludes the result-user and non-tippers, so the inline skip is gone):

```rust
        let mut tips = Vec::new();
        for player in domain::participation::tippers_in(&players, &game_ids) {
            for game in &games {
```

- [ ] **Step 4: Run the new tests to verify they pass**

Run: `cargo test -p api --test graphql tips_omits_players_with_no_tip_in_the_group tips_keeps_a_tipper_whose_prediction_is_still_hidden`
Expected: both PASS.

- [ ] **Step 5: Run the existing tips tests to confirm no regression**

Run: `cargo test -p api --test graphql tips`
Expected: all PASS (every existing tips test gives the inspected player a prediction, so it stays in the roster).

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/gql/query.rs crates/api/tests/graphql.rs
git commit -m "feat(api): All Tips excludes players with no tip in the group

Restrict the per-(player, game) loop to domain::participation::tippers_in
over the group's games. Partial tippers are kept; players with nothing in
the viewed group produce no rows."
```

---

## Task 4: Apply `standings_tippers` to the `standings` resolver

**Files:**
- Modify: `crates/api/src/gql/query.rs` (the `standings` resolver, ~lines 287-340)
- Test: `crates/api/tests/graphql.rs` (one new integration test)

- [ ] **Step 1: Write the integration test**

Append to `crates/api/tests/graphql.rs` (after the existing `standings_exposes_each_players_group_bonus` test). Reuses the `STANDINGS` query const:

```rust
#[tokio::test]
async fn standings_omits_players_with_no_standings_prediction_for_the_group() {
    // Deadline passed so locked standings are scoreable. Group A doesn't carry
    // standings in the base fixture — turn it on.
    let repo = seeded_repo(Duration::hours(-2)).await;
    {
        let mut t = repo.get_tournament().await.unwrap().unwrap();
        t.groups.get_mut(GROUP_A).unwrap().carries_standings = true;
        repo.put_tournament(&t).await.unwrap();
    }
    // Only BOB enters a standings prediction for the group; ALICE enters none.
    let standings_pred = domain::StandingsPrediction {
        group_id: GROUP_A.to_owned(),
        ordering: vec!["KOR".into(), "MEX".into(), "RSA".into(), "CZE".into()],
        draw_order: vec![],
        locked: true,
    };
    for id in [RESULT_ID, BOB] {
        let mut p = repo.get_player(id).await.unwrap().unwrap();
        p.match_predictions.push(locked_pred(GAME_1, 2, 1));
        p.match_predictions.push(locked_pred(GAME_2, 3, 0));
        p.standings_predictions.push(standings_pred.clone());
        repo.put_player(&p).await.unwrap();
    }

    let vars = Variables::from_json(json!({ "g": GROUP_A }));
    let resp = run(&repo, STANDINGS, vars, Some(BOB)).await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let d = data(&resp);
    let ids: Vec<&str> = d["standings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["playerId"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&BOB), "a standings tipper is kept: {ids:?}");
    assert!(!ids.contains(&ALICE), "a no-standings player is dropped: {ids:?}");
    assert!(!ids.contains(&RESULT_ID), "result user is never listed: {ids:?}");
}
```

- [ ] **Step 2: Run the test to verify it passes (characterization)**

Run: `cargo test -p api --test graphql standings_omits_players_with_no_standings_prediction_for_the_group`
Expected: PASS already — `standings_score` returns `None` for ALICE (no standings prediction), so she is absent even before the resolver change. This test characterizes the behaviour the resolver change must preserve.

- [ ] **Step 3: Restrict the resolver to `standings_tippers` (behaviour-preserving refactor)**

In `crates/api/src/gql/query.rs`, inside the `standings` resolver, find:

```rust
        let mut leaves = Vec::new();
        collect_leaf_groups(&tournament, &group_id, &mut leaves);

        let mut out = Vec::new();
        for group in leaves {
```

and replace with (build the leaf-group ids and the roster once):

```rust
        let mut leaves = Vec::new();
        collect_leaf_groups(&tournament, &group_id, &mut leaves);
        let leaf_group_ids: Vec<domain::GroupId> = leaves.iter().map(|g| g.id.clone()).collect();
        let roster = domain::participation::standings_tippers(&players, &leaf_group_ids);

        let mut out = Vec::new();
        for group in leaves {
```

Then find the inner per-player loop:

```rust
            for player in &players {
                if player.is_result_user {
                    continue;
                }
                if let Some(sb) =
                    standings_score(group, &games, player, result_user, now, deadline, &config)
```

and replace with (the selector excludes the result-user and players with no standings anywhere in the subtree; `.copied()` yields `&Player` so the roster is reused across leaves):

```rust
            for player in roster.iter().copied() {
                if let Some(sb) =
                    standings_score(group, &games, player, result_user, now, deadline, &config)
```

- [ ] **Step 4: Run the standings tests to confirm no regression**

Run: `cargo test -p api --test graphql standings`
Expected: all PASS (including `standings_exposes_each_players_group_bonus`, whose BOB has a standings prediction, and the new test).

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/gql/query.rs crates/api/tests/graphql.rs
git commit -m "refactor(api): standings grid uses standings_tippers selector

Replace the inline is_result_user skip with
domain::participation::standings_tippers over the group's leaf groups —
aligns the three listings on one pure-domain rule. Behaviour-preserving:
standings_score already gated players with no prediction."
```

---

## Task 5: Full verification

**Files:** none (verification only).

- [ ] **Step 1: Run the whole workspace test suite**

Run: `cargo test --workspace`
Expected: all tests PASS. (DynamoDB integration tests stay skipped without `DYNAMO_TEST=1` — that is green, not a failure.)

- [ ] **Step 2: Lint**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings. (Watch for an unused-import or needless-collect lint on the new code.)

- [ ] **Step 3: Format**

Run: `cargo fmt`
Then: `git diff --stat` — if `cargo fmt` changed anything, review and commit:

```bash
git add -A
git commit -m "style: cargo fmt"
```

(If nothing changed, skip the commit.)

- [ ] **Step 4: Manual / E2E spot-check (per the user's "Frontend work needs E2E" rule, scaled to a read-only change)**

This change ships **no frontend code**, but the design's Testing section requires confirming the grids render cleanly with the now-variable per-group roster. Bring up the local stack against pulled production data (which is what surfaced the bug) and eyeball All Tips + Scoreboard:

```sh
docker compose up -d
export DYNAMO_ENDPOINT=http://localhost:8000
cargo run -p xtask -- import tournaments/fwc26.json
cargo run -p xtask -- seed
cargo run -p api &
cd web && npm run dev
```

Verify in the browser (`:5173`):
- **Scoreboard** no longer lists players who entered nothing (e.g. the pulled `Twoeazy21` / `Balint` / `Czimi` / `Jess` / `Randy`); a participant who scored 0 still appears.
- **All Tips** for a group (e.g. Group A) no longer shows a row for a player who skipped that whole group (e.g. `VanPete` on Group A), while keeping players who tipped at least one of its games.

Report what you observed (which rows disappeared, which stayed). If anything renders broken with the variable roster, that is in-scope and must be fixed before completion.

---

## Self-Review (performed against the spec)

**Spec coverage:**
- Domain layer (`participation.rs`, four selectors, `is_participant` formula) → Task 1. ✓
- `scoreboard` resolver drops non-participants, keeps 0-scorers → Task 2. ✓
- `tips(group_id)` restricted to `tippers_in`, partial tippers kept → Task 3. ✓
- `standings(group_id)` restricted to `standings_tippers` over leaf groups → Task 4. ✓
- "What does NOT change": no `recompute.rs` / schema / storage / view-context edits — none of the tasks touch those. ✓
- Edge cases (result-user excluded everywhere; participant scored 0 kept; standings-only player dropped from All Tips but kept on Standings; per-group roster varies) → covered by Task 1 unit tests + Task 2/3/4 integration tests. ✓
- Testing section (domain truth table + selectors; api integration for all three; E2E/manual) → Tasks 1-5. ✓
- Non-goals (the "hidden" label, a "who hasn't predicted" admin view) → deliberately untouched. ✓

**Type consistency:** `participants(&[Player]) -> Vec<&Player>`, `tippers_in<'a>(&'a [Player], &[GameId]) -> Vec<&'a Player>`, `standings_tippers<'a>(&'a [Player], &[GroupId]) -> Vec<&'a Player>`, `Player::is_participant(&self) -> bool` — used identically in resolvers (`domain::participation::participants`/`tippers_in`/`standings_tippers`) and tests. `GameId`/`GroupId` are `String`, so `.id.clone()` into `Vec<domain::GameId>` matches the slice params. `roster.iter().copied()` yields `&Player` to match `standings_score`'s `&Player` param; `for player in tippers_in(...)` yields `&Player` matching the prior `for player in &players`.

**Placeholder scan:** no TBD/TODO/"add error handling" — every code step shows complete code; the only `todo!()` is the intentional RED in Task 1 Step 1, removed in Step 3.
