# Live Match Preview (#2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A per-match page at `/match/:gameId` that shows every player's predictions (the AllTips grid) plus the best-available actual score — official if entered, else the live score during the match (provisional "if it ended now"), else none — with provisional points computed by the existing pure scoring.

**Architecture:** A new `match(gameId): MatchDetail` GraphQL query resolves one "actual" to score against (official → live → none), reusing the existing per-event `ReportedResultSource` seam (no new `sportsdb` code), the `score_match_parts`/`PointsBreakdown` pure scoring, and the `tips` visibility gate (extracted into one shared helper). The web renders a three-state page and polls every 60s only while the match is live. The live score is ephemeral — it is never persisted and the provisional path never calls `recompute()`/`put_scoreboard()`.

**Tech Stack:** Rust (axum + async-graphql), React + Vite + TS, urql, Playwright.

**Spec:** `docs/superpowers/specs/2026-06-20-sportsdb-live-preview-design.md`

**Branch:** all crate/web work on a branch/worktree `sportsdb-live-preview` (per branch discipline — never on `master`).

---

## File Structure

**API (Rust)**
- `crates/api/src/gql/types.rs` — add `MatchScore` + `MatchDetail` SimpleObjects; make `Game::build` `pub(crate)`.
- `crates/api/src/gql/query.rs` — extract `scored_tip` helper from the `tips` resolver; add the `match(gameId)` resolver; add resolver tests.

**Web (TS)**
- `web/src/graphql/queries.ts` — add `MATCH_QUERY`.
- `web/src/graphql/types.ts` — add `MatchScore` + `MatchDetail` TS interfaces.
- `web/src/i18n/strings.ts` — add match-page strings (en + hu).
- `web/src/pages/MatchPage.tsx` — **create**: the three-state page.
- `web/src/App.tsx` — register `/match/:gameId`.
- `web/src/pages/TodayPage.tsx`, `web/src/pages/SchedulePage.tsx` — link match rows to the page.
- `web/e2e/match-page.spec.ts` — **create**: navigation + grid + official-state e2e.

**Shape note (refinement of the spec sketch):** `MatchScore.source` and `MatchScore.sourceStatus` are **nullable** (an official result has neither). The spec §5 sketch marked them non-null; the nullable shape below is authoritative.

---

## Task 1: `MatchScore` + `MatchDetail` GraphQL types

**Files:**
- Modify: `crates/api/src/gql/types.rs` (add types; make `Game::build` `pub(crate)`)

- [ ] **Step 1: Make `Game::build` callable from the query module**

In `crates/api/src/gql/types.rs`, change the `Game::build` signature visibility (currently `fn build(`, around line 104):

```rust
impl Game {
    pub(crate) fn build(
        g: &domain::SingleGame,
        round: domain::Round,
        now: chrono::DateTime<chrono::Utc>,
        entered_result_game_ids: &std::collections::HashSet<String>,
    ) -> Self {
```

- [ ] **Step 2: Add the `MatchScore` and `MatchDetail` types**

Append to `crates/api/src/gql/types.rs` (near the `ReportedResult` type, end of file is fine):

```rust
/// The actual score shown on a match page: either the official entered result
/// (`provisional: false`, no source) or a live SportsDB score during the match
/// (`provisional: true`). Ephemeral — never persisted.
#[derive(SimpleObject, Clone, Debug)]
pub struct MatchScore {
    pub home_score: i32,
    pub away_score: i32,
    /// `true` = live "if it ended now"; `false` = official entered result.
    pub provisional: bool,
    /// `"thesportsdb"` when provisional; `None` for an official result.
    pub source: Option<String>,
    /// SportsDB `strStatus` (e.g. `"2H"`) when provisional; `None` otherwise.
    pub source_status: Option<String>,
    /// `true` for a knockout (extra-time/penalties ambiguity vs the 90' rule).
    pub ninety_minute_uncertain: bool,
}

/// One match's detail (`#2`): the all-players tip grid plus the best-available
/// actual score. Read-only and ephemeral.
#[derive(SimpleObject, Clone, Debug)]
pub struct MatchDetail {
    /// The match itself (reuses the existing `Game` type — teams, kickoff,
    /// time flags). The web resolves team names from the `tournament` query.
    pub game: Game,
    /// `None` until there is a score to show (upcoming, or source absent).
    pub actual: Option<MatchScore>,
    /// Every participating player's tip for this game, gated exactly as `tips`.
    pub rows: Vec<Tip>,
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p api`
Expected: builds clean (warnings about unused `MatchDetail`/`MatchScore` are OK until Task 3 wires them).

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/gql/types.rs
git commit -m "feat(api): MatchDetail + MatchScore GraphQL types for live preview (#2)"
```

---

## Task 2: Extract the `scored_tip` helper from `tips`

A pure refactor: the `tips` resolver's per-`(player, game)` visibility-and-scoring block becomes one function so `tips` and the new `match` resolver cannot drift. Behaviour is unchanged — the existing `tips` tests are the guard.

**Files:**
- Modify: `crates/api/src/gql/query.rs` (add helper; refactor `tips`)

- [ ] **Step 1: Add the helper function**

In `crates/api/src/gql/query.rs`, add this free function (after the `collect_leaf_groups` helper, before `impl QueryRoot`):

```rust
/// Build one `(player, game)` tip row: apply the mutual-commitment visibility
/// gate (legacy `AllTipsHandler`) and, when the prediction is visible and an
/// `actual` exists, score it. `actual` is the result-user's prediction for the
/// official path, or a synthesized live score for the provisional path — the
/// scoring is identical. Shared by `tips` and `match`.
#[allow(clippy::too_many_arguments)]
fn scored_tip(
    viewer_id: &str,
    viewer_prediction: Option<&domain::MatchPrediction>,
    player: &domain::Player,
    game: &domain::SingleGame,
    deadline: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    actual: Option<&domain::MatchPrediction>,
    multiplier: i64,
    config: &ScoringConfig,
) -> Tip {
    let prediction = player.match_prediction(&game.id);
    let is_own = player.id == viewer_id;
    // Match kickoff or group-deadline opens every tip for the game to everyone.
    let time_open = now >= game.kickoff || deadline.is_some_and(|d| now > d);
    // Mutual commitment: another player's tip shows only once the viewer has
    // effective-locked this match; we keep the target's lock so an un-locked
    // draft is never exposed before the deadline.
    let viewer_committed = time_open || viewer_prediction.is_some_and(|p| p.locked);
    let visible =
        is_own || (viewer_committed && prediction.is_some_and(|p| p.locked || time_open));
    let breakdown = match (visible, prediction, actual) {
        (true, Some(pred), Some(res)) => Some(PointsBreakdown::build(
            score_match_parts(pred, res, config),
            multiplier,
            config,
        )),
        _ => None,
    };
    let points = breakdown.as_ref().map(|b| b.points);
    let is_perfect_tip = breakdown
        .as_ref()
        .is_some_and(|b| b.base >= config.perfect_threshold);
    Tip {
        player_id: player.id.clone(),
        nick: player.nick.clone(),
        game_id: game.id.clone(),
        prediction: if visible {
            prediction.map(MatchPrediction::from)
        } else {
            None
        },
        points,
        is_perfect: is_perfect_tip,
        breakdown,
    }
}
```

- [ ] **Step 2: Refactor `tips` to call the helper**

In the `tips` resolver, replace the inner `for game in &games { ... }` body (the block that computes `is_own`/`time_open`/`viewer_committed`/`visible`/`breakdown` and pushes a `Tip`, roughly lines 239–284) with:

```rust
            for game in &games {
                let result = result_user.and_then(|r| r.match_prediction(&game.id));
                tips.push(scored_tip(
                    &viewer.id,
                    viewer.match_prediction(&game.id),
                    player,
                    game,
                    deadline,
                    now,
                    result,
                    config.multiplier(round_of(game)),
                    &config,
                ));
            }
```

Leave everything above the inner loop (`viewer`, `players`, `games`, `deadline`, `now`, `config`, `result_user`, `round_of`, the `tippers_in` outer loop) unchanged.

- [ ] **Step 3: Run the existing tips tests — they must still pass**

Run: `cargo test -p api`
Expected: PASS (the refactor changes no behaviour). If a test fails, the extraction diverged — diff the helper against the original block.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/gql/query.rs
git commit -m "refactor(api): extract scored_tip helper from the tips resolver"
```

---

## Task 3: The `match(gameId)` resolver

**Files:**
- Modify: `crates/api/src/gql/query.rs` (add the resolver + a live-window constant)

- [ ] **Step 1: Add the live-window constant**

Near the top of `crates/api/src/gql/query.rs` (after the `use` lines):

```rust
/// Only consult the live source while a match could plausibly be in progress
/// (covers a knockout's extra time), so SportsDB is never queried for
/// long-finished or far-future games. Also caps live calls to genuinely-live
/// matches.
const LIVE_WINDOW: chrono::Duration = chrono::Duration::hours(3);
```

- [ ] **Step 2: Add the resolver method to `impl QueryRoot`**

Add inside `#[Object] impl QueryRoot` (e.g. after the `reported_results` method):

```rust
    /// One match's detail (`#2`): the all-players tip grid plus the best
    /// available actual score — official if entered, else the live score
    /// during the match (provisional), else none. Read-only and ephemeral:
    /// it never writes, and never calls recompute()/put_scoreboard().
    #[graphql(name = "match")]
    async fn match_detail(
        &self,
        ctx: &Context<'_>,
        game_id: String,
    ) -> async_graphql::Result<Option<MatchDetail>> {
        let viewer = CurrentPlayer::require(ctx)?;
        let repo = repo(ctx);
        let now = now(ctx);
        let Some(tournament) = repo.get_tournament().await? else {
            return Ok(None);
        };
        let Some(game) = tournament.games.get(&game_id) else {
            return Ok(None);
        };
        let round = tournament
            .groups
            .get(&game.group_id)
            .map(|g| g.round)
            .unwrap_or(domain::Round::GroupStage);
        let players = repo.list_players().await?;
        let config = ScoringConfig::default();
        let result_user = players.iter().find(|p| p.is_result_user);

        // entered-result game ids → for Game::build's resultPending flag.
        let entered: std::collections::HashSet<String> = result_user
            .map(|r| {
                r.match_predictions
                    .iter()
                    .map(|p| p.game_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Resolve the one actual to score against: official → live → none.
        let official = result_user.and_then(|r| r.match_prediction(&game_id));
        let (actual_pred, actual_score): (Option<domain::MatchPrediction>, Option<MatchScore>) =
            if let Some(off) = official {
                (
                    Some(off.clone()),
                    Some(MatchScore {
                        home_score: off.home_score as i32,
                        away_score: off.away_score as i32,
                        provisional: false,
                        source: None,
                        source_status: None,
                        ninety_minute_uncertain: false,
                    }),
                )
            } else if now >= game.kickoff
                && now <= game.kickoff + LIVE_WINDOW
                && game.external_id.is_some()
            {
                // Live window, no official result yet → consult the source.
                // Any error/absence degrades to "no score" (page still works).
                let ext = game.external_id.clone().unwrap();
                let source =
                    ctx.data_unchecked::<Arc<dyn crate::reported::ReportedResultSource>>();
                let events = source.lookup_events(&[ext]).await.unwrap_or_default();
                let live = events.into_iter().find_map(|e| {
                    match (e.int_home_score, e.int_away_score) {
                        (Some(h), Some(a))
                            if (0..=255).contains(&h) && (0..=255).contains(&a) =>
                        {
                            Some((h as u8, a as u8, e.str_status))
                        }
                        _ => None,
                    }
                });
                match live {
                    Some((h, a, status)) => (
                        Some(domain::MatchPrediction {
                            game_id: game_id.clone(),
                            home_score: h,
                            away_score: a,
                            locked: true,
                        }),
                        Some(MatchScore {
                            home_score: h as i32,
                            away_score: a as i32,
                            provisional: true,
                            source: Some("thesportsdb".to_string()),
                            source_status: Some(status),
                            ninety_minute_uncertain: round != domain::Round::GroupStage,
                        }),
                    ),
                    None => (None, None),
                }
            } else {
                (None, None)
            };

        // The all-players grid — same gate as `tips`.
        let deadline = tournament.deadline(&game.group_id);
        let multiplier = config.multiplier(round);
        let viewer_pred = viewer.match_prediction(&game_id);
        let game_ids = [game_id.clone()];
        let rows: Vec<Tip> = domain::participation::tippers_in(&players, &game_ids)
            .into_iter()
            .map(|player| {
                scored_tip(
                    &viewer.id,
                    viewer_pred,
                    player,
                    game,
                    deadline,
                    now,
                    actual_pred.as_ref(),
                    multiplier,
                    &config,
                )
            })
            .collect();

        Ok(Some(MatchDetail {
            game: Game::build(game, round, now, &entered),
            actual: actual_score,
            rows,
        }))
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p api`
Expected: builds clean (no more unused-type warnings for `MatchDetail`/`MatchScore`).

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/gql/query.rs
git commit -m "feat(api): match(gameId) resolver — official/live/none score + tip grid (#2)"
```

---

## Task 4: `match` resolver tests

Mirror the existing `reported_tests` module (same `StubSource`, `InMemoryRepository`, clock seam). Cover: official-priority, live-provisional, visibility gating, knockout flag, and graceful empty.

**Files:**
- Modify: `crates/api/src/gql/query.rs` (add a `match_tests` module at end of file)

- [ ] **Step 1: Write the failing tests**

Append to `crates/api/src/gql/query.rs`:

```rust
#[cfg(test)]
mod match_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use chrono::{DateTime, TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame, Team,
        TeamSlot, Tournament,
    };
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct StubSource(Vec<Event>);
    #[async_trait]
    impl ReportedResultSource for StubSource {
        async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(self
                .0
                .iter()
                .filter(|e| ids.contains(&e.id_event))
                .cloned()
                .collect())
        }
    }

    fn live_event(id_event: &str, h: i64, a: i64, status: &str) -> Event {
        Event {
            id_event: id_event.into(),
            date_event: "2026-06-11".into(),
            id_home_team: "AAA".into(),
            id_away_team: "BBB".into(),
            int_home_score: Some(h),
            int_away_score: Some(a),
            str_status: status.into(),
            str_timestamp: None,
        }
    }

    fn team(id: &str) -> Team {
        Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        }
    }

    /// A full ordinary player with one prediction for `M1` (mirrors the field
    /// set used by `reported_tests` — there is no `Player::new`).
    fn player(id: &str, h: u8, a: u8, locked: bool) -> Player {
        Player {
            id: id.into(),
            person_id: format!("p-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: vec![MatchPrediction {
                game_id: "M1".into(),
                home_score: h,
                away_score: a,
                locked,
            }],
            standings_predictions: vec![],
        }
    }

    /// One group-stage game `M1` (idEvent `E1`) kicking off at `kickoff`.
    async fn repo_with_m1(kickoff: DateTime<Utc>) -> InMemoryRepository {
        let game = SingleGame {
            id: "M1".into(),
            kickoff,
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("AAA".into()), description: "A1".into() },
            away: TeamSlot { team_id: Some("BBB".into()), description: "A2".into() },
            external_id: Some("E1".into()),
        };
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into()]),
        };
        let t = Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), game)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let repo = InMemoryRepository::new();
        repo.put_tournament(&t).await.unwrap();
        repo
    }

    fn kickoff() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 11, 18, 0, 0).unwrap()
    }

    /// Execute `query` as `viewer` at `now`, returning the JSON `data`. Mirrors
    /// the `reported_tests` pattern exactly: `build_schema` + a `Request` with
    /// the `CurrentPlayer` and `RequestNow` injected as context data.
    async fn exec(
        repo: InMemoryRepository,
        source: Arc<dyn ReportedResultSource>,
        viewer: Player,
        now: DateTime<Utc>,
        query: &str,
    ) -> serde_json::Value {
        let repo: Arc<dyn Repository> = Arc::new(repo);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(CurrentPlayer::Player(Box::new(viewer)))
            .data(crate::clock::RequestNow(now));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        resp.data.into_json().unwrap()
    }

    #[tokio::test]
    async fn live_score_yields_provisional_points() {
        let repo = repo_with_m1(kickoff()).await;
        // viewer predicted 1–0; live score is 1–0 → provisional, scored.
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![live_event("E1", 1, 0, "2H")]));
        let now = kickoff() + chrono::Duration::minutes(67); // in-play
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ homeScore awayScore provisional sourceStatus ninetyMinuteUncertain } rows{ playerId points } } }"#,
        )
        .await;
        let m = &data["match"];
        assert_eq!(m["actual"]["provisional"], true);
        assert_eq!(m["actual"]["sourceStatus"], "2H");
        assert_eq!(m["actual"]["ninetyMinuteUncertain"], false);
        // alice's 1–0 vs live 1–0 scores > 0.
        let row = m["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["playerId"] == "alice")
            .unwrap();
        assert!(row["points"].as_i64().unwrap() > 0);
    }

    #[tokio::test]
    async fn official_result_takes_priority_over_live() {
        let repo = repo_with_m1(kickoff()).await;
        // result user entered 2–2 (official); the stub says 1–0 but must be ignored.
        let mut ru = player("result-user", 2, 2, true);
        ru.is_result_user = true;
        repo.put_player(&ru).await.unwrap();
        let alice = player("alice", 2, 2, true);
        repo.put_player(&alice).await.unwrap();
        let source: Arc<dyn ReportedResultSource> =
            Arc::new(StubSource(vec![live_event("E1", 1, 0, "2H")]));
        let now = kickoff() + chrono::Duration::minutes(67);
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ homeScore awayScore provisional source } } }"#,
        )
        .await;
        let a = &data["match"]["actual"];
        assert_eq!(a["homeScore"], 2);
        assert_eq!(a["provisional"], false);
        assert_eq!(a["source"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn no_score_before_kickoff_and_others_hidden() {
        let repo = repo_with_m1(kickoff()).await;
        let alice = player("alice", 1, 0, true);
        repo.put_player(&alice).await.unwrap();
        repo.put_player(&player("bob", 3, 1, true)).await.unwrap();
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![]));
        let now = kickoff() - chrono::Duration::hours(2); // before kickoff
        let data = exec(
            repo,
            source,
            alice,
            now,
            r#"{ match(gameId:"M1"){ actual{ homeScore } rows{ playerId prediction{ homeScore } } } }"#,
        )
        .await;
        assert_eq!(data["match"]["actual"], serde_json::Value::Null);
        // bob's prediction is hidden from alice before kickoff.
        let rows = data["match"]["rows"].as_array().unwrap();
        let bob = rows.iter().find(|r| r["playerId"] == "bob").unwrap();
        assert!(bob["prediction"].is_null());
    }
}
```

- [ ] **Step 2: Confirm the injection helpers exist, then run the tests**

The `exec` helper above uses `crate::gql::build_schema(repo, source)`, `CurrentPlayer::Player(Box::new(..))`, and `crate::clock::RequestNow(now)` — the exact constructs `reported_tests` already uses (see `reported_tests::maps_finished_event_to_pending_game_for_result_user`). If any name differs, match `reported_tests` verbatim. These tests run *after* Task 3's implementation, so they should pass immediately (characterization), not fail-first.

Run: `cargo test -p api match_tests`
Expected: PASS (3 tests). If they fail to compile, reconcile the helper names against `reported_tests`.

- [ ] **Step 3: Full workspace gate**

Run: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`
Expected: all green. (`cargo fmt` to fix formatting if `--check` complains.)

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/gql/query.rs
git commit -m "test(api): match(gameId) — provisional/official/visibility coverage (#2)"
```

---

## Task 5: Web GraphQL query + types

**Files:**
- Modify: `web/src/graphql/queries.ts`
- Modify: `web/src/graphql/types.ts`

- [ ] **Step 1: Add `MATCH_QUERY`**

Append to `web/src/graphql/queries.ts`:

```ts
export const MATCH_QUERY = `
  query Match($gameId: ID!) {
    match(gameId: $gameId) {
      game {
        id kickoff venue groupId
        home { teamId description }
        away { teamId description }
        resultPending withinTodayWindow isToday
      }
      actual {
        homeScore awayScore provisional source sourceStatus ninetyMinuteUncertain
      }
      rows {
        playerId nick gameId
        prediction { gameId homeScore awayScore locked }
        points isPerfect
        breakdown { exactHome exactAway outcome base multiplier points }
      }
    }
  }
`
```

- [ ] **Step 2: Add the TS interfaces**

Append to `web/src/graphql/types.ts` (after the `Tip` interface):

```ts
export interface MatchScore {
  homeScore: number
  awayScore: number
  /** true = live "if it ended now"; false = official entered result. */
  provisional: boolean
  /** "thesportsdb" when provisional; null for an official result. */
  source: string | null
  /** SportsDB status (e.g. "2H") when provisional; null otherwise. */
  sourceStatus: string | null
  ninetyMinuteUncertain: boolean
}

export interface MatchDetail {
  game: SingleGame
  /** Null until there is a score to show (upcoming, or source absent). */
  actual: MatchScore | null
  rows: Tip[]
}
```

- [ ] **Step 3: Typecheck**

Run: `cd web && npm run build`
Expected: `tsc -b` passes (no usage yet, just type validity).

- [ ] **Step 4: Commit**

```bash
git add web/src/graphql/queries.ts web/src/graphql/types.ts
git commit -m "feat(web): MATCH_QUERY + MatchDetail/MatchScore types (#2)"
```

---

## Task 6: i18n strings

**Files:**
- Modify: `web/src/i18n/strings.ts` (add keys to BOTH the `en` and `hu` blocks)

- [ ] **Step 1: Add keys to the `en` block**

Add inside `const en = { ... }`:

```ts
  // match page (#2 live preview)
  matchPageTitle: 'Match',
  provisionalLabel: 'Provisional — if it ended now',
  liveLabel: 'Live',
  awaitingResult: 'Awaiting official result',
  ninetyMinuteNote:
    'Knockout — provisional points use the 90-minute rule; extra time may change the official result.',
```

- [ ] **Step 2: Add the SAME keys to the `hu` block**

Add inside the `hu` block (Hungarian wording, matching the casual register):

```ts
  // match page (#2 live preview)
  matchPageTitle: 'Meccs',
  provisionalLabel: 'Ideiglenes — ha most érne véget',
  liveLabel: 'Élő',
  awaitingResult: 'Hivatalos eredményre várunk',
  ninetyMinuteNote:
    'Kieséses szakasz — az ideiglenes pontok a 90 perces szabály szerint számolnak; a hosszabbítás módosíthatja a hivatalos eredményt.',
```

- [ ] **Step 3: Typecheck (the `hu` block must cover every `en` key)**

Run: `cd web && npm run build`
Expected: passes. If `hu` is missing a key the `StringKey`/record type errors — add it.

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): i18n strings for the match page (#2)"
```

---

## Task 7: `MatchPage` component

**Files:**
- Create: `web/src/pages/MatchPage.tsx`

- [ ] **Step 1: Create the page**

Create `web/src/pages/MatchPage.tsx`:

```tsx
import { useEffect, useMemo } from 'react'
import { useParams } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { MATCH_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { MatchDetail, Tournament } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { Matchup } from '../components/TeamLabel'
import { PointsBadge } from '../components/PointsBadge'
import { teamIndex, formatKickoff } from '../lib/format'

/**
 * Match page (#2). The all-players tip grid is the spine in every state; the
 * live/official score and provisional points are an overlay on top. Polls
 * every 60s only while the match is live (`actual.provisional`).
 */
export function MatchPage() {
  const { gameId = '' } = useParams()
  const { t, locale } = useI18n()
  const { label } = useAuth()

  const [tournamentResult] = useQuery<{ tournament: Tournament | null }>({
    query: TOURNAMENT_QUERY,
  })
  const [matchResult, reexecuteMatch] = useQuery<{ match: MatchDetail | null }>({
    query: MATCH_QUERY,
    variables: { gameId },
    pause: !gameId,
  })

  const tournament = tournamentResult.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament?.teams, locale],
  )
  const match = matchResult.data?.match ?? null
  const isLive = match?.actual?.provisional ?? false

  // Poll only while live. 60s matches the server cache floor — polling faster
  // would only re-read the cache, never hit SportsDB more often.
  useEffect(() => {
    if (!isLive) return
    const id = setInterval(
      () => reexecuteMatch({ requestPolicy: 'network-only' }),
      60_000,
    )
    return () => clearInterval(id)
  }, [isLive, reexecuteMatch])

  if (!label) return <NeedsLogin />
  if (matchResult.fetching || tournamentResult.fetching) return <Loading />
  if (matchResult.error) return <ErrorView error={matchResult.error} />
  if (!match) return <ErrorView error={new Error('match not found')} />

  const { game, actual, rows } = match

  return (
    <section className="match-page">
      <header className="match-head">
        <h1>
          <Matchup home={game.home} away={game.away} teams={teams} />
        </h1>
        <p className="kickoff">{formatKickoff(game.kickoff, locale)}</p>
        {actual ? (
          <p className={`score ${actual.provisional ? 'score-live' : 'score-final'}`}>
            <span className="score-value">
              {actual.homeScore}–{actual.awayScore}
            </span>
            {actual.provisional && (
              <span className="score-status">
                {t('liveLabel')}
                {actual.sourceStatus ? ` · ${actual.sourceStatus}` : ''}
              </span>
            )}
            {actual.provisional && <span className="provisional-note">{t('provisionalLabel')}</span>}
          </p>
        ) : (
          game.resultPending && <p className="awaiting">{t('awaitingResult')}</p>
        )}
        {actual?.ninetyMinuteUncertain && actual.provisional && (
          <p className="ninety-note">{t('ninetyMinuteNote')}</p>
        )}
      </header>

      <table className="tips-grid">
        <tbody>
          {rows.map((row) => (
            <tr key={row.playerId}>
              <td className="nick">{row.nick}</td>
              <td className="pred">
                {row.prediction
                  ? `${row.prediction.homeScore}–${row.prediction.awayScore}`
                  : '—'}
              </td>
              <td className="pts">
                <PointsBadge breakdown={row.breakdown} points={row.points} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
```

- [ ] **Step 2: Confirm imports resolve**

Check that `formatKickoff` and `teamIndex` are exported from `web/src/lib/format.ts` (both are used by `SchedulePage`/`AllTipsPage`), and `NeedsLogin`/`Loading`/`ErrorView` from `web/src/components/StatusViews.tsx`. If `useAuth().label` is not the right "logged in" signal, mirror exactly what `AllTipsPage` uses to gate on login.

Run: `cd web && npm run build`
Expected: `tsc -b` passes.

- [ ] **Step 3: Lint**

Run: `cd web && npm run lint`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/MatchPage.tsx
git commit -m "feat(web): MatchPage — three-state per-match view with live poll (#2)"
```

---

## Task 8: Route registration

**Files:**
- Modify: `web/src/App.tsx`

- [ ] **Step 1: Import and register the route**

In `web/src/App.tsx`, add the import alongside the other page imports:

```tsx
import { MatchPage } from './pages/MatchPage'
```

Add the route inside the `<Route element={<Layout />}>` block (next to `games`):

```tsx
        <Route path="match/:gameId" element={<MatchPage />} />
```

- [ ] **Step 2: Typecheck**

Run: `cd web && npm run build`
Expected: passes.

- [ ] **Step 3: Commit**

```bash
git add web/src/App.tsx
git commit -m "feat(web): register /match/:gameId route (#2)"
```

---

## Task 9: Link match rows from Today + Schedule

**Files:**
- Modify: `web/src/pages/SchedulePage.tsx`
- Modify: `web/src/pages/TodayPage.tsx`

- [ ] **Step 1: SchedulePage — wrap the matchup in a Link**

In `web/src/pages/SchedulePage.tsx`, add the import:

```tsx
import { Link } from 'react-router-dom'
```

Replace the matchup cell (around line 84):

```tsx
                      <td>
                        <Link to={`/match/${m.id}`}>
                          <Matchup home={m.home} away={m.away} teams={teams} />
                        </Link>
                      </td>
```

- [ ] **Step 2: TodayPage — wrap the matchup in a Link**

In `web/src/pages/TodayPage.tsx`, add `import { Link } from 'react-router-dom'` (if not already imported) and wrap the `<Matchup ... />` at ~line 118:

```tsx
                    <Link to={`/match/${m.id}`}>
                      <Matchup home={m.home} away={m.away} teams={teams} />
                    </Link>
```

(Confirm the local game variable name in TodayPage — it may be `m` or another binding; use whatever the surrounding `.map` uses and its `.id`.)

- [ ] **Step 3: Typecheck + lint**

Run: `cd web && npm run build && npm run lint`
Expected: passes.

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/SchedulePage.tsx web/src/pages/TodayPage.tsx
git commit -m "feat(web): link Today/Schedule match rows to /match/:gameId (#2)"
```

---

## Task 10: E2E — navigation + grid + official state

The live/provisional path needs a SportsDB source stub the e2e stack does not have (the `sportsdb` client has a hard-coded base URL; `THESPORTSDB_API_KEY` is unset in e2e ⇒ `NullSource`). So e2e covers the page render, the all-players grid, and navigation; the provisional-scoring logic is covered by Task 4's resolver tests. This is a deliberate, logged boundary.

**Files:**
- Create: `web/e2e/match-page.spec.ts`

- [ ] **Step 1: Inspect an existing spec for the harness conventions**

Read `web/e2e/mytips.spec.ts` and `web/e2e/helpers.ts` to copy the exact login/seed/clock helpers (dev-login, `X-Dev-Now`, navigation). Match their patterns — do not invent new ones.

- [ ] **Step 2: Write the spec**

Create `web/e2e/match-page.spec.ts` (adapt the helper calls to whatever `helpers.ts` actually exports — names below are illustrative and MUST be reconciled with Step 1):

```ts
import { test, expect } from '@playwright/test'
import { loginAs, gotoApp } from './helpers'

test.describe('match page (#2)', () => {
  test('navigates from the schedule and shows the all-players grid', async ({ page }) => {
    await loginAs(page, 'demo-ada')
    await gotoApp(page, '/games')

    // Click the first match row's matchup link → lands on /match/:id.
    const firstMatch = page.locator('table .col-match a, td a').first()
    await firstMatch.click()
    await expect(page).toHaveURL(/\/match\//)

    // The grid renders at least the logged-in player's own row.
    await expect(page.locator('.tips-grid tr')).not.toHaveCount(0)
  })

  test('shows the official score once a result is entered', async ({ page }) => {
    // Reuse the suite's mechanism for entering an official result (result-user
    // submitGroup, as mytips/admin specs do), then open that game's match page
    // and assert the final score + points render (provisional badge absent).
    // Fill in using the helpers confirmed in Step 1.
  })
})
```

- [ ] **Step 3: Run the e2e suite**

Run: `cd web && npm run e2e -- match-page`
Expected: the navigation test passes. (Per `web/.env.local` dev-stub auth — see the project's e2e auth note; ensure `VITE_AUTH0_*` are blanked so the dev auth bar shows.)

- [ ] **Step 4: Flesh out and finalize the official-score test**

Replace the placeholder body in the second test with the concrete steps confirmed from the admin/mytips specs, run again until green.

Run: `cd web && npm run e2e -- match-page`
Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add web/e2e/match-page.spec.ts
git commit -m "test(web): e2e — match page navigation + official score (#2)"
```

---

## Final verification

- [ ] **Backend:** `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`
- [ ] **Frontend:** `cd web && npm run build && npm run lint && npm run e2e`
- [ ] **Manual smoke (optional):** boot the stack (`docker compose up -d`, import, seed, `cargo run -p api`, `cd web && npm run dev`), set `X-Dev-Now` to mid-match via the dev clock, open a match row → grid + (with a configured key) live score render; without a key the page degrades to predictions-only.
- [ ] **PRD status:** flip `.scratch/sportsdb-live-preview/PRD.md` `Status:` from `ready-for-agent` to done with the merge commit.

---

## Self-Review

**Spec coverage:**
- §2.1 per-event source, no new `sportsdb` code → Task 3 reuses `ReportedResultSource::lookup_events`; no `sportsdb` crate change anywhere. ✓
- §2.2 full match-detail page, all states → Task 7 (upcoming/live/finished). ✓
- §2.3 `match(gameId)` owns the merge → Task 3. ✓
- §2.4 throttle = 60s cache floor; client polls 60s while live → Task 7 `setInterval(…, 60_000)` gated on `isLive`; server cache unchanged (already 45s, ≤60s effective). ✓
- §4 official→live→none priority + 3h live-window guard + ephemeral invariant (never writes) → Task 3 (`LIVE_WINDOW`, no repo writes). ✓
- §4 shared visibility helper → Task 2. ✓
- §5 MatchDetail/MatchScore shape (source/sourceStatus nullable refinement) → Tasks 1 & 5. ✓
- §6 knockout `ninetyMinuteUncertain` → Task 3 (`round != GroupStage`) + Task 7 note. ✓
- §7 grid spine + entry points (Today/Schedule) → Tasks 7 & 9. ✓
- §8 testing (resolver stub + e2e) → Tasks 4 & 10, with the live-path e2e gap logged. ✓
- §9 out of scope (no live scoreboard, no `sportsdb` method) → honored. ✓

**Placeholder scan:** Task 4 Step 2 and Task 10 require reconciling against the real `test_schema`/e2e helpers (their exact signatures can't be known without running) — each is an explicit "inspect then match" instruction, not a hidden TODO. All code steps carry full code.

**Type consistency:** `scored_tip` signature is identical in Tasks 2 and 3. `MatchScore`/`MatchDetail` field names match across Rust (Task 1) and TS (Task 5) under async-graphql camelCasing (`home_score`→`homeScore`, etc.). `match(gameId)` field name set once via `#[graphql(name = "match")]` (Task 3) and queried as `match(gameId:)` (Tasks 5, 10).
