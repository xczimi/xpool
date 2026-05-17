import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * My Tips flow — as demo-ada, enter scores for a group's matches, save, then
 * reload and assert the predictions persisted. This drives `submitGroup` and
 * `me` over the live API: a schema mismatch on either breaks the round-trip.
 */

// This test edits Group B; admin-scoreboard.spec.ts locks Group A. Keeping the
// two specs on different groups makes the suite order-independent.
const TEST_GROUP = 'Group B'

/** Open `TEST_GROUP` in the My Tips sub-navigation. */
async function selectTestGroup(page: Page) {
  await expect(page.locator('.tip-form')).toBeVisible()
  await page
    .locator('.group-subnav button', { hasText: new RegExp(`^${TEST_GROUP}$`) })
    .click()
  await expect(page.locator('.tip-form h3')).toContainText(TEST_GROUP)
}

/** Fill every match-prediction `<select>` in the tip form with a score. */
async function fillScores(page: Page, home: string, away: string) {
  // The first .data-table in the form is the match-prediction table; later
  // ones are the standings tables.
  const rows = page.locator('.tip-form table.data-table').first().locator('tbody tr')
  const count = await rows.count()
  expect(count, 'the group has matches').toBeGreaterThan(0)
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption(home)
    await selects.nth(1).selectOption(away)
  }
  return count
}

test('My Tips: enter scores, save, and they persist across a reload', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await page.getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await expect(page.locator('h2')).toHaveText('My Tips')

  // Edit Group B's matches — fill every match 3:1.
  await selectTestGroup(page)
  const matchCount = await fillScores(page, '3', '1')

  // Save the draft (group still editable — kickoffs are in the future).
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // Reload — the predictions must come back from the `me` query. The form
  // resets to the first group, so re-open Group B.
  await page.reload()
  await selectTestGroup(page)
  // The first .data-table in the form is the match-prediction table; later
  // ones are the standings tables.
  const rows = page.locator('.tip-form table.data-table').first().locator('tbody tr')
  await expect(rows).toHaveCount(matchCount)
  for (let i = 0; i < matchCount; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await expect(selects.nth(0)).toHaveValue('3')
    await expect(selects.nth(1)).toHaveValue('1')
  }

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
