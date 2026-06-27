# Knockout-aware Tip Labels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace group-stage prediction copy ("standings", "tiebreak order", "Pts") with knockout-appropriate copy and a simplified two-team layout, but only for knockout one-match groups.

**Architecture:** Pure frontend SPA change. `GroupTipForm` detects a knockout group via `group.round !== 'GROUP_STAGE'` and passes an `isKnockout` flag into the two standings components, which switch headings, the reorder hint, and the table columns. No backend, schema, query, or DB change. The `drawOrder` submit path is untouched.

**Tech Stack:** React + TypeScript + Vite, i18n catalogue in `web/src/i18n/strings.ts`, Playwright e2e.

Source of truth: `.scratch/knockout-tip-labels/REQUIREMENTS.md`.

---

## Confirmed facts (verified against the worktree)

- The TS `Round` enum value for the group stage is **`'GROUP_STAGE'`** (SCREAMING_SNAKE), `web/src/graphql/types.ts:8`. Detection MUST use `group.round !== 'GROUP_STAGE'`. (The requirements doc's `'GroupStage'` is wrong — use `'GROUP_STAGE'`.)
- `web/src/graphql/queries.ts:8` already selects `round` — no query change.
- `TeamStats` has `goalsFor: number` (`web/src/lib/standings.ts:9`) — the per-team Goals value.
- Existing table headers (`P`/`GD`/`Pts`) are **hardcoded literals**, not i18n keys, in `StandingsTables.tsx`. The knockout `Goals` header follows that pattern (hardcoded).
- `en` is `const en = {...} as const` (`strings.ts:19`), `StringKey = keyof typeof en`, and `hu: Record<StringKey, string>` (`strings.ts:373`). Adding a key to `en` makes it **required** in `hu` — tsc enforces both.
- New copy reuses existing CSS classes (`.standings`, `.data-table`, `.hint`, `.reorder`); the ✓ is plain text in a `<td>`. No CSS change needed.

---

## Task 1: Add knockout i18n strings (en + hu)

**Files:**
- Modify: `web/src/i18n/strings.ts` (en block ~182, hu block ~528)

- [ ] **Step 1: Add three keys to the `en` block**, immediately after the `drawOrderHint` line (`web/src/i18n/strings.ts:182`):

```ts
  drawOrderHint: 'Drag tied teams to set your tiebreak order.',
  koPredictedTitle: 'Your pick',
  koActualTitle: 'Result',
  koAdvanceHint:
    "Predict the score after 90 minutes. If it's a draw, drag to pick who advances on extra time / penalties.",
```

- [ ] **Step 2: Add the same three keys to the `hu` block**, after its `drawOrderHint` line (`web/src/i18n/strings.ts:528`):

```ts
  drawOrderHint: 'Rendezd a holtversenyes csapatokat a sorrend beállításához.',
  koPredictedTitle: 'A tipped',
  koActualTitle: 'Eredmény',
  koAdvanceHint:
    'Tippeld meg a 90 perc utáni eredményt. Döntetlen esetén húzd a sorrendet, hogy eldöntsd, ki jut tovább hosszabbítás / büntetők után.',
```

- [ ] **Step 3: Type-check the strings module**

Run: `cd web && npx tsc -b --noEmit 2>&1 | head` (or rely on Task 4 build)
Expected: no errors about missing `hu` keys. (If a key is in `en` but not `hu`, tsc errors — that's the guard working.)

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): add knockout tip-label strings (en + hu)"
```

---

## Task 2: Knockout variant in StandingsTables

**Files:**
- Modify: `web/src/pages/mytips/StandingsTables.tsx` (whole file)

- [ ] **Step 1: Replace `StandingsTable` with an `isKnockout`-aware version.** Replace the current `StandingsTable` function (lines 6–51) with:

```tsx
/** A read-only standings table. Knockout one-match groups use a simplified
 *  two-team layout (✓ advances · team · goals) instead of P/GD/Pts. */
export function StandingsTable({
  title,
  rows,
  teams,
  isKnockout = false,
}: {
  title: string
  rows: TeamStats[]
  teams: Map<string, Team>
  isKnockout?: boolean
}) {
  return (
    <div className="standings">
      <h4>{title}</h4>
      <table className="data-table compact">
        <thead>
          <tr>
            <th>#</th>
            <th>Team</th>
            {isKnockout ? (
              <th>Goals</th>
            ) : (
              <>
                <th>P</th>
                <th>GD</th>
                <th>Pts</th>
              </>
            )}
          </tr>
        </thead>
        <tbody>
          {rows.map((s, i) => (
            <tr key={s.teamId}>
              <td>{isKnockout ? (i === 0 ? '✓' : '') : i + 1}</td>
              <td>
                <TeamLabel
                  slot={{
                    teamId: s.teamId,
                    description: teams.get(s.teamId)?.name ?? s.teamId,
                  }}
                  teams={teams}
                />
              </td>
              {isKnockout ? (
                <td>{s.goalsFor}</td>
              ) : (
                <>
                  <td>{s.played}</td>
                  <td>{goalDiff(s)}</td>
                  <td>{s.points}</td>
                </>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
```

- [ ] **Step 2: Replace `PredictedStandingsEditor` with an `isKnockout`-aware version.** Replace the current function (lines 53–135) with:

```tsx
/**
 * Editable predicted-standings table — lets the player manually order tied
 * teams (the `draw_order`, UC-6 / SCORING.md §4 step 5). Move up/down buttons.
 * For knockout one-match groups it reframes as "who advances on ET/penalties"
 * with a simplified two-team layout.
 */
export function PredictedStandingsEditor({
  rows,
  teams,
  readOnly,
  onReorder,
  isKnockout = false,
}: {
  rows: TeamStats[]
  teams: Map<string, Team>
  readOnly: boolean
  onReorder: (orderedTeamIds: string[]) => void
  isKnockout?: boolean
}) {
  const { t } = useI18n()

  const move = (index: number, delta: number) => {
    const next = [...rows.map((r) => r.teamId)]
    const target = index + delta
    if (target < 0 || target >= next.length) return
    ;[next[index], next[target]] = [next[target], next[index]]
    onReorder(next)
  }

  return (
    <div className="standings">
      <h4>{t(isKnockout ? 'koPredictedTitle' : 'predictedStandings')}</h4>
      {!readOnly && (
        <p className="hint">{t(isKnockout ? 'koAdvanceHint' : 'drawOrderHint')}</p>
      )}
      <table className="data-table compact">
        <thead>
          <tr>
            <th>#</th>
            <th>Team</th>
            {isKnockout ? (
              <th>Goals</th>
            ) : (
              <>
                <th>P</th>
                <th>GD</th>
                <th>Pts</th>
              </>
            )}
            {!readOnly && <th />}
          </tr>
        </thead>
        <tbody>
          {rows.map((s, i) => (
            <tr key={s.teamId}>
              <td>{isKnockout ? (i === 0 ? '✓' : '') : i + 1}</td>
              <td>
                <TeamLabel
                  slot={{
                    teamId: s.teamId,
                    description: teams.get(s.teamId)?.name ?? s.teamId,
                  }}
                  teams={teams}
                />
              </td>
              {isKnockout ? (
                <td>{s.goalsFor}</td>
              ) : (
                <>
                  <td>{s.played}</td>
                  <td>{goalDiff(s)}</td>
                  <td>{s.points}</td>
                </>
              )}
              {!readOnly && (
                <td className="reorder">
                  <button
                    type="button"
                    aria-label={t('moveUp')}
                    disabled={i === 0}
                    onClick={() => move(i, -1)}
                  >
                    ▲
                  </button>
                  <button
                    type="button"
                    aria-label={t('moveDown')}
                    disabled={i === rows.length - 1}
                    onClick={() => move(i, 1)}
                  >
                    ▼
                  </button>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
```

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/mytips/StandingsTables.tsx
git commit -m "feat(web): knockout variant for standings tables (advances/goals)"
```

---

## Task 3: Wire `isKnockout` through GroupTipForm

**Files:**
- Modify: `web/src/pages/mytips/GroupTipForm.tsx` (~line 162 and ~338–351)

- [ ] **Step 1: Compute `isKnockout`.** After the line `const isResultUser = me.isResultUser` (`GroupTipForm.tsx:162`), add:

```tsx
  const isResultUser = me.isResultUser
  const isKnockout = group.round !== 'GROUP_STAGE'
```

- [ ] **Step 2: Pass `isKnockout` into both tables.** Replace the `standings-pair` block (currently lines ~338–352):

```tsx
      <div className="standings-pair">
        {group.carriesStandings && (
          <PredictedStandingsEditor
            rows={predicted}
            teams={teams}
            readOnly={readOnly}
            onReorder={setDrawOrder}
            isKnockout={isKnockout}
          />
        )}
        <StandingsTable
          title={t(isKnockout ? 'koActualTitle' : 'actualStandings')}
          rows={actual}
          teams={teams}
          isKnockout={isKnockout}
        />
      </div>
```

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/mytips/GroupTipForm.tsx
git commit -m "feat(web): detect knockout groups and pass isKnockout to tip tables"
```

---

## Task 4: Build + lint

**Files:** none (verification)

- [ ] **Step 1: Build**

Run: `cd web && npm run build`
Expected: `tsc -b` passes (no missing-hu-key errors), `vite build` succeeds.

- [ ] **Step 2: Lint**

Run: `cd web && npm run lint`
Expected: no errors.

- [ ] **Step 3: Unit tests (regression guard)**

Run: `cd web && npm test`
Expected: existing vitest suite stays green (no standings logic changed).

---

## Task 5: Add e2e coverage for knockout labels

**Files:**
- Create: `web/e2e/mytips-knockout-labels.spec.ts`

Model: `web/e2e/mytips-knockout-unplaced.spec.ts` (seeds Group C + F as result-user so R32 becomes "ready" with M75/M76 fully placed; clock `2026-06-26T12:00:00Z` keeps R32 editable). All 16 R32 one-match groups render the standings editor (`carries_standings: true`), so the knockout copy appears whether or not a given match's teams are resolved.

- [ ] **Step 1: Write the spec**

```ts
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * My Tips: knockout one-match groups use knockout-appropriate labels —
 * "Your pick" / "Result", the extra-time/penalties hint, a ✓ advances marker,
 * and no "Predicted standings" / "tiebreak" / "Pts" wording. Group-stage groups
 * keep the league-table wording unchanged.
 */
const CLOCK = '2026-06-26T12:00:00Z'

async function openGroupStageGroup(page: Page, groupName: string): Promise<void> {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: new RegExp(`^${groupName}$`) }).click()
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

test('My Tips: knockout matches show knockout-appropriate labels', async ({ page }) => {
  await page.addInitScript((value: string) => {
    localStorage.setItem('xpool.devNow', value)
  }, CLOCK)

  const net = watchNetwork(page)
  await page.goto('/')

  // result-user seeds Group C + F so R32 (M75/M76) becomes ready.
  await devLogin(page, 'result-user')
  await openGroupStageGroup(page, 'Group C')
  await fillAllAndSave(page)
  await openGroupStageGroup(page, 'Group F')
  await fillAllAndSave(page)

  // demo-ada opens the R32 round (editable: R32 deadline not yet passed).
  await devLogin(page, 'demo-ada')
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  const r32Tab = page.locator('.round-tab', { hasText: /^Round of 32$/ })
  await expect(r32Tab).toBeVisible()
  await r32Tab.click()
  await expect(page.locator('.tip-form').first()).toBeVisible()

  // Knockout copy is present.
  await expect(
    page.locator('.tip-form .standings h4', { hasText: /^Your pick$/ }).first(),
  ).toBeVisible()
  await expect(
    page.locator('.tip-form .standings h4', { hasText: /^Result$/ }).first(),
  ).toBeVisible()
  await expect(
    page.locator('.tip-form .standings .hint', { hasText: /extra time \/ penalties/ }).first(),
  ).toBeVisible()
  // ✓ advances marker present.
  await expect(
    page.locator('.tip-form .standings td', { hasText: /^✓$/ }).first(),
  ).toBeVisible()

  // No group-stage wording leaks into the knockout round.
  await expect(
    page.locator('.tip-form .standings h4', { hasText: 'Predicted standings' }),
  ).toHaveCount(0)
  await expect(
    page.locator('.tip-form .standings th', { hasText: /^Pts$/ }),
  ).toHaveCount(0)
  await expect(
    page.locator('.tip-form .standings .hint', { hasText: /tiebreak/ }),
  ).toHaveCount(0)

  // Capture a screenshot for visual verification.
  await page.locator('.tip-form').first().screenshot({
    path: 'test-results/knockout-tip-labels.png',
  })

  // Group Stage keeps the league-table wording.
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: /^Group C$/ }).click()
  await expect(
    page.locator('.tip-form .standings h4', { hasText: 'Predicted standings' }).first(),
  ).toBeVisible()
  await expect(
    page.locator('.tip-form .standings th', { hasText: /^Pts$/ }).first(),
  ).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the new spec**

Run: `cd web && npx playwright test e2e/mytips-knockout-labels.spec.ts`
Expected: 1 passed. (Playwright global-setup boots docker/import/seed/api automatically.)

- [ ] **Step 3: Commit**

```bash
git add web/e2e/mytips-knockout-labels.spec.ts
git commit -m "test(web): e2e for knockout tip labels + group-stage regression"
```

---

## Task 6: Verify (build + lint + full e2e + visual)

**Files:** none (verification — see superpowers:verification-before-completion)

- [ ] **Step 1: Full build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: both pass.

- [ ] **Step 2: Full e2e suite**

Run: `cd web && npm run e2e`
Expected: all specs pass (existing + new).

- [ ] **Step 3: Visual verification**

Read `web/test-results/knockout-tip-labels.png` and confirm by eye: a two-team table headed "Your pick" with columns `# / Team / Goals`, a ✓ on the top team, the extra-time/penalties hint, and a "Result" table — no "standings"/"Pts"/"tiebreak" wording. (Per the verify-frontend-visually rule: green e2e ≠ looks right.)

- [ ] **Step 4: Hungarian spot-check (optional but preferred)**

Switch the SPA to `hu` and confirm `A tipped` / `Eredmény` / the hu hint render in a knockout form. Flag the hu wording to Peter for confirmation (native speaker).

---

## Self-Review

- **Spec coverage:** hint reword ✓ (Task 1/2), "Your pick"/"Result" headings ✓ (Task 1/2/3), drop Pts → ✓+Goals columns ✓ (Task 2), knockout-only via `group.round !== 'GROUP_STAGE'` ✓ (Task 3), group-stage unchanged ✓ (Task 2 conditional + Task 5 regression), en+hu ✓ (Task 1), frontend-only ✓ (no crate/schema/query files touched), e2e + visual ✓ (Task 5/6).
- **Placeholder scan:** none — every code step is complete.
- **Type consistency:** `isKnockout?: boolean` prop name identical across both components and the parent; string keys `koPredictedTitle` / `koActualTitle` / `koAdvanceHint` identical in en, hu, and call sites; enum literal `'GROUP_STAGE'` matches `types.ts`.
- **Correction vs requirements doc:** doc said `'GroupStage'`; actual enum is `'GROUP_STAGE'` — plan uses the correct value.
