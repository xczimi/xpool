import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Admin result → scoreboard flow. A player locks a group of predictions, an
 * admin enters an official result for one of those matches, and the scoreboard
 * then credits the player non-zero points. This is the deepest end-to-end
 * path: `submitGroup` (lock) → `enterResult` → scoreboard recompute → the
 * `scoreboard` query — every wire hop the build check cannot exercise.
 */

/** Fill every match in the active tip form with the given score. */
async function fillAll(page: Page, home: string, away: string) {
  // The first .data-table in the form is the match table; later ones are the
  // standings tables (for groups that carry standings).
  const rows = matchRows(page)
  const count = await rows.count()
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption(home)
    await selects.nth(1).selectOption(away)
  }
  return count
}

/** The match-prediction rows of the active tip form. */
function matchRows(page: Page) {
  return page
    .locator('.tip-form table.data-table')
    .first()
    .locator('tbody tr')
}

test('admin result credits a predicting player on the scoreboard', async ({
  page,
}) => {
  const net = watchNetwork(page)

  // ── 1. demo-ada locks predictions for the first group ──────────────────────
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.getByRole('link', { name: 'My Tips' }).click()
  await expect(page.locator('.tip-form')).toBeVisible()

  // Predict every match 2:1 — a unique, recognisable scoreline.
  await fillAll(page, '2', '1')

  // Lock the group (all matches scored, so the Lock button is enabled).
  const lockBtn = page.getByRole('button', { name: 'Lock group' })
  await expect(lockBtn).toBeEnabled()
  await lockBtn.click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // The match label of the first locked prediction — to find it as admin.
  const firstMatchLabel = (
    await matchRows(page).first().locator('td').first().textContent()
  )?.trim()
  expect(firstMatchLabel).toBeTruthy()

  // ── 2. admin enters the official result for that match ─────────────────────
  // devLogin logs out first, then logs in as the result user.
  await devLogin(page, 'result-user')
  // The Admin nav link only appears once `me.isResultUser` has resolved.
  await expect(page.getByRole('link', { name: 'Admin' })).toBeVisible()

  await page.getByRole('link', { name: 'Admin' }).click()
  await expect(page).toHaveURL(/\/admin/)
  await expect(page.locator('h3')).toContainText('Results entry')

  // Find the result row for demo-ada's first match and enter 2:1 — an exact
  // match of her prediction, which scores the maximum 4 points.
  const resultRow = page
    .locator('table.data-table tbody tr')
    .filter({ hasText: firstMatchLabel! })
    .first()
  const inputs = resultRow.locator('.score-cell input')
  await inputs.nth(0).fill('2')
  await inputs.nth(1).fill('1')
  await resultRow.getByRole('button', { name: 'Enter result' }).click()
  await expect(resultRow.locator('.state-locked')).toBeVisible()

  // ── 3. the scoreboard credits demo-ada non-zero points ─────────────────────
  await page.getByRole('link', { name: 'Scoreboard' }).click()
  await expect(page).toHaveURL(/\/scoreboard$/)
  await expect(page.locator('h2')).toHaveText('Scoreboard')

  const adaRow = page
    .locator('table.data-table tbody tr')
    .filter({ hasText: 'ada' })
    .first()
  await expect(adaRow).toBeVisible()
  // The Total cell is the last <td>; it must be > 0.
  const totalText = await adaRow.locator('td').last().textContent()
  const total = Number((totalText ?? '0').trim())
  expect(total, 'demo-ada has non-zero points after the result').toBeGreaterThan(
    0,
  )

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
