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
