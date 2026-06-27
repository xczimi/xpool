# cluster/match-page (Wave 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three match-page features — sortable prediction columns, a "you" self-highlight, and live "what-if" (±1 goal) re-scoring columns — all client-side over the already-loaded, visibility-gated tip rows.

**Architecture:** All three live in `web/src/pages/MatchPage.tsx` and small focused helpers/components under `web/src/lib` and `web/src/components`. Sorting and what-if are pure functions unit-tested in isolation; the page wires them in. What-if re-runs the domain scoring rule (mirrored as a tiny TS helper) in the browser using each row's gated prediction and the live provisional score — **no new resolver**. Nothing reads `Date.now()`; every time-dependent branch is driven by server-derived data (`actual.provisional`, the per-row `prediction` gate). Features are committed sequentially within the one cluster branch.

**Tech Stack:** React + Vite + TypeScript, urql (POST-forced), Vitest (unit), Playwright (e2e), i18n via `web/src/i18n/strings.ts` (EN + HU).

---

## Background the worker needs

Read these before starting; they are the ground truth this plan was built on.

- **The page:** `web/src/pages/MatchPage.tsx`. The all-players grid is `<table className="data-table compact match-grid">`. Each `row` is a `Tip` (`web/src/graphql/types.ts`): `{ playerId, nick, prediction: MatchPrediction | null, points: number | null, isPerfect, breakdown, maxReachable }`. A still-hidden tip has `prediction === null` (the server gate) — **never** synthesize a prediction for those.
- **The gate:** the page already computes `gateOpen = rows.some((r) => r.playerId !== viewerId && r.prediction != null)`. That is the single source of "others' tips are revealable". Reuse it; do not invent a new gate and do not branch on the clock.
- **Viewer identity:** the page already derives `viewerId` from the `me` query (`meRaw.__typename === 'Player' ? meRaw.id : null`). Reuse it for the self-highlight.
- **Live state:** `isLive = match?.actual?.provisional ?? false`. During a live match the server populates `points`, `breakdown` (with `multiplier`), and `maxReachable` for visible rows. The e2e stack normally has `actual === null` (NullSource), **except** the M8 live stub (`XPOOL_LIVE_SCORES="2461105=1:0:2H"`) which makes `/match/M8` provisional in M8's live window — this is how `web/e2e/live-scoring.spec.ts` tests live behaviour, and how the what-if e2e will too.
- **Scoring rule (mirror target):** `crates/domain/src/scoring.rs` — per-side symmetric 4-goal rule. `exact_home = p.home == r.home || (p.home >= 4 && r.home >= 4)`; same for away; `outcome` = same sign of `home - away`. Points = `exact_home*1 + exact_away*1 + outcome*2`, then `* multiplier`. Defaults (`ScoringConfig`): exact=1, outcome=2, threshold=4. Round multipliers are already mirrored in the SPA as `STAGE_MULTIPLIERS` in `web/src/lib/rounds.ts` (`GROUP_STAGE:1 … FINAL:6`).
- **Multiplier for a match:** `tournament.groups.find((g) => g.id === game.groupId)?.round`, then `STAGE_MULTIPLIERS[round]`. The leaf group's `round` is correct for both group-stage and knockout (each KO match is its own one-match group).
- **Sticky pool:** `web/src/lib/selectedPool.ts` already persists pool choice to `localStorage`; the match page scopes rows via the `pool` query arg. What-if inherits that scope automatically (it operates on the already-pool-scoped `rows`).
- **i18n:** `web/src/i18n/strings.ts` — an `en` object (ends `} as const` at line ~375) and a `const hu: Record<StringKey, string>` block (starts ~line 377). Every key MUST be added to **both** blocks or `tsc -b` fails.
- **e2e auth:** dev-stub auth needs `web/.env.local` blanking `VITE_AUTH0_*` (otherwise Auth0 mode hides the dev auth-bar and ~10 specs fail). Task 0 verifies this exists.
- **Branch/table:** a worktree reads its own `xpool-<branch>` table; `bin/local-dev --reseed` reseeds if needed. e2e (`npm run e2e`) boots its own isolated stack — no manual seeding required for it.

---

## File structure

| File | Responsibility | Created/Modified |
| --- | --- | --- |
| `web/src/lib/matchSort.ts` | Pure sort model: `MatchSort` type, `sortRows`, `nextSort`, `readMatchSort`/`writeMatchSort` (localStorage) | Create |
| `web/src/lib/matchSort.test.ts` | Unit tests for the sort helper | Create |
| `web/src/lib/matchScoring.ts` | Pure per-match scoring mirroring `domain` (`scoreMatchBase`, `scoreMatchPoints`) | Create |
| `web/src/lib/matchScoring.test.ts` | Unit tests for the scoring helper | Create |
| `web/src/lib/whatIf.ts` | `computeWhatIf` — current + ifHome/ifAway totals & deltas | Create |
| `web/src/lib/whatIf.test.ts` | Unit tests for what-if | Create |
| `web/src/components/WhatIfCell.tsx` | Presentational cell: total + emphasised signed delta | Create |
| `web/src/pages/MatchPage.tsx` | Wire sort headers, self-highlight, what-if columns | Modify |
| `web/src/i18n/strings.ts` | New keys (EN + HU): `youBadge`, `ifHomeScores`, `ifAwayScores`, `whatIfHint` | Modify |
| `web/src/index.css` | Styles: sortable headers, `tr.is-self`, `.you-badge`, `.what-if*` | Modify |
| `web/e2e/match-page-sort.spec.ts` | e2e: sortable columns reorder rows | Create |
| `web/e2e/match-page-self-highlight.spec.ts` | e2e: own row marked `is-self` + "you" badge | Create |
| `web/e2e/match-page-what-if.spec.ts` | e2e: live what-if columns + deltas | Create |

---

## Task 0: Branch + baseline verification

**Files:** none (environment only)

- [ ] **Step 1: Ensure an isolated worktree/branch exists**

This is web-only work but still touches `web/` source, so it MUST live on a branch/worktree (never `master`). If not already in one, create it:

```bash
git -C /Users/xczimi/Private/SoccerPool/xpool worktree add .claude/worktrees/cluster-match-page -b cluster/match-page
```

Then do all remaining work inside that worktree directory.

- [ ] **Step 2: Confirm dev-stub auth env exists for e2e**

Run: `cat web/.env.local`
Expected: the file exists and blanks the Auth0 vars, e.g.:

```
VITE_AUTH0_DOMAIN=
VITE_AUTH0_CLIENT_ID=
VITE_AUTH0_AUDIENCE=
```

If the file is missing, create it with those three blank assignments (without it, dev-login e2e tests fail because the auth-bar is hidden).

- [ ] **Step 3: Establish a green baseline**

Run: `cd web && npm run build && npm run lint`
Expected: both PASS with no errors. (If this is already red before any change, stop and report — do not build on a broken base.)

- [ ] **Step 4: Workspace sanity (cluster is web-only, keep Rust green)**

Run: `cargo build --workspace && cargo clippy --workspace -- -D warnings`
Expected: PASS. No Rust changes are planned; this just confirms the baseline.

---

## Feature 1 — match-page-sort-predictions

### Task 1: Pure sort helper + tests

**Files:**
- Create: `web/src/lib/matchSort.ts`
- Test: `web/src/lib/matchSort.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `web/src/lib/matchSort.test.ts`:

```ts
import { describe, it, expect, beforeEach } from 'vitest'
import {
  sortRows,
  nextSort,
  readMatchSort,
  writeMatchSort,
  DEFAULT_MATCH_SORT,
  MATCH_SORT_KEY,
  type MatchSort,
} from './matchSort'
import type { Tip } from '../graphql/types'

function tip(over: Partial<Tip> & Pick<Tip, 'playerId' | 'nick'>): Tip {
  return {
    gameId: 'M1',
    prediction: null,
    points: null,
    isPerfect: false,
    breakdown: null,
    maxReachable: null,
    ...over,
  }
}

const ada = tip({ playerId: 'ada', nick: 'Ada', prediction: { gameId: 'M1', homeScore: 2, awayScore: 1, locked: true }, points: 1 })
const bob = tip({ playerId: 'bob', nick: 'Bob', prediction: { gameId: 'M1', homeScore: 1, awayScore: 0, locked: true }, points: 4 })
const cyd = tip({ playerId: 'cyd', nick: 'Cyd', prediction: null, points: null }) // hidden tip
const rows: Tip[] = [bob, ada, cyd] // server order

describe('sortRows', () => {
  it('standing (default) preserves server order', () => {
    expect(sortRows(rows, { column: 'standing', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'bob',
      'ada',
      'cyd',
    ])
  })

  it('sorts by player name ascending', () => {
    expect(sortRows(rows, { column: 'player', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'ada',
      'bob',
      'cyd',
    ])
  })

  it('sorts by player name descending', () => {
    expect(sortRows(rows, { column: 'player', direction: 'desc' }).map((r) => r.playerId)).toEqual([
      'cyd',
      'bob',
      'ada',
    ])
  })

  it('sorts by prediction with hidden tips always last', () => {
    // asc by home then away: bob 1-0 before ada 2-1; cyd (null) last
    expect(sortRows(rows, { column: 'prediction', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'bob',
      'ada',
      'cyd',
    ])
    // desc keeps hidden last too
    expect(sortRows(rows, { column: 'prediction', direction: 'desc' }).map((r) => r.playerId)).toEqual([
      'ada',
      'bob',
      'cyd',
    ])
  })

  it('sorts by points with nulls always last', () => {
    expect(sortRows(rows, { column: 'points', direction: 'desc' }).map((r) => r.playerId)).toEqual([
      'bob',
      'ada',
      'cyd',
    ])
    expect(sortRows(rows, { column: 'points', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'ada',
      'bob',
      'cyd',
    ])
  })

  it('does not mutate the input array', () => {
    const input = [...rows]
    sortRows(input, { column: 'player', direction: 'asc' })
    expect(input.map((r) => r.playerId)).toEqual(['bob', 'ada', 'cyd'])
  })
})

describe('nextSort', () => {
  it('toggles direction when the column is unchanged', () => {
    expect(nextSort({ column: 'player', direction: 'asc' }, 'player')).toEqual({
      column: 'player',
      direction: 'desc',
    })
  })

  it('uses the column default direction when switching columns', () => {
    expect(nextSort({ column: 'player', direction: 'asc' }, 'points')).toEqual({
      column: 'points',
      direction: 'desc',
    })
    expect(nextSort({ column: 'points', direction: 'desc' }, 'player')).toEqual({
      column: 'player',
      direction: 'asc',
    })
  })
})

describe('read/writeMatchSort', () => {
  beforeEach(() => localStorage.clear())

  it('returns the default when nothing is stored', () => {
    expect(readMatchSort()).toEqual(DEFAULT_MATCH_SORT)
  })

  it('round-trips a stored sort', () => {
    const sort: MatchSort = { column: 'points', direction: 'asc' }
    writeMatchSort(sort)
    expect(localStorage.getItem(MATCH_SORT_KEY)).toBeTruthy()
    expect(readMatchSort()).toEqual(sort)
  })

  it('falls back to the default on malformed storage', () => {
    localStorage.setItem(MATCH_SORT_KEY, 'not json')
    expect(readMatchSort()).toEqual(DEFAULT_MATCH_SORT)
  })
})
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd web && npm run test -- matchSort`
Expected: FAIL — `Cannot find module './matchSort'`.

- [ ] **Step 3: Implement the helper**

Create `web/src/lib/matchSort.ts`:

```ts
import type { Tip } from '../graphql/types'

/** Which column the match-page prediction grid is sorted by. */
export type MatchSortColumn = 'standing' | 'player' | 'prediction' | 'points'

export type SortDirection = 'asc' | 'desc'

export interface MatchSort {
  column: MatchSortColumn
  direction: SortDirection
}

/** Default = the server/scoreboard order the rows arrive in. */
export const DEFAULT_MATCH_SORT: MatchSort = { column: 'standing', direction: 'asc' }

export const MATCH_SORT_KEY = 'xpool.matchSort'

/** The direction a column adopts when first selected. */
const DEFAULT_DIRECTION: Record<MatchSortColumn, SortDirection> = {
  standing: 'asc',
  player: 'asc',
  prediction: 'asc',
  points: 'desc',
}

/**
 * Sort a copy of the gated tip rows. Hidden tips (`prediction === null`) and
 * unscored rows (`points === null`) always sink to the bottom regardless of
 * direction, so the visible data is never interleaved with placeholders. Stable:
 * ties fall back to the original (server) order via the captured index.
 */
export function sortRows(rows: readonly Tip[], sort: MatchSort): Tip[] {
  const indexed = rows.map((row, index) => ({ row, index }))
  const factor = sort.direction === 'asc' ? 1 : -1

  indexed.sort((a, b) => {
    switch (sort.column) {
      case 'player':
        return a.row.nick.localeCompare(b.row.nick) * factor || a.index - b.index
      case 'prediction': {
        const pa = a.row.prediction
        const pb = b.row.prediction
        if (!pa && !pb) return a.index - b.index
        if (!pa) return 1
        if (!pb) return -1
        return (
          ((pa.homeScore - pb.homeScore) || (pa.awayScore - pb.awayScore)) * factor ||
          a.index - b.index
        )
      }
      case 'points': {
        const va = a.row.points
        const vb = b.row.points
        if (va == null && vb == null) return a.index - b.index
        if (va == null) return 1
        if (vb == null) return -1
        return (va - vb) * factor || a.index - b.index
      }
      case 'standing':
      default:
        return (a.index - b.index) * factor || a.row.nick.localeCompare(b.row.nick)
    }
  })

  return indexed.map((entry) => entry.row)
}

/** Clicking a header: toggle direction if same column, else adopt its default. */
export function nextSort(current: MatchSort, column: MatchSortColumn): MatchSort {
  if (current.column === column) {
    return { column, direction: current.direction === 'asc' ? 'desc' : 'asc' }
  }
  return { column, direction: DEFAULT_DIRECTION[column] }
}

function isMatchSort(value: unknown): value is MatchSort {
  if (typeof value !== 'object' || value === null) return false
  const v = value as Record<string, unknown>
  return (
    (v.column === 'standing' ||
      v.column === 'player' ||
      v.column === 'prediction' ||
      v.column === 'points') &&
    (v.direction === 'asc' || v.direction === 'desc')
  )
}

/** Read the persisted sort, falling back to the default on any error. */
export function readMatchSort(): MatchSort {
  try {
    const raw = localStorage.getItem(MATCH_SORT_KEY)
    if (!raw) return DEFAULT_MATCH_SORT
    const parsed: unknown = JSON.parse(raw)
    return isMatchSort(parsed) ? parsed : DEFAULT_MATCH_SORT
  } catch {
    return DEFAULT_MATCH_SORT
  }
}

/** Persist the chosen sort (best-effort — a convenience, not load-bearing). */
export function writeMatchSort(sort: MatchSort): void {
  try {
    localStorage.setItem(MATCH_SORT_KEY, JSON.stringify(sort))
  } catch {
    /* ignore */
  }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd web && npm run test -- matchSort`
Expected: PASS (all cases green).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/matchSort.ts web/src/lib/matchSort.test.ts
git commit -m "feat(web): pure sort helper for match prediction grid"
```

### Task 2: Wire sortable headers into MatchPage

**Files:**
- Modify: `web/src/pages/MatchPage.tsx`
- Modify: `web/src/index.css`

- [ ] **Step 1: Add imports**

In `web/src/pages/MatchPage.tsx`, add to the existing import block (after the `teamIndex, formatKickoff` import):

```ts
import {
  sortRows,
  nextSort,
  readMatchSort,
  writeMatchSort,
  type MatchSort,
  type MatchSortColumn,
} from '../lib/matchSort'
```

- [ ] **Step 2: Add sort state (with the other `useState` hooks, before the early returns)**

Insert right after the `poolId` state declaration (`const [poolId, setPoolId] = ...`):

```ts
  const [sort, setSort] = useState<MatchSort>(() => readMatchSort())
  const applySort = (column: MatchSortColumn) => {
    const next = nextSort(sort, column)
    setSort(next)
    writeMatchSort(next)
  }
```

- [ ] **Step 3: Derive sorted rows + helpers (after `const { game, actual, rows } = match`)**

Insert just after the existing `const gateOpen = ...` block:

```ts
  const sortedRows = sortRows(rows, sort)
  // Points are only sortable once at least one row has been scored.
  const pointsSortable = rows.some((r) => r.points != null)
  const ariaSort = (
    column: MatchSortColumn,
  ): 'ascending' | 'descending' | 'none' =>
    sort.column === column
      ? sort.direction === 'asc'
        ? 'ascending'
        : 'descending'
      : 'none'
```

- [ ] **Step 4: Replace the `<thead>` header cells**

Replace the existing `<thead>...</thead>` block with clickable headers:

```tsx
        <thead>
          <tr>
            <th
              className={`sortable${sort.column === 'player' ? ' active' : ''}`}
              aria-sort={ariaSort('player')}
              onClick={() => applySort('player')}
            >
              {t('player')}
            </th>
            <th
              className={`sortable${sort.column === 'prediction' ? ' active' : ''}`}
              aria-sort={ariaSort('prediction')}
              onClick={() => applySort('prediction')}
            >
              {t('prediction')}
            </th>
            <th
              className={`num sortable${pointsSortable ? '' : ' disabled'}${
                sort.column === 'points' ? ' active' : ''
              }`}
              aria-sort={pointsSortable ? ariaSort('points') : 'none'}
              onClick={pointsSortable ? () => applySort('points') : undefined}
            >
              {t('points')}
            </th>
          </tr>
        </thead>
```

- [ ] **Step 5: Render sorted rows**

Change the body map from `rows.map((row) => (` to `sortedRows.map((row) => (`. (Leave the row body exactly as-is for now; Tasks 4 and 6 modify it.)

- [ ] **Step 6: Add header styling**

In `web/src/index.css`, append near the other `.match-grid` rules (after line ~1523):

```css
.match-grid th.sortable {
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
}
.match-grid th.sortable.active {
  color: var(--accent-bright);
}
.match-grid th.sortable[aria-sort='ascending']::after {
  content: ' ▲';
  font-size: 9px;
}
.match-grid th.sortable[aria-sort='descending']::after {
  content: ' ▼';
  font-size: 9px;
}
.match-grid th.sortable.disabled {
  cursor: default;
  opacity: 0.5;
}
```

- [ ] **Step 7: Verify build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add web/src/pages/MatchPage.tsx web/src/index.css
git commit -m "feat(web): sortable columns on the match prediction grid"
```

### Task 3: e2e — sortable columns reorder rows

**Files:**
- Create: `web/e2e/match-page-sort.spec.ts`

- [ ] **Step 1: Write the e2e spec**

Create `web/e2e/match-page-sort.spec.ts`:

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Sortable player predictions on the match page (cluster/match-page #1).
 *
 * Two players tip Group D before kickoff (LockTogether → editable in M4 before),
 * then the clock moves past M4's kickoff so the visibility gate opens and both
 * predictions are visible. Clicking the Player and Prediction headers reorders
 * the rows; the Points header is disabled until a result is in.
 *
 * Group D / M4 is the convention shared by match-page.spec.ts. ada tips 2-1,
 * grace tips 1-0 so player-name order (Ada, Grace) and prediction order
 * (1-0, 2-1) differ — making the sort observable.
 */

const TEST_GROUP = 'Group D'
const FIRST_GAME = 'M4'

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

async function enterTips(page: Page, player: string, home: string, away: string) {
  await devLogin(page, player)
  await setPreset(page, FIRST_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, home, away)
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')
}

test('match page: clicking column headers reorders the prediction rows', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // Two players tip differently while Group D is editable.
  await enterTips(page, 'demo-ada', '2', '1')
  await enterTips(page, 'demo-grace', '1', '0')

  // Move past M4 kickoff so the gate opens — both tips become visible.
  await setPreset(page, FIRST_GAME, 'after')

  await page.goto(`/match/${FIRST_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${FIRST_GAME}`))

  const grid = page.locator('table.match-grid')
  await expect(grid).toBeVisible()
  const nicks = () => grid.locator('tbody tr .nick')

  // Both demo players are visible (pool-scoped to the shared Demo Pool).
  await expect(grid.locator('tbody tr').filter({ hasText: 'ada' })).toBeVisible()
  await expect(grid.locator('tbody tr').filter({ hasText: 'grace' })).toBeVisible()

  // Sort by Player ascending: ada before grace.
  await grid.locator('th.sortable', { hasText: 'Player' }).click()
  await expect(grid.locator('th.sortable[aria-sort="ascending"]')).toContainText('Player')
  await expect(nicks().first()).toContainText('ada')

  // Click again → descending: grace before ada.
  await grid.locator('th.sortable', { hasText: 'Player' }).click()
  await expect(grid.locator('th.sortable[aria-sort="descending"]')).toContainText('Player')
  await expect(nicks().first()).toContainText('grace')

  // Sort by Prediction ascending: 1-0 (grace) before 2-1 (ada).
  await grid.locator('th.sortable', { hasText: 'Prediction' }).click()
  await expect(grid.locator('th.sortable[aria-sort="ascending"]')).toContainText('Prediction')
  await expect(nicks().first()).toContainText('grace')

  // Points header is disabled — no result entered yet.
  await expect(grid.locator('th.sortable.disabled')).toContainText('Points')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the spec**

Run: `cd web && npm run e2e -- match-page-sort`
Expected: PASS (the e2e harness boots its own stack via `global-setup.ts`).

- [ ] **Step 3: Commit**

```bash
git add web/e2e/match-page-sort.spec.ts
git commit -m "test(web): e2e for sortable match prediction columns"
```

---

## Feature 2 — live-match-highlight-self

### Task 4: Self-highlight the viewer's row + "you" badge

**Files:**
- Modify: `web/src/i18n/strings.ts`
- Modify: `web/src/pages/MatchPage.tsx`
- Modify: `web/src/index.css`
- Create: `web/e2e/match-page-self-highlight.spec.ts`

- [ ] **Step 1: Add the i18n key (EN)**

In `web/src/i18n/strings.ts`, inside the `en` object near the other match keys (after `unknownPlayer: '(unknown)',` ~line 90), add:

```ts
  youBadge: 'you',
```

- [ ] **Step 2: Add the i18n key (HU)**

In the `hu` block, next to its `unknownPlayer` entry (~line 441), add:

```ts
  youBadge: 'te',
```

- [ ] **Step 3: Mark the viewer's row + render the badge**

In `web/src/pages/MatchPage.tsx`, change the row `<tr>` and the `.nick` cell. Replace:

```tsx
            <tr key={row.playerId}>
              <td className="nick">{row.nick}</td>
```

with:

```tsx
            <tr
              key={row.playerId}
              className={row.playerId === viewerId ? 'is-self' : undefined}
            >
              <td className="nick">
                {row.nick}
                {row.playerId === viewerId && (
                  <span className="you-badge">{t('youBadge')}</span>
                )}
              </td>
```

(`viewerId` is already in scope.)

- [ ] **Step 4: Add the highlight styling**

In `web/src/index.css`, append after the sortable-header rules from Task 2:

```css
.match-grid tr.is-self {
  background: var(--bg-bar);
  box-shadow: inset 3px 0 0 var(--accent);
}
.you-badge {
  margin-left: 6px;
  padding: 0 5px;
  border-radius: 8px;
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  background: var(--accent);
  color: var(--accent-ink);
  vertical-align: middle;
}
```

- [ ] **Step 5: Write the e2e spec**

Create `web/e2e/match-page-self-highlight.spec.ts`:

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Highlight the current player on the match page (cluster/match-page #2).
 *
 * grace tips Group D (her own row is always visible regardless of the gate),
 * navigates to the match page, and finds her row marked `tr.is-self` with a
 * "you" badge. The highlight is a per-row class, so it is independent of any
 * sort order.
 */

const TEST_GROUP = 'Group D'
const PRE_TOURNAMENT = '2026-01-01T12:00:00Z'

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

test('match page: the logged-in player row is highlighted with a "you" badge', async ({ page }) => {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, PRE_TOURNAMENT)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  await openGroupD(page)
  await fillScores(page, '1', '0')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // Navigate to the first Group D match via the Schedule.
  await page.locator('.nav-bar').getByRole('link', { name: 'Schedule' }).click()
  const groupDSection = page.locator('.schedule-group').filter({ hasText: TEST_GROUP })
  await groupDSection.locator('tbody tr').first().locator('td a').first().click()
  await expect(page).toHaveURL(/\/match\//)

  const grid = page.locator('table.match-grid')
  await expect(grid).toBeVisible()

  // grace's row is the self row.
  const selfRow = grid.locator('tbody tr.is-self')
  await expect(selfRow).toHaveCount(1)
  await expect(selfRow).toContainText('grace')
  await expect(selfRow.locator('.you-badge')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 6: Verify build, lint, e2e**

Run: `cd web && npm run build && npm run lint && npm run e2e -- match-page-self-highlight`
Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add web/src/i18n/strings.ts web/src/pages/MatchPage.tsx web/src/index.css web/e2e/match-page-self-highlight.spec.ts
git commit -m "feat(web): highlight the current player's row on the match page"
```

---

## Feature 3 — match-page-what-if-scores

### Task 5: Pure match-scoring helper + tests

**Files:**
- Create: `web/src/lib/matchScoring.ts`
- Test: `web/src/lib/matchScoring.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `web/src/lib/matchScoring.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { scoreMatchBase, scoreMatchPoints } from './matchScoring'

describe('scoreMatchBase', () => {
  it('awards 4 for an exact correct scoreline (1+1+2)', () => {
    expect(scoreMatchBase({ homeScore: 1, awayScore: 0 }, { homeScore: 1, awayScore: 0 })).toBe(4)
  })

  it('awards 2 for the right outcome only', () => {
    // predicted 2-0 (home win), actual 1-0 (home win): outcome only
    expect(scoreMatchBase({ homeScore: 2, awayScore: 0 }, { homeScore: 1, awayScore: 0 })).toBe(2)
  })

  it('awards 3 for one exact side + outcome', () => {
    // predicted 1-0, actual 1-1 → exact home (1), wrong away, wrong outcome (win vs draw)
    expect(scoreMatchBase({ homeScore: 1, awayScore: 0 }, { homeScore: 1, awayScore: 1 })).toBe(1)
    // predicted 1-0, actual 2-0 → exact away (1) + outcome (2)
    expect(scoreMatchBase({ homeScore: 1, awayScore: 0 }, { homeScore: 2, awayScore: 0 })).toBe(3)
  })

  it('awards 0 for a wrong outcome and no exact side', () => {
    expect(scoreMatchBase({ homeScore: 0, awayScore: 1 }, { homeScore: 2, awayScore: 0 })).toBe(0)
  })

  it('applies the symmetric 4-goal rule per side', () => {
    // predicted 5-0, actual 4-0 → home counts as exact (both >= 4), away exact, outcome
    expect(scoreMatchBase({ homeScore: 5, awayScore: 0 }, { homeScore: 4, awayScore: 0 })).toBe(4)
  })

  it('scores a draw outcome', () => {
    expect(scoreMatchBase({ homeScore: 2, awayScore: 2 }, { homeScore: 0, awayScore: 0 })).toBe(2)
  })
})

describe('scoreMatchPoints', () => {
  it('multiplies the base by the round multiplier', () => {
    expect(scoreMatchPoints({ homeScore: 1, awayScore: 0 }, { homeScore: 1, awayScore: 0 }, 3)).toBe(12)
  })

  it('group-stage multiplier of 1 returns the base unchanged', () => {
    expect(scoreMatchPoints({ homeScore: 2, awayScore: 0 }, { homeScore: 1, awayScore: 0 }, 1)).toBe(2)
  })
})
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd web && npm run test -- matchScoring`
Expected: FAIL — `Cannot find module './matchScoring'`.

- [ ] **Step 3: Implement the helper**

Create `web/src/lib/matchScoring.ts`:

```ts
/**
 * Client-side mirror of the domain per-match scoring rule
 * (`crates/domain/src/scoring.rs`, SCORING.md §2–3). Used for live "what-if"
 * re-scoring in the browser — NOT a replacement for server scoring, which
 * remains authoritative. Constants mirror `ScoringConfig::default()`.
 */

/** A scoreline — both predictions and (hypothetical) results share this shape. */
export interface ScoreInput {
  homeScore: number
  awayScore: number
}

const EXACT_SCORE_POINT = 1
const OUTCOME_POINT = 2
const HIGH_SCORING_THRESHOLD = 4

/** sign of (home - away): home win > 0, draw = 0, away win < 0. */
const outcomeSign = (s: ScoreInput): number => Math.sign(s.homeScore - s.awayScore)

/**
 * Base (pre-multiplier) points for prediction `pred` vs result `actual`,
 * applying the per-side symmetric 4-goal rule: a side counts as exact when the
 * two values match OR both are at/above the high-scoring threshold.
 */
export function scoreMatchBase(pred: ScoreInput, actual: ScoreInput): number {
  const exactHome =
    pred.homeScore === actual.homeScore ||
    (pred.homeScore >= HIGH_SCORING_THRESHOLD && actual.homeScore >= HIGH_SCORING_THRESHOLD)
  const exactAway =
    pred.awayScore === actual.awayScore ||
    (pred.awayScore >= HIGH_SCORING_THRESHOLD && actual.awayScore >= HIGH_SCORING_THRESHOLD)
  const outcome = outcomeSign(pred) === outcomeSign(actual)

  return (
    (exactHome ? EXACT_SCORE_POINT : 0) +
    (exactAway ? EXACT_SCORE_POINT : 0) +
    (outcome ? OUTCOME_POINT : 0)
  )
}

/** Round-multiplied points for prediction `pred` vs result `actual`. */
export function scoreMatchPoints(pred: ScoreInput, actual: ScoreInput, multiplier: number): number {
  return scoreMatchBase(pred, actual) * multiplier
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd web && npm run test -- matchScoring`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/matchScoring.ts web/src/lib/matchScoring.test.ts
git commit -m "feat(web): client-side per-match scoring mirror for what-if"
```

### Task 6: What-if compute helper + tests

**Files:**
- Create: `web/src/lib/whatIf.ts`
- Test: `web/src/lib/whatIf.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `web/src/lib/whatIf.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { computeWhatIf } from './whatIf'

describe('computeWhatIf', () => {
  it('computes current + ifHome/ifAway totals and deltas (group ×1)', () => {
    // grace 1-0 vs live 1-0, group multiplier 1.
    // current: exact home + exact away + outcome = 4.
    // if home scores → 2-0: exact away + outcome = 3 (delta -1).
    // if away scores → 1-1: exact home only = 1 (delta -3).
    const result = computeWhatIf(
      { gameId: 'M8', homeScore: 1, awayScore: 0, locked: true },
      { homeScore: 1, awayScore: 0, provisional: true, source: null, sourceStatus: null, ninetyMinuteUncertain: false },
      1,
    )
    expect(result.current).toBe(4)
    expect(result.ifHome).toEqual({ total: 3, delta: -1 })
    expect(result.ifAway).toEqual({ total: 1, delta: -3 })
  })

  it('applies the round multiplier to totals and deltas', () => {
    // Same as above but R16 (×3): current 12, ifHome 9 (delta -3), ifAway 3 (delta -9).
    const result = computeWhatIf(
      { gameId: 'X', homeScore: 1, awayScore: 0, locked: true },
      { homeScore: 1, awayScore: 0, provisional: true, source: null, sourceStatus: null, ninetyMinuteUncertain: false },
      3,
    )
    expect(result.current).toBe(12)
    expect(result.ifHome).toEqual({ total: 9, delta: -3 })
    expect(result.ifAway).toEqual({ total: 3, delta: -9 })
  })

  it('shows a positive delta when the next goal helps', () => {
    // predicted 2-1 vs live 1-1; if home scores → 2-1 exact (4), big jump from current.
    const result = computeWhatIf(
      { gameId: 'X', homeScore: 2, awayScore: 1, locked: true },
      { homeScore: 1, awayScore: 1, provisional: true, source: null, sourceStatus: null, ninetyMinuteUncertain: false },
      1,
    )
    // current: 2-1 vs 1-1 → exact away (1), wrong home, wrong outcome (win vs draw) = 1.
    expect(result.current).toBe(1)
    // if home → 2-1: exact home + exact away + outcome = 4 (delta +3).
    expect(result.ifHome).toEqual({ total: 4, delta: 3 })
  })
})
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd web && npm run test -- whatIf`
Expected: FAIL — `Cannot find module './whatIf'`.

- [ ] **Step 3: Implement the helper**

Create `web/src/lib/whatIf.ts`:

```ts
import type { MatchPrediction, MatchScore } from '../graphql/types'
import { scoreMatchPoints } from './matchScoring'

/** A hypothetical outcome's new total and its delta vs the current points. */
export interface WhatIfOutcome {
  total: number
  delta: number
}

/** What each next goal would do to one player's score on this match. */
export interface WhatIf {
  current: number
  ifHome: WhatIfOutcome
  ifAway: WhatIfOutcome
}

/**
 * For one player's `prediction` and the live `actual` score, compute the new
 * round-multiplied total (and delta vs current) under the two single-goal
 * hypotheticals: home scores next, or away scores next.
 *
 * Gate-safety: the caller only invokes this for rows with a non-null
 * prediction, so a still-hidden tip is never re-scored or leaked.
 */
export function computeWhatIf(
  prediction: MatchPrediction,
  actual: MatchScore,
  multiplier: number,
): WhatIf {
  const current = scoreMatchPoints(prediction, actual, multiplier)
  const ifHomeTotal = scoreMatchPoints(
    prediction,
    { homeScore: actual.homeScore + 1, awayScore: actual.awayScore },
    multiplier,
  )
  const ifAwayTotal = scoreMatchPoints(
    prediction,
    { homeScore: actual.homeScore, awayScore: actual.awayScore + 1 },
    multiplier,
  )
  return {
    current,
    ifHome: { total: ifHomeTotal, delta: ifHomeTotal - current },
    ifAway: { total: ifAwayTotal, delta: ifAwayTotal - current },
  }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd web && npm run test -- whatIf`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/whatIf.ts web/src/lib/whatIf.test.ts
git commit -m "feat(web): what-if next-goal re-scoring helper"
```

### Task 7: WhatIfCell component + wire columns into MatchPage

**Files:**
- Create: `web/src/components/WhatIfCell.tsx`
- Modify: `web/src/i18n/strings.ts`
- Modify: `web/src/pages/MatchPage.tsx`
- Modify: `web/src/index.css`

- [ ] **Step 1: Add i18n keys (EN)**

In `web/src/i18n/strings.ts`, inside the `en` object near the match keys (after `points: 'Points',` ~line 87), add:

```ts
  ifHomeScores: 'If home scores',
  ifAwayScores: 'If away scores',
  whatIfHint: 'Points if the next goal goes in',
```

- [ ] **Step 2: Add i18n keys (HU)**

In the `hu` block near its `points` entry (~line 442), add:

```ts
  ifHomeScores: 'Ha a hazai gólt szerez',
  ifAwayScores: 'Ha a vendég gólt szerez',
  whatIfHint: 'Pontok, ha bemegy a következő gól',
```

- [ ] **Step 3: Create the cell component**

Create `web/src/components/WhatIfCell.tsx`:

```tsx
import type { WhatIfOutcome } from '../lib/whatIf'

/**
 * One what-if outcome: the new absolute total, with the delta-vs-current
 * emphasised (the delta is what people watch mid-match). A minus sign uses the
 * unicode minus (−) for typographic parity with the rest of the UI.
 */
export function WhatIfCell({ outcome }: { outcome: WhatIfOutcome }) {
  const direction =
    outcome.delta > 0 ? 'up' : outcome.delta < 0 ? 'down' : 'flat'
  const sign = outcome.delta > 0 ? '+' : outcome.delta < 0 ? '−' : '±'
  return (
    <span className="what-if">
      <span className="what-if-total">{outcome.total}</span>
      <span className={`what-if-delta ${direction}`}>
        {sign}
        {Math.abs(outcome.delta)}
      </span>
    </span>
  )
}
```

- [ ] **Step 4: Add imports to MatchPage**

In `web/src/pages/MatchPage.tsx`, add to the import block:

```ts
import { STAGE_MULTIPLIERS } from '../lib/rounds'
import { computeWhatIf } from '../lib/whatIf'
import { WhatIfCell } from '../components/WhatIfCell'
```

- [ ] **Step 5: Derive what-if scope + multiplier (after the `ariaSort` helper from Task 2)**

Insert just after the `sortedRows` / `ariaSort` block:

```ts
  // What-if is live-only and gated: show it once tips are revealable AND the
  // match is live (provisional). `liveActual` narrows `actual` to non-null so
  // the re-scoring below is type-safe.
  const liveActual = isLive && actual ? actual : null
  const showWhatIf = liveActual != null && gateOpen
  // The round multiplier for this match comes from its leaf group's round.
  const group = tournament?.groups.find((g) => g.id === game.groupId) ?? null
  const multiplier = group ? STAGE_MULTIPLIERS[group.round] : 1
```

- [ ] **Step 6: Add the what-if header cells**

In the `<thead>` row (from Task 2), add after the Points `<th>`:

```tsx
            {showWhatIf && (
              <>
                <th className="num what-if-col" title={t('whatIfHint')}>
                  {t('ifHomeScores')}
                </th>
                <th className="num what-if-col" title={t('whatIfHint')}>
                  {t('ifAwayScores')}
                </th>
              </>
            )}
```

- [ ] **Step 7: Render what-if cells per row**

Change the body map to compute what-if and render the two extra cells. Replace the `sortedRows.map((row) => (` opening through the closing `))` of the row with:

```tsx
          {sortedRows.map((row) => {
            const whatIf =
              liveActual && row.prediction
                ? computeWhatIf(row.prediction, liveActual, multiplier)
                : null
            return (
              <tr
                key={row.playerId}
                className={row.playerId === viewerId ? 'is-self' : undefined}
              >
                <td className="nick">
                  {row.nick}
                  {row.playerId === viewerId && (
                    <span className="you-badge">{t('youBadge')}</span>
                  )}
                </td>
                <td className="pred">
                  {row.prediction ? (
                    `${row.prediction.homeScore}–${row.prediction.awayScore}`
                  ) : (
                    <span className="match-hidden">{t('hiddenTip')}</span>
                  )}
                </td>
                <td className="pts num">
                  {row.points != null ? (
                    <PointsBadge
                      breakdown={row.breakdown}
                      points={row.points}
                      isPerfect={row.isPerfect}
                    />
                  ) : (
                    '—'
                  )}
                  {row.maxReachable != null && (
                    <span
                      className="max-reachable"
                      title={t('maxReachableTooltip')}
                    >
                      {t('maxReachableShort')} ≤ {row.maxReachable}
                    </span>
                  )}
                </td>
                {showWhatIf && (
                  <>
                    <td className="num what-if-cell">
                      {whatIf ? (
                        <WhatIfCell outcome={whatIf.ifHome} />
                      ) : (
                        <span className="match-hidden">{t('hiddenTip')}</span>
                      )}
                    </td>
                    <td className="num what-if-cell">
                      {whatIf ? (
                        <WhatIfCell outcome={whatIf.ifAway} />
                      ) : (
                        <span className="match-hidden">{t('hiddenTip')}</span>
                      )}
                    </td>
                  </>
                )}
              </tr>
            )
          })}
```

(This supersedes the row body changed in Tasks 2 and 4 — it folds the self-highlight and sort into the final shape.)

- [ ] **Step 8: Add what-if styling**

In `web/src/index.css`, append after the self-highlight rules from Task 4. First add the two delta-colour custom properties to the existing `:root` light/dark token area is optional — instead define them locally with safe literals so they work in every theme:

```css
.what-if {
  display: inline-flex;
  gap: 5px;
  align-items: baseline;
  justify-content: flex-end;
}
.what-if-total {
  font-size: 13px;
}
.what-if-delta {
  font-size: 11px;
  font-weight: 600;
}
.what-if-delta.up {
  color: #33ff66;
}
.what-if-delta.down {
  color: #ff6b6b;
}
.what-if-delta.flat {
  color: var(--muted);
}
.match-grid th.what-if-col {
  white-space: nowrap;
}
```

- [ ] **Step 9: Verify build + lint + unit**

Run: `cd web && npm run build && npm run lint && npm run test`
Expected: all PASS.

- [ ] **Step 10: Commit**

```bash
git add web/src/components/WhatIfCell.tsx web/src/i18n/strings.ts web/src/pages/MatchPage.tsx web/src/index.css
git commit -m "feat(web): live what-if next-goal columns on the match page"
```

### Task 8: e2e — live what-if columns and deltas

**Files:**
- Create: `web/e2e/match-page-what-if.spec.ts`

- [ ] **Step 1: Write the e2e spec**

Create `web/e2e/match-page-what-if.spec.ts`:

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Live what-if columns on the match page (cluster/match-page #3).
 *
 * Uses the hermetic M8 live stub (XPOOL_LIVE_SCORES="2461105=1:0:2H") — the same
 * mechanism live-scoring.spec.ts relies on — so /match/M8 is provisional in M8's
 * live window. grace tips 1-0; live score is 1-0; group multiplier ×1, so:
 *   - current = 4 (exact 1-0 + outcome)
 *   - if home scores (2-0): total 3, delta -1
 *   - if away scores (1-1): total 1, delta -3
 */

const TEST_GROUP = 'Group D'
const ENTRY_GAME = 'M4'
const LIVE_GAME = 'M8'

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

test('match page: live what-if columns show next-goal totals and deltas', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // grace tips 1-0 for Group D while editable (M4 before).
  await setPreset(page, ENTRY_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '1', '0')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // Move into M8's live window so the stub provisional score is in play.
  await setPreset(page, LIVE_GAME, 'during')
  await page.goto(`/match/${LIVE_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${LIVE_GAME}`))

  // Live score confirms the provisional path.
  await expect(page.locator('.match-scoreline.is-live')).toBeVisible()

  // What-if headers render.
  const grid = page.locator('table.match-grid')
  await expect(grid.locator('th.what-if-col')).toHaveCount(2)

  // grace's row: two what-if cells. nth(0) = if home scores, nth(1) = if away.
  const graceRow = grid.locator('tbody tr').filter({ hasText: 'grace' })
  await expect(graceRow).toBeVisible()
  const cells = graceRow.locator('.what-if-cell')
  await expect(cells).toHaveCount(2)

  // If home scores → total 3, delta down (-1).
  await expect(cells.nth(0).locator('.what-if-total')).toContainText('3')
  await expect(cells.nth(0).locator('.what-if-delta')).toHaveClass(/down/)
  await expect(cells.nth(0).locator('.what-if-delta')).toContainText('1')

  // If away scores → total 1, delta down (-3).
  await expect(cells.nth(1).locator('.what-if-total')).toContainText('1')
  await expect(cells.nth(1).locator('.what-if-delta')).toHaveClass(/down/)
  await expect(cells.nth(1).locator('.what-if-delta')).toContainText('3')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the spec**

Run: `cd web && npm run e2e -- match-page-what-if`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/match-page-what-if.spec.ts
git commit -m "test(web): e2e for live what-if next-goal columns"
```

---

## Task 9: Cluster verification + code review checkpoint

**Files:** none (verification only)

- [ ] **Step 1: Full web verification**

Run: `cd web && npm run build && npm run lint && npm run test`
Expected: all PASS (tsc + vite build clean, eslint clean, all Vitest suites green).

- [ ] **Step 2: Full e2e for the touched specs**

Run: `cd web && npm run e2e -- match-page`
Expected: PASS — covers `match-page.spec.ts` (regression), `match-page-info.spec.ts`, `match-page-sort.spec.ts`, `match-page-self-highlight.spec.ts`, `match-page-what-if.spec.ts`. Confirms the new headers/columns did not break the existing grid assertions.

- [ ] **Step 3: Workspace stays green (no Rust regressions)**

Run: `cargo build --workspace && cargo clippy --workspace -- -D warnings`
Expected: PASS. (No Rust was changed; this is the closing sanity check.)

- [ ] **Step 4: Visual check (cannot be inferred from green tests)**

Boot the local stack and look at a live match page in the browser:

```bash
bin/local-dev cluster-match-page   # repoint the dev session at this worktree
```

Then in the browser at `:5173`: open a match page and confirm (a) the sortable header arrows render and toggle, (b) your own row has the left accent stripe + "you" badge in both light and dark themes, and (c) on a live match the two what-if columns show a total with a green/red delta. New class names (`.what-if*`, `.you-badge`, `th.sortable`) all have CSS — verify they actually read as styled, not unstyled text. Note: the M8 live stub only exists in the e2e stack; for a live view in `bin/local-dev` either set `XPOOL_LIVE_SCORES` or rely on the e2e screenshots.

- [ ] **Step 5: Request code review**

REQUIRED SUB-SKILL: Use superpowers:requesting-code-review to verify the cluster meets requirements before merging. Focus the reviewer on: gate-safety (no still-hidden tip is ever re-scored or rendered), no `Date.now()` usage, immutability in `sortRows`, i18n completeness (every new key present in both `en` and `hu`), and that the what-if multiplier resolves correctly for both group-stage and knockout matches.

- [ ] **Step 6: Finish the branch**

REQUIRED SUB-SKILL: Use superpowers:finishing-a-development-branch. This is web source under `web/`, so it must merge into `master` via the solo workflow (or a PR if the reviewer flagged anything worth a record). Merge locally, then push.

---

## Self-review notes (author)

- **Spec coverage:**
  - *sort-predictions* — default = server/scoreboard order (`standing`) with nick secondary (Task 1); player/prediction/points columns clickable (Task 2); points sortable only once a result is in (`pointsSortable`, Task 2); client-side over gated rows; persisted to localStorage (`read/writeMatchSort`); self-highlight survives sort (per-row class, Task 7 row body). ✔
  - *highlight-self* — background tint + accent stripe + "you" badge, scoped to the match grid, reusing the existing `viewerId` (Task 4); no pin-to-top (out of scope per PRD). ✔
  - *what-if* — ±1 each side, two columns (Task 7); shows both absolute total and emphasised delta (`WhatIfCell`); pool scope inherited from the already-pool-scoped rows; client-side re-scoring, no new resolver (Tasks 5–6); gated behind `showWhatIf = liveActual && gateOpen`. ✔
- **Placeholder scan:** every code step contains complete code; commands have expected output; no "TBD"/"similar to". ✔
- **Type consistency:** `MatchSort`/`MatchSortColumn`/`sortRows`/`nextSort`/`readMatchSort`/`writeMatchSort` used identically across Tasks 1–2; `ScoreInput`/`scoreMatchBase`/`scoreMatchPoints` consistent Tasks 5–6; `WhatIf`/`WhatIfOutcome`/`computeWhatIf` consistent Tasks 6–7; `liveActual`/`showWhatIf`/`multiplier` defined once (Task 7 Step 5) and consumed in Steps 6–7. ✔
- **Known assumption:** the `standing` default treats the server-delivered row order as the scoreboard baseline (the match resolver returns rows in a stable order). If a future change requires true cross-tournament scoreboard position, it would need the scoreboard query — deliberately out of scope here.
```