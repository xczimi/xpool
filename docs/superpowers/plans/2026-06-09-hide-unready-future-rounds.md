# Hide Future Rounds Not Ready for Predictions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hide a future tournament round from the prediction nav (My Tips, All Tips) and the scoreboard columns until at least one of its games has both teams determined.

**Architecture:** Pure, client-side derivation. The API already materialises resolved `teamId`s onto knockout games (via `recompute.rs` → `fwc26::resolve_bracket` against the official results), so the SPA can read readiness straight off the games already on the wire. One shared helper in `web/src/lib/rounds.ts` (`readyRounds` + `visibleRoundNodes`) is consumed by `RoundNav` (drives the tabs for both My Tips and All Tips) and `ScoreboardPage` (drives the per-round columns). No GraphQL schema change, no backend change.

**Tech Stack:** React + TypeScript + Vite, urql GraphQL client, Vitest (unit), Playwright (e2e).

**Design doc:** `docs/superpowers/specs/2026-06-09-hide-unready-future-rounds-design.md`

---

## Key facts the implementer must know

- **The TS type for a game is `SingleGame`** (not `Game`) — see `web/src/graphql/types.ts:32`. `Tournament.games` is `SingleGame[]`.
- **A slot is "determined" when `teamId` is truthy.** `TeamSlot.teamId: string | null` — `null` for an unresolved knockout placeholder; the placeholder text lives in `description`.
- **A game's round = its group's round.** `game.groupId` points at a leaf group; that group's `.round` is the round (e.g. a one-match knockout group has `round: 'R32'`). Group-stage games belong to "Group A".."Group L" leaf groups, all `round: 'GROUP_STAGE'`.
- **`ROUND_ORDER` also feeds the Rules page** (`web/src/pages/RulesPage.tsx:49`) — that page lists the static scoring table and MUST keep showing every round. Do **not** filter it. Only `ScoreboardPage` filters.
- **No server guard** is added — placeholder games inside an already-open round stay technically submittable. This is the accepted soft-funnel trade-off (see the design doc's "Accepted gap"). Out of scope here.
- **Scope is My Tips + All Tips + Scoreboard.** Both tip pages share `RoundNav`, so filtering inside `RoundNav` covers both with no flag.

## Files touched

- **Modify** `web/src/lib/rounds.ts` — add `readyRounds` + `visibleRoundNodes`.
- **Modify** `web/src/lib/rounds.test.ts` — unit tests for the two new functions.
- **Modify** `web/src/components/RoundNav.tsx` — take a `games` prop, filter tabs via `visibleRoundNodes`.
- **Modify** `web/src/pages/MyTipsPage.tsx` — derive rounds from `visibleRoundNodes`, pass `games` to `RoundNav`.
- **Modify** `web/src/pages/AllTipsPage.tsx` — same two changes as My Tips.
- **Modify** `web/src/pages/ScoreboardPage.tsx` — filter the per-round columns by `readyRounds`.
- **Rewrite** `web/e2e/round-nav.spec.ts` — assert knockout tabs are hidden at a group-stage clock.
- **Modify** `web/e2e/scenario-scoreboard.spec.ts` — assert knockout columns hidden early / shown late.

---

## Task 0: Branch

Web/crate source changes must go on a branch (CLAUDE.md "Branch discipline"). All work happens on a worktree/branch; merge to `master` locally at the end.

- [ ] **Step 1: Create the branch (or worktree)**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git checkout -b hide-unready-rounds
```

(If executing via the worktree skill, the isolated worktree already satisfies this — skip.)

---

## Task 1: `readyRounds` + `visibleRoundNodes` in `rounds.ts`

**Files:**
- Modify: `web/src/lib/rounds.ts`
- Test: `web/src/lib/rounds.test.ts`

- [ ] **Step 1: Write the failing tests**

Add a `SingleGame` import and a new `describe` block to `web/src/lib/rounds.test.ts`. Update the top import to pull in the two new functions, and import the `SingleGame` type.

Change the existing import block (lines 1–14) so it also imports the new functions and the `SingleGame` type:

```ts
import { describe, expect, it } from 'vitest'
import type { GroupGame, Round, SingleGame } from '../graphql/types'
import {
  ROUND_ORDER,
  STAGE_MULTIPLIERS,
  chronologicalLeafGroups,
  currentRoundNode,
  leafGroupsOfRound,
  readyRounds,
  roundLabel,
  roundLabelKey,
  roundNodes,
  visibleRoundNodes,
} from './rounds'
import { catalogues } from '../i18n/strings'
import type { StringKey } from '../i18n/strings'
```

Append this new `describe` block to the end of the file. It reuses the same `ROOT → GROUPSTAGE{A,B} / KNOCKOUT{R32{M1,M2}, FINAL{M3}}` tree shape as the existing `roundNodes` block, plus games wired to those leaf groups:

```ts
describe('readyRounds / visibleRoundNodes', () => {
  const node = (
    id: string,
    round: Round,
    opts: { childGroupIds?: string[]; childGameIds?: string[] },
  ): GroupGame => ({
    id,
    name: id,
    parent: null,
    round,
    lockMode: 'LOCK_TOGETHER',
    carriesStandings: false,
    childGroupIds: opts.childGroupIds ?? [],
    childGameIds: opts.childGameIds ?? [],
    deadline: null,
    deadlinePassed: false,
  })

  // ROOT -> GROUPSTAGE -> {A,B}; ROOT -> KNOCKOUT -> {R32 -> {M1,M2}, FINAL -> {M3}}
  const tree: GroupGame[] = [
    node('ROOT', 'GROUP_STAGE', { childGroupIds: ['GROUPSTAGE', 'KNOCKOUT'] }),
    node('GROUPSTAGE', 'GROUP_STAGE', { childGroupIds: ['A', 'B'] }),
    node('KNOCKOUT', 'R32', { childGroupIds: ['R32', 'FINAL'] }),
    node('R32', 'R32', { childGroupIds: ['M1', 'M2'] }),
    node('FINAL', 'FINAL', { childGroupIds: ['M3'] }),
    node('A', 'GROUP_STAGE', { childGameIds: ['g1'] }),
    node('B', 'GROUP_STAGE', { childGameIds: ['g2'] }),
    node('M1', 'R32', { childGameIds: ['g3'] }),
    node('M2', 'R32', { childGameIds: ['g4'] }),
    node('M3', 'FINAL', { childGameIds: ['g5'] }),
  ]

  const game = (
    id: string,
    groupId: string,
    homeTeam: string | null,
    awayTeam: string | null,
  ): SingleGame => ({
    id,
    kickoff: '2026-06-11T12:00:00Z',
    venue: null,
    groupId,
    home: { teamId: homeTeam, description: homeTeam ?? 'TBD' },
    away: { teamId: awayTeam, description: awayTeam ?? 'TBD' },
    resultPending: false,
    withinTodayWindow: false,
    isToday: false,
  })

  // Group games always carry real teams; knockout slots start unresolved.
  const groupOnly: SingleGame[] = [
    game('g1', 'A', 'ARG', 'BRA'),
    game('g2', 'B', 'FRA', 'GER'),
    game('g3', 'M1', null, null),
    game('g4', 'M2', null, null),
    game('g5', 'M3', null, null),
  ]

  it('readyRounds is just GROUP_STAGE when every knockout slot is a placeholder', () => {
    expect(readyRounds(tree, groupOnly)).toEqual(new Set(['GROUP_STAGE']))
  })

  it('readyRounds includes a round once one of its games has BOTH teams', () => {
    const withR32 = groupOnly.map((g) =>
      g.id === 'g3' ? game('g3', 'M1', 'ARG', 'FRA') : g,
    )
    const ready = readyRounds(tree, withR32)
    expect(ready.has('R32')).toBe(true)
    // FINAL still has only a placeholder game -> excluded.
    expect(ready.has('FINAL')).toBe(false)
  })

  it('readyRounds excludes a round when only ONE slot of its game is known', () => {
    const halfR32 = groupOnly.map((g) =>
      g.id === 'g3' ? game('g3', 'M1', 'ARG', null) : g,
    )
    expect(readyRounds(tree, halfR32).has('R32')).toBe(false)
  })

  it('visibleRoundNodes drops not-yet-ready round nodes', () => {
    expect(visibleRoundNodes(tree, groupOnly).map((n) => n.id)).toEqual([
      'GROUPSTAGE',
    ])
  })

  it('visibleRoundNodes reveals a round once it is ready, keeping ROUND_ORDER', () => {
    const withR32 = groupOnly.map((g) =>
      g.id === 'g3' ? game('g3', 'M1', 'ARG', 'FRA') : g,
    )
    expect(visibleRoundNodes(tree, withR32).map((n) => n.id)).toEqual([
      'GROUPSTAGE',
      'R32',
    ])
  })
})
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd web && npm test -- rounds`
Expected: FAIL — `readyRounds`/`visibleRoundNodes` are not exported (`No "readyRounds" export is defined`).

- [ ] **Step 3: Implement the two functions**

In `web/src/lib/rounds.ts`, change the first import line to add `SingleGame`:

```ts
import type { GroupGame, Round, SingleGame } from '../graphql/types'
```

Then add the two functions immediately after `roundNodes` (after the closing `}` on line 70, before `leafGroupsOfRound`):

```ts
/**
 * The rounds whose participants are known well enough to predict: a round is
 * "ready" once at least one of its games has BOTH teams determined (a real
 * `teamId`, not a knockout placeholder). Group Stage games carry real teams
 * from import, so it is always ready. Readiness reflects the official results
 * the API has already resolved onto the games — never a player's own picks.
 */
export function readyRounds(
  groups: GroupGame[],
  games: SingleGame[],
): Set<Round> {
  const roundByGroupId = new Map(groups.map((g) => [g.id, g.round]))
  const ready = new Set<Round>()
  for (const game of games) {
    if (game.home.teamId && game.away.teamId) {
      const round = roundByGroupId.get(game.groupId)
      if (round) ready.add(round)
    }
  }
  return ready
}

/**
 * `roundNodes` filtered to the rounds ready for predictions (see `readyRounds`).
 * Drives the round-tab nav and the default round selection so neither ever
 * surfaces a round whose teams are still unknown. The ready-set only grows as
 * the tournament progresses, so a visible round never disappears underneath the
 * user.
 */
export function visibleRoundNodes(
  groups: GroupGame[],
  games: SingleGame[],
): GroupGame[] {
  const ready = readyRounds(groups, games)
  return roundNodes(groups).filter((node) => ready.has(node.round))
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd web && npm test -- rounds`
Expected: PASS — all `readyRounds / visibleRoundNodes` tests green, existing `rounds` tests still green.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/rounds.ts web/src/lib/rounds.test.ts
git commit -m "feat(web): readyRounds + visibleRoundNodes — round readiness from resolved teams"
```

---

## Task 2: `RoundNav` filters tabs by readiness

**Files:**
- Modify: `web/src/components/RoundNav.tsx`

`RoundNav` is shared by My Tips and All Tips; filtering here covers both.

- [ ] **Step 1: Add the `games` prop and use `visibleRoundNodes`**

Replace the whole file `web/src/components/RoundNav.tsx` with:

```tsx
import type { GroupGame, Round, SingleGame } from '../graphql/types'
import { useI18n } from '../i18n/useI18n'
import { leafGroupsOfRound, roundLabel, visibleRoundNodes } from '../lib/rounds'
import { GroupSubNav } from './GroupSubNav'

/**
 * Two-level tournament navigation for My Tips / All Tips. Row 1 is a tab per
 * round node (Group Stage / R32 / … / Final). Row 2 is the group pills — only
 * the Group Stage round has one; knockout rounds show all their matches in the
 * page body instead.
 *
 * Only rounds that are ready for predictions are shown — a future round whose
 * teams are still unknown (no game with both teams determined) is hidden until
 * the official results resolve it. See `visibleRoundNodes`.
 */
export function RoundNav({
  groups,
  games,
  selectedRound,
  onSelectRound,
  selectedGroupId,
  onSelectGroup,
}: {
  groups: GroupGame[]
  games: SingleGame[]
  selectedRound: Round | null
  onSelectRound: (round: Round) => void
  selectedGroupId: string | null
  onSelectGroup: (groupId: string) => void
}) {
  const { t } = useI18n()
  const rounds = visibleRoundNodes(groups, games)
  const activeNode = rounds.find((r) => r.round === selectedRound) ?? null

  return (
    <div className="round-nav">
      <div className="round-tabs">
        {rounds.map((node) => (
          <button
            key={node.id}
            type="button"
            className={node.round === selectedRound ? 'round-tab active' : 'round-tab'}
            onClick={() => onSelectRound(node.round)}
          >
            {roundLabel(node.round, t)}
          </button>
        ))}
      </div>
      {activeNode?.round === 'GROUP_STAGE' && (
        <GroupSubNav
          groups={leafGroupsOfRound(activeNode, groups)}
          selectedId={selectedGroupId}
          onSelect={onSelectGroup}
        />
      )}
    </div>
  )
}
```

- [ ] **Step 2: Verify it does not yet type-check (callers miss `games`)**

Run: `cd web && npx tsc -b --noEmit`
Expected: FAIL — `MyTipsPage.tsx` and `AllTipsPage.tsx` are missing the now-required `games` prop on `<RoundNav>`. (Tasks 3 and 4 fix these; the error confirms the prop is wired through.)

- [ ] **Step 3: Commit**

```bash
git add web/src/components/RoundNav.tsx
git commit -m "feat(web): RoundNav hides rounds not ready for predictions"
```

---

## Task 3: `MyTipsPage` uses visible rounds + passes games

**Files:**
- Modify: `web/src/pages/MyTipsPage.tsx`

- [ ] **Step 1: Swap `roundNodes` for `visibleRoundNodes`**

In `web/src/pages/MyTipsPage.tsx`, change the import on line 25 from:

```ts
import { currentRoundNode, leafGroupsOfRound, roundNodes } from '../lib/rounds'
```

to:

```ts
import { currentRoundNode, leafGroupsOfRound, visibleRoundNodes } from '../lib/rounds'
```

- [ ] **Step 2: Derive `rounds` from visible rounds**

Replace the `rounds` memo (lines 57–60):

```ts
  const rounds = useMemo(
    () => roundNodes(tournament?.groups ?? []),
    [tournament?.groups],
  )
```

with:

```ts
  const rounds = useMemo(
    () => visibleRoundNodes(tournament?.groups ?? [], tournament?.games ?? []),
    [tournament?.groups, tournament?.games],
  )
```

- [ ] **Step 3: Pass `games` to `RoundNav`**

In the `<RoundNav ... />` element (starts line 143), add the `games` prop right after `groups`:

```tsx
      <RoundNav
        groups={tournament.groups}
        games={tournament.games}
        selectedRound={activeRound}
        onSelectRound={(round) => {
          setSelectedRound(round)
          setSelectedGroupId(null)
        }}
        selectedGroupId={activeGroupId}
        onSelectGroup={setSelectedGroupId}
      />
```

- [ ] **Step 4: Verify types**

Run: `cd web && npx tsc -b --noEmit`
Expected: still FAIL, but only on `AllTipsPage.tsx` now (My Tips resolved). If `MyTipsPage.tsx` reports any error, fix before continuing.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/MyTipsPage.tsx
git commit -m "feat(web): My Tips round tabs hide unready rounds"
```

---

## Task 4: `AllTipsPage` uses visible rounds + passes games

**Files:**
- Modify: `web/src/pages/AllTipsPage.tsx`

Same two changes as My Tips (the user confirmed All Tips hides unready rounds too).

- [ ] **Step 1: Swap `roundNodes` for `visibleRoundNodes`**

In `web/src/pages/AllTipsPage.tsx`, change the import on line 13 from:

```ts
import { currentRoundNode, leafGroupsOfRound, roundNodes } from '../lib/rounds'
```

to:

```ts
import { currentRoundNode, leafGroupsOfRound, visibleRoundNodes } from '../lib/rounds'
```

- [ ] **Step 2: Derive `rounds` from visible rounds**

Replace the `rounds` memo (lines 35–38):

```ts
  const rounds = useMemo(
    () => roundNodes(tournament?.groups ?? []),
    [tournament?.groups],
  )
```

with:

```ts
  const rounds = useMemo(
    () => visibleRoundNodes(tournament?.groups ?? [], tournament?.games ?? []),
    [tournament?.groups, tournament?.games],
  )
```

- [ ] **Step 3: Pass `games` to `RoundNav`**

In the `<RoundNav ... />` element (starts line 125), add the `games` prop right after `groups`:

```tsx
      <RoundNav
        groups={tournament.groups}
        games={tournament.games}
        selectedRound={activeRound}
        onSelectRound={(round) => {
          setSelectedRound(round)
          setSelectedGroupId(null)
        }}
        selectedGroupId={activeGroupId}
        onSelectGroup={setSelectedGroupId}
      />
```

- [ ] **Step 4: Verify types**

Run: `cd web && npx tsc -b --noEmit`
Expected: PASS — both tip pages now pass `games`; no type errors anywhere.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/AllTipsPage.tsx
git commit -m "feat(web): All Tips round tabs hide unready rounds"
```

---

## Task 5: `ScoreboardPage` hides unready round columns

**Files:**
- Modify: `web/src/pages/ScoreboardPage.tsx`

The scoreboard already runs `TOURNAMENT_QUERY` as `probe` (for the poll interval), so `groups` + `games` are already in hand — no new query.

- [ ] **Step 1: Import `readyRounds`**

Change the import on line 18 from:

```ts
import { roundLabel, ROUND_ORDER, STAGE_MULTIPLIERS } from '../lib/rounds'
```

to:

```ts
import { readyRounds, roundLabel, ROUND_ORDER, STAGE_MULTIPLIERS } from '../lib/rounds'
```

- [ ] **Step 2: Compute the visible rounds**

Right after the `scoreboard` line (line 51, `const scoreboard = result.data?.scoreboard ?? null`), add:

```ts
  // Only show round columns whose teams are known — a future round with no
  // game determined yet (knockouts before the bracket resolves) is hidden,
  // mirroring the My Tips / All Tips round tabs. GROUP_STAGE is always ready.
  const ready = readyRounds(
    probe.data?.tournament?.groups ?? [],
    probe.data?.tournament?.games ?? [],
  )
  const visibleRounds = ROUND_ORDER.filter((r) => ready.has(r))
```

- [ ] **Step 3: Render only the visible round columns**

Replace the header-cell map (lines 90–98):

```tsx
            {ROUND_ORDER.map((r) => (
              <th key={r}>
                {roundLabel(r, t)}
                <br />
                <small>
                  {t('multiplier')} ×{STAGE_MULTIPLIERS[r]}
                </small>
              </th>
            ))}
```

with:

```tsx
            {visibleRounds.map((r) => (
              <th key={r}>
                {roundLabel(r, t)}
                <br />
                <small>
                  {t('multiplier')} ×{STAGE_MULTIPLIERS[r]}
                </small>
              </th>
            ))}
```

Then replace the body-cell map (lines 111–113):

```tsx
                {ROUND_ORDER.map((r) => (
                  <td key={r}>{byRound.get(r) ?? 0}</td>
                ))}
```

with:

```tsx
                {visibleRounds.map((r) => (
                  <td key={r}>{byRound.get(r) ?? 0}</td>
                ))}
```

- [ ] **Step 4: Verify types and unit tests**

Run: `cd web && npx tsc -b --noEmit && npm test`
Expected: PASS — type-clean, all unit tests green.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/ScoreboardPage.tsx
git commit -m "feat(web): scoreboard hides round columns not ready for predictions"
```

---

## Task 6: E2E — My Tips hides knockout tabs at a group-stage clock

**Files:**
- Rewrite: `web/e2e/round-nav.spec.ts`

The old test asserted all seven round tabs render with placeholder knockouts — that behaviour is exactly what we removed, so it must be rewritten. The e2e suite is serial (`workers: 1`) and shares one DynamoDB table, and the bracket only resolves when a result is materialised. **Drive the clock through the DevClock picker** (game + phase) — picking fires `devRematerialize`, which re-resolves the bracket *as-of that instant*, giving deterministic team-resolution state regardless of what an earlier spec left in the table. Picking a group-stage instant ("before M1") means no result has landed yet, so every knockout slot is an unresolved placeholder and only Group Stage is ready.

- [ ] **Step 1: Write the new spec**

Replace the whole file `web/e2e/round-nav.spec.ts` with:

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Round-aware navigation on My Tips. Only rounds ready for predictions show a
 * tab: a round is ready once one of its games has both teams determined. Before
 * the bracket resolves, every knockout round is hidden and only Group Stage is
 * navigable.
 */

// M1 is Group A's earliest kickoff = the group-stage deadline.
const GAME = 'M1'

/**
 * Pin the dev clock to GAME + phase via the auth-bar picker. The pick fires
 * `devRematerialize` (re-resolving the bracket as-of the instant) then reloads —
 * tie the action to the reload so we assert on the fresh DOM.
 */
async function setClock(page: Page, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(GAME)
  await expect(selects.nth(1)).toBeEnabled()
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  await expect(page.locator('.dev-clock-now')).toBeVisible()
}

test('My Tips hides knockout rounds until their teams are known', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  // As-of just before the first kickoff: nothing is played, so the bracket is
  // entirely unresolved — only Group Stage is ready.
  await setClock(page, 'before')

  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)

  // Only the Group Stage tab renders; the six knockout rounds are hidden.
  await expect(page.locator('.round-tab')).toHaveCount(1)
  await expect(
    page.locator('.round-tab.active', { hasText: /^Group Stage$/ }),
  ).toBeVisible()
  await expect(
    page.locator('.round-tab', { hasText: /^Round of 32$/ }),
  ).toHaveCount(0)
  await expect(
    page.locator('.round-tab', { hasText: /^Final$/ }),
  ).toHaveCount(0)

  // Group Stage still drills into a single group via the group pills.
  await expect(page.locator('.group-subnav')).toBeVisible()
  await expect(page.locator('.tip-form')).toHaveCount(1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the spec**

Run: `cd web && npm run e2e -- round-nav`
Expected: PASS — one Group Stage tab, no R32/Final tabs, group pills + one tip form. (The e2e harness boots the full stack itself; first run takes a while.)

- [ ] **Step 3: Commit**

```bash
git add web/e2e/round-nav.spec.ts
git commit -m "test(web): My Tips e2e asserts knockout rounds hidden until resolved"
```

---

## Task 7: E2E — scoreboard columns hidden early, shown late

**Files:**
- Modify: `web/e2e/scenario-scoreboard.spec.ts`

This spec already seeds the `balanced` scenario (full results) and advances the dev clock from the first match to the final, re-materialising as-of each instant. That gives us both directions for free: as-of just after M1 the bracket is unresolved (no knockout columns); as-of just after the final it is fully resolved (knockout columns present).

- [ ] **Step 1: Add column assertions at the early and late clocks**

In `web/e2e/scenario-scoreboard.spec.ts`, the test body sets the early clock then reads `early`, sets the late clock then reads `late`. Add header-column assertions right after each total read.

Replace this block (lines 75–83):

```ts
  // Early: clock just after the first match → few matches scored → small board.
  await setClock(page, 'M1', 'after')
  const early = await scoreboardTotal(page)

  // Late: clock just after the Final → all matches scored → larger board.
  await setClock(page, 'M104', 'after')
  const late = await scoreboardTotal(page)

  expect(late).toBeGreaterThan(early)
```

with:

```ts
  // Early: clock just after the first match → few matches scored → small board.
  await setClock(page, 'M1', 'after')
  const early = await scoreboardTotal(page)

  // As-of M1 the bracket is unresolved — Group Stage column shows, knockout
  // columns are hidden (no game with both teams known yet).
  await expect(page.locator('.data-table thead')).toContainText('Group Stage')
  await expect(page.locator('.data-table thead')).not.toContainText(
    'Round of 32',
  )

  // Late: clock just after the Final → all matches scored → larger board.
  await setClock(page, 'M104', 'after')
  const late = await scoreboardTotal(page)

  // As-of the final every round is resolved — the knockout columns appear.
  await expect(page.locator('.data-table thead')).toContainText('Round of 32')
  await expect(page.locator('.data-table thead')).toContainText('Final')

  expect(late).toBeGreaterThan(early)
```

- [ ] **Step 2: Run the spec**

Run: `cd web && npm run e2e -- scenario-scoreboard`
Expected: PASS — early board has no "Round of 32" column; late board has "Round of 32" and "Final" columns; late total > early total.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/scenario-scoreboard.spec.ts
git commit -m "test(web): scoreboard e2e asserts knockout columns appear only once resolved"
```

---

## Task 8: Full verification + integrate

**Files:** none (verification + merge)

- [ ] **Step 1: Lint, type-check, unit tests**

Run: `cd web && npm run lint && npm run build && npm test`
Expected: all PASS (lint clean, `tsc -b && vite build` succeeds, every unit test green).

- [ ] **Step 2: Full e2e suite**

Run: `cd web && npm run e2e`
Expected: all PASS — confirms the new behaviour holds with the rest of the suite running serially against the shared stack (no cross-spec ordering regressions).

- [ ] **Step 3: Merge to master locally (solo workflow)**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git checkout master
git merge --no-ff hide-unready-rounds -m "feat(web): hide future rounds not ready for predictions"
```

(If a worktree was used, follow the finishing-a-development-branch flow instead.)

---

## Self-review notes

- **Spec coverage:** readiness rule → Task 1; My Tips → Task 3; All Tips (confirmed in-scope) → Task 4; Scoreboard columns → Task 5; "official results only" basis → relies on materialised `teamId` (Task 1 helper reads only `teamId`, never player picks); "round-level only" → `visibleRoundNodes` filters whole rounds, never individual games; "no server guard" → no backend task, documented. Rules page intentionally untouched.
- **Type consistency:** `readyRounds(groups, games)` / `visibleRoundNodes(groups, games)` signatures are identical everywhere they appear (test, rounds.ts, RoundNav, both pages, scoreboard). Game type is `SingleGame` throughout.
- **No placeholders:** every step has concrete code and a runnable command with expected output.
