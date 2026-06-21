# Perfect Page Cluster Implementation Plan

> **For agentic workers:** Implement task-by-task; steps use checkbox (- [ ]) syntax.

**Goal:** Give the Perfect page (`web/src/pages/PerfectPage.tsx`) the same pool-scoping and a new sort-by-player view, while extracting the pool selection into a **sticky, shared** primitive that ScoreboardPage and AllTipsPage also consume.

**Architecture:** A new React context (`web/src/pools/SelectedPoolProvider.tsx`) holds the selected `poolId` (`null` = "everyone"), persisted to `localStorage`, defaulting to the viewer's first pool on first load. A shared `<PoolSelector>` component renders the picker. The `perfects` GraphQL query + Rust resolver gain a `pool: ID` argument mirroring the scoreboard/tips pool-membership filtering. A pure helper `web/src/lib/perfectOrder.ts` reorders the flat perfects list by-match (default) or by-player (player perfect-count desc, stable ties). View mode is persisted per-user in `localStorage`.

**Tech Stack:** Rust (axum + async-graphql) backend in `crates/api`; React + Vite + TS SPA in `web/` (urql GraphQL client, vitest unit tests, Playwright e2e). Server-authoritative clock — no `Date.now()` for behaviour in the SPA.

---

## Task 1 — Pure helper: `selectedPool` storage read/write/validate

Self-contained storage logic for the sticky selection, extracted so it is unit-testable without React. `null` = "everyone" (explicit); `undefined` = not-yet-chosen (defer to first pool). Persisted value is either the literal string `"__everyone__"` (an explicit "everyone" choice) or a pool id; an absent/blank key means not-yet-chosen.

**Files:**
- Create `web/src/lib/selectedPool.ts`
- Create `web/src/lib/selectedPool.test.ts`

- [ ] **Step 1: Write the failing test.** Create `web/src/lib/selectedPool.test.ts`:
  ```typescript
  import { afterEach, describe, expect, it } from 'vitest'
  import {
    SELECTED_POOL_KEY,
    readSelectedPool,
    writeSelectedPool,
    effectiveSelectedPool,
  } from './selectedPool'

  afterEach(() => localStorage.clear())

  describe('readSelectedPool', () => {
    it('returns undefined (not chosen) when nothing is stored', () => {
      expect(readSelectedPool()).toBeUndefined()
    })
    it('returns null (everyone) for the sentinel value', () => {
      localStorage.setItem(SELECTED_POOL_KEY, '__everyone__')
      expect(readSelectedPool()).toBeNull()
    })
    it('returns the stored pool id', () => {
      localStorage.setItem(SELECTED_POOL_KEY, 'pool-demo')
      expect(readSelectedPool()).toBe('pool-demo')
    })
  })

  describe('writeSelectedPool', () => {
    it('stores the sentinel for null (everyone)', () => {
      writeSelectedPool(null)
      expect(localStorage.getItem(SELECTED_POOL_KEY)).toBe('__everyone__')
      expect(readSelectedPool()).toBeNull()
    })
    it('stores a pool id verbatim', () => {
      writeSelectedPool('pool-demo')
      expect(localStorage.getItem(SELECTED_POOL_KEY)).toBe('pool-demo')
      expect(readSelectedPool()).toBe('pool-demo')
    })
  })

  describe('effectiveSelectedPool', () => {
    it('defers to the first pool id when not chosen', () => {
      expect(effectiveSelectedPool(undefined, ['p1', 'p2'])).toBe('p1')
    })
    it('is null (everyone) when not chosen and the viewer has no pools', () => {
      expect(effectiveSelectedPool(undefined, [])).toBeNull()
    })
    it('honours an explicit everyone choice over the first pool', () => {
      expect(effectiveSelectedPool(null, ['p1'])).toBeNull()
    })
    it('honours an explicit pool choice', () => {
      expect(effectiveSelectedPool('p2', ['p1', 'p2'])).toBe('p2')
    })
  })
  ```
- [ ] **Step 2: Run the test — expect FAIL** (module does not exist):
  ```sh
  cd web && npx vitest run src/lib/selectedPool.test.ts
  ```
  Expected: `Failed to resolve import "./selectedPool"`.
- [ ] **Step 3: Implement the helper.** Create `web/src/lib/selectedPool.ts`:
  ```typescript
  /**
   * Sticky pool selection persisted to localStorage, shared across the
   * Scoreboard / All Tips / Perfect pages.
   *
   * The selection is a three-state value the UI threads as `string | null |
   * undefined`:
   *   - `string`    → a specific pool id.
   *   - `null`      → the explicit "everyone" (global) board.
   *   - `undefined` → not chosen yet → defer to the viewer's first pool.
   *
   * Storage encodes the explicit "everyone" choice as a sentinel so it is
   * distinguishable from "not chosen" (an absent key).
   */
  export const SELECTED_POOL_KEY = 'xpool.selectedPool'

  /** Stored marker for an explicit "everyone" selection. */
  const EVERYONE_SENTINEL = '__everyone__'

  /** Read the persisted selection: a pool id, `null` (everyone), or `undefined`. */
  export function readSelectedPool(): string | null | undefined {
    let raw: string | null
    try {
      raw = localStorage.getItem(SELECTED_POOL_KEY)
    } catch {
      return undefined
    }
    if (raw === null || raw === '') return undefined
    if (raw === EVERYONE_SENTINEL) return null
    return raw
  }

  /** Persist a selection: a pool id, or `null` for the explicit "everyone". */
  export function writeSelectedPool(poolId: string | null): void {
    try {
      localStorage.setItem(
        SELECTED_POOL_KEY,
        poolId === null ? EVERYONE_SENTINEL : poolId,
      )
    } catch {
      /* ignore — selection is a convenience, not load-bearing state */
    }
  }

  /**
   * Resolve the selection actually sent to the API. When the user has not
   * chosen (`undefined`), default to their first pool; if they belong to no
   * pool, fall back to `null` (everyone). An explicit `null`/id is honoured.
   */
  export function effectiveSelectedPool(
    selected: string | null | undefined,
    poolIds: readonly string[],
  ): string | null {
    if (selected === undefined) return poolIds[0] ?? null
    return selected
  }
  ```
- [ ] **Step 4: Run the test — expect PASS:**
  ```sh
  cd web && npx vitest run src/lib/selectedPool.test.ts
  ```
  Expected: all 9 assertions green.
- [ ] **Step 5: Commit:**
  ```sh
  git add web/src/lib/selectedPool.ts web/src/lib/selectedPool.test.ts
  git commit -m "feat(web): pure sticky-pool-selection storage helper"
  ```

---

## Task 2 — Shared `SelectedPoolProvider` context + `useSelectedPool` hook

A React context backed by Task 1's helper, mirroring the existing `display/` provider split (context value file + provider + hook). Mounted once in `main.tsx`.

**Files:**
- Create `web/src/pools/selectedPoolContextValue.ts`
- Create `web/src/pools/SelectedPoolProvider.tsx`
- Create `web/src/pools/useSelectedPool.ts`
- Modify `web/src/main.tsx` (provider tree, lines 6-31)

- [ ] **Step 1: Create the context value.** Create `web/src/pools/selectedPoolContextValue.ts`:
  ```typescript
  import { createContext } from 'react'

  export interface SelectedPoolState {
    /** The raw three-state selection: pool id, `null` (everyone), `undefined` (unchosen). */
    selected: string | null | undefined
    /** Set the explicit selection (a pool id, or `null` for everyone). Persists. */
    setSelected: (poolId: string | null) => void
  }

  export const SelectedPoolContext = createContext<SelectedPoolState | undefined>(
    undefined,
  )
  ```
- [ ] **Step 2: Create the provider.** Create `web/src/pools/SelectedPoolProvider.tsx`:
  ```typescript
  import { useMemo, useState, type ReactNode } from 'react'
  import { readSelectedPool, writeSelectedPool } from '../lib/selectedPool'
  import {
    SelectedPoolContext,
    type SelectedPoolState,
  } from './selectedPoolContextValue'

  /** Sticky, cross-page pool selection — persisted to localStorage. */
  export function SelectedPoolProvider({ children }: { children: ReactNode }) {
    const [selected, setSelectedState] = useState<string | null | undefined>(
      readSelectedPool,
    )

    const value = useMemo<SelectedPoolState>(
      () => ({
        selected,
        setSelected: (poolId: string | null) => {
          writeSelectedPool(poolId)
          setSelectedState(poolId)
        },
      }),
      [selected],
    )

    return (
      <SelectedPoolContext.Provider value={value}>
        {children}
      </SelectedPoolContext.Provider>
    )
  }
  ```
- [ ] **Step 3: Create the hook.** Create `web/src/pools/useSelectedPool.ts`:
  ```typescript
  import { useContext } from 'react'
  import {
    SelectedPoolContext,
    type SelectedPoolState,
  } from './selectedPoolContextValue'

  export function useSelectedPool(): SelectedPoolState {
    const ctx = useContext(SelectedPoolContext)
    if (!ctx) {
      throw new Error('useSelectedPool must be used within SelectedPoolProvider')
    }
    return ctx
  }
  ```
- [ ] **Step 4: Mount the provider in `main.tsx`.** Read `web/src/main.tsx`, then add the import and wrap `<App />`. Add after line 11 (`import { ThemeProvider }...`):
  ```typescript
  import { SelectedPoolProvider } from './pools/SelectedPoolProvider'
  ```
  Then wrap inside `<GraphqlProvider>` (the selection itself needs no GraphQL, but keeping it innermost-but-one keeps it close to the pages). Change:
  ```tsx
              <GraphqlProvider>
                <BrowserRouter>
                  <App />
                </BrowserRouter>
              </GraphqlProvider>
  ```
  to:
  ```tsx
              <GraphqlProvider>
                <SelectedPoolProvider>
                  <BrowserRouter>
                    <App />
                  </BrowserRouter>
                </SelectedPoolProvider>
              </GraphqlProvider>
  ```
- [ ] **Step 5: Type-check + build.** Verify the provider tree compiles:
  ```sh
  cd web && npm run build
  ```
  Expected: `tsc -b && vite build` succeed (no unused-import or type errors).
- [ ] **Step 6: Commit:**
  ```sh
  git add web/src/pools/ web/src/main.tsx
  git commit -m "feat(web): sticky SelectedPoolProvider context + hook"
  ```

---

## Task 3 — Shared `<PoolSelector>` component

One picker component reused by all three pages. It owns nothing — it reads `useSelectedPool` and the viewer's pools (passed in), renders the native `<select>`, and writes the explicit selection. The displayed value is the **effective** selection so the default (first pool) shows selected on first load.

**Files:**
- Create `web/src/pools/PoolSelector.tsx`

- [ ] **Step 1: Implement the component.** Create `web/src/pools/PoolSelector.tsx`:
  ```typescript
  import { useI18n } from '../i18n/useI18n'
  import type { Pool } from '../graphql/types'
  import { effectiveSelectedPool } from '../lib/selectedPool'
  import { useSelectedPool } from './useSelectedPool'

  /**
   * The shared pool picker (`<label className="pool-selector">`) used by the
   * Scoreboard, All Tips and Perfect pages. Empty option = "everyone" (global);
   * a pool option scopes the listing. Selection is sticky across pages via
   * `useSelectedPool`. `pools` is the viewer's pool list (empty for a visitor).
   */
  export function PoolSelector({ pools }: { pools: Pool[] }) {
    const { t } = useI18n()
    const { selected, setSelected } = useSelectedPool()
    const effective = effectiveSelectedPool(
      selected,
      pools.map((p) => p.id),
    )

    return (
      <label className="pool-selector">
        {t('pool')}:{' '}
        <select
          value={effective ?? ''}
          onChange={(e) => setSelected(e.target.value || null)}
        >
          <option value="">{t('everyone')}</option>
          {pools.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
    )
  }
  ```
- [ ] **Step 2: Build — expect PASS:**
  ```sh
  cd web && npm run build
  ```
  Expected: green (component compiles; not yet mounted, so no behaviour change).
- [ ] **Step 3: Commit:**
  ```sh
  git add web/src/pools/PoolSelector.tsx
  git commit -m "feat(web): shared PoolSelector component"
  ```

---

## Task 4 — Migrate ScoreboardPage onto the shared selection

Replace the local `useState` + inline `<select>` with `useSelectedPool` + `<PoolSelector>`. The query variable becomes the **effective** selection.

**Files:**
- Modify `web/src/pages/ScoreboardPage.tsx` (lines 1-50 imports/state; lines 80-93 the inline selector)

- [ ] **Step 1: Replace state with the shared hook.** Read `web/src/pages/ScoreboardPage.tsx`. Replace the local pool state block (lines 22-39 area) so the resolved variable comes from the shared selection. Update imports: add
  ```typescript
  import { PoolSelector } from '../pools/PoolSelector'
  import { useSelectedPool } from '../pools/useSelectedPool'
  import { effectiveSelectedPool } from '../lib/selectedPool'
  ```
  Remove the now-unused `useState` import if nothing else uses it (keep `useMemo`). Replace:
  ```typescript
    const [poolId, setPoolId] = useState<string | null | undefined>(undefined)
  ```
  with:
  ```typescript
    const { selected } = useSelectedPool()
  ```
  and replace:
  ```typescript
    const effectivePool = poolId === undefined ? (pools[0]?.id ?? null) : poolId
  ```
  with:
  ```typescript
    const effectivePool = effectiveSelectedPool(
      selected,
      pools.map((p) => p.id),
    )
  ```
- [ ] **Step 2: Replace the inline selector markup.** Replace the whole `<label className="pool-selector">…</label>` block (the JSX around lines 80-93) with:
  ```tsx
        <PoolSelector pools={pools} />
  ```
- [ ] **Step 3: Build + lint — expect PASS:**
  ```sh
  cd web && npm run build && npm run lint
  ```
  Expected: green (no unused `setPoolId`/`useState`).
- [ ] **Step 4: Commit:**
  ```sh
  git add web/src/pages/ScoreboardPage.tsx
  git commit -m "refactor(web): scoreboard uses shared sticky pool selection"
  ```

---

## Task 5 — Migrate AllTipsPage onto the shared selection

Same migration as Task 4. AllTipsPage keeps its `useState` import (it uses `selectedRound`/`selectedGroupId`), so only the pool state changes.

**Files:**
- Modify `web/src/pages/AllTipsPage.tsx` (lines 1-47 imports/state; lines 145-158 the inline selector)

- [ ] **Step 1: Swap the pool state.** Read `web/src/pages/AllTipsPage.tsx`. Add imports:
  ```typescript
  import { PoolSelector } from '../pools/PoolSelector'
  import { useSelectedPool } from '../pools/useSelectedPool'
  import { effectiveSelectedPool } from '../lib/selectedPool'
  ```
  Replace:
  ```typescript
    const [poolId, setPoolId] = useState<string | null | undefined>(undefined)
  ```
  with:
  ```typescript
    const { selected } = useSelectedPool()
  ```
  and replace:
  ```typescript
    const effectivePool = poolId === undefined ? (pools[0]?.id ?? null) : poolId
  ```
  with:
  ```typescript
    const effectivePool = effectiveSelectedPool(
      selected,
      pools.map((p) => p.id),
    )
  ```
- [ ] **Step 2: Replace the inline selector markup.** Replace the `<label className="pool-selector">…</label>` block (around lines 145-158) with:
  ```tsx
        <PoolSelector pools={pools} />
  ```
- [ ] **Step 3: Build + lint — expect PASS:**
  ```sh
  cd web && npm run build && npm run lint
  ```
  Expected: green.
- [ ] **Step 4: Commit:**
  ```sh
  git add web/src/pages/AllTipsPage.tsx
  git commit -m "refactor(web): all-tips uses shared sticky pool selection"
  ```

---

## Task 6 — Rust: add `pool: ID` to the `perfects` resolver

Mirror the scoreboard/tips pool-membership filter: load the viewer's pool membership, require membership/ownership, and restrict perfects to that pool's members. `None` = everyone (current behaviour, no auth required). This is a Rust change → **work on a branch/worktree** (CLAUDE.md branch discipline; `crates/*` source).

**Files:**
- Modify `crates/api/src/gql/query.rs` (the `perfects` resolver, lines 404-450; add a test module at end of file)

- [ ] **Step 1: Write the failing API integration test.** Append a test module at the end of `crates/api/src/gql/query.rs` (after the `match_tests` module, before EOF). It builds an in-memory repo with a result-user, two players who each score a perfect on `M1`, and a pool containing only one of them; asserts the pool filter drops the non-member's perfect, and that omitting `pool` returns both:
  ```rust
  #[cfg(test)]
  mod perfects_tests {
      use crate::auth::CurrentPlayer;
      use crate::reported::ReportedResultSource;
      use async_trait::async_trait;
      use chrono::{TimeZone, Utc};
      use domain::{
          GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Pool, Round, SingleGame,
          Team, TeamSlot, Tournament,
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
          Team {
              id: id.into(),
              name: id.into(),
              short_code: id.into(),
              flag: None,
              external_id: None,
          }
      }

      fn player_with(id: &str, h: u8, a: u8, is_result_user: bool) -> Player {
          Player {
              id: id.into(),
              person_id: format!("p-{id}"),
              nick: id.into(),
              full_name: id.into(),
              referrer: None,
              is_result_user,
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

      async fn repo_with_two_perfects() -> InMemoryRepository {
          let game = SingleGame {
              id: "M1".into(),
              kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 18, 0, 0).unwrap(),
              venue: None,
              group_id: "A".into(),
              home: TeamSlot {
                  team_id: Some("AAA".into()),
                  description: "A1".into(),
              },
              away: TeamSlot {
                  team_id: Some("BBB".into()),
                  description: "A2".into(),
              },
              external_id: None,
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
          // Official result 2–1; both players predicted 2–1 → both perfect.
          repo.put_player(&player_with("result-user", 2, 1, true))
              .await
              .unwrap();
          repo.put_player(&player_with("alice", 2, 1, false))
              .await
              .unwrap();
          repo.put_player(&player_with("bob", 2, 1, false))
              .await
              .unwrap();
          repo
      }

      async fn exec(repo: InMemoryRepository, viewer: Player, query: &str) -> serde_json::Value {
          let repo: Arc<dyn Repository> = Arc::new(repo);
          let source: Arc<dyn ReportedResultSource> = Arc::new(NoSource);
          let schema = crate::gql::build_schema(repo, source);
          let req = async_graphql::Request::new(query)
              .data(CurrentPlayer::Player(Box::new(viewer)))
              .data(crate::clock::RequestNow(
                  "2026-06-12T12:00:00Z".parse().unwrap(),
              ));
          let resp = schema.execute(req).await;
          assert!(resp.errors.is_empty(), "{:?}", resp.errors);
          resp.data.into_json().unwrap()
      }

      #[tokio::test]
      async fn no_pool_returns_every_perfect() {
          let repo = repo_with_two_perfects().await;
          let data = exec(
              repo,
              player_with("alice", 2, 1, false),
              r#"{ perfects { playerId gameId } }"#,
          )
          .await;
          let ids: Vec<String> = data["perfects"]
              .as_array()
              .unwrap()
              .iter()
              .map(|p| p["playerId"].as_str().unwrap().to_string())
              .collect();
          assert!(ids.contains(&"alice".to_string()));
          assert!(ids.contains(&"bob".to_string()));
      }

      #[tokio::test]
      async fn pool_filter_restricts_perfects_to_members() {
          let repo = repo_with_two_perfects().await;
          // Pool P1 contains only alice (the viewer); bob is excluded.
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
              player_with("alice", 2, 1, false),
              r#"{ perfects(pool:"P1") { playerId gameId } }"#,
          )
          .await;
          let ids: Vec<String> = data["perfects"]
              .as_array()
              .unwrap()
              .iter()
              .map(|p| p["playerId"].as_str().unwrap().to_string())
              .collect();
          assert!(ids.contains(&"alice".to_string()), "alice (member) shown");
          assert!(!ids.contains(&"bob".to_string()), "bob (non-member) hidden");
      }

      #[tokio::test]
      async fn pool_filter_requires_membership() {
          let repo = repo_with_two_perfects().await;
          // Pool P2 does NOT contain bob — bob asking to scope to it is rejected.
          repo.put_pool(&Pool {
              id: "P2".into(),
              name: "Pool 2".into(),
              owner: "alice".into(),
              members: vec!["alice".into()],
              prefix: "P2".into(),
          })
          .await
          .unwrap();
          let repo: Arc<dyn Repository> = Arc::new(repo);
          let source: Arc<dyn ReportedResultSource> = Arc::new(NoSource);
          let schema = crate::gql::build_schema(repo, source);
          let req = async_graphql::Request::new(r#"{ perfects(pool:"P2") { playerId } }"#)
              .data(CurrentPlayer::Player(Box::new(player_with("bob", 2, 1, false))))
              .data(crate::clock::RequestNow(
                  "2026-06-12T12:00:00Z".parse().unwrap(),
              ));
          let resp = schema.execute(req).await;
          assert!(!resp.errors.is_empty(), "non-member must be rejected");
      }
  }
  ```
- [ ] **Step 2: Run the test — expect FAIL** (resolver has no `pool` arg, so `perfects(pool:"P1")` is an unknown-argument error and the pool-filter assertions fail):
  ```sh
  cargo test -p api perfects_tests
  ```
  Expected: compile/schema error on the `pool` argument or failing assertions.
- [ ] **Step 3: Add the `pool` argument + membership filter.** In `crates/api/src/gql/query.rs`, change the `perfects` signature and body. Replace:
  ```rust
      /// Every "perfect" (maximum-scoring) prediction across all players (UC-10).
      async fn perfects(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Perfect>> {
          let repo = repo(ctx);
          let players = repo.list_players().await?;
          let config = ScoringConfig::default();
  ```
  with:
  ```rust
      /// Every "perfect" (maximum-scoring) prediction across all players (UC-10),
      /// optionally scoped to a pool's members. A pool filter is private — it
      /// requires the viewer to be a member (or owner) of that pool, mirroring
      /// `scoreboard` / `tips` (Issue 04). `None` = the global listing (public).
      async fn perfects(
          &self,
          ctx: &Context<'_>,
          pool: Option<String>,
      ) -> async_graphql::Result<Vec<Perfect>> {
          let repo = repo(ctx);
          let players = repo.list_players().await?;
          let config = ScoringConfig::default();

          // Optional pool scoping — same private-membership rule as `scoreboard`.
          let allowed: Option<Vec<String>> = match &pool {
              Some(pool_id) => {
                  let viewer = CurrentPlayer::require(ctx)?;
                  let pools = repo.list_pools().await?;
                  let p = pools
                      .into_iter()
                      .find(|p| &p.id == pool_id)
                      .ok_or_else(|| async_graphql::Error::new("pool not found"))?;
                  if !p.members.contains(&viewer.id) && p.owner != viewer.id {
                      return Err(async_graphql::Error::new(
                          "you are not a member of this pool",
                      ));
                  }
                  Some(p.members)
              }
              None => None,
          };
  ```
- [ ] **Step 4: Apply the filter in the player loop.** In the same resolver, change the loop guard. Replace:
  ```rust
          let mut perfects = Vec::new();
          for player in &players {
              if player.is_result_user {
                  continue;
              }
  ```
  with:
  ```rust
          let mut perfects = Vec::new();
          for player in &players {
              if player.is_result_user {
                  continue;
              }
              if allowed.as_ref().is_some_and(|m| !m.contains(&player.id)) {
                  continue;
              }
  ```
- [ ] **Step 5: Run the test — expect PASS:**
  ```sh
  cargo test -p api perfects_tests
  ```
  Expected: `no_pool_returns_every_perfect`, `pool_filter_restricts_perfects_to_members`, `pool_filter_requires_membership` all green.
- [ ] **Step 6: Clippy — expect clean:**
  ```sh
  cargo clippy -p api -- -D warnings
  ```
  Expected: no warnings.
- [ ] **Step 7: Commit:**
  ```sh
  git add crates/api/src/gql/query.rs
  git commit -m "feat(api): pool-scope the perfects resolver"
  ```

---

## Task 7 — Wire the `pool` argument into the `PERFECTS_QUERY` document

**Files:**
- Modify `web/src/graphql/queries.ts` (lines 101-108, `PERFECTS_QUERY`)

- [ ] **Step 1: Add the `$pool` variable.** Replace:
  ```typescript
  export const PERFECTS_QUERY = `
    query Perfects {
      perfects {
        playerId nick gameId points
        breakdown { exactHome exactAway outcome base multiplier points }
      }
    }
  `
  ```
  with:
  ```typescript
  export const PERFECTS_QUERY = `
    query Perfects($pool: ID) {
      perfects(pool: $pool) {
        playerId nick gameId points
        breakdown { exactHome exactAway outcome base multiplier points }
      }
    }
  `
  ```
- [ ] **Step 2: Build — expect PASS:**
  ```sh
  cd web && npm run build
  ```
  Expected: green (PerfectPage still calls the query without variables — `$pool` is optional; wired in Task 9).
- [ ] **Step 3: Commit:**
  ```sh
  git add web/src/graphql/queries.ts
  git commit -m "feat(web): PERFECTS_QUERY accepts an optional pool arg"
  ```

---

## Task 8 — Pure helper: order perfects by-match vs by-player

A pure, unit-testable reorder of the **flat** perfects list. By-match (default): kickoff asc, tie-broken by nick (preserves today's behaviour). By-player: players ordered by perfect-count desc (stable tie-break by first appearance), each player's perfects kept contiguous and internally kickoff-ordered.

**Files:**
- Create `web/src/lib/perfectOrder.ts`
- Create `web/src/lib/perfectOrder.test.ts`

- [ ] **Step 1: Write the failing test.** Create `web/src/lib/perfectOrder.test.ts`:
  ```typescript
  import { describe, expect, it } from 'vitest'
  import type { Perfect } from '../graphql/types'
  import { orderPerfects, type PerfectView } from './perfectOrder'

  const bd = {
    exactHome: true,
    exactAway: true,
    outcome: true,
    base: 4,
    multiplier: 1,
    points: 4,
  }
  const p = (playerId: string, nick: string, gameId: string): Perfect => ({
    playerId,
    nick,
    gameId,
    points: 4,
    breakdown: bd,
  })

  // ada: 2 perfects (g1, g3); bob: 1 perfect (g2). Kickoffs g1<g2<g3.
  const list: Perfect[] = [p('bob', 'Bob', 'g2'), p('ada', 'Ada', 'g3'), p('ada', 'Ada', 'g1')]
  const kickoff = new Map<string, number>([
    ['g1', 100],
    ['g2', 200],
    ['g3', 300],
  ])

  describe('orderPerfects by-match (default)', () => {
    it('orders by kickoff asc, tie-broken by nick', () => {
      const out = orderPerfects(list, 'match', kickoff)
      expect(out.map((x) => x.gameId)).toEqual(['g1', 'g2', 'g3'])
    })
    it('does not mutate the input', () => {
      const copy = [...list]
      orderPerfects(list, 'match', kickoff)
      expect(list).toEqual(copy)
    })
  })

  describe('orderPerfects by-player', () => {
    it('groups each player contiguously, players by perfect-count desc', () => {
      const out = orderPerfects(list, 'player', kickoff)
      // ada (2 perfects) before bob (1); ada's own perfects kickoff-ordered.
      expect(out.map((x) => `${x.playerId}:${x.gameId}`)).toEqual([
        'ada:g1',
        'ada:g3',
        'bob:g2',
      ])
    })
    it('breaks count ties by first appearance (stable)', () => {
      // cy and dan each have 1 perfect; cy appears first in the input.
      const tied: Perfect[] = [p('cy', 'Cy', 'g2'), p('dan', 'Dan', 'g1')]
      const out = orderPerfects(tied, 'player', kickoff)
      expect(out.map((x) => x.playerId)).toEqual(['cy', 'dan'])
    })
  })
  ```
- [ ] **Step 2: Run the test — expect FAIL** (module missing):
  ```sh
  cd web && npx vitest run src/lib/perfectOrder.test.ts
  ```
  Expected: `Failed to resolve import "./perfectOrder"`.
- [ ] **Step 3: Implement the helper.** Create `web/src/lib/perfectOrder.ts`:
  ```typescript
  import type { Perfect } from '../graphql/types'

  /** The two ways the Perfect page can order its flat list. */
  export type PerfectView = 'match' | 'player'

  /** Kickoff epoch for a perfect's game; missing games sort last. */
  function kickoffOf(p: Perfect, kickoff: Map<string, number>): number {
    return kickoff.get(p.gameId) ?? Infinity
  }

  /** By-match: kickoff asc, tie-broken by nick so a match's perfects stay grouped. */
  function byMatch(list: Perfect[], kickoff: Map<string, number>): Perfect[] {
    return [...list].sort(
      (a, b) =>
        kickoffOf(a, kickoff) - kickoffOf(b, kickoff) || a.nick.localeCompare(b.nick),
    )
  }

  /**
   * By-player: each player's perfects contiguous and kickoff-ordered; players
   * ordered by perfect-count desc, ties broken by first appearance (stable).
   */
  function byPlayer(list: Perfect[], kickoff: Map<string, number>): Perfect[] {
    // Preserve first-appearance order while grouping, in one pass.
    const order: string[] = []
    const groups = new Map<string, Perfect[]>()
    for (const perfect of list) {
      const bucket = groups.get(perfect.playerId)
      if (bucket) {
        bucket.push(perfect)
      } else {
        order.push(perfect.playerId)
        groups.set(perfect.playerId, [perfect])
      }
    }

    return order
      .map((playerId, index) => ({ playerId, index }))
      // Most perfects first; equal counts keep first-appearance order (stable).
      .sort(
        (a, b) =>
          (groups.get(b.playerId)?.length ?? 0) -
            (groups.get(a.playerId)?.length ?? 0) || a.index - b.index,
      )
      .flatMap(({ playerId }) =>
        [...(groups.get(playerId) ?? [])].sort(
          (a, b) => kickoffOf(a, kickoff) - kickoffOf(b, kickoff),
        ),
      )
  }

  /** Reorder the flat perfects list for the chosen view. Never mutates input. */
  export function orderPerfects(
    list: Perfect[],
    view: PerfectView,
    kickoff: Map<string, number>,
  ): Perfect[] {
    return view === 'player' ? byPlayer(list, kickoff) : byMatch(list, kickoff)
  }
  ```
- [ ] **Step 4: Run the test — expect PASS:**
  ```sh
  cd web && npx vitest run src/lib/perfectOrder.test.ts
  ```
  Expected: all 4 assertions green.
- [ ] **Step 5: Commit:**
  ```sh
  git add web/src/lib/perfectOrder.ts web/src/lib/perfectOrder.test.ts
  git commit -m "feat(web): pure by-match/by-player perfects ordering helper"
  ```

---

## Task 9 — Wire PerfectPage: sticky pool picker + view toggle

Mount `<PoolSelector>`, scope the perfects query to the effective pool, persist the view mode, and reorder via `orderPerfects`. Pools query is auth-gated and `pause`d for visitors (PerfectPage is public), so a visitor sees the global list with the picker showing only "Everyone".

**Files:**
- Modify `web/src/pages/PerfectPage.tsx` (full rewrite of the component, lines 1-115)
- Modify `web/src/i18n/strings.ts` (add `perfectByMatch`/`perfectByPlayer` keys to `en` ~line 187 and `hu` ~line 480)

- [ ] **Step 1: Add i18n keys.** In `web/src/i18n/strings.ts`, under the `// perfect` block in `en` (after `perfectEmpty`, ~line 187) add:
  ```typescript
    perfectByMatch: 'By match',
    perfectByPlayer: 'By player',
  ```
  and in `hu` (after the `perfectEmpty` Hungarian line, ~line 480) add:
  ```typescript
    perfectByMatch: 'Meccs szerint',
    perfectByPlayer: 'Játékos szerint',
  ```
- [ ] **Step 2: Add the view-mode storage constant + reader.** Append to `web/src/lib/perfectOrder.ts` (so view persistence lives with the ordering logic):
  ```typescript

  /** localStorage key for the persisted Perfect-page view mode. */
  export const PERFECT_VIEW_KEY = 'xpool.perfectView'

  /** Read the persisted view, defaulting to by-match. Total + failure-safe. */
  export function readPerfectView(): PerfectView {
    try {
      return localStorage.getItem(PERFECT_VIEW_KEY) === 'player' ? 'player' : 'match'
    } catch {
      return 'match'
    }
  }

  /** Persist the chosen view. Convenience state — failures are swallowed. */
  export function writePerfectView(view: PerfectView): void {
    try {
      localStorage.setItem(PERFECT_VIEW_KEY, view)
    } catch {
      /* ignore */
    }
  }
  ```
- [ ] **Step 3: Extend the test for the view persistence.** Append to `web/src/lib/perfectOrder.test.ts`:
  ```typescript
  import { afterEach } from 'vitest'
  import {
    PERFECT_VIEW_KEY,
    readPerfectView,
    writePerfectView,
  } from './perfectOrder'

  afterEach(() => localStorage.clear())

  describe('perfect view persistence', () => {
    it('defaults to by-match when nothing is stored', () => {
      expect(readPerfectView()).toBe('match')
    })
    it('round-trips by-player', () => {
      writePerfectView('player')
      expect(localStorage.getItem(PERFECT_VIEW_KEY)).toBe('player')
      expect(readPerfectView()).toBe('player')
    })
  })
  ```
  Run — expect PASS:
  ```sh
  cd web && npx vitest run src/lib/perfectOrder.test.ts
  ```
  Expected: 6 assertions green.
- [ ] **Step 4: Rewrite PerfectPage.** Replace the entire contents of `web/src/pages/PerfectPage.tsx` with:
  ```typescript
  import { useMemo, useState, type ReactNode } from 'react'
  import { Link } from 'react-router-dom'
  import { useQuery } from 'urql'
  import { useAuth } from '../auth/useAuth'
  import { useI18n } from '../i18n/useI18n'
  import {
    PERFECTS_QUERY,
    POOLS_QUERY,
    RESULTS_QUERY,
    TOURNAMENT_QUERY,
  } from '../graphql/queries'
  import type { MatchPrediction, Perfect, Pool, Tournament } from '../graphql/types'
  import { ErrorView, Loading } from '../components/StatusViews'
  import { teamIndex } from '../lib/format'
  import { Matchup } from '../components/TeamLabel'
  import { PointsBadge } from '../components/PointsBadge'
  import { PoolSelector } from '../pools/PoolSelector'
  import { useSelectedPool } from '../pools/useSelectedPool'
  import { effectiveSelectedPool } from '../lib/selectedPool'
  import {
    orderPerfects,
    readPerfectView,
    writePerfectView,
    type PerfectView,
  } from '../lib/perfectOrder'

  /** Players who scored a maximum (4-point) match prediction (UC-10). Public. */
  export function PerfectPage() {
    const { t, locale } = useI18n()
    const { label } = useAuth()
    const { selected } = useSelectedPool()
    const [view, setView] = useState<PerfectView>(readPerfectView)

    // Pools require auth; PerfectPage is public, so pause for visitors — they
    // see the global list (effectivePool resolves to null with no pools).
    const [poolsResult] = useQuery<{ pools: Pool[] }>({
      query: POOLS_QUERY,
      pause: !label,
    })
    const pools = poolsResult.data?.pools ?? []
    const effectivePool = effectiveSelectedPool(
      selected,
      pools.map((p) => p.id),
    )

    const [result, reexecute] = useQuery<{ perfects: Perfect[] }>({
      query: PERFECTS_QUERY,
      variables: { pool: effectivePool },
    })
    const [tournamentResult] = useQuery<{
      tournament: Tournament | null
    }>({ query: TOURNAMENT_QUERY })
    const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
      query: RESULTS_QUERY,
    })

    const tournament = tournamentResult.data?.tournament ?? null
    const teams = useMemo(
      () => teamIndex(tournament?.teams ?? [], locale),
      [tournament, locale],
    )
    const gameLabel = useMemo(() => {
      const map = new Map<string, ReactNode>()
      for (const g of tournament?.games ?? []) {
        map.set(
          g.id,
          <Link to={`/match/${g.id}`}>
            <Matchup home={g.home} away={g.away} teams={teams} />
          </Link>,
        )
      }
      return map
    }, [tournament, teams])
    // gameId -> kickoff epoch, for the ordering helper (server-provided times;
    // Date.parse is formatting a server timestamp, not a behavioural clock read).
    const kickoffOf = useMemo(() => {
      const map = new Map<string, number>()
      for (const g of tournament?.games ?? []) {
        map.set(g.id, Date.parse(g.kickoff))
      }
      return map
    }, [tournament])
    const resultByGame = useMemo(() => {
      const map = new Map<string, MatchPrediction>()
      for (const r of resultsResult.data?.results ?? []) {
        map.set(r.gameId, r)
      }
      return map
    }, [resultsResult.data])

    const perfects = useMemo(
      () => orderPerfects(result.data?.perfects ?? [], view, kickoffOf),
      [result.data, view, kickoffOf],
    )

    const chooseView = (next: PerfectView) => {
      writePerfectView(next)
      setView(next)
    }

    if (result.fetching) return <Loading />
    if (result.error)
      return (
        <ErrorView
          message={result.error.message}
          onRetry={() => reexecute({ requestPolicy: 'network-only' })}
        />
      )

    return (
      <section className="page">
        <h2>{t('perfectTitle')}</h2>
        <p>{t('perfectIntro')}</p>

        <PoolSelector pools={pools} />

        <div className="seg-toggle" role="group" aria-label={t('perfectTitle')}>
          <button
            type="button"
            className={`seg-option${view === 'match' ? ' is-active' : ''}`}
            aria-pressed={view === 'match'}
            onClick={() => chooseView('match')}
          >
            {t('perfectByMatch')}
          </button>
          <button
            type="button"
            className={`seg-option${view === 'player' ? ' is-active' : ''}`}
            aria-pressed={view === 'player'}
            onClick={() => chooseView('player')}
          >
            {t('perfectByPlayer')}
          </button>
        </div>

        {perfects.length === 0 ? (
          <p>{t('perfectEmpty')}</p>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>{t('player')}</th>
                <th className="col-match">{t('match')}</th>
                <th>{t('result')}</th>
                <th>{t('points')}</th>
              </tr>
            </thead>
            <tbody>
              {perfects.map((p, i) => {
                const r = resultByGame.get(p.gameId)
                return (
                  <tr key={`${p.playerId}-${p.gameId}-${i}`}>
                    <td>
                      <Link to={`/player/${p.playerId}`}>{p.nick}</Link>
                    </td>
                    <td>{gameLabel.get(p.gameId) ?? p.gameId}</td>
                    <td>{r ? `${r.homeScore}–${r.awayScore}` : '—'}</td>
                    <td>
                      <PointsBadge breakdown={p.breakdown} isPerfect />
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        )}
      </section>
    )
  }
  ```
- [ ] **Step 5: Reuse the existing segmented-toggle CSS (no new CSS).** The toggle markup uses the battle-tested shared `.seg-toggle` / `.seg-option` / `.is-active` family already defined in `web/src/index.css` (~line 1286) — the same one the language/display/theme pickers use. The active option renders `background: var(--accent); color: var(--accent-ink)`. Verify the toggle renders visibly in the next step; add a `margin` wrapper only if it crowds the table (optional — the shared component already reads as one family). Do **not** invent new class names or CSS variables.
- [ ] **Step 6: Build + lint — expect PASS:**
  ```sh
  cd web && npm run build && npm run lint
  ```
  Expected: green.
- [ ] **Step 7: Commit:**
  ```sh
  git add web/src/pages/PerfectPage.tsx web/src/lib/perfectOrder.ts web/src/lib/perfectOrder.test.ts web/src/i18n/strings.ts
  git commit -m "feat(web): perfect page sticky pool picker + by-player view"
  ```

---

## Task 10 — E2E: pool scoping + by-player toggle reorders

One Playwright spec proving the two behaviours on the wire. Uses seeded demo players + result-user, the dev-stub auth bar, and watches GraphQL traffic. The default seeded "Demo Pool" holds all six demo players; to prove scoping we create a private pool with a single member and assert the perfects list shrinks to that member.

**Files:**
- Create `web/e2e/perfect-page.spec.ts`
- Verify `web/.env.local` blanks `VITE_AUTH0_*` (already present — confirm, do not recreate)

- [ ] **Step 1: Confirm dev-stub auth is enabled for e2e.** Read `web/.env.local`; confirm it blanks `VITE_AUTH0_DOMAIN`, `VITE_AUTH0_CLIENT_ID`, `VITE_AUTH0_AUDIENCE` (the auth bar/dev-login only appears in dev-stub mode). If missing, create it with those three vars blank.
- [ ] **Step 2: Write the e2e spec.** Create `web/e2e/perfect-page.spec.ts`:
  ```typescript
  import { test, expect } from '@playwright/test'
  import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

  /**
   * Perfect page (UC-10) end to end: the sticky pool picker scopes the perfects
   * list, and the by-player toggle reorders the flat list. Exercises the
   * `perfects(pool:)` argument and the SelectedPool context on the wire. The
   * e2e DynamoDB persists across runs, so the scoping test creates a uniquely
   * named single-member pool rather than relying on the shared seeded data.
   */

  test('by-player toggle reorders the perfects list', async ({ page }) => {
    const net = watchNetwork(page)
    await page.goto('/')
    await devLogin(page, 'demo-ada')
    await page.locator('.nav-bar').getByRole('link', { name: 'Perfect' }).click()
    await expect(page).toHaveURL(/\/perfect$/)

    // Scope to "Everyone" so the toggle has multiple players to reorder.
    await page.locator('.pool-selector select').selectOption('')

    // Default view is by-match. Capture the player-column order.
    const matchOrder = await page.locator('.data-table tbody tr td:first-child').allInnerTexts()

    // Switch to by-player and re-read. The page may have no perfects yet (the
    // seeded clock can precede results) — only assert reordering when rows exist.
    await page.getByRole('button', { name: 'By player' }).click()
    await expect(page.getByRole('button', { name: 'By player' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    const playerOrder = await page.locator('.data-table tbody tr td:first-child').allInnerTexts()

    if (matchOrder.length > 1) {
      // By-player groups each nick contiguously: no nick appears in two
      // non-adjacent blocks.
      const seen = new Set<string>()
      let prev = ''
      for (const nick of playerOrder) {
        if (nick !== prev) {
          expect(seen.has(nick), `${nick} should be contiguous`).toBe(false)
          seen.add(nick)
          prev = nick
        }
      }
    }

    await expectNoErrorView(page)
    await net.assertNoGraphqlErrors()
    net.assertNoPageErrors()
  })

  test('selecting a single-member pool scopes the perfects list', async ({ page }) => {
    const net = watchNetwork(page)
    const poolName = `Perfect Scope ${Date.now()}`
    await page.goto('/')
    await devLogin(page, 'demo-grace')

    // Create a fresh pool — grace is its sole member (a strict subset of all).
    await page.goto('/pools')
    await page.getByPlaceholder('e.g. Office League').fill(poolName)
    await page.getByRole('button', { name: 'Create pool' }).click()
    await expect(page.locator('.pool-card', { hasText: poolName })).toBeVisible()

    await page.locator('.nav-bar').getByRole('link', { name: 'Perfect' }).click()
    await expect(page).toHaveURL(/\/perfect$/)

    // Everyone: count the distinct players with perfects.
    await page.locator('.pool-selector select').selectOption('')
    const everyoneNicks = await page
      .locator('.data-table tbody tr td:first-child')
      .allInnerTexts()

    // Scope to grace's solo pool: every row must be grace (or the table empty).
    await page.locator('.pool-selector select').selectOption({ label: poolName })
    const scopedNicks = new Set(
      await page.locator('.data-table tbody tr td:first-child').allInnerTexts(),
    )
    // The scoped set is a subset of everyone, and never larger.
    expect(scopedNicks.size).toBeLessThanOrEqual(new Set(everyoneNicks).size)
    for (const nick of scopedNicks) {
      expect(everyoneNicks).toContain(nick)
    }

    await expectNoErrorView(page)
    await net.assertNoGraphqlErrors()
    net.assertNoPageErrors()
  })
  ```
- [ ] **Step 3: Run the e2e spec (boots its own stack):**
  ```sh
  cd web && npm run e2e -- perfect-page
  ```
  Expected: both tests green; `watchNetwork` confirms `perfects(pool:)` went over the wire as POST with no GraphQL errors.
- [ ] **Step 4: Commit:**
  ```sh
  git add web/e2e/perfect-page.spec.ts
  git commit -m "test(e2e): perfect page pool scoping + by-player toggle"
  ```

---

## Task 11 — Verification (evidence before assertions)

Run the full completion bar and confirm green output **before** claiming done. Do not assert success without the command output.

**Files:** none (verification only).

- [ ] **Step 1: Rust workspace builds, lints, tests.** Run and confirm each is green:
  ```sh
  cargo build --workspace
  cargo clippy --workspace -- -D warnings
  cargo test --workspace
  ```
  Expected: build clean; clippy no warnings; tests pass (DynamoDB integration tests skip without `DYNAMO_TEST=1` — that is expected, the suite stays green).
- [ ] **Step 2: Frontend builds + lints + unit tests.**
  ```sh
  cd web && npm run build && npm run lint && npx vitest run
  ```
  Expected: `tsc -b && vite build` green; eslint clean; all vitest suites pass (including `selectedPool.test.ts` and `perfectOrder.test.ts`).
- [ ] **Step 3: E2E green.**
  ```sh
  cd web && npm run e2e -- perfect-page
  ```
  Expected: both Perfect-page specs pass.
- [ ] **Step 4: Confirm no behaviour regression on the migrated pages.** Re-run the scoreboard/all-tips e2e to prove the shared-selection migration did not break them:
  ```sh
  cd web && npm run e2e -- scenario-scoreboard points-on-tips
  ```
  Expected: green.
- [ ] **Step 5: Spot-check no stray `Date.now()` / `new Date()` behaviour was introduced.** The only timestamp parse added is `Date.parse(g.kickoff)` for ordering a server-provided time (formatting, allowed by `.specs/TESTING.md` §3.3):
  ```sh
  cd web && grep -rn "Date.now\|new Date(" src/pages/PerfectPage.tsx src/pools/ src/lib/perfectOrder.ts src/lib/selectedPool.ts
  ```
  Expected: no matches (behavioural clock reads), confirming server-authoritative compliance.
- [ ] **Step 6: Merge + request code review.** Per CLAUDE.md branch discipline, the Rust change (Task 6) lives on a branch/worktree; merge it into `master` locally. Then **request code review** (this cluster touched the GraphQL contract and a shared context two other pages depend on — self-review-as-record is warranted; open a PR if CI gating adds value). State the evidence (the green command output from Steps 1-4) before claiming completion.

---

## Notes for the implementer

- **Branch discipline (CLAUDE.md):** any change under `crates/*` or `web/` goes on a branch/worktree, merged into `master` locally. This whole cluster is code, so do it on a branch/worktree from the start.
- **Tip-visibility gating** is unaffected: the `perfects` resolver scores only against the result-user's entered results (a perfect always has an official result), and pool scoping only filters *which players* appear — it never relaxes the mutual-commitment gate that governs `tips`.
- **Server-authoritative clock:** the SPA reads no behavioural clock. `Date.parse(g.kickoff)` is formatting/ordering a server timestamp (allowed). No `Date.now()` drives any branch.
- **DRY:** `effectiveSelectedPool` and `<PoolSelector>` are the single source of the picker across all three pages; do not reintroduce per-page `useState`/`<select>`.
