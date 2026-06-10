import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * My Tips finalize countdown — humane, urgency-scaled display (design:
 * docs/superpowers/specs/2026-06-10-humane-countdown-design.md, building on
 * docs/superpowers/specs/2026-06-09-mytips-finalize-countdown-design.md).
 *
 * The countdown is anchored to the server clock (the dev clock / X-Dev-Now),
 * then ticks via the browser — so pinning the dev clock fixes the *starting*
 * value. Group A's deadline is the opener kickoff, 2026-06-11T19:00:00Z; demo
 * players seed with no predictions, so a fresh login has Group A open.
 *
 * The granularity scales to urgency: days out → "in N days" (no ticking),
 * within the day → "in Hh Mm", and only the final hour ticks per second
 * (MM:SS). The absolute local deadline is always shown alongside.
 */

/** Pin the server-authoritative dev clock before the app loads. */
async function setDevClock(page: Page, iso: string) {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, iso)
}

test('days from the deadline: coarse relative, no ticking, absolute deadline shown', async ({
  page,
}) => {
  const net = watchNetwork(page)
  // ~3 days before Group A's deadline (the opener).
  await setDevClock(page, '2026-06-08T12:00:00Z')
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/mytips')

  const banner = page.locator('.finalize-banner')
  await expect(banner).toBeVisible()
  await expect(banner).toContainText('Group A')

  // An open group nudges the player to predict every match before the deadline.
  await expect(
    page.getByText(/all games of the group before the deadline/i).first(),
  ).toBeVisible()

  const countdown = page.locator('.finalize-countdown').first()
  await expect(countdown).toBeVisible()
  // Day-grained relative — and crucially NO ticking HH:MM:SS clock.
  await expect(countdown).toContainText(/in \d+ days?/)
  await expect(countdown).not.toContainText(/\d{2}:\d{2}:\d{2}/)
  // The absolute deadline (a local HH:MM time) is shown alongside.
  await expect(countdown).toContainText(/\d{1,2}:\d{2}/)

  // It does not tick: the rendered text is stable across a second+.
  const before = await countdown.textContent()
  await page.waitForTimeout(1500)
  expect(await countdown.textContent()).toBe(before)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('within the final hour: ticking MM:SS alongside the absolute deadline', async ({
  page,
}) => {
  const net = watchNetwork(page)
  // 15 minutes before Group A's deadline — the urgency zone.
  await setDevClock(page, '2026-06-11T18:45:00Z')
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/mytips')

  const countdown = page.locator('.finalize-countdown').first()
  await expect(countdown).toBeVisible()
  // Not the coarse day/hour tier this close in…
  await expect(countdown).not.toContainText(/in \d/)
  // …and it ticks per second (the MM:SS field changes within a second+).
  const before = await countdown.textContent()
  await page.waitForTimeout(1500)
  expect(await countdown.textContent()).not.toBe(before)

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
  // Nothing left to finalize: no banner, no open per-group timer, no hint.
  await expect(
    page.getByText(/all games of the group before the deadline/i),
  ).toHaveCount(0)
  await expect(page.locator('.finalize-banner')).toHaveCount(0)
  await expect(page.locator('.finalize-countdown')).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
