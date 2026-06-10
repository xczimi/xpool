import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Regression: locking a group must reflect in the UI *without a reload*.
 *
 * The bug: GroupTipForm seeds its match state from `me` via useState (init runs
 * once), and the group was keyed only by id — so after a successful Lock group,
 * the refetched-locked `me` never resynced into the form. The group stayed
 * "Draft" with the Lock button still enabled, inviting a second submit that the
 * server (correctly) rejects with "prediction for `M…` is already locked".
 *
 * This drives the real wire: submitGroup(lock) → refetch me → the form must now
 * render the group as Locked and read-only. Uses demo-dennis + Group C so it
 * does not collide with result-entry (demo-ada locks Group A) or mytips
 * (demo-ada edits Group B).
 */

const TEST_GROUP = 'Group C'
// The e2e clock defaults mid-tournament (groups locked). Pin before kickoff so
// Group C is editable and we exercise a real, fresh lock.
const PRE_TOURNAMENT = '2026-01-01T12:00:00Z'

async function selectTestGroup(page: Page) {
  await expect(page.locator('.tip-form')).toBeVisible()
  await page
    .locator('.group-subnav button', { hasText: new RegExp(`^${TEST_GROUP}$`) })
    .click()
  await expect(page.locator('.tip-form h3')).toContainText(TEST_GROUP)
}

async function fillAllScores(page: Page, home: string, away: string) {
  const rows = page
    .locator('.tip-form table.data-table')
    .first()
    .locator('tbody tr')
  const count = await rows.count()
  expect(count, 'the group has matches').toBeGreaterThan(0)
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption(home)
    await selects.nth(1).selectOption(away)
  }
}

test('My Tips: a locked group renders read-only immediately, with no re-lock', async ({
  page,
}) => {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, PRE_TOURNAMENT)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-dennis')

  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await selectTestGroup(page)

  // The group starts editable (Draft, Lock button present).
  await expect(page.locator('.tip-form .state-draft')).toBeVisible()
  await fillAllScores(page, '2', '0')

  // Lock it — the first (only) submit succeeds. Locking is irreversible, so the
  // button arms an inline confirm; click Confirm to commit.
  const lockBtn = page.getByRole('button', { name: 'Finalize predictions' })
  await expect(lockBtn).toBeEnabled()
  await lockBtn.click()
  await page.getByRole('button', { name: 'Confirm' }).click()

  // ── The regression assertions: WITHOUT a reload, the form must reflect the
  //    locked state, so the user is never invited to submit an already-locked
  //    group again. Before the fix the badge stayed "Draft" and the buttons
  //    remained, producing the "already locked" error on a second click. After
  //    the fix the form re-seeds from the refetched `me`: the group reads
  //    "Locked", shows the read-only notice, and the action buttons are gone.
  //    `assertNoGraphqlErrors` (below) proves the single submit succeeded.
  await expect(page.locator('.tip-form .state-locked')).toBeVisible()
  await expect(page.locator('.tip-form .flash-bar')).toContainText(
    'read-only',
  )
  await expect(page.locator('.tip-form .state-draft')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Finalize predictions' })).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Save draft' })).toHaveCount(0)

  // The lock must survive a reload too (server is the source of truth).
  await page.reload()
  await selectTestGroup(page)
  await expect(page.locator('.tip-form .state-locked')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
