import { test, expect } from '@playwright/test'
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
