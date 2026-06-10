import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Venue data (`tournaments/fwc26.json`) — every fixture now carries a
 * "Stadium, City" venue (group stage from TheSportsDB, knockout from the
 * published FIFA schedule). The Schedule page renders it in the Venue column
 * (the 3rd cell); before the backfill every cell showed the "—" empty
 * fallback. The 4th cell (Result) legitimately shows "—" for unplayed matches,
 * so the empty-cell check is scoped to the Venue column only. Exercises the
 * real data over the live GraphQL API.
 */
test('the schedule shows a venue for every fixture', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Known anchors: the opener at the Estadio Azteca and the final at MetLife.
  await expect(
    page.getByRole('cell', { name: 'Estadio Azteca, Mexico City' }).first(),
  ).toBeVisible()
  await expect(
    page.getByRole('cell', { name: 'MetLife Stadium, New York' }).first(),
  ).toBeVisible()

  // Every Venue cell (3rd column) is populated — none falls back to "—".
  const venueCells = page.locator('.schedule-group tbody tr td:nth-child(3)')
  await expect(venueCells.first()).toBeVisible()
  expect(await venueCells.count()).toBeGreaterThan(100)
  await expect(venueCells.filter({ hasText: '—' })).toHaveCount(0)

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
