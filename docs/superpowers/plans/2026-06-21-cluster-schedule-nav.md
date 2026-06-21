# Schedule-Nav Cluster Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a By-group ⇄ By-date toggle to the Schedule page (persisted per-user, day-bucketed in the viewer's local timezone), and add a player-gated "My player page" nav item that replaces the Profile nav link (Profile stays reachable from the own player-detail view).

**Architecture:** Two independent frontend features in one cluster.
(1) Schedule: a pure `src/lib/scheduleByDate.ts` helper buckets `SingleGame[]` into calendar-day sections using locale formatting of the ISO kickoff (no `Date.now()` — day boundaries are the viewer's browser tz, the same basis as `formatKickoff`); `SchedulePage` gains a localStorage-persisted view toggle and renders either the existing group sections or the new date sections, reusing the same match-row markup and `/match/:id` links.
(2) Nav: `Layout` passes the current player (`me`) into `NavBar`; `NavBar` drops the static `Profile` item and renders a dynamic player-gated `My player page` item targeting `/player/<me.id>` (hidden for visitor / unclaimed / result-user). A Profile/settings link is added to `PlayerPage` when `isOwn`.

**Tech Stack:** React 19 + TypeScript, react-router-dom 7, urql, Vite. Unit tests: Vitest (`src/**/*.test.ts`, `npm run test`). E2E: Playwright (`web/e2e/`, `npm run e2e`) — boots its own live stack via `e2e/global-setup.ts`. i18n: `src/i18n/strings.ts` (en + hu).

**Conventions to honour (from CLAUDE.md / .specs/TESTING.md):**
- Server-authoritative clock — app logic never branches on `Date.now()`. Calendar-day **grouping** by locale-formatting an ISO kickoff string is allowed (it is display formatting, identical in basis to the existing `formatKickoff`), and is isolated in a documented, unit-tested pure helper.
- Immutability: never mutate inputs; build new arrays/objects.
- i18n is first-class: every new user-facing string goes in BOTH `en` and `hu`.
- Three-state `CurrentPlayer`: `me` is a real `Player` only when `me.__typename === 'Player'`; Visitor (no session) and Unclaimed see no player-only chrome. The result-user (`isResultUser`) is excluded from the own-player nav item.

---

## File Structure

**New files**
- `web/src/lib/scheduleByDate.ts` — pure helpers: `dayKey(iso, locale)` (calendar-day section key) and `groupByDay(games, locale)` (ordered day sections). One responsibility: turn a flat, kickoff-ordered game list into ordered calendar-day buckets.
- `web/src/lib/scheduleByDate.test.ts` — Vitest unit tests for the above.
- `web/e2e/schedule-by-date.spec.ts` — Playwright: the By-date toggle sections matches by day and the choice persists across reload.
- `web/e2e/nav-my-player-page.spec.ts` — Playwright: the nav item appears for a logged-in demo player, routes to `/player/:id`, Profile is gone from the nav but reachable from the own player page.

**Modified files**
- `web/src/i18n/strings.ts` — add `scheduleViewByGroup`, `scheduleViewByDate`, `playerProfileLink` keys (en + hu). Reuse existing `playerPageOwnLink`, `navProfile`.
- `web/src/pages/SchedulePage.tsx` — add the view toggle + localStorage persistence + By-date rendering.
- `web/src/components/NavBar.tsx` — accept `me`, drop the static Profile item, render the dynamic player-gated `My player page` item.
- `web/src/components/Layout.tsx` — pass `me` to `NavBar`.
- `web/src/pages/PlayerPage.tsx` — add a Profile/settings link when `isOwn`.

---

## Task 1: Pure day-bucketing helper (`scheduleByDate.ts`)

**Files:**
- Create: `web/src/lib/scheduleByDate.ts`
- Test: `web/src/lib/scheduleByDate.test.ts`

Day bucketing must be timezone-honest: two kickoffs that fall on the same **local** calendar day share a section, even when their UTC dates differ. We derive the section key from `Intl.DateTimeFormat` parts (locale + browser tz) rather than slicing the ISO string, so the boundary is the viewer's local midnight — the same basis `formatKickoff` already renders against. No `Date.now()`: the key depends only on the kickoff string and locale.

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/scheduleByDate.test.ts`:

```typescript
import { describe, expect, it } from 'vitest'
import type { SingleGame } from '../graphql/types'
import { dayKey, groupByDay } from './scheduleByDate'

const game = (id: string, kickoff: string): SingleGame =>
  ({ id, kickoff }) as SingleGame

describe('dayKey', () => {
  it('returns the same key for two kickoffs on the same local day', () => {
    // Same local calendar day in UTC (used by the CI/runner tz-independently:
    // both are 2026-06-20 in UTC).
    const a = dayKey('2026-06-20T09:00:00Z', 'en')
    const b = dayKey('2026-06-20T21:00:00Z', 'en')
    expect(a).toBe(b)
  })

  it('returns different keys for kickoffs on different local days', () => {
    const a = dayKey('2026-06-20T12:00:00Z', 'en')
    const b = dayKey('2026-06-21T12:00:00Z', 'en')
    expect(a).not.toBe(b)
  })

  it('returns a stable, non-empty key for a valid date', () => {
    const k = dayKey('2026-06-20T12:00:00Z', 'en')
    expect(k.length).toBeGreaterThan(0)
  })

  it('falls back to the raw string for an unparseable date', () => {
    expect(dayKey('not-a-date', 'en')).toBe('not-a-date')
  })
})

describe('groupByDay', () => {
  it('buckets games into one section per local calendar day', () => {
    const games = [
      game('a', '2026-06-20T12:00:00Z'),
      game('b', '2026-06-20T18:00:00Z'),
      game('c', '2026-06-21T12:00:00Z'),
    ]
    const sections = groupByDay(games, 'en')
    expect(sections).toHaveLength(2)
    expect(sections[0].games.map((g) => g.id)).toEqual(['a', 'b'])
    expect(sections[1].games.map((g) => g.id)).toEqual(['c'])
  })

  it('orders sections chronologically and games within a section by kickoff', () => {
    const games = [
      game('c', '2026-06-21T12:00:00Z'),
      game('b', '2026-06-20T18:00:00Z'),
      game('a', '2026-06-20T12:00:00Z'),
    ]
    const sections = groupByDay(games, 'en')
    expect(sections.map((s) => s.games.map((g) => g.id))).toEqual([
      ['a', 'b'],
      ['c'],
    ])
  })

  it('gives each section a stable key and a human label', () => {
    const sections = groupByDay([game('a', '2026-06-20T12:00:00Z')], 'en')
    expect(sections[0].key.length).toBeGreaterThan(0)
    expect(sections[0].label.length).toBeGreaterThan(0)
  })

  it('does not mutate the input array', () => {
    const games = [
      game('b', '2026-06-20T18:00:00Z'),
      game('a', '2026-06-20T12:00:00Z'),
    ]
    const before = games.map((g) => g.id)
    groupByDay(games, 'en')
    expect(games.map((g) => g.id)).toEqual(before)
  })

  it('returns an empty list for no games', () => {
    expect(groupByDay([], 'en')).toEqual([])
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npm run test -- scheduleByDate`
Expected: FAIL — `Failed to resolve import "./scheduleByDate"` / "dayKey is not a function".

- [ ] **Step 3: Write minimal implementation**

Create `web/src/lib/scheduleByDate.ts`:

```typescript
import type { SingleGame } from '../graphql/types'

/** A calendar-day section of the schedule, in the viewer's local timezone. */
export interface DaySection {
  /** Stable, sortable key for the local calendar day (sortable ISO `y-m-d`). */
  key: string
  /** Human-readable day heading, locale-formatted (e.g. "Sat, Jun 20, 2026"). */
  label: string
  /** Games kicking off on this local day, ordered by kickoff ascending. */
  games: SingleGame[]
}

/**
 * Calendar-day key for an ISO kickoff, in the viewer's LOCAL timezone — the
 * same basis `formatKickoff` renders against. Derived from `Intl` date parts
 * (not a string slice), so the day boundary is the viewer's local midnight.
 * Returns a sortable `YYYY-MM-DD` string; falls back to the raw input for an
 * unparseable date. Depends only on `iso` + `locale` — never on `Date.now()`.
 */
export function dayKey(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const parts = new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(d)
  const year = parts.find((p) => p.type === 'year')?.value ?? ''
  const month = parts.find((p) => p.type === 'month')?.value ?? ''
  const day = parts.find((p) => p.type === 'day')?.value ?? ''
  return `${year}-${month}-${day}`
}

/** Human-readable heading for a local calendar day. */
function dayLabel(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleDateString(locale, {
    weekday: 'short',
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

/**
 * Bucket a flat game list into ordered local-calendar-day sections. Sections
 * are ordered chronologically; games within each section are ordered by
 * kickoff ascending. Immutable — the input array is not mutated.
 */
export function groupByDay(games: SingleGame[], locale: string): DaySection[] {
  const sorted = [...games].sort(
    (a, b) => Date.parse(a.kickoff) - Date.parse(b.kickoff),
  )
  const byKey = new Map<string, DaySection>()
  for (const g of sorted) {
    const key = dayKey(g.kickoff, locale)
    const existing = byKey.get(key)
    if (existing) {
      byKey.set(key, { ...existing, games: [...existing.games, g] })
    } else {
      byKey.set(key, { key, label: dayLabel(g.kickoff, locale), games: [g] })
    }
  }
  return [...byKey.values()].sort((a, b) => a.key.localeCompare(b.key))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npm run test -- scheduleByDate`
Expected: PASS — all `dayKey` and `groupByDay` tests green.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/scheduleByDate.ts web/src/lib/scheduleByDate.test.ts
git commit -m "feat(web): add pure local-day bucketing helper for schedule"
```

---

## Task 2: i18n strings for the schedule toggle + profile link

**Files:**
- Modify: `web/src/i18n/strings.ts`

`playerPageOwnLink` ('My player page' / 'Saját oldalam') already exists (en line ~191, hu line ~484) and is reused by the nav item. We add three new keys: the two toggle labels and a Profile/settings link label for the own player page.

- [ ] **Step 1: Add the keys to the English block**

In `web/src/i18n/strings.ts`, find the `// schedule` block in `en`:

```typescript
  // schedule
  scheduleTitle: 'Schedule',
```

Replace it with:

```typescript
  // schedule
  scheduleTitle: 'Schedule',
  scheduleViewByGroup: 'By group',
  scheduleViewByDate: 'By date',
```

- [ ] **Step 2: Add the profile-link key to the English player-detail block**

In the `en` block, find:

```typescript
  // player-detail page
  playerNotInPool: 'This player is not in your pool.',
  playerPageOwnLink: 'My player page',
```

Replace with:

```typescript
  // player-detail page
  playerNotInPool: 'This player is not in your pool.',
  playerPageOwnLink: 'My player page',
  playerProfileLink: 'Profile & settings',
```

- [ ] **Step 3: Add the matching keys to the Hungarian block**

In the `hu` block, find:

```typescript
  scheduleTitle: 'Menetrend',
```

Replace with:

```typescript
  scheduleTitle: 'Menetrend',
  scheduleViewByGroup: 'Csoport szerint',
  scheduleViewByDate: 'Dátum szerint',
```

- [ ] **Step 4: Add the profile-link key to the Hungarian player-detail block**

In the `hu` block, find:

```typescript
  // player-detail page
  playerNotInPool: 'Ez a játékos nincs a ligádban.',
  playerPageOwnLink: 'Saját oldalam',
```

Replace with:

```typescript
  // player-detail page
  playerNotInPool: 'Ez a játékos nincs a ligádban.',
  playerPageOwnLink: 'Saját oldalam',
  playerProfileLink: 'Adatok és beállítások',
```

- [ ] **Step 5: Verify the catalogue still type-checks**

Run: `cd web && npx tsc -b --noEmit`
Expected: exit 0, no errors. (The `hu` block is typed `Record<StringKey, string>`, so a missing translation would fail here.)

- [ ] **Step 6: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): add i18n strings for schedule view toggle and profile link"
```

---

## Task 3: SchedulePage — By-group ⇄ By-date view toggle

**Files:**
- Modify: `web/src/pages/SchedulePage.tsx`

We add a persisted view toggle. The chosen view is stored in `localStorage` under `xpool.scheduleView`. By-group keeps the existing leaf-group rendering verbatim; By-date reuses the identical match-row markup but iterates `groupByDay(...)` sections. The match row (`formatKickoff` cell, the `/match/:id` Matchup link, venue, result) is extracted into a single `MatchRows` sub-render so both views share it exactly (DRY).

- [ ] **Step 1: Write the full SchedulePage implementation**

Replace the entire contents of `web/src/pages/SchedulePage.tsx` with:

```typescript
import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { RESULTS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type {
  MatchPrediction,
  SingleGame,
  Team,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { byKickoff, formatKickoff, teamIndex } from '../lib/format'
import { groupByDay } from '../lib/scheduleByDate'
import { Matchup } from '../components/TeamLabel'
import { roundLabel } from '../lib/rounds'

type ScheduleView = 'group' | 'date'

const VIEW_STORAGE_KEY = 'xpool.scheduleView'

/** Read the persisted view, defaulting to 'group'. Tolerates SSR/no-storage. */
function readView(): ScheduleView {
  try {
    return localStorage.getItem(VIEW_STORAGE_KEY) === 'date' ? 'date' : 'group'
  } catch {
    return 'group'
  }
}

/** Persist the chosen view per-user. Swallows storage failures (private mode). */
function persistView(view: ScheduleView): void {
  try {
    localStorage.setItem(VIEW_STORAGE_KEY, view)
  } catch {
    // ignore — persistence is best-effort
  }
}

/** Full fixture list (UC-12). Public, read-only. Toggle: by group ⇄ by date. */
export function SchedulePage() {
  const { t, locale } = useI18n()
  const [view, setView] = useState<ScheduleView>(readView)
  const [result, reexecute] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })

  const tournament = result.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )
  const resultsByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsResult.data?.results ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [resultsResult.data])

  function chooseView(next: ScheduleView) {
    setView(next)
    persistView(next)
  }

  if (result.fetching) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!tournament) return <ErrorView />

  // Leaf groups (those holding matches), ordered chronologically by their
  // deadline — the earliest kickoff in the group — so the schedule reads in
  // time order: group stage A–L, then each knockout round.
  const leafGroups = tournament.groups
    .filter((g) => g.childGameIds.length > 0)
    .sort((a, b) => {
      const da = a.deadline ? Date.parse(a.deadline) : Number.POSITIVE_INFINITY
      const db = b.deadline ? Date.parse(b.deadline) : Number.POSITIVE_INFINITY
      return da - db
    })

  // By-date sections — all games bucketed by the viewer's local calendar day.
  const daySections = groupByDay(tournament.games, locale)

  return (
    <section className="page">
      <h2>{t('scheduleTitle')}</h2>

      <div className="schedule-view-toggle" role="group" aria-label={t('scheduleTitle')}>
        <button
          type="button"
          className={view === 'group' ? 'view-tab active' : 'view-tab'}
          aria-pressed={view === 'group'}
          onClick={() => chooseView('group')}
        >
          {t('scheduleViewByGroup')}
        </button>
        <button
          type="button"
          className={view === 'date' ? 'view-tab active' : 'view-tab'}
          aria-pressed={view === 'date'}
          onClick={() => chooseView('date')}
        >
          {t('scheduleViewByDate')}
        </button>
      </div>

      {view === 'group'
        ? leafGroups.map((group) => {
            const games = tournament.games
              .filter((m) => group.childGameIds.includes(m.id))
              .sort(byKickoff)
            return (
              <div key={group.id} className="schedule-group">
                <h3>
                  {group.name}{' '}
                  <span className="round-tag">{roundLabel(group.round, t)}</span>
                </h3>
                <ScheduleTable
                  games={games}
                  teams={teams}
                  resultsByGame={resultsByGame}
                  locale={locale}
                  t={t}
                />
              </div>
            )
          })
        : daySections.map((section) => (
            <div key={section.key} className="schedule-day">
              <h3>{section.label}</h3>
              <ScheduleTable
                games={section.games}
                teams={teams}
                resultsByGame={resultsByGame}
                locale={locale}
                t={t}
              />
            </div>
          ))}
    </section>
  )
}

/** Shared fixture table — identical rows for both the group and date views. */
function ScheduleTable({
  games,
  teams,
  resultsByGame,
  locale,
  t,
}: {
  games: SingleGame[]
  teams: Map<string, Team>
  resultsByGame: Map<string, MatchPrediction>
  locale: string
  t: (key: Parameters<ReturnType<typeof useI18n>['t']>[0]) => string
}) {
  return (
    <table className="data-table">
      <thead>
        <tr>
          <th>{t('kickoff')}</th>
          <th className="col-match">{t('match')}</th>
          <th>{t('venue')}</th>
          <th>{t('result')}</th>
        </tr>
      </thead>
      <tbody>
        {games.map((m) => {
          const r = resultsByGame.get(m.id)
          return (
            <tr key={m.id}>
              <td>{formatKickoff(m.kickoff, locale)}</td>
              <td>
                <Link to={`/match/${m.id}`}>
                  <Matchup home={m.home} away={m.away} teams={teams} />
                </Link>
              </td>
              <td>{m.venue ?? '—'}</td>
              <td>{r ? `${r.homeScore}–${r.awayScore}` : '—'}</td>
            </tr>
          )
        })}
      </tbody>
    </table>
  )
}
```

- [ ] **Step 2: Verify it type-checks and lints**

Run: `cd web && npx tsc -b --noEmit && npm run lint`
Expected: exit 0 for both. (The `t` prop typing reuses the `useI18n` translate signature so passing `t` down stays type-safe. If the inline `t` type proves awkward in your TS version, replace the `t:` prop type with `t: (key: import('../i18n/strings').StringKey) => string` and import `StringKey`.)

- [ ] **Step 3: Add toggle styling**

Append to `web/src/index.css` (or the project's main stylesheet — confirm with `grep -rl "schedule-group" web/src --include=*.css`; add to whichever file already styles `.schedule-group`):

```css
.schedule-view-toggle {
  display: inline-flex;
  gap: 0.25rem;
  margin-bottom: 1rem;
}
.schedule-view-toggle .view-tab {
  padding: 0.35rem 0.9rem;
  border: 1px solid var(--border, #ccc);
  background: transparent;
  color: inherit;
  cursor: pointer;
  border-radius: 0.4rem;
}
.schedule-view-toggle .view-tab.active {
  background: var(--accent, #2a6);
  color: #fff;
  border-color: var(--accent, #2a6);
}
.schedule-day h3 {
  margin-top: 1.5rem;
}
```

If no CSS file styles `.schedule-group`, place these rules in `web/src/index.css`.

- [ ] **Step 4: Verify the build is still green**

Run: `cd web && npm run build`
Expected: `tsc -b` then `vite build` complete with exit 0.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/SchedulePage.tsx web/src/index.css
git commit -m "feat(web): add By-group/By-date toggle to schedule page, persisted per-user"
```

---

## Task 4: NavBar — replace Profile with the player-gated "My player page" item

**Files:**
- Modify: `web/src/components/NavBar.tsx`
- Modify: `web/src/components/Layout.tsx`

The static `ITEMS` array can't express a dynamic `/player/<me.id>` target. We give `NavBar` the current `me` (a `Player | null`), remove `Profile` from `ITEMS`, and append the own-player item only when `me` is a real Player and not the result user. `Layout` already computes `me` — we just thread it down. The `isPlayer`/`isAdmin` props stay (they drive the optimistic player-nav signal and the admin gate, which `me` alone can't, since `me` may still be in flight).

- [ ] **Step 1: Rewrite NavBar to accept `me` and render the dynamic item**

Replace the entire contents of `web/src/components/NavBar.tsx` with:

```typescript
import { NavLink } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import type { StringKey } from '../i18n/strings'
import type { Player } from '../graphql/types'
import { accessFor } from '../auth/routeAccess'

interface NavItem {
  to: string
  label: StringKey
}

// Profile is intentionally absent here — it is replaced in the nav by the
// dynamic "My player page" item below, and remains reachable from the own
// player-detail page (and its /profile route still exists).
const ITEMS: NavItem[] = [
  { to: '/', label: 'navHome' },
  { to: '/today', label: 'navToday' },
  { to: '/games', label: 'navGames' },
  { to: '/mytips', label: 'navMyTips' },
  { to: '/alltips', label: 'navAllTips' },
  { to: '/scoreboard', label: 'navScoreboard' },
  { to: '/perfect', label: 'navPerfect' },
  { to: '/pools', label: 'navPools' },
  { to: '/rules', label: 'navRules' },
  { to: '/admin', label: 'navAdmin' },
]

/**
 * `isPlayer` is true only for a real linked Player — NOT merely "a session
 * exists". An authenticated-but-unclaimed viewer is not a player, so the
 * player-only links stay hidden and they see the invite dead-end instead.
 * Access per route comes from the shared `accessFor` map (single source with
 * `Layout`'s dead-end gating).
 *
 * `me` (when present) supplies the id for the dynamic "My player page" item,
 * which targets `/player/<me.id>`. It is shown only for a real Player who is
 * not the result user (the result user has no participant page).
 */
export function NavBar({
  isPlayer,
  isAdmin,
  me,
}: {
  isPlayer: boolean
  isAdmin: boolean
  me: Player | null
}) {
  const { t } = useI18n()

  const visible = ITEMS.filter((item) => {
    const access = accessFor(item.to)
    if (access === 'player') return isPlayer
    if (access === 'admin') return isAdmin
    return true
  })

  const showOwnPlayer = me !== null && !me.isResultUser

  return (
    <nav className="nav-bar">
      {visible.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.to === '/'}
          className={({ isActive }) => (isActive ? 'nav-link active' : 'nav-link')}
        >
          {t(item.label)}
        </NavLink>
      ))}
      {showOwnPlayer && (
        <NavLink
          key="own-player"
          to={`/player/${me.id}`}
          className={({ isActive }) => (isActive ? 'nav-link active' : 'nav-link')}
        >
          {t('playerPageOwnLink')}
        </NavLink>
      )}
    </nav>
  )
}
```

- [ ] **Step 2: Pass `me` from Layout into NavBar**

In `web/src/components/Layout.tsx`, find:

```typescript
      <NavBar isPlayer={showPlayerNav} isAdmin={Boolean(me?.isResultUser)} />
```

Replace with:

```typescript
      <NavBar isPlayer={showPlayerNav} isAdmin={Boolean(me?.isResultUser)} me={me} />
```

(`me` is already computed in `Layout` at line 37 as `meRaw?.__typename === 'Player' ? meRaw : null`, exactly the `Player | null` shape NavBar now expects.)

- [ ] **Step 3: Verify it type-checks and lints**

Run: `cd web && npx tsc -b --noEmit && npm run lint`
Expected: exit 0 for both.

- [ ] **Step 4: Commit**

```bash
git add web/src/components/NavBar.tsx web/src/components/Layout.tsx
git commit -m "feat(web): replace Profile nav link with player-gated My player page item"
```

---

## Task 5: PlayerPage — Profile/settings link on the own view

**Files:**
- Modify: `web/src/pages/PlayerPage.tsx`

Since Profile left the nav, the own player-detail page becomes its entry point. Add a Profile/settings link, shown only when `isOwn`, near the page heading.

- [ ] **Step 1: Import Link and render the own-page profile link**

In `web/src/pages/PlayerPage.tsx`, the first import line is:

```typescript
import { useParams } from 'react-router-dom'
```

Replace it with:

```typescript
import { Link, useParams } from 'react-router-dom'
```

- [ ] **Step 2: Add the link under the heading**

Find:

```typescript
    <section className="page player-page">
      <h2>{entry.nick}</h2>
      <PlayerHeader entry={entry} rank={rank} />
```

Replace with:

```typescript
    <section className="page player-page">
      <h2>{entry.nick}</h2>
      {isOwn && (
        <p className="player-profile-link">
          <Link to="/profile">{t('playerProfileLink')}</Link>
        </p>
      )}
      <PlayerHeader entry={entry} rank={rank} />
```

- [ ] **Step 3: Verify it type-checks, lints, builds**

Run: `cd web && npx tsc -b --noEmit && npm run lint && npm run build`
Expected: exit 0 for all three.

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/PlayerPage.tsx
git commit -m "feat(web): add Profile & settings link to own player-detail page"
```

---

## Task 6: E2E — schedule By-date toggle sections by day and persists

**Files:**
- Create: `web/e2e/schedule-by-date.spec.ts`

This is a public, read-only page — no login needed (mirrors `schedule-order.spec.ts`). It proves: (a) toggling to By-date renders day sections (`.schedule-day`) with the same `/match/:id` links, and (b) the choice survives a reload via localStorage.

- [ ] **Step 1: Write the failing e2e test**

Create `web/e2e/schedule-by-date.spec.ts`:

```typescript
import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Schedule By-date view. The schedule page offers a By-group ⇄ By-date toggle.
 * By-date buckets every fixture into local-calendar-day sections (reusing the
 * same match rows / /match/:id links), and the chosen view persists per-user
 * in localStorage across a reload. Public, read-only — no login required.
 */
test('By-date toggle sections matches by day and persists across reload', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/games')

  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Default view is By group.
  await expect(page.locator('.schedule-group').first()).toBeVisible()
  await expect(page.locator('.schedule-day')).toHaveCount(0)

  // Switch to By date → day sections appear, group sections disappear.
  await page.getByRole('button', { name: 'By date' }).click()
  const daySections = page.locator('.schedule-day')
  await expect(daySections.first()).toBeVisible()
  expect(
    await daySections.count(),
    'the tournament spans multiple calendar days',
  ).toBeGreaterThan(1)
  await expect(page.locator('.schedule-group')).toHaveCount(0)

  // Each day section still has a day heading and clickable /match/:id rows.
  await expect(daySections.first().locator('h3')).not.toBeEmpty()
  const firstMatchLink = daySections
    .first()
    .locator('tbody tr')
    .first()
    .locator('a[href^="/match/"]')
  await expect(firstMatchLink).toBeVisible()

  // Day sections are chronological: first section's first kickoff <= second's.
  const firstKickoffOf = async (i: number) => {
    const cell = daySections.nth(i).locator('tbody tr').first().locator('td').first()
    return Date.parse(((await cell.textContent()) ?? '').trim())
  }
  expect(await firstKickoffOf(1)).toBeGreaterThanOrEqual(await firstKickoffOf(0))

  // Persistence: reload → By-date is still the active view.
  await page.reload()
  await expect(page.locator('.schedule-day').first()).toBeVisible()
  await expect(page.locator('.schedule-group')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'By date' })).toHaveAttribute(
    'aria-pressed',
    'true',
  )

  // Toggle back to By group → persisted again.
  await page.getByRole('button', { name: 'By group' }).click()
  await expect(page.locator('.schedule-group').first()).toBeVisible()
  await page.reload()
  await expect(page.locator('.schedule-group').first()).toBeVisible()
  await expect(page.locator('.schedule-day')).toHaveCount(0)

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Ensure dev-stub auth is configured for e2e**

The e2e stack must run in dev-stub auth mode (no Auth0), or `.auth-bar`-dependent tests in the suite break. Confirm `web/.env.local` exists and blanks the Auth0 vars:

Run: `cd web && cat .env.local 2>/dev/null || echo MISSING`
Expected: contains `VITE_AUTH0_DOMAIN=` and `VITE_AUTH0_CLIENT_ID=` (empty values). If it prints `MISSING`, create `web/.env.local` with:

```
VITE_AUTH0_DOMAIN=
VITE_AUTH0_CLIENT_ID=
```

(This spec itself needs no login, but Task 7's spec and the rest of the suite do — keep dev-stub mode on.)

- [ ] **Step 3: Run the e2e test**

Run: `cd web && npm run e2e -- schedule-by-date`
Expected: PASS — 1 passed. (Playwright boots the live stack via `e2e/global-setup.ts`; first run is slow.)

- [ ] **Step 4: Commit**

```bash
git add web/e2e/schedule-by-date.spec.ts web/.env.local
git commit -m "test(web): e2e for schedule By-date toggle sectioning and persistence"
```

---

## Task 7: E2E — "My player page" nav item + Profile reachable from own page

**Files:**
- Create: `web/e2e/nav-my-player-page.spec.ts`

Proves: a logged-in demo player sees the `My player page` nav item routing to their `/player/:id`; `Profile` is gone from the nav; Profile is reachable from the own player page via the new link.

- [ ] **Step 1: Write the failing e2e test**

Create `web/e2e/nav-my-player-page.spec.ts`:

```typescript
import { test, expect } from '@playwright/test'
import { devLogin, devLogout, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Nav "My player page" (own-player-page-access). For a logged-in demo player a
 * player-gated nav item routes to their /player/:id. The static Profile nav
 * link is removed, but Profile stays reachable from the own player-detail page.
 */
test('My player page nav item routes to own /player/:id; Profile moved off the nav', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // Visitor (not logged in): the player-gated item is absent.
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'My player page' }),
  ).toHaveCount(0)

  await devLogin(page, 'demo-ada')

  // Profile is no longer in the nav.
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'Profile', exact: true }),
  ).toHaveCount(0)

  // The "My player page" nav item is present and routes to demo-ada's page.
  const ownNav = page
    .locator('.nav-bar')
    .getByRole('link', { name: 'My player page' })
  await expect(ownNav).toBeVisible()
  await ownNav.click()
  await expect(page).toHaveURL(/\/player\/demo-ada$/)
  await expectNoErrorView(page)

  // Profile is reachable from the own player page via the new link.
  const profileLink = page
    .locator('.player-profile-link')
    .getByRole('link', { name: 'Profile & settings' })
  await expect(profileLink).toBeVisible()
  await profileLink.click()
  await expect(page).toHaveURL(/\/profile$/)
  await expectNoErrorView(page)

  // The result user has no own-player nav item (they are excluded).
  await devLogout(page)
  await devLogin(page, 'result-user')
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'My player page' }),
  ).toHaveCount(0)

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the e2e test**

Run: `cd web && npm run e2e -- nav-my-player-page`
Expected: PASS — 1 passed.

(If `devLogin(page, 'result-user')` fails to find the option, the picker labels the result user with a suffix but the `value` is its id — `devLogin` selects by `option[value="result-user"]`, which matches. Confirm the seed id with `grep -rn "result-user" ../crates/xtask/src` if the option is missing.)

- [ ] **Step 3: Commit**

```bash
git add web/e2e/nav-my-player-page.spec.ts
git commit -m "test(web): e2e for My player page nav item and Profile relocation"
```

---

## Task 8: Cluster completion bar — full verification

**Files:** none (verification only).

REQUIRED SUB-SKILLS: Use superpowers:verification-before-completion (run every command, confirm output before claiming green) and superpowers:requesting-code-review (request review before declaring the cluster done). Evidence before assertions — paste real command output, never infer.

- [ ] **Step 1: Workspace stays green (frontend-only cluster, but keep Rust green)**

Run: `cargo build --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`
Expected: build + clippy + test all exit 0. (No Rust changed, so this is a no-regression check. DynamoDB integration tests stay skipped without `DYNAMO_TEST=1`.)

- [ ] **Step 2: Frontend unit tests + coverage**

Run: `cd web && npm run test`
Expected: all Vitest suites pass, including `scheduleByDate` (the new `src/lib/` module keeps `src/lib/` ≥ 80% line/branch/function/statement coverage per `vitest.config.ts`).

- [ ] **Step 3: Frontend build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: `tsc -b` + `vite build` succeed; eslint reports 0 problems.

- [ ] **Step 4: Grep guard — no `Date.now()` introduced in app logic**

Run: `cd web && grep -rn "Date.now()" src/lib/scheduleByDate.ts src/pages/SchedulePage.tsx src/components/NavBar.tsx src/pages/PlayerPage.tsx`
Expected: no matches (exit 1, empty output). Day bucketing uses `Intl`/`toLocaleDateString` formatting of the ISO kickoff only — no clock branching.

- [ ] **Step 5: Full e2e suite**

Run: `cd web && npm run e2e`
Expected: all specs pass, including the two new specs (`schedule-by-date`, `nav-my-player-page`) and the unchanged `schedule-order`, `player-page`, `auth` suites (no regression from the NavBar/Layout/SchedulePage changes).

- [ ] **Step 6: Visual sanity check (manual)**

Per the user's standing rule "green e2e/tsc/lint ≠ looks right": boot the dev stack (`bin/local-dev`), open `http://localhost:5173/games`, toggle By group ⇄ By date (confirm the active tab styles and day headings read sensibly), reload to confirm persistence; log in as `demo-ada` and confirm the `My player page` nav item sits where Profile was and the own page shows the `Profile & settings` link. Confirm both views render in Hungarian after switching locale in the settings gear.

- [ ] **Step 7: Request code review**

Use superpowers:requesting-code-review to verify the cluster meets the two PRDs' resolved decisions:
- Schedule: By group ⇄ By date toggle, day-bucketed in viewer-local tz, reusing match rows / `/match/:id` links, persisted per-user in localStorage, Today page untouched.
- Nav: player-gated `My player page` item → `/player/<my id>`, hidden for visitor/unclaimed/result-user, AuthBar name link kept, Profile removed from nav items but `/profile` route+page kept and reachable from the own player page; `playerPageOwnLink` reused.

Address any CRITICAL/HIGH findings, then re-run Steps 1–5.

- [ ] **Step 8: Finish the branch**

REQUIRED SUB-SKILL: Use superpowers:finishing-a-development-branch to merge into `master` (solo project — merge locally; open a PR only if the reviewer flagged complexity worth a CI record). Per CLAUDE.md branch discipline, all `web/` changes were made on a branch/worktree, not directly on `master`.

---

## Self-Review (completed by plan author)

**1. Spec coverage**
- PRD timeline-schedule → By group ⇄ By date toggle: Task 3. Day-bucketing by viewer-local day: Task 1. Reuse match rows / `/match/:id`: Task 3 (`ScheduleTable`). Persist per-user in localStorage: Task 3 (`readView`/`persistView`). Today page stays distinct: untouched — not modified by any task. ✓
- PRD own-player-page-access → top-nav `My player page` → `/player/<my id>`, player-gated, hidden for visitor/unclaimed/result-user: Task 4. Reuse `playerPageOwnLink`: Tasks 2/4. Keep AuthBar name link: untouched. Replace Profile nav link, keep `/profile` route+page: Task 4 (removed from `ITEMS`; App route untouched). Add Profile link on own player view: Task 5. Current id from ME_QUERY (`__typename==='Player'`): Task 4 via Layout's `me`. ✓
- TESTING.md → no `Date.now()` in app logic: enforced by Task 8 Step 4 grep; day key uses `Intl` formatting. i18n en+hu: Task 2. E2E proof: Tasks 6 & 7. ✓

**2. Placeholder scan:** No TBD/TODO/"handle edge cases"; every code step shows complete code; commands have expected output. ✓

**3. Type consistency:** `ScheduleView` ('group'|'date'), `VIEW_STORAGE_KEY`, `readView`/`persistView`/`chooseView`, `DaySection {key,label,games}`, `dayKey`/`groupByDay`, `NavBar` prop `me: Player | null` matching Layout's `me`, `playerProfileLink`/`scheduleViewByGroup`/`scheduleViewByDate` keys — all consistent across tasks. `ScheduleTable` is referenced in Task 3 only and defined in the same file. ✓
