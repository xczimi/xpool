# Best Third-Placed Teams Table (Phase 1 — display) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the FWC26 best-third-placed-teams ranking (already computed by `fwc26::best_thirds`/`resolve_bracket` but never exposed) as a read-only table — the player's predicted ranking next to the official one on My Tips, and the official-only ranking on Schedule — for visibility and transparency.

**Architecture:** A new pure function `fwc26::third_place_ranking` extracts the ranking the bracket resolver already computes (12 thirds → top 8 → Annexe C pairing). A new `thirdPlaceRanking(player: ID)` GraphQL query resolves it for any perspective (`null` = the official result user). The React SPA renders a presentational `ThirdPlaceTable`, mounted twice on My Tips (predicted + official) and once on Schedule (official). No change to the locked `domain::model` contract; the editable last-resort tiebreak is Phase 2 (`.scratch/best-thirds-table/phase-2-editable-tiebreak.md`).

**Tech Stack:** Rust (async-graphql, the `fwc26` + `api` crates), React + Vite + TypeScript, urql GraphQL client, Playwright e2e.

**Design spec:** `docs/superpowers/specs/2026-06-27-best-third-placed-teams-table-design.md`

---

## File Structure

**Backend**
- `crates/fwc26/src/lib.rs` — *modify*: add `ThirdPlaceRow` struct + `third_place_ranking()` pure function + private `game_for_third_slot()` helper. Reuses existing private helpers (`compute_group_standings`, `compute_team_stats_in_group`, `annexe_c`, `BEST_THIRD_SLOTS`).
- `crates/fwc26/tests/third_place_ranking_tests.rs` — *create*: unit tests with a 12-group fixture (mirrors `resolve_bracket_tests.rs`).
- `crates/api/src/gql/types.rs` — *modify*: add `ThirdPlaceEntry` + `ThirdPlaceRanking` SimpleObjects.
- `crates/api/src/gql/query.rs` — *modify*: add the `third_place_ranking` resolver + a `third_place_tests` module.

**Frontend**
- `web/src/graphql/queries.ts` — *modify*: add `THIRD_PLACE_QUERY`.
- `web/src/graphql/types.ts` — *modify*: add `ThirdPlaceEntry` + `ThirdPlaceRanking` TS interfaces.
- `web/src/components/ThirdPlaceTable.tsx` — *create*: presentational table.
- `web/src/index.css` — *modify*: `.third-place` / `.third-place-table` styles.
- `web/src/i18n/strings.ts` — *modify*: en + hu strings.
- `web/src/pages/SchedulePage.tsx` — *modify*: mount the official-only table.
- `web/src/pages/MyTipsPage.tsx` — *modify*: mount predicted + official tables.
- `web/e2e/third-place.spec.ts` — *create*: round-trip smoke test on both pages.

Each task is independently committable. Backend tasks (1–2) come first because the frontend types mirror the schema they define.

---

## Task 1: `fwc26::third_place_ranking` pure function

**Files:**
- Modify: `crates/fwc26/src/lib.rs` (add after `best_thirds`, ~line 118)
- Test: `crates/fwc26/tests/third_place_ranking_tests.rs` (create)

- [ ] **Step 1: Write the failing test**

Create `crates/fwc26/tests/third_place_ranking_tests.rs`. The fixture builders mirror `resolve_bracket_tests.rs` (same minimal 12-group tournament: each group has 3 teams `X1,X2,X3` playing a round-robin `M(n)=X1-X2, M(n+1)=X1-X3, M(n+2)=X2-X3`). With every group scored identically (`X1` wins both, `X2` draws `X3`), all 12 thirds tie on stats, so the ranking falls through to group-letter order A–L — top 8 = A,B,C,D,E,F,G,H.

```rust
//! Tests for third_place_ranking (FWC26_RULES.md §3) — the display ranking.

use domain::*;
use fwc26::third_place_ranking;
use std::collections::HashMap;

fn kickoff() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-06-11T18:00:00Z")
        .unwrap()
        .into()
}

fn team(id: &str) -> Team {
    Team {
        id: id.to_string(),
        name: id.to_string(),
        short_code: id.to_string(),
        flag: None,
        external_id: None,
    }
}

fn slot_team(team_id: &str, desc: &str) -> TeamSlot {
    TeamSlot {
        team_id: Some(team_id.to_string()),
        description: desc.to_string(),
    }
}

fn slot_placeholder(desc: &str) -> TeamSlot {
    TeamSlot {
        team_id: None,
        description: desc.to_string(),
    }
}

fn game(id: &str, group_id: &str, home: TeamSlot, away: TeamSlot) -> SingleGame {
    SingleGame {
        id: id.to_string(),
        kickoff: kickoff(),
        venue: None,
        group_id: group_id.to_string(),
        home,
        away,
        external_id: None,
    }
}

fn group_stage_group(id: &str, games: Vec<String>) -> GroupGame {
    GroupGame {
        id: id.to_string(),
        name: format!("Group {}", id.to_uppercase().replace("GROUP-", "")),
        parent: Some("root".to_string()),
        round: Round::GroupStage,
        lock_mode: LockMode::LockTogether,
        carries_standings: true,
        children: GroupChildren::Games(games),
    }
}

fn knockout_group(id: &str, game_ids: Vec<String>) -> GroupGame {
    GroupGame {
        id: id.to_string(),
        name: id.to_string(),
        parent: Some("knockout".to_string()),
        round: Round::R32,
        lock_mode: LockMode::LockPerMatch,
        carries_standings: false,
        children: GroupChildren::Games(game_ids),
    }
}

fn pred(game_id: &str, home: u8, away: u8) -> MatchPrediction {
    MatchPrediction {
        game_id: game_id.to_string(),
        home_score: home,
        away_score: away,
        locked: true,
    }
}

fn standings_pred(group_id: &str, ordering: Vec<&str>, draw_order: Vec<&str>) -> StandingsPrediction {
    StandingsPrediction {
        group_id: group_id.to_string(),
        ordering: ordering.iter().map(|s| s.to_string()).collect(),
        draw_order: draw_order.iter().map(|s| s.to_string()).collect(),
        locked: true,
    }
}

fn result_player(
    match_predictions: Vec<MatchPrediction>,
    standings_predictions: Vec<StandingsPrediction>,
) -> Player {
    Player {
        id: "result".to_string(),
        person_id: "result_person".to_string(),
        nick: "result".to_string(),
        full_name: "Result User".to_string(),
        referrer: None,
        is_result_user: true,
        version: 1,
        match_predictions,
        standings_predictions,
    }
}

/// 12 groups (A–L), 3 teams + 3 games each, plus the one R32 game whose slot is
/// "3ABCDF" (winner E faces a third, per BEST_THIRD_SLOTS) so faces_game has a
/// target. `include_all_groups = false` drops group L to exercise the
/// provisional (incomplete) path.
fn build_test_tournament(include_all_groups: bool) -> Tournament {
    let mut groups: HashMap<GroupId, GroupGame> = HashMap::new();
    let mut games: HashMap<GameId, SingleGame> = HashMap::new();
    let mut teams: HashMap<TeamId, Team> = HashMap::new();

    let last = if include_all_groups { 'L' } else { 'K' };
    let mut match_num = 1u32;
    for letter in 'A'..=last {
        let group_id = format!("group-{}", letter);
        let t1 = format!("{}1", letter);
        let t2 = format!("{}2", letter);
        let t3 = format!("{}3", letter);
        teams.insert(t1.clone(), team(&t1));
        teams.insert(t2.clone(), team(&t2));
        teams.insert(t3.clone(), team(&t3));

        let ids: Vec<String> = (match_num..match_num + 3).map(|n| format!("M{}", n)).collect();
        match_num += 3;
        games.insert(ids[0].clone(), game(&ids[0], &group_id, slot_team(&t1, &t1), slot_team(&t2, &t2)));
        games.insert(ids[1].clone(), game(&ids[1], &group_id, slot_team(&t1, &t1), slot_team(&t3, &t3)));
        games.insert(ids[2].clone(), game(&ids[2], &group_id, slot_team(&t2, &t2), slot_team(&t3, &t3)));
        groups.insert(group_id.clone(), group_stage_group(&group_id, ids));
    }

    // The R32 match for the "3ABCDF" slot (winner E vs a best third).
    games.insert(
        "M74".to_string(),
        game("M74", "r32-m74", slot_placeholder("1E"), slot_placeholder("3ABCDF")),
    );
    groups.insert("r32-m74".to_string(), knockout_group("r32-m74", vec!["M74".to_string()]));

    Tournament { root: "group-A".to_string(), groups, games, teams }
}

/// Results for one group: X1 wins both (6pts), X2 beats X3 → X2 2nd, X3 3rd.
fn group_predictions(letter: char, ids: &[String]) -> (Vec<MatchPrediction>, Vec<StandingsPrediction>) {
    let t1 = format!("{}1", letter);
    let t2 = format!("{}2", letter);
    let t3 = format!("{}3", letter);
    let mp = vec![
        pred(&ids[0], 2, 0), // X1 2-0 X2
        pred(&ids[1], 2, 0), // X1 2-0 X3
        pred(&ids[2], 1, 0), // X2 1-0 X3
    ];
    let sp = vec![standings_pred(&format!("group-{}", letter), vec![&t1, &t2, &t3], vec![])];
    (mp, sp)
}

fn all_predictions(last: char) -> (Vec<MatchPrediction>, Vec<StandingsPrediction>) {
    let mut mp = Vec::new();
    let mut sp = Vec::new();
    let mut m = 1u32;
    for letter in 'A'..=last {
        let ids: Vec<String> = (m..m + 3).map(|n| format!("M{}", n)).collect();
        let (a, b) = group_predictions(letter, &ids);
        mp.extend(a);
        sp.extend(b);
        m += 3;
    }
    (mp, sp)
}

#[test]
fn ranks_all_twelve_thirds_and_flags_top_eight() {
    let t = build_test_tournament(true);
    let (mp, sp) = all_predictions('L');
    let result = result_player(mp, sp);

    let rows = third_place_ranking(&t, &result);

    // All 12 groups determinable → 12 rows, ranked 1..=12.
    assert_eq!(rows.len(), 12, "one row per group");
    assert_eq!(rows[0].rank, 1);
    assert_eq!(rows[11].rank, 12);
    // Every third is X3 (3rd-placed team of its group).
    assert!(rows.iter().all(|r| r.team_id.ends_with('3')));
    // All tie on stats → group-letter order A..L; top 8 (A–H) qualify.
    let qualifying: Vec<char> = rows.iter().filter(|r| r.qualifies).map(|r| r.group).collect();
    assert_eq!(qualifying, vec!['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']);
    assert_eq!(rows.iter().filter(|r| r.qualifies).count(), 8);
}

#[test]
fn attaches_annexe_c_pairing_for_qualifiers() {
    let t = build_test_tournament(true);
    let (mp, sp) = all_predictions('L');
    let result = result_player(mp, sp);

    let rows = third_place_ranking(&t, &result);

    // Annexe C maps winner E to exactly one of the qualifying third-groups.
    // Whichever group it is, that row must point at game M74 (the "3ABCDF" slot).
    let faces_e: Vec<&_> = rows
        .iter()
        .filter(|r| r.faces_winner_group == Some('E'))
        .collect();
    assert_eq!(faces_e.len(), 1, "exactly one third faces winner E");
    assert_eq!(faces_e[0].faces_game.as_deref(), Some("M74"));
    assert!(faces_e[0].qualifies);
    // Non-qualifiers never carry a pairing.
    assert!(rows.iter().filter(|r| !r.qualifies).all(|r| r.faces_game.is_none()));
}

#[test]
fn provisional_when_a_group_is_undecided() {
    // Only 11 groups (A–K) have results → < 12 determinable thirds.
    let t = build_test_tournament(false);
    let (mp, sp) = all_predictions('K');
    let result = result_player(mp, sp);

    let rows = third_place_ranking(&t, &result);

    assert_eq!(rows.len(), 11, "only determinable groups produce rows");
    // With 11 thirds, the top-8 set is still resolvable (8 of 11), so Annexe C
    // MAY resolve; but the table is not complete (the resolver's `complete`
    // flag, computed in the GraphQL layer, gates on 12). Here we just assert
    // ranks are dense and qualifies count is 8.
    assert_eq!(rows.iter().filter(|r| r.qualifies).count(), 8);
    assert_eq!(rows[0].rank, 1);
    assert_eq!(rows[10].rank, 11);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p fwc26 --test third_place_ranking_tests`
Expected: FAIL to compile — `no function or associated item named third_place_ranking` / `cannot find type ThirdPlaceRow`.

- [ ] **Step 3: Write the implementation**

In `crates/fwc26/src/lib.rs`, add immediately after `best_thirds` (after line 118):

```rust
/// One row of the best-third-placed-teams ranking (`FWC26_RULES.md` §3), for
/// display / transparency. Pure: derived from a player's (or the result user's)
/// predictions. `faces_*` are populated only once the qualifying set of 8 is
/// known (via Annexe C).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThirdPlaceRow {
    /// Group letter (A–L).
    pub group: char,
    /// The third-placed team of that group.
    pub team_id: TeamId,
    pub points: i32,
    pub goal_diff: i32,
    pub goals_for: i32,
    /// 1-based position in the ranking (best = 1).
    pub rank: u32,
    /// Top-8 → advances to the R32.
    pub qualifies: bool,
    /// The group-winner this third faces in the R32 (via Annexe C), if known.
    pub faces_winner_group: Option<char>,
    /// The R32 game id this third plays in (via Annexe C), if known.
    pub faces_game: Option<GameId>,
}

/// Rank the determinable third-placed teams (`FWC26_RULES.md` §3), best first,
/// flag the top 8, and attach each qualifier's R32 pairing via Annexe C.
///
/// Only a group whose 3rd place is determinable (every group game has a result)
/// contributes a row. When 8+ thirds are known the qualifying set resolves and
/// Annexe C fills the `faces_*` fields. Shares `best_thirds`' criteria and the
/// group-letter stable fallback, so this table matches the resolved bracket.
pub fn third_place_ranking(t: &Tournament, result: &Player) -> Vec<ThirdPlaceRow> {
    let group_standings = compute_group_standings(t, result);

    // Each determinable group's third-placed team + stats, gathered in A–L
    // order (the stable last-resort tiebreak, identical to `best_thirds`).
    let mut thirds: Vec<(char, TeamId, TeamStats)> = Vec::new();
    for letter in 'A'..='L' {
        if let Some(gs) = group_standings.get(&letter) {
            if gs.order.len() >= 3 {
                let third_id = gs.order[2].clone();
                let stats = compute_team_stats_in_group(t, result, letter, &third_id);
                thirds.push((letter, third_id, stats));
            }
        }
    }

    let mut indexed: Vec<(usize, char, TeamId, TeamStats)> = thirds
        .into_iter()
        .enumerate()
        .map(|(i, (g, id, s))| (i, g, id, s))
        .collect();
    indexed.sort_by(|a, b| {
        b.3.points
            .cmp(&a.3.points)
            .then_with(|| b.3.goal_diff.cmp(&a.3.goal_diff))
            .then_with(|| b.3.goals_for.cmp(&a.3.goals_for))
            .then_with(|| a.0.cmp(&b.0)) // stable: preserve A–L input order
    });

    let qualifying_set: BTreeSet<char> = indexed.iter().take(8).map(|(_, g, _, _)| *g).collect();
    let annexe_c_map = if qualifying_set.len() == 8 {
        annexe_c(&qualifying_set)
    } else {
        None
    };

    indexed
        .iter()
        .enumerate()
        .map(|(rank0, (_, g, id, s))| {
            let qualifies = rank0 < 8;
            let (faces_winner_group, faces_game) = match (qualifies, &annexe_c_map) {
                (true, Some(annex)) => {
                    let w = annex.iter().find(|(_, third)| **third == *g).map(|(w, _)| *w);
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
        .collect()
}

/// The R32 game whose "3…" slot is occupied by the third facing group-winner
/// `winner`, via the fixed `BEST_THIRD_SLOTS` spelling.
fn game_for_third_slot(t: &Tournament, winner: char) -> Option<GameId> {
    let slot = BEST_THIRD_SLOTS.iter().find(|(_, w)| *w == winner).map(|(s, _)| *s)?;
    let needle = format!("3{}", slot);
    t.games.iter().find_map(|(id, g)| {
        if g.home.description.trim() == needle || g.away.description.trim() == needle {
            Some(id.clone())
        } else {
            None
        }
    })
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p fwc26 --test third_place_ranking_tests`
Expected: PASS (3 tests). Then `cargo test -p fwc26` to confirm no regression in the existing bracket/annexe tests.

- [ ] **Step 5: Lint + commit**

Run: `cargo clippy -p fwc26 -- -D warnings && cargo fmt`
```bash
git add crates/fwc26/src/lib.rs crates/fwc26/tests/third_place_ranking_tests.rs
git commit -m "feat(fwc26): third_place_ranking — ranked thirds + Annexe C pairing for display"
```

---

## Task 2: GraphQL `thirdPlaceRanking` query

**Files:**
- Modify: `crates/api/src/gql/types.rs` (add types after `MatchScore`/`MatchDetail`, ~line 520)
- Modify: `crates/api/src/gql/query.rs` (add resolver in the `#[Object] impl QueryRoot`, after `match_detail`; add a `third_place_tests` module)

- [ ] **Step 1: Write the failing test**

Add to the bottom of `crates/api/src/gql/query.rs`:

```rust
#[cfg(test)]
mod third_place_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame,
        StandingsPrediction, Team, TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct NoSource;
    #[async_trait]
    impl ReportedResultSource for NoSource {
        async fn lookup_events(&self, _ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(vec![])
        }
    }

    fn team(id: &str) -> Team {
        Team { id: id.into(), name: id.into(), short_code: id.into(), flag: None, external_id: None }
    }

    fn slot(team_id: &str) -> TeamSlot {
        TeamSlot { team_id: Some(team_id.into()), description: team_id.into() }
    }

    fn g(id: &str, home: &str, away: &str) -> SingleGame {
        SingleGame {
            id: id.into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 18, 0, 0).unwrap(),
            venue: None,
            group_id: "group-A".into(),
            home: slot(home),
            away: slot(away),
            external_id: None,
        }
    }

    fn preds(p1: u8, p2: u8, p3: u8, p4: u8, p5: u8, p6: u8) -> Vec<MatchPrediction> {
        vec![
            MatchPrediction { game_id: "M1".into(), home_score: p1, away_score: p2, locked: true },
            MatchPrediction { game_id: "M2".into(), home_score: p3, away_score: p4, locked: true },
            MatchPrediction { game_id: "M3".into(), home_score: p5, away_score: p6, locked: true },
        ]
    }

    fn player(id: &str, is_result_user: bool, mp: Vec<MatchPrediction>) -> Player {
        Player {
            id: id.into(),
            person_id: format!("p-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user,
            version: 0,
            match_predictions: mp,
            standings_predictions: vec![StandingsPrediction {
                group_id: "group-A".into(),
                ordering: vec![],
                draw_order: vec![],
                locked: true,
            }],
        }
    }

    /// One group A with teams AAA/BBB/CCC, round-robin M1/M2/M3.
    async fn repo_one_group() -> Arc<dyn Repository> {
        let group = GroupGame {
            id: "group-A".into(),
            name: "Group A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into(), "M2".into(), "M3".into()]),
        };
        let t = Tournament {
            root: "group-A".into(),
            groups: HashMap::from([("group-A".to_string(), group)]),
            games: HashMap::from([
                ("M1".to_string(), g("M1", "AAA", "BBB")),
                ("M2".to_string(), g("M2", "AAA", "CCC")),
                ("M3".to_string(), g("M3", "BBB", "CCC")),
            ]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
                ("CCC".to_string(), team("CCC")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        // Official: AAA wins both, BBB beats CCC → 3rd = CCC.
        repo.put_player(&player("result-user", true, preds(2, 0, 2, 0, 1, 0)))
            .await
            .unwrap();
        // A player whose results make BBB come 3rd instead (CCC wins both for them):
        // M1 AAA-BBB 0-1, M2 AAA-CCC 0-2, M3 BBB-CCC 0-1 → CCC 1st, ... 3rd = AAA.
        repo.put_player(&player("demo-ada", false, preds(0, 1, 0, 2, 0, 1)))
            .await
            .unwrap();
        Arc::new(repo)
    }

    async fn exec(repo: Arc<dyn Repository>, query: &str) -> serde_json::Value {
        let source: Arc<dyn ReportedResultSource> = Arc::new(NoSource);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(CurrentPlayer::Visitor)
            .data(crate::clock::RequestNow("2026-06-20T12:00:00Z".parse().unwrap()));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        resp.data.into_json().unwrap()
    }

    #[tokio::test]
    async fn null_player_yields_official_ranking() {
        let repo = repo_one_group().await;
        let data = exec(
            repo,
            r#"{ thirdPlaceRanking { complete entries { group rank team { id } qualifies facesGame } } }"#,
        )
        .await;
        let r = &data["thirdPlaceRanking"];
        // Only 1 group determinable → not complete.
        assert_eq!(r["complete"], false);
        let entries = r["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1, "one determinable third");
        // Official 3rd place in group A is CCC.
        assert_eq!(entries[0]["team"]["id"], "CCC");
        assert_eq!(entries[0]["rank"], 1);
        // With only 1 third, no Annexe C pairing.
        assert_eq!(entries[0]["facesGame"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn explicit_player_yields_that_players_ranking() {
        let repo = repo_one_group().await;
        let data = exec(
            repo,
            r#"{ thirdPlaceRanking(player: "demo-ada") { entries { team { id } } } }"#,
        )
        .await;
        let entries = data["thirdPlaceRanking"]["entries"].as_array().unwrap();
        // demo-ada's results put AAA 3rd (distinct from the official CCC).
        assert_eq!(entries[0]["team"]["id"], "AAA");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p api third_place_tests`
Expected: FAIL to compile — `unknown field "thirdPlaceRanking"` is a runtime GraphQL error, but the test will first fail to compile only if types are missing; since we reference no new Rust types in the test, it compiles and FAILS at runtime with `Unknown field "thirdPlaceRanking" on type "QueryRoot"`.

- [ ] **Step 3a: Add the GraphQL types**

In `crates/api/src/gql/types.rs`, append (after `MatchDetail`, end of file):

```rust
/// One row of the best-third-placed-teams ranking (`thirdPlaceRanking`,
/// `FWC26_RULES.md` §3). Read-only, for transparency.
#[derive(SimpleObject, Clone, Debug)]
pub struct ThirdPlaceEntry {
    /// Group letter (e.g. `"A"`).
    pub group: String,
    /// The third-placed team of that group.
    pub team: Team,
    pub points: i32,
    pub goal_diff: i32,
    pub goals_for: i32,
    /// 1-based ranking position (best = 1).
    pub rank: i32,
    /// Whether this third advances to the R32 (top 8).
    pub qualifies: bool,
    /// The group-winner faced in the R32 (e.g. `"E"`), once the qualifying set
    /// of 8 is known; `None` while provisional.
    pub faces_winner_group: Option<String>,
    /// The R32 game id this third plays in, once known.
    pub faces_game: Option<String>,
}

/// The best-third-placed ranking from one perspective (`thirdPlaceRanking`).
#[derive(SimpleObject, Clone, Debug)]
pub struct ThirdPlaceRanking {
    /// All determinable thirds, ranked best-first.
    pub entries: Vec<ThirdPlaceEntry>,
    /// True once all 12 groups' thirds are determinable (the qualifying set of
    /// 8 and its Annexe C pairings are final).
    pub complete: bool,
}
```

- [ ] **Step 3b: Add the resolver**

In `crates/api/src/gql/query.rs`, inside `#[Object] impl QueryRoot`, add after `match_detail` (before the closing `}` of the impl, ~line 756):

```rust
    /// The best third-placed-teams ranking (`FWC26_RULES.md` §3) for visibility
    /// and transparency. `player: null` → the official result user's ranking; a
    /// player id → that player's predicted ranking. Public (the schedule shows
    /// the official ranking without login). Resolves the pure
    /// `fwc26::third_place_ranking` — no domain logic here.
    async fn third_place_ranking(
        &self,
        ctx: &Context<'_>,
        player: Option<String>,
    ) -> async_graphql::Result<ThirdPlaceRanking> {
        let repo = repo(ctx);
        let Some(t) = repo.get_tournament().await? else {
            return Ok(ThirdPlaceRanking { entries: Vec::new(), complete: false });
        };
        let players = repo.list_players().await?;

        // Perspective: an explicit player id, else the official result user.
        let subject = match &player {
            Some(pid) => players.iter().find(|p| &p.id == pid),
            None => players.iter().find(|p| p.is_result_user),
        };
        let Some(subject) = subject else {
            return Ok(ThirdPlaceRanking { entries: Vec::new(), complete: false });
        };

        let rows = fwc26::third_place_ranking(&t, subject);
        let entries: Vec<ThirdPlaceEntry> = rows
            .iter()
            .filter_map(|r| {
                t.teams.get(&r.team_id).map(|team| ThirdPlaceEntry {
                    group: r.group.to_string(),
                    team: Team::from(team),
                    points: r.points,
                    goal_diff: r.goal_diff,
                    goals_for: r.goals_for,
                    rank: r.rank as i32,
                    qualifies: r.qualifies,
                    faces_winner_group: r.faces_winner_group.map(|c| c.to_string()),
                    faces_game: r.faces_game.clone(),
                })
            })
            .collect();
        let complete = entries.len() == 12;
        Ok(ThirdPlaceRanking { entries, complete })
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p api third_place_tests`
Expected: PASS (2 tests). Then `cargo test -p api` to confirm no regression.

- [ ] **Step 5: Lint + commit**

Run: `cargo clippy -p api -- -D warnings && cargo fmt`
```bash
git add crates/api/src/gql/types.rs crates/api/src/gql/query.rs
git commit -m "feat(api): thirdPlaceRanking query (official + per-player) over fwc26::third_place_ranking"
```

---

## Task 3: Web GraphQL query document + TS types

**Files:**
- Modify: `web/src/graphql/queries.ts`
- Modify: `web/src/graphql/types.ts`

> No standalone test step — this is type/string plumbing exercised by Tasks 5–6 and the e2e in Task 8. Verification is `tsc -b`.

- [ ] **Step 1: Add the query document**

In `web/src/graphql/queries.ts`, append:

```typescript
/** The best third-placed-teams ranking (FWC26_RULES §3). `player: null` →
 *  official; a player id → that player's predicted ranking. */
export const THIRD_PLACE_QUERY = `
  query ThirdPlaceRanking($player: ID) {
    thirdPlaceRanking(player: $player) {
      complete
      entries {
        group
        team { id name shortCode flag externalId }
        points
        goalDiff
        goalsFor
        rank
        qualifies
        facesWinnerGroup
        facesGame
      }
    }
  }
`
```

- [ ] **Step 2: Add the TS types**

In `web/src/graphql/types.ts`, append (after the existing types; `Team` is already declared in this file):

```typescript
export interface ThirdPlaceEntry {
  group: string
  team: Team
  points: number
  goalDiff: number
  goalsFor: number
  /** 1-based ranking position (best = 1). */
  rank: number
  /** Top-8 → advances to the R32. */
  qualifies: boolean
  /** The group-winner faced in the R32 (e.g. "E"), once known. */
  facesWinnerGroup: string | null
  /** The R32 game id this third plays in, once known. */
  facesGame: string | null
}

export interface ThirdPlaceRanking {
  entries: ThirdPlaceEntry[]
  /** True once all 12 groups' thirds are final. */
  complete: boolean
}
```

- [ ] **Step 3: Verify the types compile**

Run: `cd web && npm run build`
Expected: `tsc -b` passes (no usages yet, so this only checks the new declarations parse).

- [ ] **Step 4: Commit**

```bash
git add web/src/graphql/queries.ts web/src/graphql/types.ts
git commit -m "feat(web): THIRD_PLACE_QUERY + ThirdPlaceRanking types"
```

---

## Task 4: `ThirdPlaceTable` component + CSS

**Files:**
- Create: `web/src/components/ThirdPlaceTable.tsx`
- Modify: `web/src/index.css`
- Modify: `web/src/i18n/strings.ts` (the keys this component reads — full set added in Task 7; add the subset used here now so it compiles)

- [ ] **Step 1: Add the i18n keys this component needs**

In `web/src/i18n/strings.ts`, add these keys to BOTH the `en` and `hu` objects (place near the my-tips / schedule keys). English values shown; Hungarian in parentheses — use exactly these:

```typescript
  // best third-placed teams (FWC26)
  thirdsRank: '#',                                  // hu: '#'
  thirdsGroup: 'Grp',                               // hu: 'Cs'
  thirdsTeam: 'Team',                               // hu: 'Csapat'
  thirdsPts: 'Pts',                                 // hu: 'Pt'
  thirdsGd: 'GD',                                   // hu: 'Gk'
  thirdsGf: 'GF',                                   // hu: 'LG'
  thirdsFaces: 'R32 vs',                            // hu: 'R32 ell.'
  thirdsWinnerPrefix: 'Winner',                     // hu: 'Győztes'
  thirdsQualifies: 'Qualifies',                     // hu: 'Továbbjut'
  thirdsProvisional: 'Provisional — group stage incomplete.', // hu: 'Ideiglenes — a csoportkör még tart.'
  thirdsPending: 'No third-placed teams decided yet.',        // hu: 'Még egy harmadik helyezett sem dőlt el.'
```

(Task 7 adds the remaining title/heading keys used by the pages.)

- [ ] **Step 2: Create the component**

Create `web/src/components/ThirdPlaceTable.tsx`:

```typescript
import { useI18n } from '../i18n/useI18n'
import type { ThirdPlaceRanking } from '../graphql/types'

/**
 * Read-only best-third-placed-teams table (FWC26_RULES §3). The top-8 rows are
 * highlighted as qualifiers; each qualifier shows its R32 pairing (resolved via
 * Annexe C). Purely presentational — the page owns the query.
 */
export function ThirdPlaceTable({
  title,
  ranking,
}: {
  title: string
  ranking: ThirdPlaceRanking | null
}) {
  const { t } = useI18n()

  if (!ranking || ranking.entries.length === 0) {
    return (
      <div className="third-place">
        <h4>{title}</h4>
        <p className="hint">{t('thirdsPending')}</p>
      </div>
    )
  }

  return (
    <div className="third-place">
      <h4>{title}</h4>
      {!ranking.complete && <p className="hint">{t('thirdsProvisional')}</p>}
      <table className="data-table compact third-place-table">
        <thead>
          <tr>
            <th>{t('thirdsRank')}</th>
            <th>{t('thirdsGroup')}</th>
            <th>{t('thirdsTeam')}</th>
            <th className="num">{t('thirdsPts')}</th>
            <th className="num">{t('thirdsGd')}</th>
            <th className="num">{t('thirdsGf')}</th>
            <th>{t('thirdsFaces')}</th>
          </tr>
        </thead>
        <tbody>
          {ranking.entries.map((e) => (
            <tr
              key={e.group}
              className={e.qualifies ? 'qualifies' : 'eliminated'}
            >
              <td>{e.rank}</td>
              <td>{e.group}</td>
              <td>
                {e.team.flag ? `${e.team.flag} ` : ''}
                {e.team.name}
              </td>
              <td className="num">{e.points}</td>
              <td className="num">{e.goalDiff}</td>
              <td className="num">{e.goalsFor}</td>
              <td>
                {e.qualifies && e.facesWinnerGroup
                  ? `${t('thirdsWinnerPrefix')} ${e.facesWinnerGroup}`
                  : e.qualifies
                    ? t('thirdsQualifies')
                    : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
```

- [ ] **Step 3: Add the CSS**

In `web/src/index.css`, near the `.standings` block (~line 817), append:

```css
.third-place {
  flex: 1;
  min-width: 320px;
}
.third-place h4 {
  margin-bottom: 8px;
  color: var(--amber);
  font-size: 10px;
  letter-spacing: 1px;
}
.third-place-table tr.qualifies td {
  background: rgba(0, 200, 0, 0.08);
}
.third-place-table tr.qualifies td:first-child {
  border-left: 3px solid var(--amber-bright);
}
.third-place-table tr.eliminated td {
  opacity: 0.55;
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cd web && npm run build && npm run lint`
Expected: PASS. (The component is not mounted yet, but `tsc -b` type-checks the file and its imports.)

- [ ] **Step 5: Commit**

```bash
git add web/src/components/ThirdPlaceTable.tsx web/src/index.css web/src/i18n/strings.ts
git commit -m "feat(web): ThirdPlaceTable presentational component + styles"
```

---

## Task 5: Mount on the Schedule page (official only)

**Files:**
- Modify: `web/src/pages/SchedulePage.tsx`

- [ ] **Step 1: Add imports**

At the top of `web/src/pages/SchedulePage.tsx`, add to the existing import groups:

```typescript
import { THIRD_PLACE_QUERY } from '../graphql/queries'
import type { ThirdPlaceRanking } from '../graphql/types'
import { ThirdPlaceTable } from '../components/ThirdPlaceTable'
```

- [ ] **Step 2: Add the query**

In `SchedulePage`, alongside the existing `useQuery` calls (after the `RESULTS_QUERY` one), add:

```typescript
  const [thirdsResult] = useQuery<{ thirdPlaceRanking: ThirdPlaceRanking }>({
    query: THIRD_PLACE_QUERY,
    variables: { player: null },
  })
  const officialThirds = thirdsResult.data?.thirdPlaceRanking ?? null
```

- [ ] **Step 3: Render the table**

In the returned JSX, immediately AFTER the `{view === 'group' ? ... : ...}` block and BEFORE the closing `</section>`, add:

```typescript
      <div className="schedule-group" data-testid="third-place-section">
        <h3>{t('thirdsScheduleTitle')}</h3>
        <ThirdPlaceTable title={t('thirdsOfficial')} ranking={officialThirds} />
      </div>
```

- [ ] **Step 4: Add the two title keys**

In `web/src/i18n/strings.ts`, add to BOTH `en` and `hu`:

```typescript
  thirdsScheduleTitle: 'Best third-placed teams',  // hu: 'Legjobb harmadik helyezettek'
  thirdsOfficial: 'Official ranking',              // hu: 'Hivatalos rangsor'
```

- [ ] **Step 5: Verify**

Run: `cd web && npm run build && npm run lint`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/SchedulePage.tsx web/src/i18n/strings.ts
git commit -m "feat(web): show official best-thirds ranking on the Schedule page"
```

---

## Task 6: Mount on the My Tips page (predicted + official)

**Files:**
- Modify: `web/src/pages/MyTipsPage.tsx`

- [ ] **Step 1: Add imports**

At the top of `web/src/pages/MyTipsPage.tsx`, add:

```typescript
import { THIRD_PLACE_QUERY } from '../graphql/queries'
import type { ThirdPlaceRanking } from '../graphql/types'
import { ThirdPlaceTable } from '../components/ThirdPlaceTable'
```

- [ ] **Step 2: Add the two queries**

In `MyTipsPage`, after the existing `me` is derived (the line `const me = meRaw?.__typename === 'Player' ? meRaw : null`), add:

```typescript
  // Predicted ranking (this player) + official ranking, shown side by side.
  const [myThirdsResult] = useQuery<{ thirdPlaceRanking: ThirdPlaceRanking }>({
    query: THIRD_PLACE_QUERY,
    variables: { player: me?.id ?? null },
    pause: !me,
  })
  const [officialThirdsResult] = useQuery<{ thirdPlaceRanking: ThirdPlaceRanking }>({
    query: THIRD_PLACE_QUERY,
    variables: { player: null },
  })
  const myThirds = myThirdsResult.data?.thirdPlaceRanking ?? null
  const officialThirds = officialThirdsResult.data?.thirdPlaceRanking ?? null
```

- [ ] **Step 3: Render after the groups**

In the returned JSX, after the `{shownGroups.length > 0 ? (...) : (...)}` block and BEFORE the closing `</section>`, add:

```typescript
      <div className="tip-form" data-testid="third-place-section">
        <h3>{t('thirdsTitle')}</h3>
        <p className="hint">{t('thirdsBlurb')}</p>
        <div className="standings-pair">
          <ThirdPlaceTable title={t('thirdsPredicted')} ranking={myThirds} />
          <ThirdPlaceTable title={t('thirdsOfficial')} ranking={officialThirds} />
        </div>
      </div>
```

- [ ] **Step 4: Add the title/blurb keys**

In `web/src/i18n/strings.ts`, add to BOTH `en` and `hu` (`thirdsOfficial` already exists from Task 5 — do not duplicate it):

```typescript
  thirdsTitle: 'Best third-placed teams',          // hu: 'Legjobb harmadik helyezettek'
  thirdsBlurb: 'How your predicted group results rank the 12 third-placed teams — the top 8 advance.', // hu: 'A tippelt csoporteredményeid hogyan rangsorolják a 12 harmadik helyezettet — a legjobb 8 továbbjut.'
  thirdsPredicted: 'Your prediction',              // hu: 'A te tipped'
```

- [ ] **Step 5: Verify**

Run: `cd web && npm run build && npm run lint`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/MyTipsPage.tsx web/src/i18n/strings.ts
git commit -m "feat(web): show predicted + official best-thirds on My Tips"
```

---

## Task 7: i18n completeness check (en + hu parity)

**Files:**
- Modify: `web/src/i18n/strings.ts` (only if a key is missing in one locale)

All thirds keys were added incrementally in Tasks 4–6. This task verifies en/hu parity (every `thirds*` key exists in BOTH objects with a real translation — no English fallback left in `hu`).

- [ ] **Step 1: List the keys per locale and diff**

Run:
```bash
cd web && node -e "const s=require('fs').readFileSync('src/i18n/strings.ts','utf8'); const keys=t=>[...t.matchAll(/^\s+(thirds\w+):/gm)].map(m=>m[1]); console.log('count:', (s.match(/thirds\w+:/g)||[]).length)"
```
Expected: a count that is an even number, and (by inspection) every `thirds*` key appears exactly twice — once under `en`, once under `hu`. If any key appears once, add the missing locale entry.

- [ ] **Step 2: Verify the build (TS enforces `StringKey` exhaustiveness via `keyof typeof en`)**

Run: `cd web && npm run build`
Expected: PASS. Any `t('thirds...')` whose key is absent from `en` is a compile error — a green build proves every key the pages use exists in `en`. Manually confirm the same keys exist in `hu`.

- [ ] **Step 3: Commit (only if changes were needed)**

```bash
git add web/src/i18n/strings.ts
git commit -m "i18n: complete en/hu parity for best-thirds strings"
```

---

## Task 8: End-to-end test (round-trip on both pages)

**Files:**
- Create: `web/e2e/third-place.spec.ts`

This guards the full schema round-trip (a mismatch in the `thirdPlaceRanking` query vs the resolver breaks it) and that both pages render without errors. Deep qualifier assertions depend on seeded full-tournament results; this spec asserts structural presence + a non-erroring round-trip, which is the load-bearing guarantee for frontend work in this repo.

- [ ] **Step 1: Confirm dev-stub auth is enabled**

Verify `web/.env.local` exists and blanks the Auth0 vars (required so the dev-login picker is available to e2e):
```bash
cat web/.env.local 2>/dev/null || echo "MISSING — create it"
```
Expected: the file exists with `VITE_AUTH0_DOMAIN=` / `VITE_AUTH0_CLIENT_ID=` / `VITE_AUTH0_AUDIENCE=` (blank). If missing, create it with those three blank keys.

- [ ] **Step 2: Write the spec**

Create `web/e2e/third-place.spec.ts`:

```typescript
import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Best third-placed teams table — drives the `thirdPlaceRanking` query end to
 * end on both surfaces. A schema mismatch (query vs resolver) surfaces here as
 * a GraphQL error; a render crash surfaces as a page error. The section header
 * must appear on each page regardless of how much of the group stage is
 * decided (an empty ranking still renders the pending hint).
 */

test('Schedule shows the official best-thirds section without errors', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await page.locator('.nav-bar').getByRole('link', { name: 'Schedule' }).click()
  await expect(page).toHaveURL(/\/schedule$/)

  const section = page.getByTestId('third-place-section')
  await expect(section).toBeVisible()
  await expect(section.locator('h3')).toHaveText('Best third-placed teams')
  // Either a table (some thirds decided) or the pending hint — both are valid.
  await expect(
    section.locator('table.third-place-table, p.hint'),
  ).not.toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('My Tips shows predicted + official best-thirds without errors', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)

  const section = page.getByTestId('third-place-section')
  await expect(section).toBeVisible()
  await expect(section.locator('h3')).toHaveText('Best third-placed teams')
  // Two ThirdPlaceTable panels (predicted + official), each a `.third-place`.
  await expect(section.locator('.third-place')).toHaveCount(2)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 3: Run the e2e suite**

Run: `cd web && npm run e2e -- third-place`
Expected: both tests PASS. (`npm run e2e` boots the full live stack via `e2e/global-setup.ts`.) If the nav link label differs (e.g. localized), adjust the `getByRole('link', { name: ... })` selector to match the rendered nav.

- [ ] **Step 4: Visually verify the rendered page**

Per repo practice (green e2e ≠ looks right), look at the actual pages:
```bash
# With the dev stack running (bin/local-dev), open in a browser:
#   http://localhost:5173/schedule   → official best-thirds table at the bottom
#   http://localhost:5173/mytips     → predicted + official side by side
```
Confirm: the top-8 rows are highlighted (green tint + amber left border), eliminated rows are dimmed, qualifier rows show "Winner X", and the columns line up. Fix any CSS that reads wrong (the `.num` right-align, the `.standings-pair` wrap on narrow widths).

- [ ] **Step 5: Commit**

```bash
git add web/e2e/third-place.spec.ts
git commit -m "test(web/e2e): best-thirds round-trip on Schedule + My Tips"
```

---

## Final verification

- [ ] `cargo test --workspace` — all Rust tests green.
- [ ] `cargo clippy --workspace -- -D warnings` — no warnings.
- [ ] `cd web && npm run build && npm run lint` — TS + eslint green.
- [ ] `cd web && npm run e2e -- third-place` — e2e green.
- [ ] Visual check of `/schedule` and `/mytips` (Task 8 Step 4).

## Self-review notes (addressed in this plan)

- **Spec coverage:** backend pure fn (Task 1) ✓; GraphQL `thirdPlaceRanking(player)` with null=official (Task 2) ✓; web query+types (Task 3) ✓; `ThirdPlaceTable` (Task 4) ✓; Schedule official-only (Task 5) ✓; My Tips predicted+official (Task 6) ✓; i18n en/hu (Tasks 4–7) ✓; tests at fwc26/api/e2e layers (Tasks 1, 2, 8) ✓.
- **Type consistency:** `ThirdPlaceRow` (Rust, snake_case) → `ThirdPlaceEntry`/`ThirdPlaceRanking` GraphQL (camelCase via async-graphql) → `ThirdPlaceEntry`/`ThirdPlaceRanking` TS — field names align (`goalDiff`/`facesWinnerGroup`/`facesGame`). The component reads `e.team.name`/`e.team.flag` from the `Team` type already declared in `web/src/graphql/types.ts`.
- **Out of scope (Phase 2):** editable last-resort tiebreak — `.scratch/best-thirds-table/phase-2-editable-tiebreak.md`.
