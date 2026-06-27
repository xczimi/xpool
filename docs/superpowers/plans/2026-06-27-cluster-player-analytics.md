# Player Analytics Cluster (Head-to-Head + Points Timeline) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two web-only player-analytics features — a head-to-head two-player comparison at `/h2h/:a/:b` and a hand-rolled SVG cumulative-points timeline chart (overlaid in the H2H view) — entirely client-side from the existing GraphQL queries.

**Architecture:** This cluster (`cluster/player-analytics`, Wave 1) OWNS `web/src/App.tsx` routing and adds new files under `web/src/pages` and `web/src/components`. All data is reused client-side: `SCOREBOARD_QUERY` (per-round point breakdown → cumulative trajectory + totals/positions) and `TIPS_QUERY` (per-match per-player predictions + points, with server-applied hidden-until-revealable gating). **No new GraphQL resolver is needed** — verified: `Tip.points` is per-match multiplied points and `Tip.prediction` is `null` when gated. Player URLs in this app are clean handles (`/player/demo-alan` works; `playerId === 'demo-alan'`), so `/h2h/:a/:b` reuses those handles directly — no UUIDs, no separate slug resolution. No `Date.now()`: the x-axis rounds are derived from server data (`readyRounds`), and tip-gating is server-driven.

**Tech Stack:** React 19 + Vite + TypeScript, urql GraphQL client, vitest (unit), Playwright (e2e). Pure SVG `<polyline>` chart (no charting library), mirroring `web/src/components/BrandIcon.tsx`.

---

## File Structure

**New files**

- `web/src/lib/cumulativePoints.ts` — pure reducer: running cumulative points per round for one scoreboard entry.
- `web/src/lib/cumulativePoints.test.ts` — vitest unit tests for the reducer.
- `web/src/lib/headToHead.ts` — pure helpers: two-player summary (totals/positions/delta), per-round deltas, per-match diffs from tips.
- `web/src/lib/headToHead.test.ts` — vitest unit tests for the H2H reducers.
- `web/src/components/PointsTimelineChart.tsx` — hand-rolled SVG line chart, overlay-capable.
- `web/src/components/H2HPicker.tsx` — "pick two" entry control rendered on the scoreboard.
- `web/src/pages/H2HPage.tsx` — the head-to-head page (route `/h2h/:a/:b`).
- `web/e2e/points-timeline.spec.ts` — e2e: player page renders the trajectory chart.
- `web/e2e/h2h.spec.ts` — e2e: head-to-head route renders a two-player comparison with an overlaid chart, plus the scoreboard picker entry.

**Modified files**

- `web/src/App.tsx` — add the `/h2h/:a/:b` route.
- `web/src/i18n/strings.ts` — append EN + HU keys (one block each).
- `web/src/index.css` — append chart, h2h, and picker CSS (sentinel-anchored).
- `web/src/pages/PlayerPage.tsx` — render single-series chart + a "Compare with me" link.
- `web/src/pages/ScoreboardPage.tsx` — render the `H2HPicker`.

**Reused as-is (read-only)**

- `web/src/graphql/queries.ts` — `SCOREBOARD_QUERY`, `TIPS_QUERY`, `TOURNAMENT_QUERY`, `POOLS_QUERY`.
- `web/src/lib/rounds.ts` — `ROUND_ORDER`, `readyRounds`, `roundLabel`, `visibleRoundNodes`, `currentRoundNode`, `leafGroupsOfRound`.
- `web/src/lib/playerPage.ts` — `playerEntry`, `playerRank`.
- `web/src/lib/selectedPool.ts` + `web/src/pools/useSelectedPool.ts` — pool scoping.

---

## Conventions reference (do not re-derive)

- Scoreboard entry shape (`web/src/graphql/types.ts`):
  ```ts
  export interface ScoreEntry { playerId: string; nick: string; total: number; stages: StageScore[] }
  export interface StageScore { round: Round; points: number }
  export type Round = 'GROUP_STAGE' | 'R32' | 'R16' | 'QF' | 'SF' | 'THIRD_PLACE' | 'FINAL'
  ```
- Tip shape (`web/src/graphql/types.ts`): `Tip { playerId; nick; gameId; prediction: MatchPrediction | null; points: number | null; isPerfect; breakdown; maxReachable }`; `MatchPrediction { gameId; homeScore; awayScore; locked }`. **`prediction === null` means the server gated it** — never compute gating client-side.
- i18n: `const { t } = useI18n()` from `'../i18n/useI18n'`; `t(key)` returns the localised string; `roundLabel(round, t)` localises a round name.
- Pool scoping (mirror `ScoreboardPage.tsx`): `const { selected } = useSelectedPool()` then `effectiveSelectedPool(selected, pools.map(p => p.id))`.
- CSS palette (`web/src/index.css`): `--accent`, `--accent-bright`, `--bg-deep`, `--ink`, `--muted`, `--border`; mono font is `'VT323', monospace`.

---

### Task 1: Cumulative-points reducer (pure, TDD)

**Files:**
- Create: `web/src/lib/cumulativePoints.ts`
- Test: `web/src/lib/cumulativePoints.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/cumulativePoints.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import type { Round, ScoreEntry } from '../graphql/types'
import { cumulativeSeries } from './cumulativePoints'

const entry = (stages: Array<[Round, number]>): ScoreEntry => ({
  playerId: 'p',
  nick: 'p',
  total: stages.reduce((n, [, v]) => n + v, 0),
  stages: stages.map(([round, points]) => ({ round, points })),
})

describe('cumulativeSeries', () => {
  it('returns an empty series for no rounds', () => {
    expect(cumulativeSeries(entry([['GROUP_STAGE', 5]]), [])).toEqual([])
  })

  it('accumulates points across the supplied round order', () => {
    const e = entry([
      ['GROUP_STAGE', 5],
      ['R32', 4],
      ['R16', 6],
    ])
    const rounds: Round[] = ['GROUP_STAGE', 'R32', 'R16']
    expect(cumulativeSeries(e, rounds)).toEqual([
      { round: 'GROUP_STAGE', points: 5, cumulative: 5 },
      { round: 'R32', points: 4, cumulative: 9 },
      { round: 'R16', points: 6, cumulative: 15 },
    ])
  })

  it('treats rounds absent from the entry as zero points', () => {
    const e = entry([['GROUP_STAGE', 3]])
    const rounds: Round[] = ['GROUP_STAGE', 'R32', 'R16']
    expect(cumulativeSeries(e, rounds)).toEqual([
      { round: 'GROUP_STAGE', points: 3, cumulative: 3 },
      { round: 'R32', points: 0, cumulative: 3 },
      { round: 'R16', points: 0, cumulative: 3 },
    ])
  })

  it('honours the caller-supplied round order, not the stage order', () => {
    const e = entry([
      ['R16', 6],
      ['GROUP_STAGE', 5],
    ])
    const rounds: Round[] = ['GROUP_STAGE', 'R16']
    expect(cumulativeSeries(e, rounds).map((p) => p.cumulative)).toEqual([5, 11])
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npm run test -- cumulativePoints`
Expected: FAIL — `Failed to resolve import "./cumulativePoints"` / `cumulativeSeries is not a function`.

- [ ] **Step 3: Write minimal implementation**

Create `web/src/lib/cumulativePoints.ts`:

```ts
import type { Round, ScoreEntry } from '../graphql/types'

/** One x-axis point: a round, its points, and the running total through it. */
export interface CumulativePoint {
  round: Round
  points: number
  cumulative: number
}

/**
 * Running cumulative points for a scoreboard entry over an ordered list of
 * rounds. `rounds` is supplied by the caller (ROUND_ORDER filtered by
 * readyRounds), so the series never reaches past the server-derived horizon —
 * there is no Date.now() here. Rounds absent from the entry's stages
 * contribute 0. Pure and immutable.
 */
export function cumulativeSeries(
  entry: ScoreEntry,
  rounds: Round[],
): CumulativePoint[] {
  const byRound = new Map(entry.stages.map((s) => [s.round, s.points]))
  let running = 0
  return rounds.map((round) => {
    const points = byRound.get(round) ?? 0
    running += points
    return { round, points, cumulative: running }
  })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npm run test -- cumulativePoints`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/cumulativePoints.ts web/src/lib/cumulativePoints.test.ts
git commit -m "feat(web): cumulative-points reducer for player analytics"
```

---

### Task 2: i18n keys (EN + HU) for the whole cluster

**Files:**
- Modify: `web/src/i18n/strings.ts`

Add every cluster string up front so later UI tasks just reference them (unused keys are harmless and keep the build green). `StringKey = keyof typeof en`, and `hu` is `Record<StringKey, string>`, so **both** blocks must get the same keys.

- [ ] **Step 1: Add the EN keys**

In `web/src/i18n/strings.ts`, find this line in the `en` block:

```ts
  scoreboardTitle: 'Scoreboard',
```

Replace it with:

```ts
  scoreboardTitle: 'Scoreboard',

  // player analytics (head-to-head + points timeline)
  timelineTitle: 'Points trajectory',
  h2hTitle: 'Head-to-head',
  h2hPickTwo: 'Compare two players',
  h2hPickPrompt: 'Pick a player',
  h2hCompare: 'Compare',
  h2hCompareWithMe: 'Compare with me',
  h2hTotalDelta: 'Total difference',
  h2hPerMatch: 'Where they differ',
  h2hRoundLabel: 'Round',
  h2hMatch: 'Match',
  h2hNoDiffs: 'No differences in this round',
```

- [ ] **Step 2: Add the matching HU keys**

In the same file, find this line in the `hu` block:

```ts
  scoreboardTitle: 'Tippverseny',
```

Replace it with:

```ts
  scoreboardTitle: 'Tippverseny',

  // player analytics (head-to-head + points timeline)
  timelineTitle: 'Pontok alakulása',
  h2hTitle: 'Párharc',
  h2hPickTwo: 'Két játékos összevetése',
  h2hPickPrompt: 'Válassz játékost',
  h2hCompare: 'Összevetés',
  h2hCompareWithMe: 'Hasonlítsd hozzám',
  h2hTotalDelta: 'Összpont-különbség',
  h2hPerMatch: 'Ahol eltérnek',
  h2hRoundLabel: 'Forduló',
  h2hMatch: 'Mérkőzés',
  h2hNoDiffs: 'Nincs eltérés ebben a fordulóban',
```

- [ ] **Step 3: Verify the build (type-checks the catalogue parity)**

Run: `cd web && npm run build`
Expected: PASS. A missing key in `hu` would fail `tsc` because `hu` is `Record<StringKey, string>`.

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): i18n strings for player-analytics cluster"
```

---

### Task 3: Hand-rolled SVG timeline chart component + CSS

**Files:**
- Create: `web/src/components/PointsTimelineChart.tsx`
- Modify: `web/src/index.css`

- [ ] **Step 1: Create the chart component**

Create `web/src/components/PointsTimelineChart.tsx`:

```tsx
import type { CumulativePoint } from '../lib/cumulativePoints'

/** One overlaid line: a label, a stroke colour, and its cumulative points. */
export interface TimelineSeries {
  label: string
  color: string
  points: CumulativePoint[]
}

/** Two legible series colours on the dark scoreboard background (amber, cyan). */
export const TIMELINE_COLORS = ['#ffd76a', '#21d4fd'] as const

const W = 480
const H = 200
const PAD_L = 36
const PAD_R = 12
const PAD_T = 12
const PAD_B = 28

/**
 * Hand-rolled SVG line chart — no charting library, mirroring BrandIcon's
 * pure-SVG approach. Plots cumulative points per round, one <polyline> per
 * series, so it supports head-to-head overlay. Every input is pre-computed and
 * already localised (`xLabels`), so the component is pure and clock-free.
 */
export function PointsTimelineChart({
  series,
  xLabels,
  title,
}: {
  series: TimelineSeries[]
  xLabels: string[]
  title?: string
}) {
  const n = xLabels.length
  const maxY = Math.max(
    1,
    ...series.flatMap((s) => s.points.map((p) => p.cumulative)),
  )
  const innerW = W - PAD_L - PAD_R
  const innerH = H - PAD_T - PAD_B
  const x = (i: number) => PAD_L + (n <= 1 ? 0 : (innerW * i) / (n - 1))
  const y = (v: number) => PAD_T + innerH - (innerH * v) / maxY

  return (
    <figure className="points-timeline">
      {title && <figcaption>{title}</figcaption>}
      <svg
        viewBox={`0 0 ${W} ${H}`}
        role="img"
        aria-label={title ?? 'points timeline'}
        preserveAspectRatio="xMidYMid meet"
      >
        <line
          className="pt-axis"
          x1={PAD_L}
          y1={PAD_T + innerH}
          x2={W - PAD_R}
          y2={PAD_T + innerH}
        />
        {series.map((s) => (
          <polyline
            key={s.label}
            className="pt-line"
            fill="none"
            stroke={s.color}
            points={s.points.map((p, i) => `${x(i)},${y(p.cumulative)}`).join(' ')}
          />
        ))}
        {series.flatMap((s) =>
          s.points.map((p, i) => (
            <circle
              key={`${s.label}-${i}`}
              cx={x(i)}
              cy={y(p.cumulative)}
              r={2.5}
              fill={s.color}
            />
          )),
        )}
        {xLabels.map((lbl, i) => (
          <text
            key={`${lbl}-${i}`}
            className="pt-xlabel"
            x={x(i)}
            y={H - 8}
            textAnchor="middle"
          >
            {lbl}
          </text>
        ))}
      </svg>
      {series.length > 1 && (
        <ul className="pt-legend">
          {series.map((s) => (
            <li key={s.label} className="pt-legend-item">
              <span
                className="pt-swatch"
                style={{ background: s.color }}
                aria-hidden="true"
              />
              {s.label}
            </li>
          ))}
        </ul>
      )}
    </figure>
  )
}
```

- [ ] **Step 2: Append the chart CSS**

In `web/src/index.css`, find the current last line:

```css
.match-grid .match-hidden { color: var(--muted); }
```

Replace it with (keeps the line, appends a sentinel-terminated block):

```css
.match-grid .match-hidden { color: var(--muted); }

/* === player-analytics: timeline chart === */
.points-timeline { margin: 16px 0; max-width: 520px; }
.points-timeline figcaption {
  font-family: 'VT323', monospace;
  color: var(--accent-bright);
  margin-bottom: 4px;
}
.points-timeline svg { width: 100%; height: auto; display: block; }
.points-timeline .pt-axis { stroke: var(--border); stroke-width: 1; }
.points-timeline .pt-line { stroke-width: 2; }
.points-timeline .pt-xlabel {
  fill: var(--muted);
  font-size: 11px;
  font-family: 'VT323', monospace;
}
.pt-legend { list-style: none; display: flex; gap: 16px; padding: 0; margin: 8px 0 0; }
.pt-legend-item {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--ink);
  font-family: 'VT323', monospace;
}
.pt-swatch { width: 12px; height: 12px; border-radius: 2px; display: inline-block; }
/* --- end timeline chart --- */
```

- [ ] **Step 3: Verify build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: PASS (component compiles; no unused symbols).

- [ ] **Step 4: Commit**

```bash
git add web/src/components/PointsTimelineChart.tsx web/src/index.css
git commit -m "feat(web): hand-rolled SVG points-timeline chart"
```

---

### Task 4: Render the chart on the player page + e2e

**Files:**
- Modify: `web/src/pages/PlayerPage.tsx`
- Create: `web/e2e/points-timeline.spec.ts`

- [ ] **Step 1: Add imports to PlayerPage**

In `web/src/pages/PlayerPage.tsx`, find:

```tsx
import { PlayerRounds } from './player/PlayerRounds'
```

Replace it with:

```tsx
import { PlayerRounds } from './player/PlayerRounds'
import { PointsTimelineChart, TIMELINE_COLORS } from '../components/PointsTimelineChart'
import { cumulativeSeries } from '../lib/cumulativePoints'
import { ROUND_ORDER, readyRounds, roundLabel } from '../lib/rounds'
```

- [ ] **Step 2: Compute the trajectory rounds before the main return**

In the same file, find the main return opener:

```tsx
  return (
    <section className="page player-page">
```

Replace it with:

```tsx
  // x-axis rounds = the canonical order filtered to rounds whose teams are
  // known (server-derived; no Date.now()). `tournament` is non-null here.
  const timelineRounds = ROUND_ORDER.filter((r) =>
    readyRounds(tournament.groups, tournament.games).has(r),
  )

  return (
    <section className="page player-page">
```

- [ ] **Step 3: Render the single-series chart after the header**

In the same file, find:

```tsx
      <PlayerHeader entry={shownEntry} rank={rank} />
```

Replace it with:

```tsx
      <PlayerHeader entry={shownEntry} rank={rank} />
      <PointsTimelineChart
        title={t('timelineTitle')}
        xLabels={timelineRounds.map((r) => roundLabel(r, t))}
        series={[
          {
            label: shownEntry.nick,
            color: TIMELINE_COLORS[0],
            points: cumulativeSeries(shownEntry, timelineRounds),
          },
        ]}
      />
```

- [ ] **Step 4: Verify build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: PASS.

- [ ] **Step 5: Write the e2e spec**

Create `web/e2e/points-timeline.spec.ts`:

```ts
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Points-timeline chart end to end. Seeds the `balanced` scenario, logs in,
 * clocks past the Final (so the board is materialised) and asserts the player
 * page renders a single trajectory <polyline>.
 */
const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

type Phase = 'before' | 'during' | 'after'

async function setClock(page: Page, gameIndex: number, phase: Phase) {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption({ index: gameIndex })
  await expect(selects.nth(1)).toBeEnabled()
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  await expect(page.locator('.dev-clock-now')).toBeVisible()
}

async function lastGameIndex(page: Page): Promise<number> {
  const count = await page
    .locator('.dev-clock select')
    .nth(0)
    .locator('option')
    .count()
  return count - 1
}

test.beforeAll(() => {
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

test('player page renders the points trajectory chart', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')

  await page.locator('.auth-player-link').click()
  await expect(page).toHaveURL(/\/me$/)

  await expect(page.locator('.points-timeline svg')).toBeVisible()
  await expect(page.locator('.points-timeline polyline')).toHaveCount(1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 6: Run the e2e spec**

Run: `cd web && npm run e2e -- points-timeline`
Expected: PASS (the spec boots the live stack via `e2e/global-setup.ts`).

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/PlayerPage.tsx web/e2e/points-timeline.spec.ts
git commit -m "feat(web): points trajectory chart on the player page"
```

---

### Task 5: Head-to-head reducers (pure, TDD)

**Files:**
- Create: `web/src/lib/headToHead.ts`
- Test: `web/src/lib/headToHead.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/headToHead.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import type { Round, ScoreEntry, Tip } from '../graphql/types'
import { h2hSummary, matchDiffs, roundDeltas } from './headToHead'

const entry = (
  playerId: string,
  total: number,
  stages: Array<[Round, number]>,
): ScoreEntry => ({
  playerId,
  nick: playerId,
  total,
  stages: stages.map(([round, points]) => ({ round, points })),
})

const tip = (
  playerId: string,
  gameId: string,
  pred: { homeScore: number; awayScore: number } | null,
  points: number | null,
): Tip => ({
  playerId,
  nick: playerId,
  gameId,
  prediction: pred ? { gameId, ...pred, locked: true } : null,
  points,
  isPerfect: false,
  breakdown: null,
  maxReachable: null,
})

describe('h2hSummary', () => {
  it('returns null when either player is missing from the board', () => {
    const board = [entry('a', 10, [])]
    expect(h2hSummary(board, 'a', 'b')).toBeNull()
  })

  it('computes ranks and the a-minus-b total delta', () => {
    const board = [
      entry('a', 10, []),
      entry('b', 7, []),
      entry('c', 20, []),
    ]
    const s = h2hSummary(board, 'a', 'b')
    expect(s?.totalDelta).toBe(3)
    expect(s?.rankA).toBe(2)
    expect(s?.rankB).toBe(3)
  })
})

describe('roundDeltas', () => {
  it('subtracts per-round points over the supplied rounds, zero-filling gaps', () => {
    const a = entry('a', 9, [
      ['GROUP_STAGE', 5],
      ['R32', 4],
    ])
    const b = entry('b', 6, [['GROUP_STAGE', 6]])
    const rounds: Round[] = ['GROUP_STAGE', 'R32']
    expect(roundDeltas(a, b, rounds)).toEqual([
      { round: 'GROUP_STAGE', pointsA: 5, pointsB: 6, delta: -1 },
      { round: 'R32', pointsA: 4, pointsB: 0, delta: 4 },
    ])
  })
})

describe('matchDiffs', () => {
  it('omits matches where predictions and points are identical', () => {
    const tips = [
      tip('a', 'g1', { homeScore: 1, awayScore: 0 }, 4),
      tip('b', 'g1', { homeScore: 1, awayScore: 0 }, 4),
    ]
    expect(matchDiffs(tips, 'a', 'b')).toEqual([])
  })

  it('keeps matches where the predictions differ', () => {
    const tips = [
      tip('a', 'g1', { homeScore: 1, awayScore: 0 }, 4),
      tip('b', 'g1', { homeScore: 2, awayScore: 2 }, 0),
    ]
    const rows = matchDiffs(tips, 'a', 'b')
    expect(rows).toHaveLength(1)
    expect(rows[0]).toMatchObject({
      gameId: 'g1',
      predA: { homeScore: 1, awayScore: 0 },
      predB: { homeScore: 2, awayScore: 2 },
      pointsA: 4,
      pointsB: 0,
      hiddenA: false,
      hiddenB: false,
    })
  })

  it('always keeps a row when one side is gated-hidden', () => {
    const tips = [
      tip('a', 'g1', { homeScore: 1, awayScore: 0 }, null),
      tip('b', 'g1', null, null),
    ]
    const rows = matchDiffs(tips, 'a', 'b')
    expect(rows).toHaveLength(1)
    expect(rows[0].hiddenB).toBe(true)
    expect(rows[0].predB).toBeNull()
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npm run test -- headToHead`
Expected: FAIL — `Failed to resolve import "./headToHead"`.

- [ ] **Step 3: Write minimal implementation**

Create `web/src/lib/headToHead.ts`:

```ts
import type { Round, ScoreEntry, Tip } from '../graphql/types'
import { playerEntry, playerRank } from './playerPage'

export interface H2HSummary {
  a: ScoreEntry
  b: ScoreEntry
  rankA: number | null
  rankB: number | null
  /** a.total - b.total (positive ⇒ a is ahead). */
  totalDelta: number
}

/**
 * Resolve both sides from the materialised scoreboard. Returns null if either
 * is absent (e.g. a participant with no scored entry yet). Pure.
 */
export function h2hSummary(
  scoreboard: ScoreEntry[],
  idA: string,
  idB: string,
): H2HSummary | null {
  const a = playerEntry(scoreboard, idA)
  const b = playerEntry(scoreboard, idB)
  if (!a || !b) return null
  return {
    a,
    b,
    rankA: playerRank(scoreboard, idA),
    rankB: playerRank(scoreboard, idB),
    totalDelta: a.total - b.total,
  }
}

export interface RoundDelta {
  round: Round
  pointsA: number
  pointsB: number
  /** pointsA - pointsB. */
  delta: number
}

/** Per-round point comparison over the supplied ordered rounds. Pure. */
export function roundDeltas(
  a: ScoreEntry,
  b: ScoreEntry,
  rounds: Round[],
): RoundDelta[] {
  const ma = new Map(a.stages.map((s) => [s.round, s.points]))
  const mb = new Map(b.stages.map((s) => [s.round, s.points]))
  return rounds.map((round) => {
    const pointsA = ma.get(round) ?? 0
    const pointsB = mb.get(round) ?? 0
    return { round, pointsA, pointsB, delta: pointsA - pointsB }
  })
}

export interface ScoreCell {
  homeScore: number
  awayScore: number
}

export interface MatchDiff {
  gameId: string
  /** null when this player's pick is gated-hidden by the server. */
  predA: ScoreCell | null
  predB: ScoreCell | null
  pointsA: number | null
  pointsB: number | null
  /** true when the server withheld the prediction (Tip.prediction === null). */
  hiddenA: boolean
  hiddenB: boolean
}

/**
 * Per-match comparison for two players from a round's `tips`. The TIPS_QUERY
 * result already applies hidden-until-revealable gating: a withheld prediction
 * arrives as null, so this never branches on a clock. Only rows where the two
 * predictions OR their points differ are returned — the "where they differ"
 * view — except that a row with either side gated-hidden is always kept so the
 * gate stays visible rather than being silently dropped. Pure.
 */
export function matchDiffs(tips: Tip[], idA: string, idB: string): MatchDiff[] {
  const aByGame = new Map<string, Tip>()
  const bByGame = new Map<string, Tip>()
  for (const t of tips) {
    if (t.playerId === idA) aByGame.set(t.gameId, t)
    if (t.playerId === idB) bByGame.set(t.gameId, t)
  }
  const gameIds = [...new Set([...aByGame.keys(), ...bByGame.keys()])]
  const rows: MatchDiff[] = []
  for (const gameId of gameIds) {
    const ta = aByGame.get(gameId) ?? null
    const tb = bByGame.get(gameId) ?? null
    const predA = ta?.prediction
      ? { homeScore: ta.prediction.homeScore, awayScore: ta.prediction.awayScore }
      : null
    const predB = tb?.prediction
      ? { homeScore: tb.prediction.homeScore, awayScore: tb.prediction.awayScore }
      : null
    const hiddenA = ta != null && ta.prediction == null
    const hiddenB = tb != null && tb.prediction == null
    const samePred =
      predA != null &&
      predB != null &&
      predA.homeScore === predB.homeScore &&
      predA.awayScore === predB.awayScore
    const samePoints = (ta?.points ?? null) === (tb?.points ?? null)
    if (samePred && samePoints && !hiddenA && !hiddenB) continue
    rows.push({
      gameId,
      predA,
      predB,
      pointsA: ta?.points ?? null,
      pointsB: tb?.points ?? null,
      hiddenA,
      hiddenB,
    })
  }
  return rows
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npm run test -- headToHead`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/headToHead.ts web/src/lib/headToHead.test.ts
git commit -m "feat(web): head-to-head reducers (summary, round deltas, match diffs)"
```

---

### Task 6: Head-to-head page + route

**Files:**
- Create: `web/src/pages/H2HPage.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/index.css`

- [ ] **Step 1: Create the H2H page**

Create `web/src/pages/H2HPage.tsx`:

```tsx
import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  POOLS_QUERY,
  SCOREBOARD_QUERY,
  TIPS_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type { Pool, Round, ScoreEntry, Tip, Tournament } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { PoolSelector } from '../pools/PoolSelector'
import { useSelectedPool } from '../pools/useSelectedPool'
import { effectiveSelectedPool } from '../lib/selectedPool'
import {
  ROUND_ORDER,
  currentRoundNode,
  leafGroupsOfRound,
  readyRounds,
  roundLabel,
  visibleRoundNodes,
} from '../lib/rounds'
import { cumulativeSeries } from '../lib/cumulativePoints'
import { h2hSummary, matchDiffs, roundDeltas } from '../lib/headToHead'
import type { ScoreCell } from '../lib/headToHead'
import { PointsTimelineChart, TIMELINE_COLORS } from '../components/PointsTimelineChart'

/**
 * Head-to-head: two players compared within the selected pool. All data is
 * reused client-side — SCOREBOARD_QUERY for totals/positions and the per-round
 * trajectory, TIPS_QUERY for the per-match breakdown (its hidden-until-
 * revealable gating is server-applied). No new resolver. The route params are
 * the same clean player handles the scoreboard links use.
 */
export function H2HPage() {
  const { a = '', b = '' } = useParams<{ a: string; b: string }>()
  const { t } = useI18n()
  const { label } = useAuth()
  const { selected } = useSelectedPool()
  const [selectedRound, setSelectedRound] = useState<Round | null>(null)

  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = poolsResult.data?.pools ?? []
  const effectivePool = effectiveSelectedPool(
    selected,
    pools.map((p) => p.id),
  )

  const [scoreboardResult] = useQuery<{ scoreboard: ScoreEntry[] }>({
    query: SCOREBOARD_QUERY,
    variables: { pool: effectivePool },
  })
  const [tournamentResult] = useQuery<{ tournament: Tournament | null }>({
    query: TOURNAMENT_QUERY,
  })

  const scoreboard = scoreboardResult.data?.scoreboard ?? []
  const tournament = tournamentResult.data?.tournament ?? null

  // Per-match round selector (mirrors All Tips): group stage queries one leaf
  // group; a knockout round queries the round node — the tips resolver walks
  // its subtree. Default to the current round.
  const roundNodes = visibleRoundNodes(
    tournament?.groups ?? [],
    tournament?.games ?? [],
  )
  const activeRound =
    selectedRound ??
    currentRoundNode(roundNodes)?.round ??
    roundNodes[0]?.round ??
    null
  const activeRoundNode = roundNodes.find((r) => r.round === activeRound) ?? null
  const isGroupStage = activeRound === 'GROUP_STAGE'
  const roundLeaves = activeRoundNode
    ? leafGroupsOfRound(activeRoundNode, tournament?.groups ?? [])
    : []
  const tipsGroupId = isGroupStage
    ? (roundLeaves[0]?.id ?? null)
    : (activeRoundNode?.id ?? null)

  const [tipsResult] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId: tipsGroupId, pool: effectivePool },
    pause: !label || !tipsGroupId,
  })

  if (!label) return <NeedsLogin />
  if (scoreboardResult.fetching || tournamentResult.fetching) return <Loading />
  if (scoreboardResult.error)
    return <ErrorView message={scoreboardResult.error.message} />
  if (!tournament) return <ErrorView />

  const summary = h2hSummary(scoreboard, a, b)
  if (!summary) {
    return (
      <section className="page">
        <p>{t('playerNotInPool')}</p>
      </section>
    )
  }

  const rounds = ROUND_ORDER.filter((r) =>
    readyRounds(tournament.groups, tournament.games).has(r),
  )
  const series = [
    {
      label: summary.a.nick,
      color: TIMELINE_COLORS[0],
      points: cumulativeSeries(summary.a, rounds),
    },
    {
      label: summary.b.nick,
      color: TIMELINE_COLORS[1],
      points: cumulativeSeries(summary.b, rounds),
    },
  ]
  const deltas = roundDeltas(summary.a, summary.b, rounds)
  const diffs = matchDiffs(tipsResult.data?.tips ?? [], a, b)

  const cell = (pred: ScoreCell | null, hidden: boolean) =>
    hidden ? t('hiddenTip') : pred ? `${pred.homeScore}–${pred.awayScore}` : '—'

  return (
    <section className="page h2h-page">
      <h2>{t('h2hTitle')}</h2>
      <PoolSelector pools={pools} />

      <div className="h2h-summary">
        <div className="h2h-stat">
          <Link to={`/player/${summary.a.playerId}`}>{summary.a.nick}</Link>
          <span className="h2h-stat-value">{summary.a.total}</span>
          <span className="h2h-stat-rank">#{summary.rankA ?? '—'}</span>
        </div>
        <div className="h2h-stat h2h-delta">
          <span className="h2h-stat-label">{t('h2hTotalDelta')}</span>
          <span className="h2h-stat-value">
            {summary.totalDelta > 0 ? '+' : ''}
            {summary.totalDelta}
          </span>
        </div>
        <div className="h2h-stat">
          <Link to={`/player/${summary.b.playerId}`}>{summary.b.nick}</Link>
          <span className="h2h-stat-value">{summary.b.total}</span>
          <span className="h2h-stat-rank">#{summary.rankB ?? '—'}</span>
        </div>
      </div>

      <PointsTimelineChart
        title={t('timelineTitle')}
        xLabels={rounds.map((r) => roundLabel(r, t))}
        series={series}
      />

      <table className="data-table h2h-delta-table">
        <thead>
          <tr>
            <th>{t('h2hRoundLabel')}</th>
            <th>{summary.a.nick}</th>
            <th>{summary.b.nick}</th>
            <th>Δ</th>
          </tr>
        </thead>
        <tbody>
          {deltas.map((d) => (
            <tr key={d.round}>
              <td>{roundLabel(d.round, t)}</td>
              <td>{d.pointsA}</td>
              <td>{d.pointsB}</td>
              <td>
                {d.delta > 0 ? '+' : ''}
                {d.delta}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>{t('h2hPerMatch')}</h3>
      <label className="h2h-round-select">
        {t('h2hRoundLabel')}{' '}
        <select
          value={activeRound ?? ''}
          onChange={(e) => setSelectedRound(e.target.value as Round)}
        >
          {roundNodes.map((node) => (
            <option key={node.round} value={node.round}>
              {roundLabel(node.round, t)}
            </option>
          ))}
        </select>
      </label>
      {diffs.length === 0 ? (
        <p className="h2h-no-diffs">{t('h2hNoDiffs')}</p>
      ) : (
        <table className="data-table h2h-match-table">
          <thead>
            <tr>
              <th>{t('h2hMatch')}</th>
              <th>{summary.a.nick}</th>
              <th>{summary.b.nick}</th>
            </tr>
          </thead>
          <tbody>
            {diffs.map((m) => (
              <tr key={m.gameId}>
                <td>{m.gameId}</td>
                <td className={m.hiddenA ? 'h2h-hidden' : undefined}>
                  {cell(m.predA, m.hiddenA)}
                  {m.pointsA != null && ` (${m.pointsA})`}
                </td>
                <td className={m.hiddenB ? 'h2h-hidden' : undefined}>
                  {cell(m.predB, m.hiddenB)}
                  {m.pointsB != null && ` (${m.pointsB})`}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  )
}
```

> Note: the per-match table's first column shows the raw `gameId` in this v1 — functional and low-risk. Enriching it to localised team labels (reusing `Matchup`/`TeamLabel`) is deliberately deferred; it isn't required by the resolved scope ("per-match breakdown where they differ").

- [ ] **Step 2: Wire the route into App.tsx**

In `web/src/App.tsx`, find:

```tsx
import { MatchPage } from './pages/MatchPage'
```

Replace it with:

```tsx
import { MatchPage } from './pages/MatchPage'
import { H2HPage } from './pages/H2HPage'
```

Then find:

```tsx
        <Route path="player/:id" element={<PlayerPage />} />
```

Replace it with:

```tsx
        <Route path="player/:id" element={<PlayerPage />} />
        <Route path="h2h/:a/:b" element={<H2HPage />} />
```

- [ ] **Step 3: Append the H2H CSS**

In `web/src/index.css`, find the sentinel added in Task 3:

```css
/* --- end timeline chart --- */
```

Replace it with:

```css
/* --- end timeline chart --- */

/* === player-analytics: head-to-head === */
.h2h-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin: 12px 0;
  max-width: 520px;
}
.h2h-stat { display: flex; flex-direction: column; align-items: center; gap: 2px; }
.h2h-stat-value {
  font-family: 'VT323', monospace;
  font-size: 28px;
  color: var(--accent-bright);
}
.h2h-stat-rank,
.h2h-stat-label { color: var(--muted); font-family: 'VT323', monospace; }
.h2h-delta-table td:last-child,
.h2h-delta-table th:last-child { text-align: right; }
.h2h-round-select {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  margin: 8px 0;
  color: var(--ink);
}
.h2h-match-table .h2h-hidden { color: var(--muted); }
.h2h-no-diffs { color: var(--muted); font-family: 'VT323', monospace; }
/* --- end head-to-head --- */
```

- [ ] **Step 4: Verify build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/H2HPage.tsx web/src/App.tsx web/src/index.css
git commit -m "feat(web): head-to-head page at /h2h/:a/:b"
```

---

### Task 7: Entry points — scoreboard picker + "Compare with me" link

**Files:**
- Create: `web/src/components/H2HPicker.tsx`
- Modify: `web/src/pages/ScoreboardPage.tsx`
- Modify: `web/src/pages/PlayerPage.tsx`
- Modify: `web/src/index.css`

- [ ] **Step 1: Create the picker component**

Create `web/src/components/H2HPicker.tsx`:

```tsx
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import type { ScoreEntry } from '../graphql/types'

/** "Pick two" head-to-head entry rendered above the scoreboard. */
export function H2HPicker({ entries }: { entries: ScoreEntry[] }) {
  const { t } = useI18n()
  const navigate = useNavigate()
  const [a, setA] = useState('')
  const [b, setB] = useState('')
  const ready = a !== '' && b !== '' && a !== b

  const options = entries.map((e) => (
    <option key={e.playerId} value={e.playerId}>
      {e.nick}
    </option>
  ))

  return (
    <div className="h2h-picker">
      <span className="h2h-picker-label">{t('h2hPickTwo')}</span>
      <select
        value={a}
        aria-label={t('h2hPickPrompt')}
        onChange={(e) => setA(e.target.value)}
      >
        <option value="">—</option>
        {options}
      </select>
      <span className="h2h-picker-vs">×</span>
      <select
        value={b}
        aria-label={t('h2hPickPrompt')}
        onChange={(e) => setB(e.target.value)}
      >
        <option value="">—</option>
        {options}
      </select>
      <button
        type="button"
        className="h2h-picker-go"
        disabled={!ready}
        onClick={() => navigate(`/h2h/${a}/${b}`)}
      >
        {t('h2hCompare')}
      </button>
    </div>
  )
}
```

- [ ] **Step 2: Render the picker on the scoreboard**

In `web/src/pages/ScoreboardPage.tsx`, find:

```tsx
import { useSelectedPool } from '../pools/useSelectedPool'
```

Replace it with:

```tsx
import { useSelectedPool } from '../pools/useSelectedPool'
import { H2HPicker } from '../components/H2HPicker'
```

Then find:

```tsx
      <PoolSelector pools={pools} />
```

Replace it with:

```tsx
      <PoolSelector pools={pools} />
      <H2HPicker entries={ranked} />
```

- [ ] **Step 3: Add the "Compare with me" link on a pool-mate's page**

In `web/src/pages/PlayerPage.tsx`, find:

```tsx
      {isOwn && (
        <p className="player-profile-link">
          <Link to="/profile">{t('playerProfileLink')}</Link>
        </p>
      )}
```

Replace it with:

```tsx
      {isOwn && (
        <p className="player-profile-link">
          <Link to="/profile">{t('playerProfileLink')}</Link>
        </p>
      )}
      {!isOwn && myId && (
        <p className="player-profile-link">
          <Link to={`/h2h/${myId}/${id}`}>{t('h2hCompareWithMe')}</Link>
        </p>
      )}
```

- [ ] **Step 4: Append the picker CSS**

In `web/src/index.css`, find the sentinel added in Task 6:

```css
/* --- end head-to-head --- */
```

Replace it with:

```css
/* --- end head-to-head --- */

/* === player-analytics: h2h picker === */
.h2h-picker {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 16px;
  flex-wrap: wrap;
}
.h2h-picker-label { color: var(--muted); font-family: 'VT323', monospace; }
.h2h-picker-vs { color: var(--accent); font-family: 'VT323', monospace; }
.h2h-picker-go { font-family: 'VT323', monospace; }
.h2h-picker-go:disabled { opacity: 0.5; cursor: not-allowed; }
/* --- end h2h picker --- */
```

- [ ] **Step 5: Verify build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/H2HPicker.tsx web/src/pages/ScoreboardPage.tsx web/src/pages/PlayerPage.tsx web/src/index.css
git commit -m "feat(web): h2h entry points (scoreboard picker + compare-with-me link)"
```

---

### Task 8: Head-to-head e2e

**Files:**
- Create: `web/e2e/h2h.spec.ts`

- [ ] **Step 1: Write the e2e spec**

Create `web/e2e/h2h.spec.ts`:

```ts
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Head-to-head end to end. Seeds the `balanced` scenario, logs in, clocks past
 * the Final (board materialised + tips revealed), then drives both the direct
 * route and the scoreboard "pick two" entry. demo-ada's nick renders as "ada",
 * demo-alan's as "alan"; the route params are the player handles.
 */
const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

type Phase = 'before' | 'during' | 'after'

async function setClock(page: Page, gameIndex: number, phase: Phase) {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption({ index: gameIndex })
  await expect(selects.nth(1)).toBeEnabled()
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  await expect(page.locator('.dev-clock-now')).toBeVisible()
}

async function lastGameIndex(page: Page): Promise<number> {
  const count = await page
    .locator('.dev-clock select')
    .nth(0)
    .locator('option')
    .count()
  return count - 1
}

test.beforeAll(() => {
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

test('direct route compares two players with an overlaid trajectory', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')
  await page.goto('/h2h/demo-ada/demo-alan')

  await expect(page.locator('.h2h-summary')).toBeVisible()
  // Both players overlaid → two polylines.
  await expect(page.locator('.points-timeline polyline')).toHaveCount(2)
  await expect(page.locator('.h2h-delta-table')).toBeVisible()
  // The per-match section renders either a diff table or the "no differences" note.
  await expect(
    page.locator('.h2h-match-table, .h2h-no-diffs'),
  ).toHaveCount(1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('scoreboard "pick two" navigates to the head-to-head view', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')
  await page.goto('/scoreboard')

  const picker = page.locator('.h2h-picker')
  await expect(picker).toBeVisible()
  await picker.locator('select').nth(0).selectOption('demo-ada')
  await picker.locator('select').nth(1).selectOption('demo-alan')
  await picker.locator('.h2h-picker-go').click()

  await expect(page).toHaveURL(/\/h2h\/demo-ada\/demo-alan$/)
  await expect(page.locator('.h2h-summary')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the e2e spec**

Run: `cd web && npm run e2e -- h2h`
Expected: PASS (2 tests). If the per-match assertion is flaky because the default round (Final) yields one shared game, the `.h2h-match-table, .h2h-no-diffs` count-of-1 assertion still holds (exactly one of the two renders).

- [ ] **Step 3: Commit**

```bash
git add web/e2e/h2h.spec.ts
git commit -m "test(web): e2e for head-to-head view and scoreboard picker"
```

---

### Task 9: Cluster verification + request code review

**Files:** none (verification only).

- [ ] **Step 1: Full web unit tests**

Run: `cd web && npm run test`
Expected: PASS — including `cumulativePoints` (4) and `headToHead` (7) plus all pre-existing suites.

- [ ] **Step 2: Web build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: both PASS, no warnings.

- [ ] **Step 3: Rust workspace still green (no Rust changed, but confirm)**

Run: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: PASS. This cluster added **no resolver and no Rust code**; this step just proves the branch didn't drift.

- [ ] **Step 4: Full e2e for the two new specs**

Run: `cd web && npm run e2e -- points-timeline h2h`
Expected: PASS (3 tests total). The suite boots its own isolated stack (API `:3001`, Vite `:5174`, DynamoDB `:8001`).

- [ ] **Step 5: Visual check (do not skip — green tests ≠ looks right)**

Start the local stack (`bin/local-dev`, or `cargo run -p api` + `cd web && npm run dev`), log in as `demo-ada`, pin the dev clock past the Final, then:
- Open `/me` — confirm the trajectory chart renders a legible amber line on the dark background, with round labels on the x-axis.
- Open `/scoreboard` — confirm the "pick two" control renders; pick `ada` × `alan` and Compare.
- On `/h2h/demo-ada/demo-alan` — confirm: the two stat columns + total-delta center, an overlaid two-colour chart (amber + cyan) with a legend, the per-round delta table (Δ column right-aligned), and the per-match section (round selector + either a diff table or the "no differences" note).
- Toggle the language to Hungarian — confirm all new strings are translated (no raw keys like `h2hTitle`).
- Switch the accent theme — confirm chart axis/labels still read against the changed background.

- [ ] **Step 6: Request code review**

Use the **superpowers:requesting-code-review** skill to review the full cluster diff against `master`. Focus the reviewer on: immutability in the pure reducers, no `Date.now()` anywhere in the new code, tip-gating respected (no client-side deadline logic), and the no-new-resolver client-only data path.

- [ ] **Step 7: Final commit (if review fixes were applied)**

```bash
git add -A
git commit -m "chore(web): address player-analytics cluster review feedback"
```

---

## Self-Review

**Spec coverage (H2H):** two players only ✓ (Task 6); `/h2h/:a/:b` clean handles ✓ (route Task 6; handles verified — `/player/demo-alan` works); reuse scoreboard data client-side, no resolver ✓ (Tasks 5–6); entry from scoreboard + player page ✓ (Task 7); pool-scoped ✓ (`effectiveSelectedPool` in Task 6); points + positions + total delta ✓ (`h2hSummary`); per-match breakdown where they differ ✓ (`matchDiffs`); tip-gating respected ✓ (server `prediction === null`, never a clock).

**Spec coverage (timeline):** hand-rolled SVG `<polyline>` ✓ (Task 3, mirrors BrandIcon); cumulative points ✓ (`cumulativeSeries`); x-axis by round ✓ (`ROUND_ORDER` filtered by `readyRounds`); client-side from SCOREBOARD_QUERY stages, no resolver ✓; overlay support ✓ (`series[]`, used in H2H Task 6); no `Date.now()` ✓ (rounds derived from server data).

**Per-cluster bar:** cargo green ✓ (Task 9 Step 3); web build+lint ✓ (Steps 2 & per task); e2e per feature ✓ (Tasks 4 & 8); unit-tested cumulative reducer ✓ (Task 1, TDD) + H2H reducers (Task 5); visual check note ✓ (Step 5); last task = verification + requesting-code-review ✓ (Task 9).

**Placeholder scan:** no TBD/TODO; every code step shows full code. The per-match first column uses `gameId` (explicitly noted as v1, not a placeholder).

**Type consistency:** `CumulativePoint`, `TimelineSeries`/`TIMELINE_COLORS`, `H2HSummary`/`RoundDelta`/`MatchDiff`/`ScoreCell`, and `cumulativeSeries`/`h2hSummary`/`roundDeltas`/`matchDiffs` names match across the reducer, component, and page tasks. i18n keys added in Task 2 (`timelineTitle`, `h2hTitle`, `h2hPickTwo`, `h2hPickPrompt`, `h2hCompare`, `h2hCompareWithMe`, `h2hTotalDelta`, `h2hPerMatch`, `h2hRoundLabel`, `h2hMatch`, `h2hNoDiffs`) are exactly the keys referenced in Tasks 6–7; reused existing keys: `hiddenTip`, `playerNotInPool`, `player`, `playerProfileLink`. CSS sentinels chain correctly: `--- end timeline chart ---` (Task 3) → `--- end head-to-head ---` (Task 6) → `--- end h2h picker ---` (Task 7).
