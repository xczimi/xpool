import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Best third-placed teams table — drives the `thirdPlaceRanking` query end to
 * end on both surfaces. A schema mismatch (query vs resolver) surfaces here as
 * a GraphQL error; a render crash as a page error. The section header must
 * appear on each page regardless of how much of the group stage is decided (an
 * empty ranking still renders the pending hint).
 */

test('Schedule shows the official best-thirds section without errors', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  // Schedule nav link is "Schedule" (navGames); route is /games
  await page.locator('.nav-bar').getByRole('link', { name: 'Schedule' }).click()
  await expect(page).toHaveURL(/\/games$/)

  const section = page.getByTestId('third-place-section')
  await expect(section).toBeVisible()
  await expect(section.locator('h3')).toHaveText('Best third-placed teams')
  // Either a rendered table or the pending/provisional hint must be present
  await expect(
    section.locator('table.third-place-table, p.hint'),
  ).not.toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('My Tips shows predicted + official best-thirds without errors', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)

  const section = page.getByTestId('third-place-section')
  await expect(section).toBeVisible()
  await expect(section.locator('h3')).toHaveText('Best third-placed teams')
  // Two ThirdPlaceTable panels: predicted (mine) + official
  await expect(section.locator('.third-place')).toHaveCount(2)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

/**
 * The best-thirds table belongs to the group stage (it ranks the 3rd-placed
 * teams to decide which advance). It must appear on My Tips' Group Stage tab and
 * be absent from every knockout round tab.
 *
 * The e2e seed has no official results, so the R32 tab is normally hidden. We
 * seed Group C + Group F as result-user to place M75/M76 and make R32 "ready"
 * (readyRounds detects ≥ 1 game with both teams) — mirroring
 * `mytips-knockout-unplaced.spec.ts`. With the R32 tab visible we can assert the
 * thirds section flips off when the knockout round is selected.
 */

// Clock past the last Group C / Group F games (June 24-25) so the recompute on
// result-user save has the full group-stage slice; R32 games start June 28, so
// R32's deadline has not passed.
const KNOCKOUT_CLOCK = '2026-06-26T12:00:00Z'

/** Select a group-stage leaf group in My Tips as the current viewer. */
async function openGroupStageGroup(page: Page, groupName: string): Promise<void> {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await page
    .locator('.group-subnav button', { hasText: new RegExp(`^${groupName}$`) })
    .click()
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

test('My Tips best-thirds shows on Group Stage tab, hidden on knockout tab', async ({
  page,
}) => {
  await page.addInitScript((value: string) => {
    localStorage.setItem('xpool.devNow', value)
  }, KNOCKOUT_CLOCK)

  const net = watchNetwork(page)
  await page.goto('/')

  // result-user seeds Group C + Group F official results → M75/M76 fully placed
  // → R32 becomes "ready" and its tab appears.
  await devLogin(page, 'result-user')
  await openGroupStageGroup(page, 'Group C')
  await fillAllAndSave(page)
  await openGroupStageGroup(page, 'Group F')
  await fillAllAndSave(page)

  // demo-ada views My Tips.
  await devLogin(page, 'demo-ada')
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)

  // Group Stage tab: the best-thirds section is present.
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await expect(page.getByTestId('third-place-section')).toBeVisible()

  // Round of 32 tab: the best-thirds section must be gone.
  const r32Tab = page.locator('.round-tab', { hasText: /^Round of 32$/ })
  await expect(r32Tab).toBeVisible()
  await r32Tab.click()
  await expect(page.getByTestId('third-place-section')).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
