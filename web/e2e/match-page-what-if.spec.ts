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
