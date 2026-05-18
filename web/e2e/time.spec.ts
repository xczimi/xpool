import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * The dev clock (X-Dev-Now) is the server-authoritative test clock
 * (.specs/TESTING.md §3). Moving it changes time-dependent screens — proof
 * the whole clock seam works end to end.
 *
 * Fixture dates verified against tournaments/fwc26.json:
 *  - Group stage runs 2026-06-11 through 2026-06-29.
 *  - 2026-06-20 has 4 matches; the ±2-day Today window covers 2026-06-18..22.
 *  - 2026-01-01 is ~5 months before the first match: Today window is empty.
 */

/** Set the dev clock via localStorage before the app loads. */
async function setDevClock(page: import('@playwright/test').Page, iso: string) {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, iso)
}

test('Today is empty well before the tournament, populated during it', async ({
  page,
}) => {
  const net = watchNetwork(page)

  // Clock far before any match -> Today window catches nothing.
  await setDevClock(page, '2026-01-01T12:00:00Z')
  await page.goto('/today')
  await expect(page.getByText('No matches near now.')).toBeVisible()
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()

  // Clock during the group stage (2026-06-20 has 4 matches) -> Today shows matches.
  await setDevClock(page, '2026-06-20T12:00:00Z')
  await page.goto('/today')
  await expect(page.getByText('No matches near now.')).toHaveCount(0)
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
