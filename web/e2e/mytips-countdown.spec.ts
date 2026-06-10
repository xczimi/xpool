import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * My Tips finalize countdown (design:
 * docs/superpowers/specs/2026-06-09-mytips-finalize-countdown-design.md).
 *
 * The countdown is anchored to the server clock (the dev clock / X-Dev-Now),
 * then ticks via the browser — so pinning the dev clock fixes the *starting*
 * value. Group A's deadline is the opener kickoff, 2026-06-11T19:00:00Z; demo
 * players seed with no predictions, so a fresh login has Group A open.
 */

/** Pin the server-authoritative dev clock before the app loads. */
async function setDevClock(page: Page, iso: string) {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, iso)
}

test('shows the next-to-finalize banner and a per-group countdown while open', async ({
  page,
}) => {
  const net = watchNetwork(page)
  // 15 minutes before Group A's deadline (the opener).
  await setDevClock(page, '2026-06-11T18:45:00Z')
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/mytips')

  // Page-level banner: names the soonest open group and shows an HH:MM:SS timer.
  const banner = page.locator('.finalize-banner')
  await expect(banner).toBeVisible()
  await expect(banner).toContainText('Group A')
  await expect(banner).toContainText(/\d{2}:\d{2}:\d{2}/)

  // The open group's form carries its own inline countdown.
  await expect(page.locator('.finalize-countdown').first()).toBeVisible()
  await expect(page.locator('.finalize-countdown').first()).toContainText(
    /\d{2}:\d{2}:\d{2}/,
  )

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('no countdown once every deadline has passed', async ({ page }) => {
  const net = watchNetwork(page)
  // After the final — every group's deadline is in the past.
  await setDevClock(page, '2026-08-01T12:00:00Z')
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/mytips')

  await expect(page.getByRole('heading', { name: 'My Tips' })).toBeVisible()
  // Nothing left to finalize: no banner, no open per-group timer.
  await expect(page.locator('.finalize-banner')).toHaveCount(0)
  await expect(page.locator('.finalize-countdown')).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
