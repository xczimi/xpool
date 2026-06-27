import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * My Tips: knockout score inputs are disabled until both teams are placed.
 *
 * When an R32 (or any knockout) game has one or both team slots unresolved
 * (team_id is null), the prediction form must show a "Teams not yet determined"
 * notice instead of score `<select>` elements — the API rejects such submissions
 * anyway (Phase C-api), and the UI should not invite them.
 *
 * Setup: the e2e seed has no official results, so the R32 bracket is fully
 * unresolved and the R32 round tab does not appear at all. To exercise the
 * partially-resolved state (R32 tab visible, some slots still null), we seed
 * results for Group C and Group F as the result-user. That places
 * the Group C winner/runner-up and Group F winner/runner-up into M75 and M76
 * respectively, making R32 "ready" (readyRounds detects ≥ 1 game with both
 * teams). All other R32 games (M73 = 2A vs 2B, etc.) remain unresolved since
 * Groups A and B have no official results.
 */

// Clock past the last Group C / Group F games (M52, M57, M58 — June 24-25) so
// the full group-stage slice is available when recompute runs on result-user
// save. R32 games start June 28, so R32 deadlinePassed is still false.
const CLOCK = '2026-06-26T12:00:00Z'

/** Navigate to a group-stage leaf group in My Tips. */
async function openGroupStageGroup(page: Page, groupName: string): Promise<void> {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)
  // With the June-26 clock all group-stage deadlines have passed but R32 is
  // still open — My Tips may default to R32. Explicitly select Group Stage.
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: new RegExp(`^${groupName}$`) }).click()
  await expect(page.locator('.tip-form h3').first()).toContainText(groupName)
}

/** Fill every visible score-input select in the tip form with 2-1 and save. */
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

test('My Tips: knockout match with unplaced teams shows notice instead of score inputs', async ({
  page,
}) => {
  // Pin the clock after all Group C and Group F games so the recompute
  // triggered by result-user saves includes all their predictions.
  await page.addInitScript((value: string) => {
    localStorage.setItem('xpool.devNow', value)
  }, CLOCK)

  const net = watchNetwork(page)
  await page.goto('/')

  // ── 1. result-user seeds Group C + Group F official results ─────────────────
  // After each save the API recomputes the bracket (submitGroup → recompute).
  // Group C done: M76.home (1C) and M75.away (2C) get real team_ids.
  // Group F done: M75.home (1F) and M76.away (2F) get real team_ids.
  // After both saves M75 and M76 are fully placed → R32 becomes "ready".
  await devLogin(page, 'result-user')
  await openGroupStageGroup(page, 'Group C')
  await fillAllAndSave(page)
  await openGroupStageGroup(page, 'Group F')
  await fillAllAndSave(page)

  // ── 2. demo-ada navigates to the R32 round in My Tips ────────────────────────
  await devLogin(page, 'demo-ada')
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)

  // R32 tab must now be visible — M75 and M76 have both team slots filled.
  const r32Tab = page.locator('.round-tab', { hasText: /^Round of 32$/ })
  await expect(r32Tab).toBeVisible()
  await r32Tab.click()

  // R32 stacks all 16 one-match tip forms. Among them, M73 (2A vs 2B) and the
  // other matches whose slots come from un-seeded groups still have null
  // team_ids. Find the first score cell whose team slots are unresolved.
  const undecidedCell = page
    .locator('.tip-form table.data-table tbody .score-cell')
    .filter({ has: page.locator('span.hint') })
    .first()
  await expect(undecidedCell).toBeVisible()
  await expect(undecidedCell.locator('span.hint')).toHaveText('Teams not yet determined')
  // No score-input selects may be present for an undecided matchup.
  await expect(undecidedCell.locator('select')).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
