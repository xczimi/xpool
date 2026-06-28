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
