# cluster/mytips-nav Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship two serialized My Tips features — (A) URL-hash anchors that smooth-scroll into any group/knockout sub-section, then (B) a mobile-specific swipe-one-group-per-screen prediction-entry flow with big steppers and autosave.

**Architecture:** Both features edit `web/src/pages/MyTipsPage.tsx` and `web/src/pages/mytips/*`, so they **must be built in order: A first (structural, leaves the form UX untouched), then B (refactors score-entry inside the now-anchored structure).** Pure logic goes in `web/src/lib/` (vitest-covered, ≥80%); React hooks go in a new `web/src/hooks/` dir (deliberately outside the lib coverage glob, mirroring how `usePolledQuery.ts` is excluded). New components live as small files under `web/src/pages/mytips/`. Server-authoritative clock is preserved — locked/read-only state comes from server flags (`group.deadlinePassed`, `MatchPrediction.locked`), never `Date.now()`.

**Tech Stack:** React 18 + TypeScript + Vite, urql GraphQL client, vitest (node env, pure-logic only), Playwright e2e (isolated live stack on ports :3001/:5174/:8001), CSS in `web/src/index.css` (mobile breakpoint `@media (max-width: 640px)`).

---

## File Structure

**Part A — knockout-subgroup-anchors (build FIRST):**
- Create `web/src/lib/hashAnchor.ts` — pure `hashToId(hash)` helper (+ test).
- Create `web/src/hooks/useHashScroll.ts` — effect hook: smooth-scroll the `#id` element into view + brief highlight.
- Modify `web/src/pages/mytips/GroupTipForm.tsx` — add `id={group.id}` to the root `.tip-form` div.
- Modify `web/src/index.css` — `scroll-margin-top` + `anchor-pulse` highlight keyframes.
- Modify `web/src/pages/MyTipsPage.tsx` — read `useLocation().hash`, call `useHashScroll`.
- Create `web/e2e/anchor-deeplink.spec.ts` — deep-link a knockout round, assert the target section scrolled into view.

**Part B — mobile-prediction-entry (build SECOND):**
- Create `web/src/lib/score.ts` — pure `clampScore` / `stepScore` / `predictedCount` (+ test).
- Create `web/src/lib/debounce.ts` — pure `debounce(fn, delay)` factory (+ test).
- Create `web/src/hooks/useDebouncedCallback.ts` — React wrapper over `debounce`.
- Create `web/src/hooks/useIsMobile.ts` — extracted from `useResolvedDisplayMode.ts` (shared 640px media query).
- Modify `web/src/display/useResolvedDisplayMode.ts` — import the extracted `useIsMobile` (DRY).
- Create `web/src/pages/mytips/types.ts` — shared `PredictionInput` / `StandingsInput`.
- Modify `web/src/pages/mytips/GroupTipForm.tsx` — import those types instead of local copies.
- Create `web/src/pages/mytips/ScoreStepper.tsx` — big +/− thumb-friendly stepper.
- Create `web/src/pages/mytips/MobileGroupCard.tsx` — one group's card: steppers, autosave, saved/N-of-M, read-only, finalize.
- Create `web/src/pages/mytips/MobileGroupEntry.tsx` — swipe shell: progress, Prev/Next, renders the active card.
- Modify `web/src/pages/MyTipsPage.tsx` — render `MobileGroupEntry` when `isMobile && isGroupStage && !me.isResultUser`.
- Modify `web/src/index.css` — mobile card + stepper styles.
- Modify `web/src/i18n/strings.ts` — append EN + HU keys.
- Create `web/e2e/mobile-prediction-entry.spec.ts` — mobile-viewport stepper entry + autosave + Next group.

---

# PART A — knockout-subgroup-anchors (BUILD FIRST)

> Resolved scope (`.scratch/knockout-subgroup-anchors/PRD.md` — 2026-06-27 grill): element `id` = stable `group.id` (e.g. `KO-M76`, or `E`); smooth-scroll on hash; cover BOTH group-stage and knockout; the round tab stays the only routed level (react-router), the hash is client-side scroll; optional brief highlight on arrive.

## Task A1: `hashToId` pure helper

**Files:**
- Create: `web/src/lib/hashAnchor.ts`
- Test: `web/src/lib/hashAnchor.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/hashAnchor.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { hashToId } from './hashAnchor'

describe('hashToId', () => {
  it('strips the leading # from a knockout group id', () => {
    expect(hashToId('#KO-M76')).toBe('KO-M76')
  })

  it('returns a group-stage id unchanged', () => {
    expect(hashToId('#E')).toBe('E')
  })

  it('returns empty string for an empty hash', () => {
    expect(hashToId('')).toBe('')
  })

  it('returns empty string for a bare #', () => {
    expect(hashToId('#')).toBe('')
  })

  it('decodes a percent-encoded hash', () => {
    expect(hashToId('#KO%2DM76')).toBe('KO-M76')
  })

  it('falls back to the raw value when decoding throws', () => {
    expect(hashToId('#%E0%A4%A')).toBe('%E0%A4%A')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npm run test -- hashAnchor`
Expected: FAIL — `Cannot find module './hashAnchor'`.

- [ ] **Step 3: Write minimal implementation**

Create `web/src/lib/hashAnchor.ts`:

```ts
/**
 * Normalise a `location.hash` (`#KO-M76`) to a DOM element id (`KO-M76`).
 * Returns `''` for an empty/bare hash so callers can early-return. The id is a
 * stable `group.id` (group-stage `A`..`L` or knockout `KO-M73`), so a percent-
 * encoded hash is decoded; an undecodable hash falls back to its raw form.
 */
export function hashToId(hash: string): string {
  const raw = hash.replace(/^#/, '')
  if (!raw) return ''
  try {
    return decodeURIComponent(raw)
  } catch {
    return raw
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npm run test -- hashAnchor`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/hashAnchor.ts web/src/lib/hashAnchor.test.ts
git commit -m "feat(web): hashToId helper for My Tips anchors"
```

## Task A2: `useHashScroll` hook

**Files:**
- Create: `web/src/hooks/useHashScroll.ts`

(No vitest — this is an effect hook touching `document`/`requestAnimationFrame`; the vitest env is `node` with no DOM. It is exercised by the e2e in Task A6. It lives under `web/src/hooks/` so it is outside the `src/lib/**` coverage glob.)

- [ ] **Step 1: Write the hook**

Create `web/src/hooks/useHashScroll.ts`:

```ts
import { useEffect } from 'react'
import { hashToId } from '../lib/hashAnchor'

/** Briefly applied to the scrolled-to section (see `.tip-form--anchored`). */
const ANCHOR_CLASS = 'tip-form--anchored'
const HIGHLIGHT_MS = 1600

/**
 * Smooth-scroll the element whose id matches `location.hash` into view and
 * pulse it briefly. `contentKey` is a signal that the anchorable content has
 * (re)rendered — e.g. `"R32:10"` (active round + section count) — so the scroll
 * re-runs after async data loads or a round-tab switch, when the target element
 * first exists in the DOM. A `requestAnimationFrame` lets that render commit
 * before we look the element up.
 *
 * The hash is client-side scroll only; react-router still owns the routed round
 * level (`/mytips/:groupId`). No `Date.now()` / clock involvement.
 */
export function useHashScroll(hash: string, contentKey: string): void {
  useEffect(() => {
    const id = hashToId(hash)
    if (!id) return
    const raf = requestAnimationFrame(() => {
      const el = document.getElementById(id)
      if (!el) return
      el.scrollIntoView({ behavior: 'smooth', block: 'start' })
      el.classList.add(ANCHOR_CLASS)
      window.setTimeout(() => el.classList.remove(ANCHOR_CLASS), HIGHLIGHT_MS)
    })
    return () => cancelAnimationFrame(raf)
  }, [hash, contentKey])
}
```

- [ ] **Step 2: Type-check**

Run: `cd web && npx tsc -b`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/hooks/useHashScroll.ts
git commit -m "feat(web): useHashScroll for smooth anchor scrolling"
```

## Task A3: Give each group section a stable id

**Files:**
- Modify: `web/src/pages/mytips/GroupTipForm.tsx:243-244`

- [ ] **Step 1: Add the `id` to the root div**

In `web/src/pages/mytips/GroupTipForm.tsx`, change the opening tag of the returned root element:

```tsx
  return (
    <div className="tip-form">
```

to:

```tsx
  return (
    <div className="tip-form" id={group.id}>
```

- [ ] **Step 2: Type-check + build**

Run: `cd web && npm run build`
Expected: build succeeds (the `id` prop is valid; covers both group-stage and knockout because every section renders a `GroupTipForm`).

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/mytips/GroupTipForm.tsx
git commit -m "feat(web): anchorable id on each group tip section"
```

## Task A4: Anchor highlight CSS

**Files:**
- Modify: `web/src/index.css` (append a block after the existing `.tip-form` rules; locate `.tip-form,` near line 960)

- [ ] **Step 1: Add scroll-margin + pulse keyframes**

Append to `web/src/index.css` (end of file is fine):

```css
/* ============================================================================
   MY TIPS ANCHORS — smooth-scroll target offset + brief arrive pulse
   ============================================================================ */
.tip-form {
  scroll-margin-top: 16px;
}
@keyframes anchor-pulse {
  0%   { box-shadow: 0 0 0 2px var(--amber, #ff8c00); }
  100% { box-shadow: 0 0 0 2px transparent; }
}
.tip-form--anchored {
  animation: anchor-pulse 1.4s ease-out;
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/index.css
git commit -m "feat(web): anchor-arrive highlight + scroll offset"
```

## Task A5: Wire the hook into MyTipsPage

**Files:**
- Modify: `web/src/pages/MyTipsPage.tsx` (imports at top; hook call after `tipsGroupId` is derived, before the early returns at line ~218)

- [ ] **Step 1: Add imports**

In `web/src/pages/MyTipsPage.tsx`, change:

```tsx
import { useNavigate, useParams } from 'react-router-dom'
```

to:

```tsx
import { useLocation, useNavigate, useParams } from 'react-router-dom'
```

and add, after the existing `import { GroupTipForm } from './mytips/GroupTipForm'` line:

```tsx
import { useHashScroll } from '../hooks/useHashScroll'
```

- [ ] **Step 2: Read the location and call the hook**

In the component body, just after `const navigate = useNavigate()` (line ~52) add:

```tsx
  const location = useLocation()
```

Then, immediately AFTER the `const tipsGroupId = ...` line (line ~155, which is still above every early `return`), add:

```tsx
  // Hash anchors: scroll the `#<group.id>` section into view once the active
  // round's sections have rendered. `contentKey` re-triggers the scroll after
  // async data loads or a round switch changes which sections exist.
  useHashScroll(location.hash, `${activeRound ?? ''}:${roundLeaves.length}`)
```

- [ ] **Step 3: Build**

Run: `cd web && npm run build && npm run lint`
Expected: both green.

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/MyTipsPage.tsx
git commit -m "feat(web): scroll My Tips to the hash-anchored section"
```

## Task A6: e2e — deep-link a knockout round to a sub-section

**Files:**
- Create: `web/e2e/anchor-deeplink.spec.ts`

Notes for the implementer:
- Knockout rounds are **hidden until a feeding game has both teams determined** (`visibleRoundNodes`). So the spec first logs in as `result-user` and seeds **Group C + Group F** (this resolves R32 matches M75/M76, making the R32 round visible) — exactly the setup `mytips-knockout-labels.spec.ts` uses.
- The R32 round-node group id is `R32`; its leaf one-match groups are `KO-M73`..`KO-M92`. Deep-link `/mytips/R32#KO-M80` selects the R32 round (react-router) and the hash scrolls to the `KO-M80` section, which renders even with undetermined teams.

- [ ] **Step 1: Write the spec**

Create `web/e2e/anchor-deeplink.spec.ts`:

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * My Tips hash anchors (`/mytips/<round>#<group.id>`) smooth-scroll the target
 * section into view, without adding a second tab level. The round tab is the
 * only routed level; the hash is client-side scroll.
 *
 * Setup: result-user seeds Group C + F so the R32 round becomes visible
 * (M75/M76 get both teams), then we deep-link to a section LOW in the stacked
 * R32 list (KO-M80) and assert the page scrolled to it.
 */
const CLOCK = '2026-06-26T12:00:00Z'
const TARGET = 'KO-M80'

async function openGroupStageGroup(page: Page, groupName: string): Promise<void> {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await page
    .locator('.group-subnav button', { hasText: new RegExp(`^${groupName}$`) })
    .click()
  await expect(page.locator('.tip-form h3').first()).toContainText(groupName)
}

async function fillAllAndSave(page: Page): Promise<void> {
  const rows = page.locator('.tip-form table.data-table').first().locator('tbody tr')
  const count = await rows.count()
  expect(count, 'group has matches').toBeGreaterThan(0)
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption('2')
    await selects.nth(1).selectOption('1')
  }
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.tip-form .flash-bar')).toContainText('Saved')
}

test('My Tips: deep-link scrolls to a knockout sub-section', async ({ page }) => {
  await page.addInitScript((value: string) => {
    localStorage.setItem('xpool.devNow', value)
  }, CLOCK)

  const net = watchNetwork(page)
  await page.goto('/')

  // Make R32 visible by resolving its feeding groups.
  await devLogin(page, 'result-user')
  await openGroupStageGroup(page, 'Group C')
  await fillAllAndSave(page)
  await openGroupStageGroup(page, 'Group F')
  await fillAllAndSave(page)

  // A regular player deep-links straight to a low R32 sub-section.
  await devLogin(page, 'demo-margaret')
  await page.goto(`/mytips/R32#${TARGET}`)

  // The R32 round tab is active and the target section exists.
  await expect(
    page.locator('.round-tab.active', { hasText: /^Round of 32$/ }),
  ).toBeVisible()
  const target = page.locator(`#${TARGET}`)
  await expect(target).toBeVisible()

  // The page scrolled (not still pinned at the top) and the target is in view.
  await expect
    .poll(async () => page.evaluate(() => window.scrollY), { timeout: 10_000 })
    .toBeGreaterThan(0)
  await expect
    .poll(async () =>
      target.evaluate((el) => {
        const r = el.getBoundingClientRect()
        return r.top >= 0 && r.top < window.innerHeight
      }),
    )
    .toBe(true)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the spec**

Run: `cd web && npm run e2e -- anchor-deeplink`
Expected: PASS. (First run boots the isolated stack; allow time.)

- [ ] **Step 3: Commit**

```bash
git add web/e2e/anchor-deeplink.spec.ts
git commit -m "test(web): e2e deep-link scrolls to knockout sub-section"
```

## Task A7: Part A verification checkpoint

- [ ] **Step 1: Full web gate**

Run: `cd web && npm run build && npm run lint && npm run test`
Expected: all green (coverage thresholds hold — `hashAnchor.ts` is fully tested).

- [ ] **Step 2: Workspace gate**

Run: `cargo build --workspace && cargo test --workspace`
Expected: green (no Rust changed; this confirms the tree is clean).

- [ ] **Step 3: Commit any incidental fixes, then proceed to Part B.**

---

# PART B — mobile-prediction-entry (BUILD SECOND)

> Resolved scope (`.scratch/mobile-prediction-entry/PRD.md` — 2026-06-27 grill): SWIPE one-group-per-screen (card per group + progress "Group C · 3 of 12", swipe / "Next group"); big +/− steppers replacing the tiny 0–9 `<select>`; autosave drafts + per-group "saved / N of M predicted"; deadline-aware read-only when locked; a MOBILE-SPECIFIC view/flow (not just responsive); knockout / draw-order entry stays desktop-only this round.

## Task B1: Score-stepper pure logic

**Files:**
- Create: `web/src/lib/score.ts`
- Test: `web/src/lib/score.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/score.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { clampScore, predictedCount, SCORE_MAX, stepScore } from './score'

describe('clampScore', () => {
  it('floors at 0', () => expect(clampScore(-3)).toBe(0))
  it('caps at SCORE_MAX', () => expect(clampScore(99)).toBe(SCORE_MAX))
  it('truncates fractional input', () => expect(clampScore(3.9)).toBe(3))
  it('treats NaN as 0', () => expect(clampScore(Number.NaN)).toBe(0))
})

describe('stepScore', () => {
  it('first + from unset commits 0', () => expect(stepScore(null, 1)).toBe(0))
  it('- from unset stays unset', () => expect(stepScore(null, -1)).toBeNull())
  it('+ increments within range', () => expect(stepScore(2, 1)).toBe(3))
  it('- decrements within range', () => expect(stepScore(2, -1)).toBe(1))
  it('- below 0 unsets the value', () => expect(stepScore(0, -1)).toBeNull())
  it('+ cannot exceed SCORE_MAX', () => expect(stepScore(SCORE_MAX, 1)).toBe(SCORE_MAX))
})

describe('predictedCount', () => {
  it('counts only fully-entered matches', () => {
    expect(
      predictedCount([
        { home: 1, away: 0 },
        { home: 2, away: null },
        { home: null, away: null },
        { home: 0, away: 0 },
      ]),
    ).toBe(2)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npm run test -- score`
Expected: FAIL — `Cannot find module './score'`.

- [ ] **Step 3: Write minimal implementation**

Create `web/src/lib/score.ts`:

```ts
/**
 * Pure helpers for the mobile +/− score stepper. Scores are non-negative
 * integers; `null` means "not yet entered". `SCORE_MAX` is a generous sanity
 * cap (the legacy desktop `<select>` only offered 0–9, but real scores can run
 * higher, so we cap at 20 rather than 9).
 */
export const SCORE_MIN = 0
export const SCORE_MAX = 20

export function clampScore(n: number): number {
  if (Number.isNaN(n)) return SCORE_MIN
  return Math.max(SCORE_MIN, Math.min(SCORE_MAX, Math.trunc(n)))
}

/**
 * Apply a +1 / -1 step. `+` from unset commits the minimum (0); `-` from unset
 * stays unset; `-` below 0 unsets again; `+` saturates at `SCORE_MAX`.
 */
export function stepScore(current: number | null, delta: number): number | null {
  if (current === null) {
    return delta > 0 ? SCORE_MIN : null
  }
  const next = current + delta
  if (next < SCORE_MIN) return null
  return clampScore(next)
}

/** How many matches have BOTH sides entered. */
export function predictedCount(
  matches: ReadonlyArray<{ home: number | null; away: number | null }>,
): number {
  return matches.filter((m) => m.home !== null && m.away !== null).length
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npm run test -- score`
Expected: PASS (all describe blocks green).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/score.ts web/src/lib/score.test.ts
git commit -m "feat(web): pure score-stepper logic for mobile entry"
```

## Task B2: Debounce pure core

**Files:**
- Create: `web/src/lib/debounce.ts`
- Test: `web/src/lib/debounce.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/debounce.test.ts`:

```ts
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { debounce } from './debounce'

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

describe('debounce', () => {
  it('fires once after the delay with the latest args', () => {
    const spy = vi.fn()
    const d = debounce(spy, 200)
    d.call('a')
    d.call('b')
    expect(spy).not.toHaveBeenCalled()
    vi.advanceTimersByTime(200)
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledWith('b')
  })

  it('cancel() prevents a pending call', () => {
    const spy = vi.fn()
    const d = debounce(spy, 200)
    d.call('x')
    d.cancel()
    vi.advanceTimersByTime(200)
    expect(spy).not.toHaveBeenCalled()
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npm run test -- debounce`
Expected: FAIL — `Cannot find module './debounce'`.

- [ ] **Step 3: Write minimal implementation**

Create `web/src/lib/debounce.ts`:

```ts
export interface Debounced<A extends unknown[]> {
  call: (...args: A) => void
  cancel: () => void
}

/**
 * Trailing-edge debounce: `call` schedules `fn` after `delayMs`, replacing any
 * pending invocation so only the latest args fire. Pure (no React) so it is
 * unit-testable with fake timers; the `useDebouncedCallback` hook wraps it.
 */
export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): Debounced<A> {
  let timer: ReturnType<typeof setTimeout> | null = null
  const cancel = () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }
  const call = (...args: A) => {
    cancel()
    timer = setTimeout(() => {
      timer = null
      fn(...args)
    }, delayMs)
  }
  return { call, cancel }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npm run test -- debounce`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/debounce.ts web/src/lib/debounce.test.ts
git commit -m "feat(web): pure debounce factory"
```

## Task B3: `useDebouncedCallback` hook

**Files:**
- Create: `web/src/hooks/useDebouncedCallback.ts`

- [ ] **Step 1: Write the hook**

Create `web/src/hooks/useDebouncedCallback.ts`:

```ts
import { useEffect, useMemo, useRef } from 'react'
import { debounce } from '../lib/debounce'

/**
 * A stable debounced callback. The latest `fn` is always invoked (via a ref),
 * so callers can pass a fresh closure each render without resetting the timer.
 * The pending call is cancelled on unmount / delay change.
 */
export function useDebouncedCallback<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): (...args: A) => void {
  const fnRef = useRef(fn)
  useEffect(() => {
    fnRef.current = fn
  }, [fn])

  const debounced = useMemo(
    () => debounce((...args: A) => fnRef.current(...args), delayMs),
    [delayMs],
  )

  useEffect(() => debounced.cancel, [debounced])

  return debounced.call
}
```

- [ ] **Step 2: Type-check**

Run: `cd web && npx tsc -b`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/hooks/useDebouncedCallback.ts
git commit -m "feat(web): useDebouncedCallback hook"
```

## Task B4: Extract `useIsMobile` (shared 640px query)

**Files:**
- Create: `web/src/hooks/useIsMobile.ts`
- Modify: `web/src/display/useResolvedDisplayMode.ts`

- [ ] **Step 1: Create the shared hook**

Create `web/src/hooks/useIsMobile.ts`:

```ts
import { useEffect, useState } from 'react'

// Matches the SPA's mobile breakpoint in index.css (640px) so layout-mode
// branches flip exactly when the CSS goes mobile.
export const MOBILE_QUERY = '(max-width: 640px)'

/** Track the mobile media query, updating live on resize / rotate. */
export function useIsMobile(): boolean {
  const [isMobile, setIsMobile] = useState(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return false
    return window.matchMedia(MOBILE_QUERY).matches
  })

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return
    const mql = window.matchMedia(MOBILE_QUERY)
    const onChange = (e: MediaQueryListEvent) => setIsMobile(e.matches)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [])

  return isMobile
}
```

- [ ] **Step 2: Refactor `useResolvedDisplayMode.ts` to import it**

Replace the entire contents of `web/src/display/useResolvedDisplayMode.ts` with:

```ts
import {
  composeDisplayMode,
  type ConcreteDisplayMode,
} from '../lib/displayMode'
import { useDisplayMode } from './useDisplayMode'
import { useIsMobile } from '../hooks/useIsMobile'

/** The two display axes composed into a concrete rendering for the viewport. */
export function useResolvedDisplayMode(): ConcreteDisplayMode {
  const { flag, text } = useDisplayMode()
  const isMobile = useIsMobile()
  return composeDisplayMode(flag, text, isMobile)
}
```

- [ ] **Step 3: Verify nothing else relied on the inline copy**

Run: `cd web && npm run build && npm run test`
Expected: green (the `displayMode` composition tests still pass; this was a pure extraction).

- [ ] **Step 4: Commit**

```bash
git add web/src/hooks/useIsMobile.ts web/src/display/useResolvedDisplayMode.ts
git commit -m "refactor(web): extract shared useIsMobile hook"
```

## Task B5: Shared tip-input types

**Files:**
- Create: `web/src/pages/mytips/types.ts`
- Modify: `web/src/pages/mytips/GroupTipForm.tsx:26-35`

- [ ] **Step 1: Create the shared types**

Create `web/src/pages/mytips/types.ts`:

```ts
/** Inputs to `submitGroup` — shared by the desktop form and the mobile flow. */
export interface PredictionInput {
  gameId: string
  homeScore: number
  awayScore: number
}

export interface StandingsInput {
  ordering: string[]
  drawOrder: string[]
}
```

- [ ] **Step 2: Use them in `GroupTipForm.tsx`**

In `web/src/pages/mytips/GroupTipForm.tsx`, delete the two local interface declarations:

```tsx
interface PredictionInput {
  gameId: string
  homeScore: number
  awayScore: number
}

interface StandingsInput {
  ordering: string[]
  drawOrder: string[]
}
```

and add this import next to the existing `./StandingsTables` import:

```tsx
import type { PredictionInput, StandingsInput } from './types'
```

- [ ] **Step 3: Build**

Run: `cd web && npm run build`
Expected: green (signatures unchanged).

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/mytips/types.ts web/src/pages/mytips/GroupTipForm.tsx
git commit -m "refactor(web): share tip-input types across My Tips forms"
```

## Task B6: i18n strings (EN + HU)

**Files:**
- Modify: `web/src/i18n/strings.ts` (EN block ends ~line 376; HU block ends ~line 716)

- [ ] **Step 1: Append the EN keys**

In `web/src/i18n/strings.ts`, inside the `const en = { ... }` object, just before its closing `}` (right after the `enterAllGamesHint` / My Tips group, e.g. after the `lockedNotice` line ~188), add:

```ts
  // mobile prediction entry
  mobileOf: 'of',
  mobilePredicted: 'predicted',
  mobileSaving: 'Saving…',
  mobileSaveError: 'Save failed',
  nextGroup: 'Next group',
  prevGroup: 'Previous group',
  incScore: 'increase',
  decScore: 'decrease',
```

- [ ] **Step 2: Append the matching HU keys**

Inside the `const hu: Record<StringKey, string> = { ... }` object (after the corresponding `lockedNotice` HU line ~538), add:

```ts
  // mobile prediction entry
  mobileOf: '/',
  mobilePredicted: 'megtippelve',
  mobileSaving: 'Mentés…',
  mobileSaveError: 'Mentés sikertelen',
  nextGroup: 'Következő csoport',
  prevGroup: 'Előző csoport',
  incScore: 'növel',
  decScore: 'csökkent',
```

- [ ] **Step 3: Type-check (the `hu` record is keyed by `StringKey`, so any mismatch fails the build)**

Run: `cd web && npm run build`
Expected: green — both blocks have identical keys.

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): i18n strings for mobile prediction entry"
```

## Task B7: `ScoreStepper` component

**Files:**
- Create: `web/src/pages/mytips/ScoreStepper.tsx`

- [ ] **Step 1: Write the component**

Create `web/src/pages/mytips/ScoreStepper.tsx`:

```tsx
import { useI18n } from '../../i18n/useI18n'
import { stepScore } from '../../lib/score'

/**
 * Big thumb-friendly +/− score stepper for the mobile entry flow, replacing the
 * tiny 0–9 `<select>`. `null` renders as `–` (not yet entered). All numeric
 * clamping lives in the pure `stepScore` helper.
 */
export function ScoreStepper({
  value,
  onChange,
}: {
  value: number | null
  onChange: (next: number | null) => void
}) {
  const { t } = useI18n()
  return (
    <span className="score-stepper">
      <button
        type="button"
        className="score-stepper-dec"
        aria-label={t('decScore')}
        onClick={() => onChange(stepScore(value, -1))}
      >
        −
      </button>
      <span className="score-stepper-value">{value === null ? '–' : value}</span>
      <button
        type="button"
        className="score-stepper-inc"
        aria-label={t('incScore')}
        onClick={() => onChange(stepScore(value, +1))}
      >
        +
      </button>
    </span>
  )
}
```

- [ ] **Step 2: Build**

Run: `cd web && npm run build`
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/mytips/ScoreStepper.tsx
git commit -m "feat(web): big +/- ScoreStepper for mobile entry"
```

## Task B8: `MobileGroupCard` component

**Files:**
- Create: `web/src/pages/mytips/MobileGroupCard.tsx`

Notes:
- Group-stage only (the caller gates on `isGroupStage && !me.isResultUser`).
- Autosave (lock=false) is debounced 800ms and does NOT refetch `me`, so typing is never interrupted. Standings ordering is auto-derived from the draft scores (`computeStandings`) with `drawOrder: []` — draw-order tie editing stays desktop-only, but we still submit the derived ordering so group standings keep scoring.
- Read-only is server-driven: `group.deadlinePassed` OR every match locked — never `Date.now()`.

- [ ] **Step 1: Write the component**

Create `web/src/pages/mytips/MobileGroupCard.tsx`:

```tsx
import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import type { OperationResult } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import type {
  GroupGame,
  MatchPrediction,
  Player,
  PointsBreakdown,
  StandingsScore,
  Tournament,
} from '../../graphql/types'
import { byKickoff, teamIndex } from '../../lib/format'
import { computeStandings, applyDrawOrder } from '../../lib/standings'
import { predictedCount } from '../../lib/score'
import { Matchup } from '../../components/TeamLabel'
import { Countdown } from '../../components/Countdown'
import { PointsBadge } from '../../components/PointsBadge'
import { InlineConfirm } from '../../components/InlineConfirm'
import { ScoreStepper } from './ScoreStepper'
import { useDebouncedCallback } from '../../hooks/useDebouncedCallback'
import type { PredictionInput, StandingsInput } from './types'

interface Cell {
  home: number | null
  away: number | null
}

type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

/**
 * One group's mobile prediction card: big steppers, debounced autosave, a
 * per-group "N of M predicted" + save status, server-driven read-only, and a
 * Finalize action. Group-stage score entry only.
 */
export function MobileGroupCard({
  tournament,
  group,
  me,
  results,
  pointsByGame,
  serverNowMs,
  onExpire,
  onAutosave,
  onFinalize,
}: {
  tournament: Tournament
  group: GroupGame
  me: Player
  results: MatchPrediction[]
  pointsByGame?: Map<
    string,
    { breakdown: PointsBreakdown | null; isPerfect: boolean }
  >
  /** Reserved for future per-group bonus display; unused for now. */
  standings?: StandingsScore | null
  serverNowMs: number
  onExpire?: () => void
  onAutosave: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
  onFinalize: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
}) {
  const { t, locale } = useI18n()
  const teams = useMemo(() => teamIndex(tournament.teams, locale), [tournament, locale])

  const games = useMemo(
    () =>
      tournament.games
        .filter((g) => group.childGameIds.includes(g.id))
        .sort(byKickoff),
    [tournament, group],
  )

  const resultsByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of results) map.set(r.gameId, r)
    return map
  }, [results])

  const initial = useMemo(() => {
    const map: Record<string, Cell> = {}
    for (const g of games) {
      const p = me.matchPredictions.find((mp) => mp.gameId === g.id)
      map[g.id] = { home: p ? p.homeScore : null, away: p ? p.awayScore : null }
    }
    return map
  }, [games, me])

  // Seeded once; the parent keys this card by `group.id`, so switching groups
  // remounts and reseeds from the freshest `me`.
  const [cells, setCells] = useState<Record<string, Cell>>(initial)
  const [status, setStatus] = useState<SaveStatus>('idle')
  const [busy, setBusy] = useState(false)

  const deadlinePassed = group.deadlinePassed
  const allLocked =
    games.length > 0 &&
    games.every((g) => me.matchPredictions.find((p) => p.gameId === g.id)?.locked)
  const readOnly = (deadlinePassed || allLocked) && !me.isResultUser

  const total = games.length
  const done = predictedCount(games.map((g) => cells[g.id]))
  const allComplete = total > 0 && done === total

  const buildPredictions = (state: Record<string, Cell>): PredictionInput[] =>
    games
      .filter((g) => state[g.id].home !== null && state[g.id].away !== null)
      .map((g) => ({
        gameId: g.id,
        homeScore: state[g.id].home as number,
        awayScore: state[g.id].away as number,
      }))

  const buildStandings = (state: Record<string, Cell>): StandingsInput | null => {
    if (!group.carriesStandings) return null
    const ranked = applyDrawOrder(
      computeStandings(games, (gid) => {
        const c = state[gid]
        return c.home !== null && c.away !== null
          ? { home: c.home, away: c.away }
          : null
      }),
      [],
    )
    return { ordering: ranked.map((s) => s.teamId), drawOrder: [] }
  }

  const persist = useDebouncedCallback((state: Record<string, Cell>) => {
    void (async () => {
      try {
        const res = await onAutosave(
          group.id,
          buildPredictions(state),
          buildStandings(state),
        )
        setStatus(res.error ? 'error' : 'saved')
      } catch {
        setStatus('error')
      }
    })()
  }, 800)

  const setScore = (gameId: string, side: 'home' | 'away', value: number | null) => {
    const next = { ...cells, [gameId]: { ...cells[gameId], [side]: value } }
    setCells(next)
    setStatus('saving')
    persist(next)
  }

  const finalize = async () => {
    setBusy(true)
    try {
      await onFinalize(group.id, buildPredictions(cells), buildStandings(cells))
    } finally {
      setBusy(false)
    }
  }

  const statusText =
    status === 'saving'
      ? t('mobileSaving')
      : status === 'saved'
        ? t('saved')
        : status === 'error'
          ? t('mobileSaveError')
          : ''

  return (
    <div className="mobile-group-card">
      <div className="mobile-group-head">
        <h3>
          {group.name}{' '}
          <span className={readOnly ? 'state-locked' : 'state-draft'}>
            {readOnly ? t('locked') : t('draft')}
          </span>
        </h3>
        {!readOnly && group.deadline && (
          <span className="finalize-countdown">
            {t('finalizeBy')}{' '}
            <Countdown
              deadline={group.deadline}
              serverNowMs={serverNowMs}
              onExpire={onExpire}
            />
          </span>
        )}
      </div>

      <div className="mobile-group-status">
        <span>
          {done} {t('mobileOf')} {total} {t('mobilePredicted')}
        </span>
        {statusText && (
          <span className={`mobile-save-status${status === 'saved' ? ' saved' : ''}`}>
            {statusText}
          </span>
        )}
      </div>

      {readOnly && <p className="flash-bar">{t('lockedNotice')}</p>}

      <div className="mobile-matches">
        {games.map((game) => {
          const c = cells[game.id]
          const teamsPlaced = !!game.home.teamId && !!game.away.teamId
          const result = resultsByGame.get(game.id)
          const pt = pointsByGame?.get(game.id)
          return (
            <div className="mobile-match" key={game.id}>
              <Link to={`/match/${game.id}`}>
                <Matchup home={game.home} away={game.away} teams={teams} />
              </Link>
              {!teamsPlaced ? (
                <span className="hint">{t('teamsNotDetermined')}</span>
              ) : readOnly ? (
                <div className="mobile-match-scores">
                  <span className="score-locked">
                    {c.home === null ? '–' : c.home} :{' '}
                    {c.away === null ? '–' : c.away}
                  </span>
                </div>
              ) : (
                <div className="mobile-match-scores">
                  <ScoreStepper
                    value={c.home}
                    onChange={(v) => setScore(game.id, 'home', v)}
                  />
                  <span className="mobile-match-sep">:</span>
                  <ScoreStepper
                    value={c.away}
                    onChange={(v) => setScore(game.id, 'away', v)}
                  />
                </div>
              )}
              {result && (
                <span className="mobile-match-result">
                  {t('result')}: {result.homeScore}–{result.awayScore}
                </span>
              )}
              {pt?.breakdown && (
                <PointsBadge breakdown={pt.breakdown} isPerfect={pt.isPerfect} />
              )}
            </div>
          )
        })}
      </div>

      {!readOnly && (
        <div className="tip-actions">
          <InlineConfirm
            className="primary"
            confirmClassName="primary"
            disabled={busy || !allComplete}
            title={!allComplete ? t('enterAllGamesHint') : ''}
            question={t('lockConfirm')}
            onConfirm={() => void finalize()}
          >
            {t('lockGroup')}
          </InlineConfirm>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: green. (If lint flags the unused `standings` prop, keep it but reference it in a comment or remove the prop and its pass-through in Task B9 — the prop is reserved; the simplest fix is to drop `standings` from both the type and the call site. Prefer dropping it if lint is strict.)

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/mytips/MobileGroupCard.tsx
git commit -m "feat(web): MobileGroupCard with steppers + autosave"
```

## Task B9: `MobileGroupEntry` swipe shell

**Files:**
- Create: `web/src/pages/mytips/MobileGroupEntry.tsx`

- [ ] **Step 1: Write the component**

Create `web/src/pages/mytips/MobileGroupEntry.tsx`:

```tsx
import { useRef } from 'react'
import type { OperationResult } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import type {
  GroupGame,
  MatchPrediction,
  Player,
  PointsBreakdown,
  StandingsScore,
  Tournament,
} from '../../graphql/types'
import { MobileGroupCard } from './MobileGroupCard'
import type { PredictionInput, StandingsInput } from './types'

/**
 * Swipe one-group-per-screen mobile prediction flow. Shows a progress line
 * ("Group C · 3 of 12"), Prev/Next controls and left/right swipe, and renders
 * the active group's `MobileGroupCard`. Group selection is driven through
 * `onSelectGroup` (which navigates `/mytips/<id>`), so the URL stays the source
 * of truth and the desktop nav stays consistent.
 */
export function MobileGroupEntry({
  tournament,
  groups,
  activeGroupId,
  onSelectGroup,
  me,
  results,
  pointsByGame,
  standingsByGroup,
  serverNowMs,
  onExpire,
  onAutosave,
  onFinalize,
}: {
  tournament: Tournament
  /** All group-stage leaf groups, in display order. */
  groups: GroupGame[]
  activeGroupId: string | null
  onSelectGroup: (groupId: string) => void
  me: Player
  results: MatchPrediction[]
  pointsByGame?: Map<
    string,
    { breakdown: PointsBreakdown | null; isPerfect: boolean }
  >
  standingsByGroup: Map<string, StandingsScore>
  serverNowMs: number
  onExpire?: () => void
  onAutosave: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
  onFinalize: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
}) {
  const { t } = useI18n()
  const startX = useRef<number | null>(null)

  const rawIndex = groups.findIndex((g) => g.id === activeGroupId)
  const index = rawIndex >= 0 ? rawIndex : 0
  const active = groups[index]
  const total = groups.length

  const goto = (i: number) => {
    if (i >= 0 && i < total) onSelectGroup(groups[i].id)
  }

  const onTouchStart = (e: React.TouchEvent) => {
    startX.current = e.changedTouches[0].clientX
  }
  const onTouchEnd = (e: React.TouchEvent) => {
    if (startX.current === null) return
    const dx = e.changedTouches[0].clientX - startX.current
    startX.current = null
    if (Math.abs(dx) < 50) return
    goto(dx < 0 ? index + 1 : index - 1)
  }

  if (!active) return null

  return (
    <div className="mobile-entry" onTouchStart={onTouchStart} onTouchEnd={onTouchEnd}>
      <div className="mobile-entry-progress">
        <span className="mobile-entry-label">
          {active.name} · {index + 1} {t('mobileOf')} {total}
        </span>
        <span className="mobile-entry-nav">
          <button
            type="button"
            className="mobile-entry-prev"
            disabled={index === 0}
            onClick={() => goto(index - 1)}
          >
            {t('prevGroup')}
          </button>
          <button
            type="button"
            className="mobile-entry-next"
            disabled={index === total - 1}
            onClick={() => goto(index + 1)}
          >
            {t('nextGroup')}
          </button>
        </span>
      </div>
      <MobileGroupCard
        key={active.id}
        tournament={tournament}
        group={active}
        me={me}
        results={results}
        pointsByGame={pointsByGame}
        standings={standingsByGroup.get(active.id) ?? null}
        serverNowMs={serverNowMs}
        onExpire={onExpire}
        onAutosave={onAutosave}
        onFinalize={onFinalize}
      />
    </div>
  )
}
```

(If you dropped the `standings` prop from `MobileGroupCard` in Task B8, also drop the `standings={...}` line and the `standingsByGroup` plumbing here.)

- [ ] **Step 2: Build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/mytips/MobileGroupEntry.tsx
git commit -m "feat(web): MobileGroupEntry swipe-per-group shell"
```

## Task B10: Wire MyTipsPage to render the mobile flow

**Files:**
- Modify: `web/src/pages/MyTipsPage.tsx`

- [ ] **Step 1: Add imports**

Add, next to the existing `import { useHashScroll } from '../hooks/useHashScroll'` line (added in Task A5):

```tsx
import { useIsMobile } from '../hooks/useIsMobile'
import { MobileGroupEntry } from './mytips/MobileGroupEntry'
import type { PredictionInput, StandingsInput } from './mytips/types'
```

- [ ] **Step 2: Add the `isMobile` hook**

Just after `const location = useLocation()` (added in Task A5), add:

```tsx
  const isMobile = useIsMobile()
```

- [ ] **Step 3: Add the save handler + extracted `selectGroup`**

Find the `const refetchAll = () => { ... }` definition (line ~255). Immediately AFTER it, add:

```tsx
  // One submit path for both desktop and mobile. Autosave (lock=false) skips
  // the `me` refetch so typing is never interrupted; finalize (lock=true)
  // refetches so the locked/read-only state re-seeds from the server.
  const saveGroup = async (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
    lock: boolean,
  ) => {
    const res = await submitGroup({ groupId, predictions, standings, lock })
    if (lock) await refetchMe({ requestPolicy: 'network-only' })
    return res
  }

  const selectGroup = (groupId: string) => {
    setSelectedGroupId(groupId)
    navigate(`/mytips/${groupId}`)
  }

  const useMobileEntry =
    isMobile &&
    isGroupStage &&
    !me.isResultUser &&
    roundLeaves.length > 0 &&
    activeGroupId !== null
```

- [ ] **Step 4: Use `selectGroup` in `RoundNav`**

In the `<RoundNav .../>` JSX, replace the inline group handler:

```tsx
        onSelectGroup={(groupId) => {
          setSelectedGroupId(groupId)
          navigate(`/mytips/${groupId}`)
        }}
```

with:

```tsx
        onSelectGroup={selectGroup}
```

- [ ] **Step 5: Branch the render between mobile flow and desktop list**

Replace this block:

```tsx
      {shownGroups.length > 0 ? (
        shownGroups.map((group) => {
```

with:

```tsx
      {useMobileEntry ? (
        <MobileGroupEntry
          tournament={tournament}
          groups={roundLeaves}
          activeGroupId={activeGroupId}
          onSelectGroup={selectGroup}
          me={me}
          results={results}
          pointsByGame={pointsByGame}
          standingsByGroup={standingsByGroup}
          serverNowMs={serverNowMs}
          onExpire={refetchAll}
          onAutosave={(groupId, predictions, standings) =>
            saveGroup(groupId, predictions, standings, false)
          }
          onFinalize={(groupId, predictions, standings) =>
            saveGroup(groupId, predictions, standings, true)
          }
        />
      ) : shownGroups.length > 0 ? (
        shownGroups.map((group) => {
```

The existing `.map(...)` body and its closing are unchanged; only the opening ternary changed. Confirm the closing of the conditional still reads:

```tsx
        })
      ) : (
        <p>{t('selectGroup')}</p>
      )}
```

(If you dropped the `standings` prop earlier, remove the `standingsByGroup={standingsByGroup}` line here too.)

- [ ] **Step 6: Build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: green.

- [ ] **Step 7: Commit**

```bash
git add web/src/pages/MyTipsPage.tsx
git commit -m "feat(web): render mobile prediction flow on phone viewports"
```

## Task B11: Mobile card + stepper CSS

**Files:**
- Modify: `web/src/index.css` (append at end of file)

- [ ] **Step 1: Add the styles**

Append to `web/src/index.css`:

```css
/* ============================================================================
   MOBILE PREDICTION ENTRY — swipe one-group-per-screen card + big steppers
   ============================================================================ */
.mobile-entry {
  display: flex;
  flex-direction: column;
  gap: 12px;
  touch-action: pan-y;
}
.mobile-entry-progress {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-family: 'Press Start 2P', monospace;
  font-size: 10px;
  color: var(--amber-bright);
}
.mobile-entry-nav {
  display: flex;
  gap: 8px;
}
.mobile-group-card {
  border: 1px solid var(--bg-card-border);
  border-radius: 8px;
  padding: 14px;
  background: var(--bg-card);
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.mobile-group-head {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.mobile-group-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-family: 'Press Start 2P', monospace;
  font-size: 9px;
  color: var(--amber-dim);
}
.mobile-save-status.saved {
  color: #5fd35f;
}
.mobile-matches {
  display: flex;
  flex-direction: column;
}
.mobile-match {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 0;
  border-bottom: 1px solid var(--bg-card-border);
}
.mobile-match:last-child {
  border-bottom: none;
}
.mobile-match-scores {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 14px;
}
.mobile-match-sep {
  font-family: 'Press Start 2P', monospace;
  color: var(--amber-bright);
}
.mobile-match-result {
  font-size: 10px;
  color: var(--amber-dim);
  text-align: center;
}
.score-stepper {
  display: flex;
  align-items: center;
  gap: 10px;
}
.score-stepper button {
  width: 44px;
  height: 44px;
  font-size: 18px;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}
.score-stepper-value {
  min-width: 40px;
  text-align: center;
  font-family: 'Press Start 2P', monospace;
  font-size: 18px;
  color: var(--amber-bright);
}
```

- [ ] **Step 2: Visual check (manual, REQUIRED — green tsc/lint ≠ looks right)**

Boot the dev stack and look at the rendered page at phone width:

Run: `cd web && npm run dev` (with the API + DynamoDB running per CLAUDE.md), open `http://localhost:5173/mytips`, then in devtools toggle a 390×844 device. Verify: one group card shows at a time, steppers are large/tappable, the progress line reads "Group … · n of N", tapping +/− updates the value and the status flips to "Saving…" then "Saved.", and Prev/Next switch groups. Confirm a locked / past-deadline group renders read-only (no steppers).

- [ ] **Step 3: Commit**

```bash
git add web/src/index.css
git commit -m "feat(web): styles for mobile prediction card + steppers"
```

## Task B12: e2e — mobile stepper entry + autosave

**Files:**
- Create: `web/e2e/mobile-prediction-entry.spec.ts`

Notes:
- A phone viewport (`test.use({ viewport })`) makes the page's `matchMedia('(max-width: 640px)')` match, so `useIsMobile()` is true and the mobile flow renders — no Playwright config change needed.
- Use `demo-grace` + **Group G** (untouched by other specs → order-independent). Pre-tournament clock keeps group-stage tips editable.

- [ ] **Step 1: Write the spec**

Create `web/e2e/mobile-prediction-entry.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Mobile prediction entry: on a phone viewport the group-stage tips render as a
 * swipe one-group-per-screen card with big +/− steppers. Entering a score
 * autosaves (a `submitGroup` POST) and persists across reload; "Next group"
 * advances the progress.
 *
 * Group G / demo-grace, pre-tournament so group-stage tips are editable, and
 * untouched by other specs (order-independent, mutates only Group G).
 */
const PRE_TOURNAMENT = '2026-01-01T12:00:00Z'

test.use({ viewport: { width: 390, height: 844 } })

test('Mobile My Tips: stepper entry autosaves and persists', async ({ page }) => {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, PRE_TOURNAMENT)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  await page.goto('/mytips/G')

  // The mobile flow is rendered (not the desktop table).
  const entry = page.locator('.mobile-entry')
  await expect(entry).toBeVisible()
  await expect(page.locator('.mobile-entry-label')).toContainText('Group G')

  // Tap the first match's HOME + stepper three times → value "3".
  const firstMatch = page.locator('.mobile-match').first()
  const homeStepper = firstMatch.locator('.score-stepper').first()
  await homeStepper.locator('.score-stepper-inc').click()
  await homeStepper.locator('.score-stepper-inc').click()
  await homeStepper.locator('.score-stepper-inc').click()
  await expect(homeStepper.locator('.score-stepper-value')).toHaveText('3')

  // Autosave fires (debounced) → status shows the saved string.
  await expect(page.locator('.mobile-save-status.saved')).toBeVisible()

  // Reload: the autosaved draft re-seeds the stepper.
  await page.goto('/mytips/G')
  const reloadedHome = page
    .locator('.mobile-match')
    .first()
    .locator('.score-stepper')
    .first()
    .locator('.score-stepper-value')
  await expect(reloadedHome).toHaveText('3')

  // "Next group" advances the progress index.
  const label = page.locator('.mobile-entry-label')
  const before = (await label.textContent()) ?? ''
  const beforeIndex = Number(before.match(/·\s*(\d+)\s/)?.[1] ?? '0')
  await page.locator('.mobile-entry-next').click()
  await expect
    .poll(async () => {
      const text = (await label.textContent()) ?? ''
      return Number(text.match(/·\s*(\d+)\s/)?.[1] ?? '0')
    })
    .toBe(beforeIndex + 1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the spec**

Run: `cd web && npm run e2e -- mobile-prediction-entry`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/mobile-prediction-entry.spec.ts
git commit -m "test(web): e2e mobile stepper entry + autosave"
```

## Task B13: Cluster verification + code-review checkpoint

- [ ] **Step 1: Full web gate**

Run: `cd web && npm run build && npm run lint && npm run test`
Expected: all green. Confirm coverage thresholds still pass (new lib files `hashAnchor.ts`, `score.ts`, `debounce.ts` are fully tested; the new hooks live under `src/hooks/` outside the coverage glob).

- [ ] **Step 2: Run BOTH new e2e specs together (no regressions)**

Run: `cd web && npm run e2e -- anchor-deeplink mobile-prediction-entry`
Expected: both PASS in the same isolated stack run.

- [ ] **Step 3: Workspace gate**

Run: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: green (no Rust changed; proves the tree is clean).

- [ ] **Step 4: Confirm conventions held**

Verify by inspection: no `Date.now()` introduced (locked/read-only uses `group.deadlinePassed` + `MatchPrediction.locked`); EN and HU string blocks have identical keys; immutable state updates (spreads, no mutation); files are small and focused.

- [ ] **Step 5: Request code review**

REQUIRED SUB-SKILL: Use superpowers:requesting-code-review to verify the cluster meets requirements before merging. Address CRITICAL/HIGH findings, then finish the branch via superpowers:finishing-a-development-branch.

---

## Self-Review (performed against both PRDs)

**Spec coverage — knockout-subgroup-anchors:**
- Stable `group.id` element id → A3. ✓
- Smooth-scroll on hash → A2 (`scrollIntoView({behavior:'smooth'})`) + A5. ✓
- Covers BOTH group-stage and knockout → A3 adds the id to every `GroupTipForm`. ✓
- Round tab stays the only routed level; hash is client-side scroll → A5 reads `useLocation().hash`, no new routes. ✓
- Optional brief highlight on arrive → A2 `tip-form--anchored` + A4 keyframes. ✓
- Files match the PRD (MyTipsPage hash effect, GroupTipForm id, e2e deep-link) → A3/A5/A6. ✓

**Spec coverage — mobile-prediction-entry:**
- Swipe one-group-per-screen + progress "Group C · 3 of 12" + "Next group" → B9. ✓
- Big +/− steppers replacing the 0–9 `<select>` → B7 (mobile flow; desktop `<select>` untouched, so the existing knockout `.score-cell select` e2e still passes). ✓
- Autosave drafts + per-group "saved / N of M predicted" → B8 (debounced autosave, `predictedCount`, status). ✓
- Deadline-aware read-only when locked → B8 (`group.deadlinePassed` / `locked`, server-driven). ✓
- Mobile-SPECIFIC view (not just responsive) → B8/B9 distinct components, gated by `useIsMobile`. ✓
- Knockout / draw-order entry stays desktop-only → mobile flow gated on `isGroupStage`; autosave sends derived ordering with `drawOrder: []`. ✓

**Serialization:** Part A (A1–A7) lands entirely before Part B (B1–B13). Both touch `MyTipsPage.tsx`; building A first means B's edits compose onto A's already-merged `useLocation`/imports (B10 references the A5 import line). ✓

**Placeholder scan:** No TBD/TODO; every code step has complete code; commands have expected output. ✓

**Type consistency:** `PredictionInput`/`StandingsInput` defined once (B5) and imported by GroupTipForm, MobileGroupCard, MobileGroupEntry, MyTipsPage. `stepScore`/`clampScore`/`predictedCount`/`SCORE_MAX` names match between `score.ts`, its test, and `ScoreStepper`/`MobileGroupCard`. `hashToId` name consistent across A1/A2. `useIsMobile`/`MOBILE_QUERY` consistent in B4. ✓
