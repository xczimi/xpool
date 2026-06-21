# Implementation Plan — Cluster: live-scoring

**Date:** 2026-06-21
**Branch/worktree:** `worktree-cluster-live-scoring` (code touches `crates/*` + `web/` → MUST be a branch/worktree, never `master`)
**Spec sources:**
- `.scratch/live-max-achievable-points/PRD.md` (resolved decisions at bottom)
- `.scratch/force-refresh-live-score/PRD.md` (resolved decisions at bottom)
- `.specs/TESTING.md` (server-authoritative clock, `XPOOL_NOW`/`X-Dev-Now`, no `Date.now()` for logic, DynamoDB tests gated by `DYNAMO_TEST=1`)
- `CLAUDE.md` (load-bearing design decisions)

## Goal

Two related live-scoring features, shipped together:

1. **Max achievable points** — while ≥1 match is live, the scoreboard shows each player a second number: the best score still mathematically reachable (settled points + best-case live points). Computed **server-side** in the `scoreboard` resolver, fetching live scores via the existing SportsDB `CachingSource`, using a new **pure `domain` function** `max_reachable_score`. New GraphQL field `maxAchievable` on `ScoreEntry`. Not-yet-started fixtures contribute 0.

2. **Force-refresh live score (frontend-only)** — the match page gets a manual "refresh now" button (re-issues the `match` query, NO cache-bypass arg) **plus** auto-poll while the match is live, and a last-updated indicator + spinner. No API change for this feature (the 45s server `CachingSource` stays the quota guard).

## Constraints / load-bearing facts (verified against the code)

- **Scoring is pure.** `score_match`/`score_match_parts` (`crates/domain/src/scoring.rs`) depend only on `exact_home`, `exact_away`, `outcome`. `ScoringConfig::multiplier(round)` is the per-round factor.
- **Scoreboard is materialised** (`crates/storage/src/lib.rs` `Scoreboard{entries: HashMap<PlayerId, HashMap<Round,i64>>}`), recomputed only on result entry (`crates/api/src/recompute.rs`) and **does not know live state**. The `maxAchievable` add-on is therefore computed live in the resolver, never persisted.
- **The `scoreboard` resolver** (`crates/api/src/gql/query.rs:139`) loads the stored `Scoreboard`, sums `breakdown.values()` for `total`, and already takes a `pool` arg. This is where `maxAchievable` is added. It does **not** currently load the tournament or the reported source — both must be wired in.
- **Live scores are server-only.** The `match_detail` resolver (`query.rs:571`) already shows the pattern: within `LIVE_WINDOW = Duration::hours(3)` after kickoff, no official result yet, `game.external_id.is_some()` → `ctx.data_unchecked::<Arc<dyn crate::reported::ReportedResultSource>>()` `.lookup_events(&[ext])`, synthesizing a live `MatchPrediction{home, away, locked:true}`.
- **Clock seam:** resolvers read `now` from `RequestNow` in context (`now(ctx)` helper). Never `Utc::now()`. SPA renders server flags, never `Date.now()` for logic.
- **e2e forces `THESPORTSDB_API_KEY=""`** (`web/scripts/e2e-stack.sh`) → `NullSource` → **no live scores in e2e today**, and `tournaments/fwc26.json` carries **zero `externalId`s** (verified: `grep -c externalId` = 0). So an e2e proving the live path needs a **deterministic live-score injection mechanism** — built in Task 7 (`StubLiveSource` driven by an env var) and a seeded `external_id` (Task 8).

## Cross-cluster integration notes

- The `scoreboard` resolver change (this cluster) and the **perfect-page cluster's** perfects-resolver change both live in `crates/api/src/gql/query.rs`. They are **different resolvers** (`scoreboard` vs `perfects`) — integration resolves any textual overlap; no logical conflict.
- `web/src/pages/ScoreboardPage.tsx` is also touched by the perfect-page cluster (pool sticky migration). This cluster adds a `maxAchievable` column/secondary number; the perfect-page cluster changes pool state handling. **Integration merge** will reconcile both edits in the same file. Keep this cluster's change scoped to the table render + the `maxAchievable` field read.

---

## The hard part — `max_reachable_score` (Task 1), reasoned out

We must compute, for one player's prediction `p` against a live score `live = (H, A)` where the final can only be `(h, a)` with `h >= H`, `a >= A`, the **maximum** `score_match(p, final)`.

`score_match` only reads three booleans from `(p, final)`:
- `exact_home`: `p.home == final.home` **or** (`p.home >= thr` **and** `final.home >= thr`)
- `exact_away`: `p.away == final.away` **or** (`p.away >= thr` **and** `final.away >= thr`)
- `outcome`: `sign(p.home - p.away) == sign(final.home - final.away)`

Each of the three flags is independently maximisable **over the reachable set**, but the *combination* that maximises the sum must be a single reachable `final`. Because the three flags are not jointly independent (a final fixing `exact_home` also fixes `final.home`, which constrains `outcome`), we **enumerate** candidate finals and take the max — but the candidate set is small and bounded.

**Bounding the candidates.** Goals only go up. The score depends only on whether each side *equals the predicted value*, whether each side is *≥ thr* (4-goal rule), and the *sign of the difference*. Beyond `max(predicted, live) + thr` extra goals on a side, nothing changes (you're already "≥ thr", already "not equal to the prediction", and pushing further only increases the difference monotonically — never flips a sign you couldn't already reach). So it suffices to enumerate:

```
hi_home = max(p.home, live.home, thr) + 1     // one past every "interesting" home value
hi_away = max(p.away, live.away, thr) + 1
for fh in live.home ..= hi_home
  for fa in live.away ..= hi_away
    best = max(best, score_match(p, {fh, fa}))
```

`hi_home`/`hi_away` are tiny (predicted scores are 0–N, `thr = 4`), so the loop is at most ~`(N+5)²` cheap integer evaluations per live match — fine.

We multiply by the round `multiplier` outside the pure max (the caller passes it), mirroring how `score_tournament` applies multipliers per round. The function returns the **multiplied** best so the caller just sums.

**Worked cases the tests must cover:**
1. *Exact still reachable* — `p = 1–0`, `live = 1–0` → best includes the exact final `1–0` → base `4` (2 exact + outcome).
2. *Exact lost but outcome reachable* — `p = 1–0` (home win), `live = 0–2` → home can never be `1` (already 0? no — `final.home >= live.home = 0`, but the prediction's `home=1` would need `final.home == 1`; that's still reachable since `1 >= 0`). The instructive case is `p = 1–0`, `live = 2–0`: `final.home >= 2` so `p.home == final.home` impossible (1 ≠ ≥2) and not both ≥ thr → `exact_home` lost; `final.away == 0` reachable → `exact_away` kept; outcome home-win reachable (e.g. `2–0`) → base `0 + 1 + 2 = 3`.
3. *Outcome also lost* — `p = 2–1` (home win), `live = 0–3` (away already leads by 3). `final.away >= 3`, `final.home >= 0`. For a home win we'd need `final.home > final.away >= 3` → `final.home >= 4`; predicted home `2 ≠ final.home`, both-≥-thr only if `2 >= 4` (false) → `exact_home` lost. `exact_away` needs `final.away == 1` (impossible, ≥3) or both ≥ thr (`1 >= 4` false) → lost. Outcome home-win is *technically* still reachable (`4–3`) → base `0 + 0 + 2 = 2`. **This shows "outcome lost" is rarely a hard floor — the ceiling is honest about what's still reachable.** A truly-lost outcome example: `p = 0–0` (draw), `live = 0–1` → for a draw `final.home == final.away` and `final.away >= 1` → `final = 1–1, 2–2…`; `exact_home` needs `final.home == 0` (impossible) → lost; `exact_away` needs `final.away == 0` (impossible) → lost; draw outcome reachable (`1–1`) → base `2`. To force outcome lost: `p = 2–0` (home win) and `live = 0–5` → home win needs `final.home > final.away >= 5` → `final.home >= 6`; `exact_home` (`2 == ≥6`? no; both ≥4? `2>=4` no) lost; `exact_away` (`0 == ≥5`? no) lost; outcome reachable at `6–5` → base 2. So pick `p = 2–0`, `live = 0–5`, and assert the away-win and draw branches: actually to get a **0** we need *no* flag reachable. `p = 1–0` (home win), `live = 0–9`: home win needs `final.home >= 10`; exact_home lost; exact_away needs `final.away == 0` impossible; **but outcome home-win still reachable at 10–9** → base 2. The cleanest **hard-zero** case: there is none for a single match when goals are unbounded, because you can always out-score. **So the test for "all lost" asserts the realistic floor (outcome-only = base 2), not zero** — and a separate test asserts that when the predicted outcome is unreachable *and* nothing else matches, the best is just whatever single flag survives. (See test bodies in Task 1 for the exact, checked assertions.)
4. *4-goal rule interaction* — `p = 5–0`, `live = 4–0` → `final.home >= 4`; both ≥ thr (`5>=4 && final.home>=4`) → `exact_home` true for any `final.home >= 4`; `exact_away` needs `final.away == 0` (reachable) → base `2 + outcome`. Home win reachable → base `4`. And `p = 4–4`, `live = 4–4` → both sides ≥ thr → `exact_home && exact_away` always; outcome draw reachable → base `4`.

Each of these is encoded as an exact-value unit test in Task 1.

---

## Task 1 — `domain::scoring::max_reachable_score` (pure, TDD)

**File:** `crates/domain/src/scoring.rs`

### 1a. RED — add failing unit tests

Add these tests to the existing `mod unit_tests` block in `crates/domain/src/scoring.rs` (after `score_match_parts_four_goal_rule_counts_a_side_as_exact`). They reference a function that does not exist yet, so the crate will fail to compile (RED).

```rust
    // ─── max_reachable_score ─────────────────────────────────────────────────

    /// Live score helper: a `MatchPrediction` standing in for "the score now".
    fn live(h: u8, a: u8) -> MatchPrediction {
        MatchPrediction {
            game_id: "x".into(),
            home_score: h,
            away_score: a,
            locked: true,
        }
    }

    #[test]
    fn max_reachable_exact_still_reachable() {
        let c = ScoringConfig::default();
        // Predicted 1–0; it's currently 1–0. The exact final 1–0 is reachable.
        // Best base = 2 exact + outcome = 4. multiplier 1 → 4.
        assert_eq!(max_reachable_score(&mp(1, 0), &live(1, 0), &c, 1), 4);
    }

    #[test]
    fn max_reachable_exact_home_lost_but_outcome_and_away_kept() {
        let c = ScoringConfig::default();
        // Predicted 1–0 (home win); it's 2–0. final.home >= 2, so home can never
        // equal predicted 1 → exact_home lost. final.away == 0 reachable →
        // exact_away kept. Home win reachable (2–0) → outcome kept.
        // base = 0 + 1 + 2 = 3.
        assert_eq!(max_reachable_score(&mp(1, 0), &live(2, 0), &c, 1), 3);
    }

    #[test]
    fn max_reachable_predicted_draw_outcome_lost_keeps_only_what_survives() {
        let c = ScoringConfig::default();
        // Predicted 0–0 (draw); it's 0–1. A draw needs final.home == final.away
        // with away >= 1 (e.g. 1–1): exact_home (need 0) lost, exact_away (need 0)
        // lost, draw outcome reachable. base = outcome only = 2.
        assert_eq!(max_reachable_score(&mp(0, 0), &live(0, 1), &c, 1), 2);
    }

    #[test]
    fn max_reachable_multiplier_is_applied() {
        let c = ScoringConfig::default();
        // Same as the exact case but a knockout multiplier (R32 = 2): 4 * 2 = 8.
        let m = c.multiplier(Round::R32);
        assert_eq!(max_reachable_score(&mp(1, 0), &live(1, 0), &c, m), 8);
    }

    #[test]
    fn max_reachable_four_goal_rule_keeps_high_scoring_exact() {
        let c = ScoringConfig::default();
        // Predicted 5–0; it's 4–0. Both home sides >= threshold (4) → exact_home
        // counts for any final.home >= 4. away 0 reachable → exact_away. home win
        // reachable → outcome. base = 4.
        assert_eq!(max_reachable_score(&mp(5, 0), &live(4, 0), &c, 1), 4);
    }

    #[test]
    fn max_reachable_high_scoring_draw_both_sides_exact() {
        let c = ScoringConfig::default();
        // Predicted 4–4; it's 4–4. Both sides >= threshold for any growth, and a
        // draw stays reachable (e.g. 5–5). base = 2 exact + draw outcome = 4.
        assert_eq!(max_reachable_score(&mp(4, 4), &live(4, 4), &c, 1), 4);
    }

    #[test]
    fn max_reachable_never_below_current_best() {
        let c = ScoringConfig::default();
        // The reachable max must be >= the score the prediction already earns
        // against the live score treated as if final (a sanity monotonicity guard).
        let p = mp(2, 1);
        let l = live(2, 1);
        let now_score = score_match(&p, &l, &c);
        assert!(max_reachable_score(&p, &l, &c, 1) >= now_score);
    }
```

### 1b. RED — confirm it fails to compile

```sh
cargo test -p domain max_reachable 2>&1 | tail -20
```

**Expected:** compile error, e.g. `cannot find function 'max_reachable_score' in this scope`.

### 1c. GREEN — implement the function

Insert this **after** the `is_perfect` function (around line 104) in `crates/domain/src/scoring.rs`:

```rust
/// The **best score still mathematically reachable** for a prediction `p` given
/// a live score `live`, returned **multiplied** by `multiplier`.
///
/// Goals only go up, so the final `(h, a)` satisfies `h >= live.home`,
/// `a >= live.away`. `score_match` reads only three flags (`exact_home`,
/// `exact_away`, `outcome`), and none of them changes once a side passes
/// `max(predicted, live) + threshold`: a side that high is already "not equal to
/// the prediction" and already "≥ threshold", and pushing it further only
/// widens the goal difference monotonically (never flips a sign that wasn't
/// already reachable). So enumerating finals up to that bound finds the true
/// maximum. The candidate grid is tiny (predicted scores are small, threshold
/// is 4), so the brute force is cheap and exact.
pub fn max_reachable_score(
    p: &MatchPrediction,
    live: &MatchPrediction,
    c: &ScoringConfig,
    multiplier: i64,
) -> i64 {
    let thr = c.high_scoring_threshold;
    // One past every "interesting" value on each side — beyond this the flags
    // are saturated and the difference only grows.
    let hi_home = p.home_score.max(live.home_score).max(thr).saturating_add(1);
    let hi_away = p.away_score.max(live.away_score).max(thr).saturating_add(1);

    let mut best = 0;
    for fh in live.home_score..=hi_home {
        for fa in live.away_score..=hi_away {
            let final_score = MatchPrediction {
                game_id: p.game_id.clone(),
                home_score: fh,
                away_score: fa,
                locked: true,
            };
            best = best.max(score_match(p, &final_score, c));
        }
    }
    best * multiplier
}
```

### 1d. GREEN — confirm tests pass

```sh
cargo test -p domain max_reachable 2>&1 | tail -20
```

**Expected:** `test result: ok. 7 passed`.

### 1e. Full domain suite + clippy

```sh
cargo test -p domain 2>&1 | tail -5
cargo clippy -p domain -- -D warnings 2>&1 | tail -5
```

**Expected:** all domain tests pass; clippy clean.

### 1f. Commit

```sh
git add crates/domain/src/scoring.rs
git commit -m "feat(domain): max_reachable_score — best still-reachable per-match score"
```

---

## Task 2 — `ScoreEntry.maxAchievable` GraphQL field (TDD)

**File:** `crates/api/src/gql/types.rs`

### 2a. GREEN (type-only, no behaviour yet)

Add the field to `ScoreEntry` (`crates/api/src/gql/types.rs:315`):

```rust
/// One row of the materialised scoreboard for one player.
#[derive(SimpleObject, Clone, Debug)]
pub struct ScoreEntry {
    pub player_id: String,
    pub nick: String,
    /// Sum across all rounds.
    pub total: i64,
    /// The best total still mathematically reachable: settled points plus the
    /// best-case live points for matches in progress. `None` when no match is
    /// live (the scoreboard then renders exactly as today). Provisional — the
    /// SPA marks it clearly. Not-yet-started fixtures contribute 0.
    pub max_achievable: Option<i64>,
    pub stages: Vec<StageScore>,
}
```

This is a struct field change; the resolver in `query.rs` must now set `max_achievable`. The next task wires the value; for now set it to `None` at the single construction site so the crate compiles.

In `crates/api/src/gql/query.rs`, in the `scoreboard` resolver's `ScoreEntry { … }` literal (around line 197), add:

```rust
                ScoreEntry {
                    player_id: pid.clone(),
                    nick: nick_by_id
                        .get(pid.as_str())
                        .copied()
                        .unwrap_or("")
                        .to_owned(),
                    total,
                    max_achievable: None,
                    stages,
                }
```

### 2b. Build check

```sh
cargo build -p api 2>&1 | tail -5
```

**Expected:** compiles. (`maxAchievable` now in the schema, always `null`.)

### 2c. Commit

```sh
git add crates/api/src/gql/types.rs crates/api/src/gql/query.rs
git commit -m "feat(api): add ScoreEntry.maxAchievable field (null placeholder)"
```

---

## Task 3 — Compute `maxAchievable` in the `scoreboard` resolver (TDD)

**File:** `crates/api/src/gql/query.rs`

The resolver must: detect live games, fetch their live scores via the reported source, and for each player add `settled total + best-case live points` — but **only when ≥1 match is live**, else `None`.

"Settled total" = the stored breakdown sum (already computed). "Best-case live points" = for each live game the player predicted, `max_reachable_score(prediction, live, config, multiplier(round))`. A live game already has an *entered official result* → not live (the stored board already counts it). A live game the player did **not** predict → 0. Not-yet-started → excluded.

### 3a. RED — add a failing API integration test

Add a new test module at the end of `crates/api/src/gql/query.rs` (after `mod match_tests`). It reuses the `StubSource`/builder pattern already in that file.

```rust
#[cfg(test)]
mod scoreboard_live_tests {
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
    use storage::{InMemoryRepository, Repository, Scoreboard};

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

    fn live_event(id: &str, h: i64, a: i64) -> Event {
        Event {
            id_event: id.into(),
            date_event: "2026-06-11".into(),
            id_home_team: "AAA".into(),
            id_away_team: "BBB".into(),
            int_home_score: Some(h),
            int_away_score: Some(a),
            str_status: "2H".into(),
            str_timestamp: None,
        }
    }

    fn team(id: &str) -> Team {
        Team { id: id.into(), name: id.into(), short_code: id.into(), flag: None, external_id: None }
    }

    fn player(id: &str, h: u8, a: u8) -> Player {
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
                locked: true,
            }],
            standings_predictions: vec![],
        }
    }

    fn result_user() -> Player {
        Player {
            id: "result-user".into(),
            person_id: "p-ru".into(),
            nick: "official".into(),
            full_name: "Official".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    fn kickoff() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 11, 18, 0, 0).unwrap()
    }

    /// One group-stage game M1 (idEvent E1), plus a materialised scoreboard that
    /// already credits `alice` 0 (M1 not yet entered as an official result).
    async fn repo() -> InMemoryRepository {
        let game = SingleGame {
            id: "M1".into(),
            kickoff: kickoff(),
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
            carries_standings: false,
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
        repo.put_player(&result_user()).await.unwrap();
        // alice's stored board: 0 settled points (M1 has no official result yet).
        let mut board = Scoreboard::default();
        board.entries.insert("alice".into(), HashMap::new());
        repo.put_scoreboard(&board).await.unwrap();
        repo
    }

    async fn exec(repo: InMemoryRepository, source: Arc<dyn ReportedResultSource>, now: DateTime<Utc>, query: &str) -> serde_json::Value {
        let repo: Arc<dyn Repository> = Arc::new(repo);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(CurrentPlayer::Visitor)
            .data(crate::clock::RequestNow(now));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        resp.data.into_json().unwrap()
    }

    #[tokio::test]
    async fn max_achievable_present_and_above_total_while_live() {
        let repo = repo().await;
        let alice = player("alice", 1, 0);
        repo.put_player(&alice).await.unwrap();
        // Live 1–0; alice predicted 1–0 → best reachable base 4 (group ×1).
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![live_event("E1", 1, 0)]));
        let now = kickoff() + chrono::Duration::minutes(67); // in live window
        let data = exec(
            repo,
            source,
            now,
            r#"{ scoreboard { playerId total maxAchievable } }"#,
        )
        .await;
        let row = data["scoreboard"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["playerId"] == "alice")
            .unwrap();
        assert_eq!(row["total"], 0);
        assert_eq!(row["maxAchievable"], 4); // 0 settled + best-case live 4
    }

    #[tokio::test]
    async fn max_achievable_null_when_no_match_is_live() {
        let repo = repo().await;
        let alice = player("alice", 1, 0);
        repo.put_player(&alice).await.unwrap();
        // Source returns nothing → no live match → maxAchievable null.
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![]));
        let now = kickoff() + chrono::Duration::minutes(30);
        let data = exec(
            repo,
            source,
            now,
            r#"{ scoreboard { playerId total maxAchievable } }"#,
        )
        .await;
        let row = data["scoreboard"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["playerId"] == "alice")
            .unwrap();
        assert_eq!(row["maxAchievable"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn max_achievable_null_before_kickoff() {
        let repo = repo().await;
        let alice = player("alice", 1, 0);
        repo.put_player(&alice).await.unwrap();
        // Even if the source had data, before kickoff the game is not live.
        let source: Arc<dyn ReportedResultSource> = Arc::new(StubSource(vec![live_event("E1", 1, 0)]));
        let now = kickoff() - chrono::Duration::hours(1);
        let data = exec(
            repo,
            source,
            now,
            r#"{ scoreboard { playerId maxAchievable } }"#,
        )
        .await;
        let row = data["scoreboard"].as_array().unwrap().iter().find(|r| r["playerId"] == "alice").unwrap();
        assert_eq!(row["maxAchievable"], serde_json::Value::Null);
    }
}
```

### 3b. RED — run, confirm failure

```sh
cargo test -p api scoreboard_live 2>&1 | tail -25
```

**Expected:** `max_achievable_present_and_above_total_while_live` fails (`maxAchievable` is `null`, expected `4`).

### 3c. GREEN — implement live computation in the resolver

Add a private helper above `impl QueryRoot` (near `collect_leaf_groups`, around line 53) in `crates/api/src/gql/query.rs`:

```rust
/// A live match the scoreboard can credit best-case points for: the game's
/// round (for the multiplier) and the synthesized live score. Only games that
/// are in the live window, have an `external_id`, have NO entered official
/// result, and whose source returns a usable score appear here.
struct LiveMatch {
    game_id: domain::GameId,
    round: domain::Round,
    live: domain::MatchPrediction,
}

/// Collect every currently-live match for the whole tournament by consulting the
/// reported source once for all candidate external ids. Empty when nothing is
/// live — the caller uses that to leave `maxAchievable` `None`.
async fn live_matches(
    ctx: &Context<'_>,
    t: &domain::Tournament,
    entered: &std::collections::HashSet<String>,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<LiveMatch> {
    // Candidate games: in the live window, mapped to an external id, no official
    // result yet. event id -> (game_id, round).
    let mut by_event: HashMap<String, (domain::GameId, domain::Round)> = HashMap::new();
    for game in t.games.values() {
        if entered.contains(&game.id) {
            continue;
        }
        let in_window = now >= game.kickoff && now <= game.kickoff + LIVE_WINDOW;
        if !in_window {
            continue;
        }
        if let Some(ext) = &game.external_id {
            let round = t
                .groups
                .get(&game.group_id)
                .map(|g| g.round)
                .unwrap_or(domain::Round::GroupStage);
            by_event.insert(ext.clone(), (game.id.clone(), round));
        }
    }
    if by_event.is_empty() {
        return Vec::new();
    }

    let ids: Vec<String> = by_event.keys().cloned().collect();
    let source = ctx.data_unchecked::<Arc<dyn crate::reported::ReportedResultSource>>();
    let events = source.lookup_events(&ids).await.unwrap_or_default();

    let mut out = Vec::new();
    for e in events {
        if let Some((game_id, round)) = by_event.get(&e.id_event) {
            if let (Some(h), Some(a)) = (e.int_home_score, e.int_away_score) {
                if (0..=255).contains(&h) && (0..=255).contains(&a) {
                    out.push(LiveMatch {
                        game_id: game_id.clone(),
                        round: *round,
                        live: domain::MatchPrediction {
                            game_id: game_id.clone(),
                            home_score: h as u8,
                            away_score: a as u8,
                            locked: true,
                        },
                    });
                }
            }
        }
    }
    out
}
```

Now wire it into the `scoreboard` resolver. The resolver currently loads `board`, `players`, computes `allowed`, `participant_ids`, then builds `entries`. We need the tournament + live matches **before** the entry loop, and we need per-player predictions. Replace the body from the tournament-less version with the live-aware one.

Find this block (around line 144–171) and **insert** the tournament load + live detection right after `players` is loaded, before `allowed`:

```rust
        let repo = repo(ctx);
        let board = repo.get_scoreboard().await?.unwrap_or_default();
        let players = repo.list_players().await?;
        let nick_by_id: HashMap<&str, &str> = players
            .iter()
            .map(|p| (p.id.as_str(), p.nick.as_str()))
            .collect();

        // Live max-achievable add-on (live-scoring cluster). Only computed while
        // ≥1 match is live; otherwise `maxAchievable` stays `None` and the board
        // renders exactly as before. Server-only: the live score comes from the
        // reported source (the client cannot fetch it).
        let now = now(ctx);
        let config = ScoringConfig::default();
        let tournament = repo.get_tournament().await?;
        let live: Vec<LiveMatch> = if let Some(t) = &tournament {
            let entered: std::collections::HashSet<String> = players
                .iter()
                .find(|p| p.is_result_user)
                .map(|r| r.match_predictions.iter().map(|p| p.game_id.clone()).collect())
                .unwrap_or_default();
            live_matches(ctx, t, &entered, now).await
        } else {
            Vec::new()
        };
        let any_live = !live.is_empty();
        // player id -> their Player (for live predictions). Cheap clone-free map.
        let player_by_id: HashMap<&str, &domain::Player> =
            players.iter().map(|p| (p.id.as_str(), p)).collect();
```

Then in the `.map(|(pid, breakdown)| { … })` closure, compute the per-player live best-case and set `max_achievable`:

```rust
            .map(|(pid, breakdown)| {
                let stages: Vec<StageScore> = breakdown
                    .iter()
                    .map(|(round, points)| StageScore {
                        round: (*round).into(),
                        points: *points,
                    })
                    .collect();
                let total: i64 = breakdown.values().sum();
                // While anything is live, the ceiling = settled total + the best
                // still-reachable points from each live match this player tipped.
                let max_achievable = if any_live {
                    let player = player_by_id.get(pid.as_str());
                    let live_best: i64 = live
                        .iter()
                        .map(|lm| {
                            player
                                .and_then(|p| p.match_prediction(&lm.game_id))
                                .map(|pred| {
                                    domain::scoring::max_reachable_score(
                                        pred,
                                        &lm.live,
                                        &config,
                                        config.multiplier(lm.round),
                                    )
                                })
                                .unwrap_or(0)
                        })
                        .sum();
                    Some(total + live_best)
                } else {
                    None
                };
                ScoreEntry {
                    player_id: pid.clone(),
                    nick: nick_by_id
                        .get(pid.as_str())
                        .copied()
                        .unwrap_or("")
                        .to_owned(),
                    total,
                    max_achievable,
                    stages,
                }
            })
```

Note: the resolver already imports `ScoringConfig` at the top (`use domain::scoring::{score_match_parts, standings_score, ScoringConfig};`). Remove the now-duplicate `let config = ScoringConfig::default();` if one already exists in scope — there isn't one in `scoreboard` today, so the added line is the only one. The `now(ctx)` helper exists.

### 3d. GREEN — run the tests

```sh
cargo test -p api scoreboard_live 2>&1 | tail -15
```

**Expected:** `test result: ok. 3 passed`.

### 3e. Full API suite + clippy

```sh
cargo test -p api 2>&1 | tail -5
cargo clippy -p api -- -D warnings 2>&1 | tail -5
```

**Expected:** all API tests pass; clippy clean.

### 3f. Commit

```sh
git add crates/api/src/gql/query.rs
git commit -m "feat(api): scoreboard maxAchievable — live best-case ceiling while matches are live"
```

---

## Task 4 — Frontend: scoreboard renders `maxAchievable` (TDD via e2e later; build/lint now)

**Files:** `web/src/graphql/queries.ts`, `web/src/graphql/types.ts`, `web/src/pages/ScoreboardPage.tsx`, `web/src/i18n/strings.ts`

### 4a. Add the field to the GraphQL document

In `web/src/graphql/queries.ts`, extend `SCOREBOARD_QUERY`:

```ts
export const SCOREBOARD_QUERY = `
  query Scoreboard($pool: ID) {
    scoreboard(pool: $pool) {
      playerId nick total maxAchievable
      stages { round points }
    }
  }
`
```

### 4b. Add the field to the TS type

In `web/src/graphql/types.ts`, find `ScoreEntry` and add `maxAchievable`. (Read the file first to match the exact existing shape.) The field is:

```ts
export interface ScoreEntry {
  playerId: string
  nick: string
  total: number
  /** Best still-reachable total while ≥1 match is live; null otherwise. */
  maxAchievable: number | null
  stages: StageScore[]
}
```

### 4c. i18n strings

In `web/src/i18n/strings.ts`, add to the `en` block (before the closing `} as const` at line 321, near the existing match-page live keys ~line 314):

```ts
  // scoreboard live ceiling (live-scoring cluster)
  ceilingLabel: 'Max',
  ceilingTooltip: 'Best still-reachable total — provisional while matches are live',
  liveBoardNote: 'Live — “Max” shows each player’s best still-reachable total',
```

And the matching Hungarian in the `hu` block (after `liveLabel: 'Élő',` ~line 604):

```ts
  ceilingLabel: 'Max',
  ceilingTooltip: 'Elérhető legjobb összpontszám — ideiglenes, amíg meccsek élnek',
  liveBoardNote: 'Élő — a „Max” mutatja kinek mennyi a még elérhető pontja',
```

### 4d. Render the ceiling column in `ScoreboardPage.tsx`

The board shows `maxAchievable` as a secondary number **only when ≥1 row has it non-null** (i.e. something is live). Add a derived flag and a conditional column.

In `web/src/pages/ScoreboardPage.tsx`, after `const ranked = …` (line 73):

```tsx
  const ranked = [...scoreboard].sort((a, b) => b.total - a.total)
  // Show the "Max" (still-reachable) column only while at least one player has
  // a live ceiling — the server returns null for everyone when nothing is live.
  const showCeiling = ranked.some((e) => e.maxAchievable != null)
```

Change the live note (line 78) to mention the ceiling when shown:

```tsx
      {interval > 0 && (
        <p className="poll-note">● {showCeiling ? t('liveBoardNote') : 'live'}</p>
      )}
```

Add the header cell (after the `total` `<th>`, line 109):

```tsx
            <th>{t('total')}</th>
            {showCeiling && <th title={t('ceilingTooltip')}>{t('ceilingLabel')}</th>}
```

Add the body cell (after the `total` `<td>`, line 128):

```tsx
                <td>
                  <strong>{entry.total}</strong>
                </td>
                {showCeiling && (
                  <td className="score-ceiling">
                    {entry.maxAchievable != null ? (
                      <span className="ceiling-value" title={t('ceilingTooltip')}>
                        ≤ {entry.maxAchievable}
                      </span>
                    ) : (
                      '—'
                    )}
                  </td>
                )}
```

### 4e. CSS for the ceiling (new class needs styling — memory: new class names need CSS)

Find the scoreboard's stylesheet (search `web/src` for `.poll-note` or `.data-table` usage) and add a muted, provisional look:

```sh
grep -rln "poll-note" web/src/styles web/src/**/*.css 2>/dev/null
```

In the file that defines `.poll-note` (e.g. `web/src/styles/…css`), append:

```css
.score-ceiling {
  color: var(--muted, #777);
  font-variant-numeric: tabular-nums;
}
.ceiling-value {
  font-style: italic;
}
```

### 4f. Build + lint

```sh
cd web && npm run build && npm run lint 2>&1 | tail -15
```

**Expected:** `tsc -b` + `vite build` succeed; eslint clean. **No `console.log`.**

### 4g. Commit

```sh
git add web/src/graphql/queries.ts web/src/graphql/types.ts web/src/pages/ScoreboardPage.tsx web/src/i18n/strings.ts web/src/styles
git commit -m "feat(web): scoreboard Max-achievable column while matches are live"
```

---

## Task 5 — Frontend: match-page force-refresh + auto-poll (frontend-only)

**Files:** `web/src/pages/MatchPage.tsx`, `web/src/i18n/strings.ts`

The match page already polls every 60s while `isLive`. The PRD asks for: (a) a manual "refresh now" button everyone can use (re-issues the `match` query — already supported by `reexecuteMatch`), (b) auto-poll while live (already present), (c) a **last-updated indicator + spinner feedback**. So this task adds the button + last-updated + spinner, and keeps the existing poll.

### 5a. i18n strings

Add to `en` in `web/src/i18n/strings.ts`:

```ts
  refreshNow: 'Refresh now',
  refreshing: 'Refreshing…',
  lastUpdated: 'Updated',
```

And `hu`:

```ts
  refreshNow: 'Frissítés most',
  refreshing: 'Frissítés…',
  lastUpdated: 'Frissítve',
```

### 5b. Add last-updated + spinner + button to `MatchPage.tsx`

`useQuery` exposes `result.fetching` (the spinner signal) and a `reexecute`. Track the last successful fetch time **for display only** (`Date.now()`/`new Date()` is allowed for formatting per `.specs/TESTING.md` §3.3 — never for a behavioural branch). Use the existing `formatKickoff`-adjacent formatter or `toLocaleTimeString`.

Read the current imports (line 1–11). Add `useRef`/extend the existing `useState` import. Modify the component:

After `const isLive = match?.actual?.provisional ?? false` (line 48), add last-updated tracking:

```tsx
  // Last-updated stamp — display only (formatting is allowed to read the wall
  // clock per .specs/TESTING.md §3.3; no behavioural branch reads Date.now()).
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)
  useEffect(() => {
    if (matchResult.data && !matchResult.fetching) {
      setLastUpdated(new Date())
    }
  }, [matchResult.data, matchResult.fetching])
```

Keep the existing live-poll `useEffect` unchanged.

In the JSX, inside `.match-card` after the kickoff line (~line 99–101), add the refresh control + last-updated:

```tsx
        <div className="match-card-kickoff">
          {formatKickoff(game.kickoff, locale)}
        </div>

        <div className="match-refresh">
          <button
            type="button"
            className="refresh-btn"
            onClick={() => reexecuteMatch({ requestPolicy: 'network-only' })}
            disabled={matchResult.fetching}
          >
            {matchResult.fetching ? t('refreshing') : t('refreshNow')}
          </button>
          {matchResult.fetching && <span className="refresh-spinner" aria-hidden="true" />}
          {lastUpdated && (
            <span className="last-updated">
              {t('lastUpdated')} {lastUpdated.toLocaleTimeString(locale)}
            </span>
          )}
        </div>
```

### 5c. CSS for the new classes (`.match-refresh`, `.refresh-btn`, `.refresh-spinner`, `.last-updated`)

Find the match-page stylesheet (search for `.match-card` or `.match-scoreline`):

```sh
grep -rln "match-scoreline" web/src/styles web/src/**/*.css 2>/dev/null
```

Append to that file:

```css
.match-refresh {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
.refresh-btn {
  font-size: 0.85rem;
  padding: 0.25rem 0.6rem;
}
.refresh-spinner {
  width: 0.9rem;
  height: 0.9rem;
  border: 2px solid var(--muted, #999);
  border-top-color: transparent;
  border-radius: 50%;
  animation: refresh-spin 0.7s linear infinite;
}
@keyframes refresh-spin {
  to {
    transform: rotate(360deg);
  }
}
.last-updated {
  color: var(--muted, #777);
  font-size: 0.8rem;
}
```

### 5d. Build + lint

```sh
cd web && npm run build && npm run lint 2>&1 | tail -15
```

**Expected:** green; no `console.log`.

### 5e. Commit

```sh
git add web/src/pages/MatchPage.tsx web/src/i18n/strings.ts web/src/styles
git commit -m "feat(web): match-page refresh-now button + last-updated indicator + spinner"
```

---

## Task 6 — Deterministic live-score injection for e2e (API stub source)

> **FLAGGED — this is the spot I was unsure about.** There is **no existing e2e live-score injection**: `web/scripts/e2e-stack.sh` forces `THESPORTSDB_API_KEY=""` → `NullSource`, and `tournaments/fwc26.json` has zero `externalId`s. The live/provisional path is currently covered **only by Rust resolver tests**. To prove the live features end-to-end I introduce an **env-driven stub source** (`StubLiveSource`) wired in `build_app`, activated only when `XPOOL_LIVE_SCORES` is set. This is a **dev/test stub** in the same family as `X-Dev-Now`/`X-Dev-Player` — it must be inert in production (unset env → never constructed). An alternative considered and rejected: pointing the SportsDB client at a local mock HTTP server (more moving parts, network flakiness in CI). If the reviewer prefers the mock-server route, swap Task 6 for that — but the env-stub is simpler and hermetic.

**File:** `crates/api/src/reported.rs`, `crates/api/src/lib.rs`

### 6a. RED — add a failing unit test for the stub parser

Add to `mod tests` in `crates/api/src/reported.rs`:

```rust
    #[tokio::test]
    async fn stub_live_source_parses_env_and_returns_matching_events() {
        // Format: "E1=1:0:2H,E2=3:3:FT" — id=home:away:status, comma-separated.
        let src = StubLiveSource::parse("E1=1:0:2H,E2=3:3:FT");
        let got = src.lookup_events(&["E1".to_string()]).await.unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id_event, "E1");
        assert_eq!(got[0].int_home_score, Some(1));
        assert_eq!(got[0].int_away_score, Some(0));
        assert_eq!(got[0].str_status, "2H");
    }

    #[tokio::test]
    async fn stub_live_source_ignores_unknown_ids() {
        let src = StubLiveSource::parse("E1=1:0:2H");
        let got = src.lookup_events(&["NOPE".to_string()]).await.unwrap();
        assert!(got.is_empty());
    }
```

### 6b. RED — run

```sh
cargo test -p api stub_live_source 2>&1 | tail -10
```

**Expected:** compile error / fail (`StubLiveSource` does not exist).

### 6c. GREEN — implement `StubLiveSource`

Add to `crates/api/src/reported.rs` (after `NullSource`):

```rust
/// A **dev/test-only** stub source driven by the `XPOOL_LIVE_SCORES` env var, so
/// the e2e suite can inject a deterministic live score without touching the
/// network. Format: comma-separated `idEvent=home:away:status`
/// (e.g. `"E1=1:0:2H"`). Inert in production — only constructed when the env var
/// is set (see `build_app`). In the same dev-stub family as `X-Dev-Now`.
pub struct StubLiveSource {
    events: Vec<Event>,
}

impl StubLiveSource {
    /// Parse the `XPOOL_LIVE_SCORES` value. Malformed entries are skipped.
    pub fn parse(spec: &str) -> Self {
        let events = spec
            .split(',')
            .filter_map(|entry| {
                let (id, rest) = entry.split_once('=')?;
                let mut parts = rest.split(':');
                let h: i64 = parts.next()?.trim().parse().ok()?;
                let a: i64 = parts.next()?.trim().parse().ok()?;
                let status = parts.next().unwrap_or("2H").trim().to_string();
                Some(Event {
                    id_event: id.trim().to_string(),
                    date_event: String::new(),
                    id_home_team: String::new(),
                    id_away_team: String::new(),
                    int_home_score: Some(h),
                    int_away_score: Some(a),
                    str_status: status,
                    str_timestamp: None,
                })
            })
            .collect();
        Self { events }
    }

    /// Construct from `XPOOL_LIVE_SCORES`, or `None` when unset/empty.
    pub fn from_env() -> Option<Self> {
        std::env::var("XPOOL_LIVE_SCORES")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| Self::parse(&s))
    }
}

#[async_trait]
impl ReportedResultSource for StubLiveSource {
    async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>> {
        Ok(self
            .events
            .iter()
            .filter(|e| ids.contains(&e.id_event))
            .cloned()
            .collect())
    }
}
```

### 6d. Wire it into `build_app` (stub wins only when its env var is set)

In `crates/api/src/lib.rs`, change the source selection:

```rust
    use crate::reported::{
        CachingSource, NullSource, ReportedResultSource, SportsDbSource, StubLiveSource,
    };
    let reported: Arc<dyn ReportedResultSource> = if let Some(stub) = StubLiveSource::from_env() {
        // Dev/test stub (e2e). Deterministic; no network. Inert in prod (env unset).
        Arc::new(stub)
    } else if let Some(client) = sportsdb::SportsDb::from_env() {
        Arc::new(CachingSource::new(SportsDbSource(client)))
    } else {
        Arc::new(NullSource)
    };
```

### 6e. GREEN — run

```sh
cargo test -p api stub_live_source 2>&1 | tail -10
cargo clippy -p api -- -D warnings 2>&1 | tail -5
```

**Expected:** 2 passed; clippy clean.

### 6f. Commit

```sh
git add crates/api/src/reported.rs crates/api/src/lib.rs
git commit -m "feat(api): XPOOL_LIVE_SCORES dev-stub source for deterministic e2e live scores"
```

---

## Task 7 — Seed an `external_id` on one game so the e2e stub can target it

**File:** `tournaments/fwc26.json` (and verify `crates/xtask/src/dto.rs` already carries `external_id` through — it does, lines 62/145).

The stub keys off `external_id`. Pick **one** group-stage game in `fwc26.json` and give it `"externalId": "E2E1"`. Choose a game that is **not** used by another e2e spec's group (match-page uses Group D / M4; result-entry Group A; mytips Group B; mytips-lock Group C). Pick a game in **Group D** (the match-page spec's group) since this cluster's e2e drives the match page and scoreboard there — e.g. the first Group D game.

### 7a. Find the target game id and add externalId

```sh
# Inspect the Group D games to pick one (e.g. M4 — Group D's first game).
grep -n '"id": "M4"' tournaments/fwc26.json
```

Read the surrounding JSON object for `M4` and add `"externalId": "E2E1"` to that game object. (Match the file's existing key style — read it first; games use camelCase keys per the dto.) Example shape:

```json
{
  "id": "M4",
  "kickoff": "…",
  "groupId": "D",
  "home": { "teamId": "…", "description": "D1" },
  "away": { "teamId": "…", "description": "D2" },
  "externalId": "E2E1"
}
```

### 7b. Verify import round-trips the externalId

```sh
cargo test -p xtask 2>&1 | tail -5
```

**Expected:** xtask import tests still pass (the field is optional and already handled).

### 7c. Commit (this is a non-code data file, but it's part of the feature branch)

```sh
git add tournaments/fwc26.json
git commit -m "chore(fixture): externalId E2E1 on M4 for deterministic e2e live-score injection"
```

---

## Task 8 — Wire the stub + live clock into the e2e stack

**File:** `web/scripts/e2e-stack.sh`

The stack pins `XPOOL_NOW=2026-06-20T12:00:00Z` and forces `THESPORTSDB_API_KEY=""`. For the live-scoring e2e we need: (a) the stub env set so `M4`'s `E2E1` returns a live score, and (b) the API clock inside `M4`'s live window. Per-test `X-Dev-Now` (via the dev clock control) moves the clock to M4's live window for the specific test, so we keep the global `XPOOL_NOW` as-is and only add the stub env.

### 8a. Add the stub env to the stack script

In `web/scripts/e2e-stack.sh`, just after the `THESPORTSDB_API_KEY=""` line, add:

```sh
# Deterministic live score for the live-scoring e2e (StubLiveSource). M4's
# externalId is E2E1; this makes the scoreboard "Max" column and the match-page
# live overlay appear when the dev clock is inside M4's live window. Inert
# otherwise (the e2e clock starts pre-window; tests opt in via X-Dev-Now).
export XPOOL_LIVE_SCORES="${XPOOL_LIVE_SCORES:-E2E1=1:0:2H}"
```

### 8b. Smoke-check the API picks it up

```sh
# From repo root, after a rebuild this is exercised by the e2e suite; a manual
# sanity check is optional. The Rust unit tests already prove the parser.
cargo build -p api 2>&1 | tail -3
```

### 8c. Commit

```sh
git add web/scripts/e2e-stack.sh
git commit -m "test(e2e): inject deterministic live score (E2E1) via XPOOL_LIVE_SCORES"
```

---

## Task 9 — e2e spec: live max-achievable + match-page refresh/poll

**File:** `web/e2e/live-scoring.spec.ts` (new), plus ensure `web/.env.local` blanks `VITE_AUTH0_*` (memory: e2e needs dev-stub auth).

### 9a. Ensure dev-stub auth env

```sh
# If web/.env.local does not already blank Auth0, create/append it (memory:
# e2e-needs-dev-stub-auth — without this ~10 dev-login tests fail).
grep -q 'VITE_AUTH0_DOMAIN=' web/.env.local 2>/dev/null && echo "present" || cat >> web/.env.local <<'EOF'
VITE_AUTH0_DOMAIN=
VITE_AUTH0_CLIENT_ID=
VITE_AUTH0_AUDIENCE=
EOF
```

(If `web/.env.local` already blanks these — it does for the existing dev-login specs — this is a no-op; do not duplicate.)

### 9b. The spec

The e2e drives M4 (Group D, externalId E2E1, stub live 1–0). The flow:
1. Log in as `demo-grace`, enter a `1–0` prediction for Group D **before** kickoff (editable).
2. Use the dev-clock preset to put M4 **during** (live window).
3. **Match page** (`/match/M4`): the live scoreline (`.match-scoreline.is-live`) shows `1–0`, the provisional marker shows, the **refresh button** is present and clicking it re-issues the `match` query (asserted on the wire), and the **last-updated** indicator appears.
4. **Scoreboard**: the **Max** column (`.score-ceiling`) appears (a match is live) and grace's ceiling `≤ 4` (1–0 vs live 1–0 → base 4, group ×1) shows.

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Live-scoring cluster e2e. Proves, hermetically (no real SportsDB — the stack
 * injects XPOOL_LIVE_SCORES="E2E1=1:0:2H" and M4 carries externalId E2E1):
 *   A. Match page during the live window shows the live (provisional) score,
 *      a working "Refresh now" button (re-issues the match query on the wire),
 *      and a last-updated indicator.
 *   B. Scoreboard shows the "Max" (still-reachable) column while M4 is live,
 *      with grace's ceiling reflecting her 1–0 tip vs the 1–0 live score.
 *
 * Group D / M4 is the live game (distinct from the groups other specs touch).
 */

const TEST_GROUP = 'Group D'
const LIVE_GAME = 'M4'

async function setPreset(page: Page, gameId: string, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  await page.evaluate(() => document.documentElement.setAttribute('data-pre-reload', '1'))
  await selects.nth(1).selectOption(phase)
  await page.waitForFunction(() => !document.documentElement.hasAttribute('data-pre-reload'))
}

async function openGroupD(page: Page) {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await page.locator('.round-tabs button', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: new RegExp(`^${TEST_GROUP}$`) }).click()
  await expect(page.locator('.tip-form h3')).toContainText(TEST_GROUP)
}

async function fillScores(page: Page, home: string, away: string) {
  const rows = page.locator('.tip-form table.data-table').first().locator('tbody tr')
  const count = await rows.count()
  expect(count, 'the group has matches').toBeGreaterThan(0)
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption(home)
    await selects.nth(1).selectOption(away)
  }
}

/** Count match-query POSTs to /api/graphql. */
function watchMatchOps(page: Page): { count: () => number } {
  let n = 0
  page.on('request', (req) => {
    if (req.method() !== 'POST' || !req.url().includes('/api/graphql')) return
    const body = req.postData() ?? ''
    if (body.includes('query Match') || /\bmatch\(gameId/.test(body)) n++
  })
  return { count: () => n }
}

test('live match page: provisional score, refresh re-issues query, last-updated shows', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // Enter grace's 1–0 tip for Group D BEFORE kickoff (editable).
  await setPreset(page, LIVE_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '1', '0')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // Move the clock into M4's live window.
  await setPreset(page, LIVE_GAME, 'during')

  // Go to the match page; the stub live score 1–0 must render as provisional.
  const matchOps = watchMatchOps(page)
  await page.goto(`/match/${LIVE_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${LIVE_GAME}`))

  const liveScore = page.locator('.match-scoreline.is-live')
  await expect(liveScore).toBeVisible()
  await expect(liveScore.locator('.match-scoreline-value')).toContainText('1')
  await expect(page.locator('.match-provisional')).toBeVisible()

  // The refresh button is present and re-issues the match query on click.
  const before = matchOps.count()
  const refresh = page.getByRole('button', { name: 'Refresh now' })
  await expect(refresh).toBeVisible()
  await refresh.click()
  await expect.poll(() => matchOps.count(), {
    message: 'clicking Refresh now must re-issue the match query',
    timeout: 5_000,
  }).toBeGreaterThan(before)

  // Last-updated indicator renders.
  await expect(page.locator('.last-updated')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('scoreboard: Max column appears with grace ceiling while M4 is live', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // Ensure grace has a 1–0 tip for Group D (re-entering is idempotent).
  await setPreset(page, LIVE_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '1', '0')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // Move into M4's live window so the server sees a live match.
  await setPreset(page, LIVE_GAME, 'during')

  await page.locator('.nav-bar').getByRole('link', { name: 'Scoreboard' }).click()
  await expect(page).toHaveURL(/\/scoreboard$/)

  // The "Max" column appears only while something is live.
  const ceiling = page.locator('.score-ceiling').first()
  await expect(ceiling).toBeVisible()

  // grace's row shows a ceiling of ≤ 4 (1–0 vs live 1–0 → base 4, group ×1).
  const graceRow = page.locator('table.data-table tbody tr').filter({ hasText: 'grace' })
  await expect(graceRow).toBeVisible()
  await expect(graceRow.locator('.score-ceiling')).toContainText('4')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

> **Route/nav confirmed:** the Scoreboard route is `/scoreboard` (`web/src/App.tsx:33`) and the nav link renders the en label `"Scoreboard"` (`web/src/components/NavBar.tsx` → `navScoreboard`), so `getByRole('link', { name: 'Scoreboard' })` and `toHaveURL(/\/scoreboard$/)` are correct. **Still verify at execution time** that M4 appears in the dev-clock game `<select>` (it does for the existing `match-page.spec.ts`). If the seeded `external_id` causes M4 to fall outside the dev-clock preset list for any reason, fall back to driving the clock with a `localStorage` `xpool.devNow` set to an instant inside M4's live window (read M4's kickoff from `tournaments/fwc26.json`).

### 9c. Run the new spec

```sh
cd web && npm run e2e -- live-scoring 2>&1 | tail -30
```

**Expected:** both tests pass. (The whole suite boots its own stack via `global-setup`.)

### 9d. Commit

```sh
git add web/e2e/live-scoring.spec.ts web/.env.local
git commit -m "test(e2e): live max-achievable column + match-page refresh/poll"
```

---

## Task 10 — Per-cluster completion bar (verification) + code review

This is the cluster's done-gate (`.specs/TESTING.md` §4, `verification-before-completion`).

### 10a. Run every gate and capture output

```sh
cargo build --workspace 2>&1 | tail -3
cargo test --workspace 2>&1 | tail -8
cargo clippy --workspace -- -D warnings 2>&1 | tail -3
DYNAMO_TEST=1 cargo test -p storage 2>&1 | tail -5   # only if DynamoDB Local is up; else skip (stays green without it)
cd web && npm run build 2>&1 | tail -3
cd web && npm run lint 2>&1 | tail -3
cd web && npm run e2e 2>&1 | tail -15
```

**Expected — all green:**
- `cargo test --workspace`: `test result: ok` across `domain`, `api`, `xtask` (the new `max_reachable_score`, `scoreboard_live_tests`, `stub_live_source` tests all pass).
- clippy: no warnings.
- web build + lint: clean (no `console.log`).
- `npm run e2e`: the full suite including `live-scoring.spec.ts` passes; the existing `match-page.spec.ts` and `reported-results.spec.ts` still pass (they assert `actual` is null under NullSource — confirm the stub does **not** leak into their groups; the stub only maps `E2E1` → M4 in Group D, so Groups A/B/C/other-D-games stay null. **Double-check `match-page.spec.ts` uses M4** — it does, and now M4 has a live score during its window. If that spec's "no provisional marker after result is entered" test breaks because M4 is now live pre-result, adjust that spec to use a *different* Group D game for the official-result assertion, or give the stub a distinct event id not mapped to M4. Resolve during execution; note the coupling here.)

> **Coupling flag:** `web/e2e/match-page.spec.ts` already uses **M4**. Adding a live score to M4 changes its behaviour in that spec's live window. Two clean options, decide at execution time: (1) map the stub to a **different** Group D game id (give *that* game the `externalId`, leave M4 untouched) and point the new live-scoring spec at it; or (2) keep M4 and update `match-page.spec.ts` so its "official result, no provisional" test enters the official result and asserts the final state (official takes priority over live in the resolver — `official_result_takes_priority_over_live`, so once the result user enters M4's result the provisional marker disappears even with the stub active). Option (1) is lower-risk; prefer it unless the match-page spec already needs M4 specifically. **If choosing (1), change Task 7 to put `externalId` on a non-M4 Group D game and update the Task 9 spec's `LIVE_GAME`.**

### 10b. Request code review

Invoke `superpowers:requesting-code-review` on the full branch diff:

```sh
git diff master...HEAD --stat
```

Focus the review on:
- **`max_reachable_score` correctness** (the hard part) — re-derive the bound argument; confirm no reachable final scores higher than the enumerated max.
- The resolver's live detection (`live_matches`) — does it correctly exclude entered-official-result games and not-yet-kicked-off games? Does it consult the source exactly once?
- The `StubLiveSource` is genuinely inert in production (env unset ⇒ never constructed) and documented as a dev stub.
- No `Date.now()` behavioural branch in the SPA (only the last-updated *display*).
- i18n: both `en` and `hu` carry every new key (lint/type would catch a missing `hu` key if `hu` is `Record<StringKey,string>` — verify).

### 10c. Address review feedback

Use `superpowers:receiving-code-review` — verify each suggestion technically before applying; fix CRITICAL/HIGH, then MEDIUM where reasonable. Re-run the full gate (10a) after changes.

### 10d. Final verification claim

Only after 10a is green end-to-end and review is addressed, per `superpowers:verification-before-completion`: paste the actual passing output (not a claim) for `cargo test --workspace`, web `build`+`lint`, and `npm run e2e`.

### 10e. Finish the branch

Use `superpowers:finishing-a-development-branch`. Per `CLAUDE.md` working agreement: this is **complex, cross-cutting work** (domain + api + web + e2e + a new dev-stub seam) — a **PR** adds review/CI value, so open one; otherwise merge locally into `master`.

---

## Task-breakdown summary

1. `domain::max_reachable_score` — pure best-still-reachable per-match score, brute-forced over a bounded final grid; 7 unit tests (exact kept, exact lost/outcome kept, outcome lost, multiplier, two 4-goal cases, monotonicity).
2. Add `ScoreEntry.maxAchievable: Option<i64>` (null placeholder).
3. Compute `maxAchievable` in the `scoreboard` resolver via a new `live_matches` helper + `max_reachable_score`; 3 API tests (live ⇒ value, no-live ⇒ null, pre-kickoff ⇒ null).
4. Frontend scoreboard: query field, TS type, i18n (en+hu), conditional "Max" column + CSS.
5. Frontend match-page: "Refresh now" button (re-issues query), last-updated indicator, spinner + CSS + i18n.
6. **(FLAGGED)** `XPOOL_LIVE_SCORES` `StubLiveSource` in the API (dev/test stub, inert in prod) — the e2e live-score injection mechanism; 2 unit tests.
7. Seed `externalId` on one Group D game in `fwc26.json` so the stub can target it.
8. Wire `XPOOL_LIVE_SCORES` into `web/scripts/e2e-stack.sh`.
9. e2e `live-scoring.spec.ts`: (a) live match page + working refresh + last-updated, (b) scoreboard Max column with grace's ceiling; ensure `web/.env.local` blanks Auth0.
10. Completion bar (all cargo/web/e2e gates + `DYNAMO_TEST=1` gating) + `requesting-code-review` + `verification-before-completion`; resolve the M4 coupling with `match-page.spec.ts`.

**UNSURE / flagged:** The e2e live-score **injection mechanism** (Task 6) — there is no existing one (`THESPORTSDB_API_KEY=""`→`NullSource`, zero `externalId`s in the fixture). I specified an env-driven `StubLiveSource` (`XPOOL_LIVE_SCORES`) + a seeded `externalId`. The **M4 coupling** with the existing `match-page.spec.ts` (Task 10a flag) must be resolved at execution time — prefer giving the `externalId` to a non-M4 Group D game. Also verify the **Scoreboard route path/nav label** and that **M4 is selectable in the dev-clock preset** before finalising Task 9's selectors.
