import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Unified result entry. A player locks Group A predictions before the deadline;
 * the result user then enters the official results through *My Tips* (not a
 * separate admin screen) after kickoff, and the scoreboard credits the player.
 * Exercises submitGroup (player lock) → submitGroup (result user, deadline-exempt)
 * → recompute → scoreboard — every wire hop the build check cannot reach.
 */

const GAME = 'M1' // Group A's earliest kickoff = the group's deadline.

/** Pick a game + phase in the auth-bar dev clock; it applies and reloads. */
async function setPreset(page: Page, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(GAME)
  await expect(selects.nth(1)).toBeEnabled()
  await selects.nth(1).selectOption(phase)
}

/** Open Group A in My Tips. */
async function openGroupA(page: Page) {
  await page.getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await page.locator('.round-tabs button', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: /^Group A$/ }).click()
  await expect(page.locator('.tip-form h3')).toContainText('Group A')
}

/** The match-prediction rows of the active tip form. */
function matchRows(page: Page) {
  return page.locator('.tip-form table.data-table').first().locator('tbody tr')
}

/** Fill every match in the active tip form with the given score. */
async function fillAll(page: Page, home: string, away: string) {
  const rows = matchRows(page)
  const count = await rows.count()
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption(home)
    await selects.nth(1).selectOption(away)
  }
}

test('result user enters results via My Tips and the scoreboard updates', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // ── 1. demo-ada locks Group A predictions BEFORE the deadline ──────────────
  await devLogin(page, 'demo-ada')
  await setPreset(page, 'before') // Group A open
  await openGroupA(page)
  await fillAll(page, '2', '1')
  const lockBtn = page.getByRole('button', { name: 'Lock group' })
  await expect(lockBtn).toBeEnabled()
  await lockBtn.click()
  // After a successful lock the form re-seeds from the refetched `me`: the group
  // becomes Locked and read-only (no more action buttons). See mytips-lock.spec.
  await expect(page.locator('.tip-form .state-locked')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Lock group' })).toHaveCount(0)

  // ── 2. the result user enters official results via My Tips AFTER kickoff ────
  await devLogin(page, 'result-user')
  await setPreset(page, 'after') // Group A deadline passed
  await openGroupA(page)
  // Unlike a regular player (locked out post-deadline), the result user can edit.
  await expect(page.getByRole('button', { name: 'Save draft' })).toBeVisible()
  await fillAll(page, '2', '1') // exact match of ada's prediction → max points
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.tip-form .flash-bar')).toContainText('Saved')

  // ── 3. the scoreboard credits demo-ada non-zero points ─────────────────────
  await page.getByRole('link', { name: 'Scoreboard' }).click()
  await expect(page).toHaveURL(/\/scoreboard$/)
  const adaRow = page
    .locator('table.data-table tbody tr')
    .filter({ hasText: 'ada' })
    .first()
  await expect(adaRow).toBeVisible()
  const totalText = await adaRow.locator('td').last().textContent()
  expect(Number((totalText ?? '0').trim()), 'demo-ada credited').toBeGreaterThan(0)

  // ── 4. there is no /admin Results screen anymore ───────────────────────────
  await page.getByRole('link', { name: 'Admin' }).click()
  await expect(page).toHaveURL(/\/admin/)
  await expect(page.locator('.group-subnav')).not.toContainText('Results entry')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
