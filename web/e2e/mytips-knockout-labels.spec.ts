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
