# Knockout-only Scoreboard (cluster/standings) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a knockout-only re-engagement view of the scoreboard — the same pool-scoped board re-summed from knockout-stage matches only, fresh from zero — surfaced both as a toggle on `ScoreboardPage` and as a standalone linkable route `/scoreboard/knockout`.

**Architecture:** This is a *pure re-slicing* of the already-materialised scoreboard. The materialised `Scoreboard` (`storage::Scoreboard`) already stores each player's points as a per-`Round` breakdown (`playerId → {round → points}`), recomputed wholesale by `recompute.rs` on result entry. The knockout-only total is therefore the sum of the breakdown over knockout rounds only (everything past `GroupStage`); group-stage points are simply excluded, which is exactly "everyone starts the knockouts at zero". No new materialisation, no domain deadline/entry change, no `tournament_id` threading. One pure domain predicate (`Round::is_knockout`), one new GraphQL resolver (`knockoutScoreboard`) that re-uses the existing board, and an SPA toggle whose two options are routes.

**Tech Stack:** Rust (axum + async-graphql, `crates/domain` + `crates/api`), React + Vite + TypeScript SPA (urql GraphQL client, react-router), Playwright e2e.

## Global Constraints

- **Single-tournament domain, multi-tournament storage** — never thread a `tournament_id` through `domain` types.
- **Official results are the "result user"** (`is_result_user`); player listings exclude it; scoring is symmetric. The knockout board re-uses `domain::participation::participants` for the same exclusion.
- **Resolvers do NO I/O and NO domain logic** — load coarse storage items once, then call pure `domain` functions. The knockout/group split lives in `domain` (`Round::is_knockout`), not in the resolver.
- **Server-authoritative clock** — the SPA never branches on `Date.now()`. No new clock logic is introduced here (the board is already materialised as-of the request clock).
- **i18n is first-class (English + Hungarian)** — every new UI string gets both locales in `web/src/i18n/strings.ts`.
- **Immutability** — create new objects, never mutate (Rust: build new `Vec`/`ScoreEntry`; TS: spread/derive, no in-place edits).
- **Branch discipline** — `crates/*` and `web/` changes go on a branch/worktree, never straight to `master`. Each branch uses its own `xpool-<branch>` table; e2e runs its own isolated stack.
- **No entry-policy change** — knockout tips are entered normally per each KO match's own deadline. This feature is a VIEW only.
- **Quality bar (per cluster):** `cargo build --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` all green; `npm run build` + `npm run lint` green; one e2e proving the toggle + route render a knockout-only board.

---

## File Structure

**Rust**
- `crates/domain/src/model.rs` — add `impl Round { pub fn is_knockout(self) -> bool }` (additive to the locked contract — a method, not a type change) + a unit-test module.
- `crates/api/src/gql/query.rs` — add two private helpers (`pool_member_filter`, `score_entries`), refactor the existing `scoreboard` resolver to use them, add the `knockout_scoreboard` resolver, add a `scoreboard_tests` module.

**Web**
- `web/src/graphql/queries.ts` — add `KNOCKOUT_SCOREBOARD_QUERY` (aliases the field back to `scoreboard` so the page reads the same response shape).
- `web/src/i18n/strings.ts` — add `knockoutOnly` + `scoreboardKnockoutTitle` (EN + HU). Re-uses the existing `overall` key for the other toggle label.
- `web/src/components/ScoreboardModeToggle.tsx` — new small component: the Overall ⇄ Knockout-only switch (two `NavLink`s, so the toggle state IS the URL).
- `web/src/pages/ScoreboardPage.tsx` — accept a `mode` prop, pick the query, render the toggle, drop the Group Stage column in knockout mode.
- `web/src/App.tsx` — append the `/scoreboard/knockout` route. **Shared seam:** `App.tsx` is owned by cluster `player-analytics`; this cluster only *appends* its route and leaves a comment marking the seam so a merge conflict is obvious and resolvable by keeping both route sets.
- `web/src/index.css` — `.scoreboard-toggle` styles (new class names need CSS).

**E2E**
- `web/e2e/knockout-only-scoreboard.spec.ts` — new spec: seeds the `balanced` scenario, advances the dev clock past the Final, asserts the toggle + route render a knockout-only board with the Group Stage column dropped and a smaller total than overall.

---

## Task 1: Domain predicate `Round::is_knockout`

**Files:**
- Modify: `crates/domain/src/model.rs` (add `impl Round` after the `Round` enum, ~line 59; add a test module)
- Test: `crates/domain/src/model.rs` (`#[cfg(test)] mod round_tests`)

**Interfaces:**
- Produces: `domain::Round::is_knockout(self) -> bool` — a free predicate usable as a function item `fn(domain::Round) -> bool`. `true` for every round except `Round::GroupStage`. Consumed by Task 2's `score_entries`.

- [ ] **Step 1: Write the failing test**

Add this test module to the **end** of `crates/domain/src/model.rs`:

```rust
#[cfg(test)]
mod round_tests {
    use super::*;

    #[test]
    fn group_stage_is_not_knockout() {
        assert!(!Round::GroupStage.is_knockout());
    }

    #[test]
    fn every_post_group_round_is_knockout() {
        for r in [
            Round::R32,
            Round::R16,
            Round::QF,
            Round::SF,
            Round::ThirdPlace,
            Round::Final,
        ] {
            assert!(r.is_knockout(), "{r:?} should count as knockout");
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p domain round_tests`
Expected: FAIL to compile with "no method named `is_knockout` found for enum `Round`".

- [ ] **Step 3: Write minimal implementation**

Insert directly **after** the `Round` enum definition (after its closing `}` near line 59) in `crates/domain/src/model.rs`:

```rust
impl Round {
    /// Every round past the group stage. Drives the knockout-only scoreboard
    /// (a re-engagement VIEW — `.scratch/knockout-only-scoreboard/PRD.md`): the
    /// materialised per-round breakdown is re-summed over these rounds only, so
    /// every player starts the knockout race from zero. This is the single
    /// piece of group-vs-knockout logic the API delegates to (resolvers carry
    /// no domain logic).
    pub fn is_knockout(self) -> bool {
        !matches!(self, Round::GroupStage)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p domain round_tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/model.rs
git commit -m "feat(domain): add Round::is_knockout predicate"
```

---

## Task 2: `knockoutScoreboard` resolver

**Files:**
- Modify: `crates/api/src/gql/query.rs` — add `pool_member_filter` + `score_entries` helpers (after the `collect_leaf_groups` fn, before `scored_tip`), refactor `scoreboard` (lines ~149-222), add `knockout_scoreboard` (immediately after `scoreboard`), add `scoreboard_tests` module (at end of file).
- Test: `crates/api/src/gql/query.rs` (`#[cfg(test)] mod scoreboard_tests`)

**Interfaces:**
- Consumes: `domain::Round::is_knockout` (Task 1); `domain::participation::participants`; `storage::Scoreboard { entries: HashMap<PlayerId, HashMap<Round, i64>> }`; `crate::gql::types::{ScoreEntry, StageScore}`.
- Produces: GraphQL field `knockoutScoreboard(pool: ID): [ScoreEntry!]!` — identical shape to `scoreboard`, but each row's `total` and `stages` cover knockout rounds only. Same pool-membership privacy rule and the same participant filtering as `scoreboard`. Consumed by Task 3's `KNOCKOUT_SCOREBOARD_QUERY`.
- Produces (internal): `fn pool_member_filter(ctx, repo, pool) -> Result<Option<Vec<String>>>` and `fn score_entries(board, nick_by_id, allowed, participant_ids, keep_round) -> Vec<ScoreEntry>`.

- [ ] **Step 1: Write the failing test**

Add this module at the **end** of `crates/api/src/gql/query.rs` (it seeds the materialised board directly — the resolver's job is purely to re-slice it, so no tournament/recompute is needed):

```rust
#[cfg(test)]
mod scoreboard_tests {
    use crate::auth::CurrentPlayer;
    use crate::reported::ReportedResultSource;
    use async_trait::async_trait;
    use domain::{MatchPrediction, Player, Pool, Round};
    use sportsdb::Event;
    use std::collections::HashMap;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository, Scoreboard};

    struct NoSource;
    #[async_trait]
    impl ReportedResultSource for NoSource {
        async fn lookup_events(&self, _ids: &[String]) -> anyhow::Result<Vec<Event>> {
            Ok(vec![])
        }
    }

    /// A competing player (one dummy locked prediction so `participants()`
    /// counts them — the actual scores come from the seeded board, not here).
    fn player(id: &str) -> Player {
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
                home_score: 1,
                away_score: 0,
                locked: true,
            }],
            standings_predictions: vec![],
        }
    }

    fn result_user() -> Player {
        Player {
            id: "result-user".into(),
            person_id: "p".into(),
            nick: "official".into(),
            full_name: "Official".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    /// alice: 4 group + 8 R32 = 12 overall, 8 knockout.
    /// bob:  10 group + 2 R32 = 12 overall, 2 knockout.
    async fn repo_with_board() -> InMemoryRepository {
        let repo = InMemoryRepository::new();
        repo.put_player(&result_user()).await.unwrap();
        repo.put_player(&player("alice")).await.unwrap();
        repo.put_player(&player("bob")).await.unwrap();
        let mut board = Scoreboard::default();
        board.entries.insert(
            "alice".into(),
            HashMap::from([(Round::GroupStage, 4), (Round::R32, 8)]),
        );
        board.entries.insert(
            "bob".into(),
            HashMap::from([(Round::GroupStage, 10), (Round::R32, 2)]),
        );
        repo.put_scoreboard(&board).await.unwrap();
        repo
    }

    async fn exec(repo: InMemoryRepository, viewer: CurrentPlayer, query: &str) -> serde_json::Value {
        let repo: Arc<dyn Repository> = Arc::new(repo);
        let source: Arc<dyn ReportedResultSource> = Arc::new(NoSource);
        let schema = crate::gql::build_schema(repo, source);
        let req = async_graphql::Request::new(query)
            .data(viewer)
            .data(crate::clock::RequestNow(
                "2026-07-19T12:00:00Z".parse().unwrap(),
            ));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "{:?}", resp.errors);
        resp.data.into_json().unwrap()
    }

    #[tokio::test]
    async fn knockout_board_resums_knockout_rounds_only() {
        let repo = repo_with_board().await;
        let data = exec(
            repo,
            CurrentPlayer::Visitor,
            r#"{ knockoutScoreboard { playerId total stages { round points } } }"#,
        )
        .await;
        let rows = data["knockoutScoreboard"].as_array().unwrap();
        // alice (8) ranks above bob (2): knockout total desc, then player_id asc.
        assert_eq!(rows[0]["playerId"], "alice");
        assert_eq!(rows[0]["total"], 8);
        assert_eq!(rows[1]["playerId"], "bob");
        assert_eq!(rows[1]["total"], 2);
        // The group-stage stage is excluded entirely — only knockout rounds appear.
        let alice_rounds: Vec<&str> = rows[0]["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["round"].as_str().unwrap())
            .collect();
        assert!(alice_rounds.contains(&"R32"));
        assert!(!alice_rounds.contains(&"GROUP_STAGE"));
    }

    #[tokio::test]
    async fn overall_board_still_sums_every_round() {
        // Regression guard for the shared-helper refactor: overall is unchanged.
        let repo = repo_with_board().await;
        let data = exec(
            repo,
            CurrentPlayer::Visitor,
            r#"{ scoreboard { playerId total } }"#,
        )
        .await;
        let rows = data["scoreboard"].as_array().unwrap();
        // alice & bob both total 12; tie broken by player_id asc → alice first.
        assert_eq!(rows[0]["playerId"], "alice");
        assert_eq!(rows[0]["total"], 12);
        assert_eq!(rows[1]["playerId"], "bob");
        assert_eq!(rows[1]["total"], 12);
    }

    #[tokio::test]
    async fn knockout_board_pool_filter_restricts_to_members() {
        let repo = repo_with_board().await;
        // Pool P1 has only alice (the viewer); bob is excluded.
        repo.put_pool(&Pool {
            id: "P1".into(),
            name: "Pool 1".into(),
            owner: "alice".into(),
            members: vec!["alice".into()],
            prefix: "P1".into(),
        })
        .await
        .unwrap();
        let data = exec(
            repo,
            CurrentPlayer::Player(Box::new(player("alice"))),
            r#"{ knockoutScoreboard(pool: "P1") { playerId } }"#,
        )
        .await;
        let ids: Vec<String> = data["knockoutScoreboard"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["playerId"].as_str().unwrap().to_string())
            .collect();
        assert!(ids.contains(&"alice".to_string()), "alice (member) shown");
        assert!(!ids.contains(&"bob".to_string()), "bob (non-member) hidden");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p api scoreboard_tests`
Expected: FAIL to compile — `knockoutScoreboard` is not a known field (the query errors, or the schema lacks the resolver).

- [ ] **Step 3a: Add the shared helpers**

Insert these two functions into `crates/api/src/gql/query.rs` immediately **after** the `collect_leaf_groups` function (before `scored_tip`):

```rust
/// Resolve an optional pool filter to its member-id list, enforcing the
/// private-membership rule (Issue 04): a pool filter requires the viewer to be
/// a member or the owner of that pool. `None` pool → `Ok(None)` (the global,
/// public board). Shared by `scoreboard` and `knockout_scoreboard` so the two
/// never drift.
async fn pool_member_filter(
    ctx: &Context<'_>,
    repo: &dyn Repository,
    pool: Option<String>,
) -> async_graphql::Result<Option<Vec<String>>> {
    match pool {
        Some(pool_id) => {
            let viewer = CurrentPlayer::require(ctx)?;
            let pools = repo.list_pools().await?;
            let p = pools
                .into_iter()
                .find(|p| p.id == pool_id)
                .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
            if !p.members.contains(&viewer.id) && p.owner != viewer.id {
                return Err(async_graphql::Error::new(
                    "you are not a member of this pool",
                ));
            }
            Ok(Some(p.members))
        }
        None => Ok(None),
    }
}

/// Build the sorted scoreboard rows from the materialised board, keeping only
/// the stages whose round satisfies `keep_round` and re-summing each row's
/// total over them. `keep_round = |_| true` yields the overall board;
/// `domain::Round::is_knockout` yields the knockout-only board (group-stage
/// points excluded, so everyone starts the knockouts from zero). Drops
/// non-participants and applies the optional pool-member filter. Ordering is the
/// shared rule: total descending, then player id ascending.
fn score_entries(
    board: &storage::Scoreboard,
    nick_by_id: &HashMap<&str, &str>,
    allowed: Option<&[String]>,
    participant_ids: &std::collections::HashSet<&str>,
    keep_round: impl Fn(domain::Round) -> bool,
) -> Vec<ScoreEntry> {
    let mut entries: Vec<ScoreEntry> = board
        .entries
        .iter()
        .filter(|(pid, _)| allowed.is_none_or(|m| m.contains(pid)))
        .filter(|(pid, _)| participant_ids.contains(pid.as_str()))
        .map(|(pid, breakdown)| {
            let stages: Vec<StageScore> = breakdown
                .iter()
                .filter(|(round, _)| keep_round(**round))
                .map(|(round, points)| StageScore {
                    round: (*round).into(),
                    points: *points,
                })
                .collect();
            let total: i64 = breakdown
                .iter()
                .filter(|(round, _)| keep_round(**round))
                .map(|(_, points)| *points)
                .sum();
            ScoreEntry {
                player_id: pid.clone(),
                nick: nick_by_id
                    .get(pid.as_str())
                    .copied()
                    .unwrap_or("")
                    .to_owned(),
                total,
                stages,
            }
        })
        .collect();
    entries.sort_by(|a, b| b.total.cmp(&a.total).then(a.player_id.cmp(&b.player_id)));
    entries
}
```

- [ ] **Step 3b: Refactor `scoreboard` to delegate to the helpers**

Replace the **entire body** of the existing `scoreboard` resolver (the method `async fn scoreboard(...)`, lines ~150-222) with:

```rust
    /// The materialised scoreboard, optionally filtered to a pool's members.
    async fn scoreboard(
        &self,
        ctx: &Context<'_>,
        pool: Option<String>,
    ) -> async_graphql::Result<Vec<ScoreEntry>> {
        let repo = repo(ctx);
        let board = repo.get_scoreboard().await?.unwrap_or_default();
        let players = repo.list_players().await?;
        let nick_by_id: HashMap<&str, &str> = players
            .iter()
            .map(|p| (p.id.as_str(), p.nick.as_str()))
            .collect();

        // Pool scoping (Issue 04) + participant filtering — the same rules the
        // knockout board re-uses, so the two boards list the same people.
        let allowed = pool_member_filter(ctx, repo, pool).await?;
        let participant_ids: std::collections::HashSet<&str> =
            domain::participation::participants(&players)
                .iter()
                .map(|p| p.id.as_str())
                .collect();

        Ok(score_entries(
            &board,
            &nick_by_id,
            allowed.as_deref(),
            &participant_ids,
            |_| true,
        ))
    }
```

- [ ] **Step 3c: Add the `knockout_scoreboard` resolver**

Insert this method into `impl QueryRoot` immediately **after** the refactored `scoreboard` method:

```rust
    /// The knockout-only scoreboard (`.scratch/knockout-only-scoreboard/PRD.md`).
    /// A re-engagement VIEW: the same materialised board re-summed over knockout
    /// rounds only, so every player starts the back half of the tournament from
    /// zero. Identical shape, pool-scoping and participant rules as `scoreboard`
    /// — group-stage points are simply excluded. No new materialisation: it
    /// re-slices the board already built by `recompute.rs`. KO tips are still
    /// entered normally per each match's own deadline; this changes no entry
    /// policy.
    async fn knockout_scoreboard(
        &self,
        ctx: &Context<'_>,
        pool: Option<String>,
    ) -> async_graphql::Result<Vec<ScoreEntry>> {
        let repo = repo(ctx);
        let board = repo.get_scoreboard().await?.unwrap_or_default();
        let players = repo.list_players().await?;
        let nick_by_id: HashMap<&str, &str> = players
            .iter()
            .map(|p| (p.id.as_str(), p.nick.as_str()))
            .collect();

        let allowed = pool_member_filter(ctx, repo, pool).await?;
        let participant_ids: std::collections::HashSet<&str> =
            domain::participation::participants(&players)
                .iter()
                .map(|p| p.id.as_str())
                .collect();

        Ok(score_entries(
            &board,
            &nick_by_id,
            allowed.as_deref(),
            &participant_ids,
            domain::Round::is_knockout,
        ))
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p api scoreboard_tests`
Expected: PASS (3 tests).

Then the full crate + lints:

Run: `cargo test -p api && cargo clippy -p api -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/gql/query.rs
git commit -m "feat(api): add knockoutScoreboard resolver re-slicing the materialised board"
```

---

## Task 3: SPA — toggle, mode, route, knockout query

**Files:**
- Modify: `web/src/graphql/queries.ts` — add `KNOCKOUT_SCOREBOARD_QUERY` after `SCOREBOARD_QUERY` (~line 73).
- Modify: `web/src/i18n/strings.ts` — add `knockoutOnly` + `scoreboardKnockoutTitle` to the `en` block (after `overall`, ~line 210) and the `hu` block (after `overall`, ~line 557).
- Create: `web/src/components/ScoreboardModeToggle.tsx`.
- Modify: `web/src/pages/ScoreboardPage.tsx` — add a `mode` prop, pick the query, render the toggle, drop the Group Stage column in knockout mode.
- Modify: `web/src/App.tsx` — append the `/scoreboard/knockout` route + seam comment.
- Modify: `web/src/index.css` — add `.scoreboard-toggle` styles.

**Interfaces:**
- Consumes: GraphQL field `knockoutScoreboard(pool: ID): [ScoreEntry!]!` (Task 2). Re-uses the existing `ScoreEntry` TS type (`web/src/graphql/types.ts`) unchanged — the new query aliases the field back to `scoreboard`, so the response shape is `{ scoreboard: ScoreEntry[] }` in both modes.
- Produces: route `/scoreboard/knockout` rendering `<ScoreboardPage mode="knockout" />`; component `ScoreboardModeToggle`; i18n keys `knockoutOnly`, `scoreboardKnockoutTitle`.

- [ ] **Step 1: Add the knockout query document**

In `web/src/graphql/queries.ts`, add directly **after** the `SCOREBOARD_QUERY` export (~line 73):

```ts
/**
 * Knockout-only scoreboard — re-sums points from knockout-stage matches only
 * (re-engagement view). The field is aliased back to `scoreboard` so the page
 * reads the same `data.scoreboard` shape in both modes.
 */
export const KNOCKOUT_SCOREBOARD_QUERY = `
  query KnockoutScoreboard($pool: ID) {
    scoreboard: knockoutScoreboard(pool: $pool) {
      playerId nick total
      stages { round points }
    }
  }
`
```

- [ ] **Step 2: Add the i18n strings (EN + HU)**

In `web/src/i18n/strings.ts`, inside the `en` block, after the `overall: 'Overall',` line (~line 210), add:

```ts
  knockoutOnly: 'Knockout only',
  scoreboardKnockoutTitle: 'Knockout Scoreboard',
```

Inside the `hu` block, after the `overall: 'Összesített',` line (~line 557), add:

```ts
  knockoutOnly: 'Csak kieséses',
  scoreboardKnockoutTitle: 'Kieséses tippverseny',
```

- [ ] **Step 3: Create the toggle component**

Create `web/src/components/ScoreboardModeToggle.tsx`:

```tsx
import { NavLink } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'

/**
 * Overall ⇄ Knockout-only switch for the scoreboard. Each option is a route, so
 * the toggle state IS the URL — `/scoreboard/knockout` is directly linkable and
 * shareable. The knockout board re-sums points from knockout matches only
 * (a re-engagement view — `.scratch/knockout-only-scoreboard/PRD.md`).
 */
export function ScoreboardModeToggle() {
  const { t } = useI18n()
  const cls = ({ isActive }: { isActive: boolean }) =>
    isActive ? 'active' : undefined
  return (
    <nav className="scoreboard-toggle" aria-label={t('scoreboardTitle')}>
      <NavLink to="/scoreboard" end className={cls}>
        {t('overall')}
      </NavLink>
      <NavLink to="/scoreboard/knockout" className={cls}>
        {t('knockoutOnly')}
      </NavLink>
    </nav>
  )
}
```

- [ ] **Step 4: Rewrite `ScoreboardPage.tsx` with the `mode` prop**

Replace the **entire contents** of `web/src/pages/ScoreboardPage.tsx` with:

```tsx
import { useMemo } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  KNOCKOUT_SCOREBOARD_QUERY,
  POOLS_QUERY,
  SCOREBOARD_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type {
  Pool,
  ScoreEntry,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { ScoreboardModeToggle } from '../components/ScoreboardModeToggle'
import { usePolledQuery } from '../lib/usePolledQuery'
import { pollIntervalMs } from '../lib/polling'
import { readyRounds, roundLabel, ROUND_ORDER, STAGE_MULTIPLIERS } from '../lib/rounds'
import { PoolSelector } from '../pools/PoolSelector'
import { useSelectedPool } from '../pools/useSelectedPool'
import { effectiveSelectedPool } from '../lib/selectedPool'

type ScoreboardMode = 'overall' | 'knockout'

/**
 * Ranked leaderboard, overall + per stage, with pool selector (UC-8).
 * `mode` picks the board: `overall` (group + knockout) or `knockout` (knockout
 * matches only, summed fresh from zero — a re-engagement view). The mode is
 * route-driven (`/scoreboard` vs `/scoreboard/knockout`) so the knockout board
 * is directly linkable.
 */
export function ScoreboardPage({ mode = 'overall' }: { mode?: ScoreboardMode }) {
  const { t } = useI18n()
  const { label } = useAuth()
  // Sticky, cross-page pool selection (see SelectedPoolProvider): `undefined`
  // = not chosen → default to the first pool; `null` = explicit "everyone";
  // a string = a specific pool.
  const { selected } = useSelectedPool()

  // `pools` requires authentication (API.md §8) — the scoreboard itself is
  // public, so the pool selector is only populated for a logged-in player.
  // Issuing `pools` as a visitor would surface an auth error on a public page.
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = poolsResult.data?.pools ?? []
  // Default to the first pool the player belongs to; global stays reachable.
  const effectivePool = effectiveSelectedPool(
    selected,
    pools.map((p) => p.id),
  )

  const [probe] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const interval = useMemo(
    () => pollIntervalMs(probe.data?.tournament?.games ?? []),
    [probe.data],
  )
  // Both queries return `{ scoreboard: ScoreEntry[] }` — the knockout query
  // aliases the field — so the rest of the component is mode-agnostic.
  const query = mode === 'knockout' ? KNOCKOUT_SCOREBOARD_QUERY : SCOREBOARD_QUERY
  const [result, reexecute] = usePolledQuery<{
    scoreboard: ScoreEntry[]
  }>({ query, variables: { pool: effectivePool } }, interval)

  const scoreboard = result.data?.scoreboard ?? null

  // Only show round columns whose teams are known — a future round with no
  // game determined yet (knockouts before the bracket resolves) is hidden,
  // mirroring the My Tips / All Tips round tabs. GROUP_STAGE is always ready,
  // but it is dropped in knockout mode (it never contributes there).
  const ready = readyRounds(
    probe.data?.tournament?.groups ?? [],
    probe.data?.tournament?.games ?? [],
  )
  const visibleRounds = ROUND_ORDER.filter(
    (r) => ready.has(r) && (mode === 'overall' || r !== 'GROUP_STAGE'),
  )

  if (result.fetching && !scoreboard) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!scoreboard) return <ErrorView />

  const ranked = [...scoreboard].sort((a, b) => b.total - a.total)
  const title =
    mode === 'knockout' ? t('scoreboardKnockoutTitle') : t('scoreboardTitle')

  return (
    <section className="page">
      <h2>{title}</h2>
      {interval > 0 && <p className="poll-note">● live</p>}

      <ScoreboardModeToggle />
      <PoolSelector pools={pools} />

      <table className="data-table">
        <thead>
          <tr>
            <th>{t('rank')}</th>
            <th>{t('player')}</th>
            {visibleRounds.map((r) => (
              <th key={r}>
                {roundLabel(r, t)}
                <br />
                <small>
                  {t('multiplier')} ×{STAGE_MULTIPLIERS[r]}
                </small>
              </th>
            ))}
            <th>{t('total')}</th>
          </tr>
        </thead>
        <tbody>
          {ranked.map((entry, i) => {
            const byRound = new Map(
              entry.stages.map((s) => [s.round, s.points]),
            )
            return (
              <tr key={entry.playerId}>
                <td>{i + 1}</td>
                <td>
                  <Link to={`/player/${entry.playerId}`}>{entry.nick}</Link>
                </td>
                {visibleRounds.map((r) => (
                  <td key={r}>{byRound.get(r) ?? 0}</td>
                ))}
                <td>
                  <strong>{entry.total}</strong>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </section>
  )
}
```

- [ ] **Step 5: Add the route in `App.tsx`**

In `web/src/App.tsx`, replace the single scoreboard route line:

```tsx
        <Route path="scoreboard" element={<ScoreboardPage />} />
```

with:

```tsx
        {/* SHARED SEAM (owned by cluster player-analytics, which also appends
            routes here): on merge, keep BOTH route sets. The knockout board is
            a standalone linkable route as well as a toggle on the page. */}
        <Route path="scoreboard" element={<ScoreboardPage />} />
        <Route
          path="scoreboard/knockout"
          element={<ScoreboardPage mode="knockout" />}
        />
```

- [ ] **Step 6: Add the toggle CSS**

In `web/src/index.css`, add this block immediately **before** the `.data-table {` rule (~line 436, just under the `TABLES` banner comment is fine):

```css
/* Overall ⇄ Knockout-only scoreboard switch. Segmented look; the active
   option (current route) gets the amber accent. */
.scoreboard-toggle {
  display: inline-flex;
  gap: 0;
  margin: 8px 0 4px;
  border: 2px solid var(--bg-card-border);
}

.scoreboard-toggle a {
  padding: 6px 14px;
  font-family: 'Press Start 2P', monospace;
  font-size: 9px;
  letter-spacing: 1px;
  text-transform: uppercase;
  text-decoration: none;
  color: var(--text-dim);
  background: var(--bg-card);
}

.scoreboard-toggle a + a {
  border-left: 2px solid var(--bg-card-border);
}

.scoreboard-toggle a.active {
  color: var(--bg-card);
  background: var(--amber);
}
```

- [ ] **Step 7: Build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: `tsc -b` + `vite build` succeed; eslint clean.

- [ ] **Step 8: Visual check (manual)**

Start the dev stack (or `bin/local-dev`), open `/scoreboard`, confirm: the Overall ⇄ Knockout only toggle renders as a segmented control with the active option highlighted; clicking "Knockout only" navigates to `/scoreboard/knockout`, drops the Group Stage column, and re-titles the page. Confirm the toggle is styled (not unstyled link text) — new class names need CSS.

- [ ] **Step 9: Commit**

```bash
git add web/src/graphql/queries.ts web/src/i18n/strings.ts \
  web/src/components/ScoreboardModeToggle.tsx web/src/pages/ScoreboardPage.tsx \
  web/src/App.tsx web/src/index.css
git commit -m "feat(web): knockout-only scoreboard toggle + /scoreboard/knockout route"
```

---

## Task 4: E2E — toggle + route render a knockout-only board

**Files:**
- Create: `web/e2e/knockout-only-scoreboard.spec.ts`

**Interfaces:**
- Consumes: the running e2e stack (booted by `e2e/global-setup.ts`), the `balanced` scenario seed, `devLogin`/`watchNetwork`/`expectNoErrorView` from `e2e/helpers.ts`, the `/scoreboard` + `/scoreboard/knockout` routes (Task 3), the `knockoutScoreboard` resolver (Task 2).
- Produces: a Playwright spec proving the knockout-only board renders via both the toggle and the direct route, with the Group Stage column dropped and a strictly smaller total than overall.

- [ ] **Step 1: Write the e2e spec**

Create `web/e2e/knockout-only-scoreboard.spec.ts`:

```ts
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Knockout-only scoreboard, end to end. Seed the `balanced` scenario (full
 * results, ~12 players), advance the dev clock past the Final so every round is
 * scored, then assert the Overall ⇄ Knockout-only toggle and the standalone
 * `/scoreboard/knockout` route render a board that drops the Group Stage column
 * and totals strictly less than the overall board (group-stage points excluded
 * — everyone starts the knockouts from zero).
 */

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

/** Sum of the visible per-player scoreboard totals (the `<strong>` cells). */
async function scoreboardTotal(page: Page): Promise<number> {
  const totals = page.locator('.data-table tbody strong')
  await expect(totals.first()).toBeVisible()
  const texts = await totals.allInnerTexts()
  return texts.reduce((sum, text) => {
    const n = Number(text.replace(/[^\d-]/g, ''))
    return Number.isNaN(n) ? sum : sum + n
  }, 0)
}

/**
 * Pick a game + phase in the auth-bar dev clock; it applies, fires
 * devRematerialize, then reloads. Read totals from the RELOADED board.
 */
async function setClock(page: Page, gameId: string, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  await expect(page.locator('.dev-clock-now')).toBeVisible()
  await expect(page.locator('.data-table tbody tr').first()).toBeVisible()
}

test.beforeAll(() => {
  // Seed the `balanced` scenario into the same table the live stack booted
  // (its name is written by the e2e stack script to web/.e2e-table).
  const table = readFileSync(resolve(repoRoot, 'web/.e2e-table'), 'utf8').trim()
  execFileSync('cargo', ['run', '-p', 'xtask', '--', 'scenario', 'balanced'], {
    cwd: repoRoot,
    stdio: 'inherit',
    env: {
      ...process.env,
      XPOOL_TABLE: table,
      DYNAMO_ENDPOINT: 'http://localhost:8001',
    },
  })
})

test('knockout-only board: toggle + route drop the group stage and total less', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/scoreboard')

  // Advance to just after the Final so every round is scored.
  await setClock(page, 'M104', 'after')

  // Overall board: group + knockout columns present.
  await expect(page.locator('.data-table thead')).toContainText('Group Stage')
  await expect(page.locator('.data-table thead')).toContainText('Round of 32')
  const overall = await scoreboardTotal(page)

  // Switch to knockout-only via the toggle.
  await page.locator('.scoreboard-toggle').getByText('Knockout only').click()
  await expect(page).toHaveURL(/\/scoreboard\/knockout$/)
  await expect(page.locator('.data-table tbody tr').first()).toBeVisible()

  // Group Stage column is dropped; knockout rounds remain.
  await expect(page.locator('.data-table thead')).not.toContainText('Group Stage')
  await expect(page.locator('.data-table thead')).toContainText('Round of 32')
  await expect(page.locator('.data-table thead')).toContainText('Final')

  // Knockout-only totals exclude group-stage points → strictly smaller.
  const knockout = await scoreboardTotal(page)
  expect(knockout).toBeGreaterThan(0)
  expect(knockout).toBeLessThan(overall)

  // The route is independently linkable (deep-link, not just via the toggle).
  await page.goto('/scoreboard/knockout')
  await expect(page.locator('.data-table tbody tr').first()).toBeVisible()
  await expect(page.locator('.data-table thead')).not.toContainText('Group Stage')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the e2e spec**

Run: `cd web && npm run e2e -- knockout-only-scoreboard`
Expected: PASS. The suite boots its own isolated stack (API `:3001`, Vite `:5174`, DynamoDB `:8001`); it coexists with any running `bin/local-dev` session.

Note (dev-stub auth): if the dev-login step fails because the auth bar is hidden, ensure `web/.env.local` blanks `VITE_AUTH0_*` (Auth0 mode hides `.auth-bar`). This is the standing requirement for all dev-login e2e specs.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/knockout-only-scoreboard.spec.ts
git commit -m "test(web): e2e for knockout-only scoreboard toggle + route"
```

---

## Task 5: Verification + request code review

**Files:** none (verification only)

- [ ] **Step 1: Full workspace build + lint + test**

Run:
```bash
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```
Expected: all green, no clippy warnings.

- [ ] **Step 2: Full web build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: green.

- [ ] **Step 3: Re-run the new e2e**

Run: `cd web && npm run e2e -- knockout-only-scoreboard`
Expected: PASS.

- [ ] **Step 4: Confirm scope boundaries held**

Confirm by inspection:
- No `tournament_id` was threaded through `domain`.
- No deadline/entry-policy code changed (this is a VIEW only — `git diff` touches only `model.rs` (one method + tests), `query.rs`, and the web files listed above).
- The resolver contains no domain logic — the only group-vs-knockout decision is `domain::Round::is_knockout`.
- `App.tsx` change is append-only with the shared-seam comment intact.

- [ ] **Step 5: Request code review**

**REQUIRED SUB-SKILL:** Use superpowers:requesting-code-review to verify the work meets the cluster quality bar before merging. Provide the reviewer the PRD (`.scratch/knockout-only-scoreboard/PRD.md`, "Resolved decisions (2026-06-27 grill)" section is authoritative) and this plan. Address CRITICAL and HIGH findings; fix MEDIUM where reasonable.

- [ ] **Step 6: Commit any review fixes, then finish the branch**

After review fixes are committed, follow CLAUDE.md branch discipline: merge the branch into `master` locally (PR only if review-as-record/CI adds value), then push.

---

## Self-Review

**1. Spec coverage (against the PRD's Resolved decisions):**
- "No entry barrier / re-engagement VIEW, no domain deadline change" → no deadline code touched (Task 5 Step 4 guards this); `Round::is_knockout` is the only domain addition.
- "Scoring: re-sum knockout-stage matches only, fresh from zero" → `score_entries` with `keep_round = is_knockout` (Task 2); group-stage excluded ⇒ everyone starts at zero.
- "Surfacing — BOTH a toggle AND a standalone route" → `ScoreboardModeToggle` (toggle) + `/scoreboard/knockout` route (Task 3); e2e exercises both (Task 4).
- "Pool-scoped (follows existing pool selection)" → `pool_member_filter` + `PoolSelector`/`effectiveSelectedPool` re-used unchanged (Tasks 2, 3).
- "Ties/start: everyone starts at zero; tie-break reuses overall ordering rules" → same comparator `total desc, then player_id asc` in `score_entries` (Task 2); zero-start is inherent to excluding the group stage; the `overall_board_still_sums_every_round` test guards the shared ordering rule.
- "Cluster owns ScoreboardPage.tsx; one resolver in query.rs; minimal domain/fwc26 filtering" → exactly one resolver, one domain predicate, ScoreboardPage owned; `fwc26` untouched (knockout/group split is a `Round` property, no bracket re-resolution needed).

**2. Placeholder scan:** No TBD/TODO/"handle edge cases"/"similar to Task N" — every step carries full code or an exact command.

**3. Type consistency:** `Round::is_knockout(self) -> bool` (Task 1) is consumed as `domain::Round::is_knockout` (a `fn(Round)->bool`) by `score_entries` (Task 2). `score_entries`/`pool_member_filter` signatures are defined once and called consistently by both `scoreboard` and `knockout_scoreboard`. The web `KNOCKOUT_SCOREBOARD_QUERY` aliases the field to `scoreboard`, so `ScoreboardPage`'s `{ scoreboard: ScoreEntry[] }` result type is identical in both modes. CSS class `.scoreboard-toggle` matches the component `className` and the e2e locator.
</content>
</invoke>
