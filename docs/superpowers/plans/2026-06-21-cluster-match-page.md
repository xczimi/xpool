# Match Page Cluster Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two informational features to the match page (`/match/:gameId`): render the match **venue** near the kickoff time, and show **prediction stats** (most common scoreline, home/draw/away split, "N nailed it") computed client-side from the already-loaded, visibility-gated tip rows — visible only after the visibility gate opens.

**Architecture:** Both features are **frontend-only**. `venue` is already on the GraphQL `Game` type and already selected in `MATCH_QUERY`, so venue is a pure render change. Prediction stats are derived from the `match.rows` the API already returns — server-side gating in `scored_tip` (`crates/api/src/gql/query.rs`) means a row's `prediction` is `null` while still hidden, so aggregating only non-null predictions automatically respects tip visibility and leaks nothing pre-gate. The aggregation lives in a **pure, unit-tested helper** in `web/src/lib/` so it is testable without a DOM. The stats panel scope is the **selected pool** — `MatchPage` already owns a local pool selector (native `<select className="pool-selector">`) that drives the match query's `pool` arg.

**NOTE — cross-cluster integration:** The perfect-page cluster is building a sticky shared pool context. This plan deliberately keeps `MatchPage`'s **existing local** pool selection (it already exists at `MatchPage.tsx:25-93`). When the sticky pool context lands, swap that local `useState` + `pools[0]` defaulting for the shared context — the `effectivePool` value feeding `MATCH_QUERY` and the stats panel is the single integration seam. Do **not** add a dependency on any perfect-page file in this cluster; it must build and test standalone on this branch.

**Tech Stack:** React + Vite + TypeScript, urql GraphQL client, vitest (unit), Playwright (e2e), Rust (axum + async-graphql, unchanged here), en/hu i18n in `web/src/i18n/strings.ts`. Server-authoritative clock — no `Date.now()` for behavioural decisions; the gate is the server-returned `prediction === null`, never a browser clock.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `web/src/lib/predictionStats.ts` | Pure aggregation: most-common scoreline(s), outcome split, nailed-it count, from `Tip[]` | **Create** |
| `web/src/lib/predictionStats.test.ts` | vitest unit tests for the aggregation helper | **Create** |
| `web/src/components/PredictionStats.tsx` | Presentational panel rendering the aggregates; renders nothing pre-gate | **Create** |
| `web/src/pages/MatchPage.tsx` | Render `<div className="match-card-venue">` + mount `<PredictionStats>` | **Modify** (`:95-129`) |
| `web/src/i18n/strings.ts` | Add stats i18n keys (en + hu); `venue` label already exists | **Modify** (`en` ~`:314`, `hu` ~`:602`) |
| `web/src/index.css` | Styles for `.match-card-venue` and `.prediction-stats` | **Modify** (after `:1347`) |
| `web/e2e/match-page-info.spec.ts` | e2e: venue renders; stats hidden before gate, shown after | **Create** |

Venue requires **no** backend change — `crates/api/src/gql/types.rs:94` already exposes `venue: Option<String>`, `web/src/graphql/types.ts:35` already types `venue: string | null`, and `MATCH_QUERY` (`web/src/graphql/queries.ts:233`) already selects it.

---

## Task 1: Pure prediction-stats aggregation helper

The match page's `rows` are `Tip[]`. A `Tip` with a still-hidden prediction has `prediction === null` (server gate). The helper aggregates **only** rows whose `prediction` is non-null, so it is gate-safe by construction. `actual` (the official/live score) drives the "nailed it" count, but **only** when it is a final result (`provisional === false`) — a provisional live score must not be reported as "nailed it".

**Files:**
- Create: `web/src/lib/predictionStats.ts`
- Test: `web/src/lib/predictionStats.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/predictionStats.test.ts`:

```typescript
import { describe, expect, it } from 'vitest'
import type { MatchScore, Tip } from '../graphql/types'
import { computePredictionStats } from './predictionStats'

/** Build a Tip with a visible prediction (helper for terse fixtures). */
function tip(playerId: string, home: number, away: number): Tip {
  return {
    playerId,
    nick: playerId,
    gameId: 'g1',
    prediction: { gameId: 'g1', homeScore: home, awayScore: away, locked: true },
    points: null,
    isPerfect: false,
    breakdown: null,
  }
}

/** A Tip whose prediction is still hidden (server gate not yet open). */
function hiddenTip(playerId: string): Tip {
  return {
    playerId,
    nick: playerId,
    gameId: 'g1',
    prediction: null,
    points: null,
    isPerfect: false,
    breakdown: null,
  }
}

const finalScore: MatchScore = {
  homeScore: 2,
  awayScore: 1,
  provisional: false,
  source: null,
  sourceStatus: null,
  ninetyMinuteUncertain: false,
}

describe('computePredictionStats', () => {
  it('returns null when no predictions are visible (gate closed)', () => {
    const stats = computePredictionStats([hiddenTip('a'), hiddenTip('b')], null)
    expect(stats).toBeNull()
  })

  it('counts the most common scoreline and reports its count', () => {
    const rows = [tip('a', 2, 1), tip('b', 2, 1), tip('c', 1, 0)]
    const stats = computePredictionStats(rows, null)
    expect(stats).not.toBeNull()
    expect(stats!.total).toBe(3)
    expect(stats!.mostCommon).toEqual([{ homeScore: 2, awayScore: 1, count: 2 }])
  })

  it('reports all scorelines tied for most common', () => {
    const rows = [tip('a', 2, 1), tip('b', 1, 0)]
    const stats = computePredictionStats(rows, null)!
    // Both appear once → both are "most common"; order is by descending count
    // then home-major, away-major for determinism.
    expect(stats.mostCommon).toEqual([
      { homeScore: 1, awayScore: 0, count: 1 },
      { homeScore: 2, awayScore: 1, count: 1 },
    ])
  })

  it('splits outcomes into home / draw / away', () => {
    const rows = [
      tip('a', 2, 1), // home
      tip('b', 3, 0), // home
      tip('c', 1, 1), // draw
      tip('d', 0, 2), // away
    ]
    const stats = computePredictionStats(rows, null)!
    expect(stats.outcomeSplit).toEqual({ home: 2, draw: 1, away: 1 })
  })

  it('ignores hidden predictions when aggregating', () => {
    const rows = [tip('a', 2, 1), hiddenTip('b'), tip('c', 2, 1)]
    const stats = computePredictionStats(rows, null)!
    expect(stats.total).toBe(2)
    expect(stats.mostCommon).toEqual([{ homeScore: 2, awayScore: 1, count: 2 }])
  })

  it('counts how many nailed a FINAL result', () => {
    const rows = [tip('a', 2, 1), tip('b', 2, 1), tip('c', 0, 0)]
    const stats = computePredictionStats(rows, finalScore)!
    expect(stats.nailedIt).toBe(2)
  })

  it('does not count nailed-it for a provisional (live) score', () => {
    const rows = [tip('a', 2, 1)]
    const provisional: MatchScore = { ...finalScore, provisional: true }
    const stats = computePredictionStats(rows, provisional)!
    expect(stats.nailedIt).toBeNull()
  })

  it('reports nailedIt = 0 when nobody matched a final result', () => {
    const rows = [tip('a', 0, 0), tip('b', 3, 3)]
    const stats = computePredictionStats(rows, finalScore)!
    expect(stats.nailedIt).toBe(0)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npx vitest run src/lib/predictionStats.test.ts`
Expected: FAIL — `Failed to resolve import "./predictionStats"` / `computePredictionStats is not a function`.

- [ ] **Step 3: Write minimal implementation**

Create `web/src/lib/predictionStats.ts`:

```typescript
import type { MatchScore, Tip } from '../graphql/types'

/** One scoreline and how many players predicted it. */
export interface ScorelineCount {
  homeScore: number
  awayScore: number
  count: number
}

/** Home/draw/away split of the visible predictions. */
export interface OutcomeSplit {
  home: number
  draw: number
  away: number
}

/** Aggregate prediction stats for one match, over the visible (gated) rows. */
export interface PredictionStats {
  /** Number of visible (non-hidden) predictions aggregated. */
  total: number
  /** Scoreline(s) tied for most predicted, descending count then deterministic. */
  mostCommon: ScorelineCount[]
  outcomeSplit: OutcomeSplit
  /**
   * How many predicted the exact FINAL result. `null` when there is no final
   * result yet (no `actual`, or `actual.provisional` — a live score must never
   * be reported as "nailed it").
   */
  nailedIt: number | null
}

/**
 * Aggregate the visibility-gated tip rows for a match. Returns `null` when no
 * prediction is visible yet (the gate is closed) — the caller renders nothing.
 *
 * Gate-safety: a still-hidden tip has `prediction === null` (the server gate in
 * `scored_tip`, `crates/api/src/gql/query.rs`). We aggregate only non-null
 * predictions, so before the gate opens there is nothing to leak.
 *
 * Pool scope: scoping is the caller's job — it passes the rows already filtered
 * to the selected pool (the `MATCH_QUERY` `pool` arg).
 */
export function computePredictionStats(
  rows: Tip[],
  actual: MatchScore | null,
): PredictionStats | null {
  const visible = rows.flatMap((r) => (r.prediction ? [r.prediction] : []))
  if (visible.length === 0) return null

  // Count each distinct scoreline.
  const counts = new Map<string, ScorelineCount>()
  for (const p of visible) {
    const key = `${p.homeScore}-${p.awayScore}`
    const existing = counts.get(key)
    counts.set(
      key,
      existing
        ? { ...existing, count: existing.count + 1 }
        : { homeScore: p.homeScore, awayScore: p.awayScore, count: 1 },
    )
  }

  // Most common = every scoreline tied for the top count. Sort descending by
  // count, then by home then away score for a stable, deterministic order.
  const ranked = [...counts.values()].sort(
    (a, b) =>
      b.count - a.count ||
      a.homeScore - b.homeScore ||
      a.awayScore - b.awayScore,
  )
  const topCount = ranked[0].count
  const mostCommon = ranked.filter((s) => s.count === topCount)

  const outcomeSplit = visible.reduce<OutcomeSplit>(
    (acc, p) => {
      if (p.homeScore > p.awayScore) return { ...acc, home: acc.home + 1 }
      if (p.homeScore < p.awayScore) return { ...acc, away: acc.away + 1 }
      return { ...acc, draw: acc.draw + 1 }
    },
    { home: 0, draw: 0, away: 0 },
  )

  const nailedIt =
    actual && !actual.provisional
      ? visible.filter(
          (p) =>
            p.homeScore === actual.homeScore &&
            p.awayScore === actual.awayScore,
        ).length
      : null

  return { total: visible.length, mostCommon, outcomeSplit, nailedIt }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npx vitest run src/lib/predictionStats.test.ts`
Expected: PASS — 8 tests passing.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/predictionStats.ts web/src/lib/predictionStats.test.ts
git commit -m "feat(web): pure prediction-stats aggregation helper"
```

---

## Task 2: i18n strings for venue + stats (en + hu)

The `venue` label already exists in both catalogues (`strings.ts:83` en, `:384` hu) — used by the Schedule page. Add the **stats** labels only. The `StringKey` type is derived from the `en` object's keys, so adding a key to `en` makes it required in `hu` (a missing key fails `tsc`).

**Files:**
- Modify: `web/src/i18n/strings.ts` (`en` block after `:319`, `hu` block after `:607`)

- [ ] **Step 1: Add the keys to the `en` catalogue**

In `web/src/i18n/strings.ts`, find the `en` "match page (#2 live preview)" block (currently ends with `ninetyMinuteNote:` around line 319). Add after `ninetyMinuteNote`'s closing string, before the next comment block:

```typescript
  // match page (#2) — prediction stats panel
  predictionStatsTitle: 'What everyone tipped',
  predictionStatsHidden: 'Other players’ tips appear once the match opens.',
  mostCommonScore: 'Most common score',
  mostCommonScorePlural: 'Most common scores',
  outcomeSplitLabel: 'Outcome',
  outcomeHome: 'Home win',
  outcomeDraw: 'Draw',
  outcomeAway: 'Away win',
  nailedItLabel: 'Nailed the exact score',
  statsTipCount: 'tips',
```

- [ ] **Step 2: Add the matching keys to the `hu` catalogue**

In the `hu` block, find its "match page (#2 live preview)" section (the `ninetyMinuteNote:` around line 607). Add after it:

```typescript
  // match page (#2) — prediction stats panel
  predictionStatsTitle: 'Mit tippeltek a többiek',
  predictionStatsHidden: 'A többiek tippjei a meccs kezdetekor jelennek meg.',
  mostCommonScore: 'Leggyakoribb eredmény',
  mostCommonScorePlural: 'Leggyakoribb eredmények',
  outcomeSplitLabel: 'Kimenetel',
  outcomeHome: 'Hazai győzelem',
  outcomeDraw: 'Döntetlen',
  outcomeAway: 'Vendég győzelem',
  nailedItLabel: 'Eltalálta a pontos eredményt',
  statsTipCount: 'tipp',
```

- [ ] **Step 3: Verify the catalogues type-check (both keys present)**

Run: `cd web && npx tsc -b --noEmit`
Expected: no errors. (A key in `en` but missing in `hu` would fail here with `Property 'predictionStatsTitle' is missing in type` on the `hu` object.)

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): i18n keys for match-page prediction stats (en+hu)"
```

---

## Task 3: PredictionStats presentational component

A focused, presentational component: given the rows and `actual`, it calls the Task 1 helper and renders the panel — or renders `null` when the helper returns `null` (gate closed). It pulls labels from `useI18n`. The "hidden until kickoff" note is rendered by `MatchPage` only when the gate is closed (see Task 4), so this component renders **nothing** when there is no data — keeping it a pure stats renderer.

**Files:**
- Create: `web/src/components/PredictionStats.tsx`

- [ ] **Step 1: Write the failing test**

Create `web/src/components/PredictionStats.test.tsx`:

```typescript
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { MatchScore, Tip } from '../graphql/types'
import { I18nProvider } from '../i18n/I18nProvider'
import { PredictionStats } from './PredictionStats'

function tip(id: string, home: number, away: number): Tip {
  return {
    playerId: id,
    nick: id,
    gameId: 'g1',
    prediction: { gameId: 'g1', homeScore: home, awayScore: away, locked: true },
    points: null,
    isPerfect: false,
    breakdown: null,
  }
}

function renderPanel(rows: Tip[], actual: MatchScore | null) {
  return render(
    <I18nProvider>
      <PredictionStats rows={rows} actual={actual} />
    </I18nProvider>,
  )
}

describe('PredictionStats', () => {
  it('renders nothing when no predictions are visible', () => {
    const { container } = renderPanel([], null)
    expect(container.querySelector('.prediction-stats')).toBeNull()
  })

  it('renders the most common scoreline and tip count', () => {
    renderPanel([tip('a', 2, 1), tip('b', 2, 1), tip('c', 1, 0)], null)
    expect(screen.getByText('What everyone tipped')).toBeInTheDocument()
    expect(screen.getByText('2–1')).toBeInTheDocument()
  })

  it('shows the nailed-it count when a final result is in', () => {
    const final: MatchScore = {
      homeScore: 2,
      awayScore: 1,
      provisional: false,
      source: null,
      sourceStatus: null,
      ninetyMinuteUncertain: false,
    }
    renderPanel([tip('a', 2, 1), tip('b', 0, 0)], final)
    expect(screen.getByText(/Nailed the exact score/)).toBeInTheDocument()
  })
})
```

> If `@testing-library/react` / `I18nProvider` import paths differ on this branch, adjust the imports to match the project's existing component tests (search `web/src/components/*.test.tsx`). The component code in Step 3 is the contract; the test asserts behaviour, not import shape.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npx vitest run src/components/PredictionStats.test.tsx`
Expected: FAIL — `Failed to resolve import "./PredictionStats"`.

- [ ] **Step 3: Write minimal implementation**

Create `web/src/components/PredictionStats.tsx`:

```typescript
import { useI18n } from '../i18n/useI18n'
import type { MatchScore, Tip } from '../graphql/types'
import { computePredictionStats } from '../lib/predictionStats'

interface PredictionStatsProps {
  /** The match's visibility-gated tip rows (already pool-scoped by the query). */
  rows: Tip[]
  /** The official/live score, if any — drives the "nailed it" line. */
  actual: MatchScore | null
}

/**
 * Aggregate "what everyone tipped" for one match. Renders nothing until the
 * visibility gate opens (the helper returns `null` when no prediction is
 * visible) — the caller shows a "hidden until kickoff" note in that state.
 */
export function PredictionStats({ rows, actual }: PredictionStatsProps) {
  const { t } = useI18n()
  const stats = computePredictionStats(rows, actual)
  if (!stats) return null

  const scoreLabel =
    stats.mostCommon.length > 1 ? t('mostCommonScorePlural') : t('mostCommonScore')

  return (
    <section className="prediction-stats" aria-label={t('predictionStatsTitle')}>
      <h3 className="prediction-stats-title">{t('predictionStatsTitle')}</h3>

      <dl className="prediction-stats-list">
        <div className="prediction-stats-row">
          <dt>{scoreLabel}</dt>
          <dd>
            {stats.mostCommon.map((s) => (
              <span key={`${s.homeScore}-${s.awayScore}`} className="stats-scoreline">
                {s.homeScore}–{s.awayScore}{' '}
                <small className="stats-count">
                  ×{s.count}
                </small>
              </span>
            ))}
          </dd>
        </div>

        <div className="prediction-stats-row">
          <dt>{t('outcomeSplitLabel')}</dt>
          <dd className="stats-outcome-split">
            <span className="stats-outcome">
              {t('outcomeHome')}: <strong>{stats.outcomeSplit.home}</strong>
            </span>
            <span className="stats-outcome">
              {t('outcomeDraw')}: <strong>{stats.outcomeSplit.draw}</strong>
            </span>
            <span className="stats-outcome">
              {t('outcomeAway')}: <strong>{stats.outcomeSplit.away}</strong>
            </span>
          </dd>
        </div>

        {stats.nailedIt != null && (
          <div className="prediction-stats-row">
            <dt>{t('nailedItLabel')}</dt>
            <dd>
              <strong>{stats.nailedIt}</strong>
            </dd>
          </div>
        )}
      </dl>

      <p className="prediction-stats-total">
        {stats.total} {t('statsTipCount')}
      </p>
    </section>
  )
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npx vitest run src/components/PredictionStats.test.tsx`
Expected: PASS — 3 tests passing.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/PredictionStats.tsx web/src/components/PredictionStats.test.tsx
git commit -m "feat(web): PredictionStats panel component"
```

---

## Task 4: Wire venue + stats into MatchPage

`MatchPage.tsx` already destructures `{ game, actual, rows }` (`:72`) and already owns the local pool selector feeding `effectivePool` → `MATCH_QUERY`'s `pool` var (`:25-40`, `:78-93`). This task adds (a) a venue line in `.match-card`, and (b) the stats panel below the card, with the "hidden until kickoff" note when the gate is closed.

**Gate detection (no `Date.now()`):** the gate is open when at least one **non-own** row has a visible prediction — i.e. the server already revealed others' tips. The viewer's own row is always visible, so it cannot be the signal. `computePredictionStats` over `rows` returns `null` when nothing is visible, but the viewer's own visible prediction would make the panel show with a single tip even pre-gate. To gate correctly, render the panel only when **another** player's prediction is visible.

**Files:**
- Modify: `web/src/pages/MatchPage.tsx`

- [ ] **Step 1: Import the new component and helper**

In `web/src/pages/MatchPage.tsx`, the auth hook gives the viewer. Add imports near the existing ones (after the `PointsBadge` import, `:10`):

```typescript
import { PredictionStats } from '../components/PredictionStats'
```

- [ ] **Step 2: Compute the gate flag and own-player id**

`MatchPage` has `const { label } = useAuth()` (`:21`). The viewer's player id is needed to exclude the own row from the gate check. Read it from `useAuth` — replace `:21`:

```typescript
  const { label, playerId: viewerId } = useAuth()
```

> Verify `useAuth()` exposes `playerId` (search `web/src/auth/useAuth.ts`). If it is named differently (e.g. `id` on a `player` object), adjust this destructure and the `gateOpen` check below to match — the contract is "the viewer's own player id".

Then, after `const { game, actual, rows } = match` (`:72`), add:

```typescript
  // The stats gate mirrors the server: a NON-own row with a visible prediction
  // means the server has revealed others' tips (deadline passed / kickoff). The
  // viewer's own prediction is always visible, so it is excluded here. No
  // Date.now() — the gate is entirely server-derived (the row's `prediction`).
  const gateOpen = rows.some(
    (r) => r.playerId !== viewerId && r.prediction != null,
  )
```

- [ ] **Step 3: Add the venue line inside `.match-card`**

After the `.match-card-kickoff` div (currently `:99-101`), add a venue sibling that hides gracefully when null:

```tsx
        <div className="match-card-kickoff">
          {formatKickoff(game.kickoff, locale)}
        </div>
        {game.venue && (
          <div className="match-card-venue">
            {t('venue')}: {game.venue}
          </div>
        )}
```

- [ ] **Step 4: Mount the stats panel below the match card**

After the closing `</div>` of `.match-card` (currently `:129`) and before the `<table className="data-table compact match-grid">` (`:131`), add:

```tsx
      {gateOpen ? (
        <PredictionStats rows={rows} actual={actual} />
      ) : (
        <p className="match-note match-muted prediction-stats-hidden">
          {t('predictionStatsHidden')}
        </p>
      )}
```

- [ ] **Step 5: Build + lint to verify wiring**

Run: `cd web && npm run build && npm run lint`
Expected: both green — `tsc -b` compiles (all i18n keys resolve, `viewerId` typed), eslint clean.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/MatchPage.tsx
git commit -m "feat(web): render venue and prediction stats on the match page"
```

---

## Task 5: Styles for venue line and stats panel

Match the existing retro card aesthetic (`VT323` / `Press Start 2P`, amber/muted palette) already used by `.match-card-kickoff` and `.match-scoreline`.

**Files:**
- Modify: `web/src/index.css` (after the `.match-card-kickoff` rule, `:1347`)

- [ ] **Step 1: Add the CSS**

In `web/src/index.css`, after the `.match-card-kickoff { … }` block (`:1344-1347`), add:

```css
.match-card-venue {
  font-family: 'VT323', monospace;
  font-size: 15px;
  color: var(--muted);
}
.prediction-stats {
  margin: 12px 0 4px;
  padding: 14px 16px;
  background: var(--bg-card);
  border: 2px solid var(--bg-card-border);
}
.prediction-stats-title {
  font-family: 'Press Start 2P', monospace;
  font-size: 10px;
  letter-spacing: 1px;
  text-transform: uppercase;
  margin: 0 0 10px;
  color: var(--amber-bright);
}
.prediction-stats-list {
  margin: 0;
  display: grid;
  gap: 8px;
}
.prediction-stats-row {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 8px;
  font-family: 'VT323', monospace;
  font-size: 16px;
}
.prediction-stats-row dt {
  color: var(--muted);
}
.prediction-stats-row dd {
  margin: 0;
  color: var(--ink);
}
.stats-scoreline {
  margin-right: 10px;
}
.stats-count {
  color: var(--muted);
}
.stats-outcome-split {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}
.prediction-stats-total {
  font-family: 'VT323', monospace;
  font-size: 14px;
  color: var(--muted);
  margin: 10px 0 0;
  text-align: right;
}
.prediction-stats-hidden {
  text-align: center;
}
```

- [ ] **Step 2: Build to verify CSS is valid**

Run: `cd web && npm run build`
Expected: green — Vite bundles the CSS without error.

- [ ] **Step 3: Commit**

```bash
git add web/src/index.css
git commit -m "feat(web): styles for match-page venue and prediction stats"
```

---

## Task 6: e2e — venue renders, stats gated on visibility

Mirror the existing `web/e2e/match-page.spec.ts` patterns: dev clock presets (`setPreset`), `openGroupD`, `fillScores`, `devLogin`, `watchNetwork`. Use **Group D / M4** to stay isolated from other specs (see the header comment in `match-page.spec.ts`). Two players are needed so the gate can be proven both closed (one tipper, viewer's own only) and open (others' tips revealed post-kickoff).

**e2e dev-stub auth note:** e2e runs with `web/.env.local` blanking `VITE_AUTH0_*` so the dev `.auth-bar` (login picker + dev clock) renders. Confirm `web/.env.local` exists with the Auth0 vars blanked (per project memory `e2e-needs-dev-stub-auth`); the suite's global setup typically handles this, but if the auth bar is missing, create `web/.env.local` with:

```
VITE_AUTH0_DOMAIN=
VITE_AUTH0_CLIENT_ID=
VITE_AUTH0_AUDIENCE=
```

**Files:**
- Create: `web/e2e/match-page-info.spec.ts`

- [ ] **Step 1: Write the e2e spec**

Create `web/e2e/match-page-info.spec.ts`:

```typescript
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Match page (#2) — venue render + prediction-stats visibility gate.
 *
 *   1. Venue: the match card shows a `.match-card-venue` line (the fixture
 *      carries a "Stadium, City" venue for every game — see schedule-venue.spec).
 *   2. Stats hidden BEFORE the gate: with the clock before the group deadline,
 *      another player's tip is still hidden, so `.prediction-stats` is absent and
 *      the `.prediction-stats-hidden` note shows instead.
 *   3. Stats shown AFTER the gate: once kickoff passes, others' tips are revealed
 *      and `.prediction-stats` renders with the most-common scoreline.
 *
 * Uses Group D / M4 for isolation (same convention as match-page.spec.ts). Two
 * tippers (demo-grace + demo-ada) are seeded so the viewer is not the only one
 * with a visible prediction — the gate signal is a NON-own visible tip.
 */

const TEST_GROUP = 'Group D'
const FIRST_GAME = 'M4'

async function setPreset(page: Page, gameId: string, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  await page.evaluate(() =>
    document.documentElement.setAttribute('data-pre-reload', '1'),
  )
  await selects.nth(1).selectOption(phase)
  await page.waitForFunction(
    () => !document.documentElement.hasAttribute('data-pre-reload'),
  )
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

test('match page: venue renders and prediction stats are gated by visibility', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // ── ada tips BEFORE kickoff so there IS another player's tip to reveal ──────
  await devLogin(page, 'demo-ada')
  await setPreset(page, FIRST_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '2', '1')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // ── grace tips BEFORE kickoff; grace is the viewer for the assertions ───────
  await devLogin(page, 'demo-grace')
  await setPreset(page, FIRST_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '2', '1')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // ── Match page BEFORE the gate: venue shows; stats are hidden ───────────────
  await page.goto(`/match/${FIRST_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${FIRST_GAME}`))

  // Venue line renders with the fixture's stadium text.
  const venue = page.locator('.match-card-venue')
  await expect(venue).toBeVisible()
  await expect(venue).toContainText('Venue:')

  // Gate closed (before kickoff, grace not committed to seeing ada's tip):
  // the stats panel is absent and the "hidden until kickoff" note shows.
  await expect(page.locator('.prediction-stats')).toHaveCount(0)
  await expect(page.locator('.prediction-stats-hidden')).toBeVisible()

  // ── Move the clock AFTER kickoff: others' tips reveal, stats appear ─────────
  // Use the dev clock on the match page itself (the auth bar persists).
  await setPreset(page, FIRST_GAME, 'after')
  await page.goto(`/match/${FIRST_GAME}`)

  // Gate open: the stats panel renders with the most-common scoreline (2–1,
  // both ada and grace tipped it) and the hidden note is gone.
  const stats = page.locator('.prediction-stats')
  await expect(stats).toBeVisible()
  await expect(stats).toContainText('What everyone tipped')
  await expect(stats.locator('.stats-scoreline').first()).toContainText('2')
  await expect(page.locator('.prediction-stats-hidden')).toHaveCount(0)

  // Venue still renders after the gate opens.
  await expect(page.locator('.match-card-venue')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

> **If the gate timing differs:** the visibility rule is mutual-commitment (`scored_tip`, `query.rs:79-80`) — another player's tip shows once the viewer has effective-locked OR `time_open` (kickoff/deadline passed). The `after` preset makes `time_open` true, which always reveals. If the `before` preset reveals tips too early on this branch (e.g. both players auto-locked), pin a clock strictly before the Group D deadline using `localStorage` `xpool.devNow` (as `match-page.spec.ts:77-79` does) instead of the `before` preset. The behavioural contract under test is: **hidden note before the gate, stats panel after** — adjust the clock mechanism to satisfy it, not the assertion.

- [ ] **Step 2: Run the e2e spec**

Run: `cd web && npm run e2e -- match-page-info`
Expected: PASS — the spec boots the e2e stack (`global-setup.ts`), runs, and the one test passes. (First run is slow — it builds and seeds the stack.)

- [ ] **Step 3: Commit**

```bash
git add web/e2e/match-page-info.spec.ts
git commit -m "test(web): e2e for match-page venue and gated prediction stats"
```

---

## Task 7: Full cluster verification

REQUIRED SUB-SKILLS: superpowers:requesting-code-review and superpowers:verification-before-completion — run the commands and confirm output **before** claiming done; evidence before assertions.

**Files:** none (verification only).

- [ ] **Step 1: Rust workspace stays green (no backend change, but prove it)**

Run: `cargo build --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`
Expected: build OK, clippy clean (no warnings), all tests pass. (This cluster touches no Rust; this confirms nothing regressed.)

- [ ] **Step 2: Web unit + build + lint**

Run: `cd web && npx vitest run && npm run build && npm run lint`
Expected: vitest green (incl. `predictionStats.test.ts` 8 tests + `PredictionStats.test.tsx` 3 tests); `tsc -b && vite build` green; eslint clean.

- [ ] **Step 3: Full e2e suite**

Run: `cd web && npm run e2e`
Expected: green — including `match-page-info.spec.ts` and the existing `match-page.spec.ts` / `schedule-venue.spec.ts` (no regressions).

- [ ] **Step 4: Completion-bar checklist (confirm each with the evidence above)**

- [ ] `cargo build` / `clippy -D warnings` / `test --workspace` green (Step 1)
- [ ] `npm run build` + `npm run lint` green (Step 2)
- [ ] vitest unit test for the client-side aggregation helper present and passing — `web/src/lib/predictionStats.test.ts`, incl. the "most common scoreline" case (Step 2)
- [ ] Playwright e2e proves venue renders AND stats appear **only after** the gate opens / stay hidden before (Step 3)
- [ ] tip-visibility gating respected — stats aggregate only non-null `prediction` rows; gate is a non-own visible tip
- [ ] NO `Date.now()` / `new Date()` for behavioural logic — gate is server-derived (`prediction` null/non-null), venue uses existing `formatKickoff` (display only)
- [ ] i18n en + hu present for every new key (Task 2; `tsc` enforces both catalogues)

- [ ] **Step 5: Request code review**

Use superpowers:requesting-code-review to review the diff before merge. Verify the reviewer confirms: gate-safety of the aggregation (no leak of hidden tips), no behavioural `Date.now()`, i18n parity, and that the local pool selector is untouched (so the future sticky-pool swap is a clean seam).

- [ ] **Step 6: Final commit / merge**

This is a solo project — merge the branch into `master` locally once green (see CLAUDE.md "Branch discipline"). Open a PR only if self-review-as-a-record adds value.

```bash
git checkout master && git merge --no-ff <branch> && git push
```

---

## Self-Review

**1. Spec coverage**

- VENUE PRD — "informational text only, render `venue` near kickoff, hide when null, i18n label en+hu" → Task 4 Step 3 (render, `{game.venue && …}` hides null), `venue` key already in both catalogues (Task 2 note), Task 6 asserts it. ✓ No map link (PRD resolved). ✓
- STATS PRD — "client-side, pool-scoped, most common scoreline(s)+count, home/draw/away split, N nailed it once result in, hide until gate opens, no backend change" → Task 1 (helper: mostCommon ties, outcomeSplit, nailedIt only on final), Task 3 (panel), Task 4 (gate via non-own visible tip + hidden note), pool scope via existing `effectivePool`/`MATCH_QUERY pool` arg. ✓
- Cross-cluster note — local pool selector kept, integration seam documented in header + Task 7 Step 5. ✓
- Completion bar — Task 7. ✓ Most-common-scoreline unit test — Task 1 Step 1 (`'counts the most common scoreline'`, `'reports all scorelines tied'`). ✓ Pure fn in `web/src/lib/` — `predictionStats.ts`. ✓

**2. Placeholder scan** — every code step shows complete TS/CSS/Rust commands. Two "if it differs on this branch" notes (useAuth `playerId` shape in Task 4 Step 2; gate timing in Task 6 Step 1) are verification guards with a fallback, not placeholders — the contract and primary code are concrete. ✓

**3. Type consistency** — `computePredictionStats(rows: Tip[], actual: MatchScore | null): PredictionStats | null` is used identically in Task 1 (def + test), Task 3 (component), and indirectly Task 4. `PredictionStats` fields (`total`, `mostCommon: ScorelineCount[]`, `outcomeSplit: OutcomeSplit`, `nailedIt: number | null`) match across helper, test, and component. `ScorelineCount` `{ homeScore, awayScore, count }` consistent. i18n keys used in Task 3 (`predictionStatsTitle`, `mostCommonScore`, `mostCommonScorePlural`, `outcomeSplitLabel`, `outcomeHome/Draw/Away`, `nailedItLabel`, `statsTipCount`) and Task 4 (`predictionStatsHidden`, `venue`) are all defined in Task 2 (or pre-existing for `venue`). CSS classes in Task 3/4 (`.prediction-stats`, `.prediction-stats-title`, `.prediction-stats-list`, `.prediction-stats-row`, `.stats-scoreline`, `.stats-count`, `.stats-outcome-split`, `.match-card-venue`, `.prediction-stats-hidden`) all styled in Task 5 and asserted in Task 6. ✓
