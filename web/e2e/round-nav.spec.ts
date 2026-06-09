import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Round-aware navigation on My Tips. Only rounds ready for predictions show a
 * tab: a round is ready once one of its games has both teams determined. Before
 * the bracket resolves, every knockout round is hidden and only Group Stage is
 * navigable.
 */

// M1 is Group A's earliest kickoff = the group-stage deadline.
const GAME = 'M1'

/**
 * Pin the dev clock to GAME + phase via the auth-bar picker. The pick fires
 * `devRematerialize` (re-resolving the bracket as-of the instant) then reloads —
 * tie the action to the reload so we assert on the fresh DOM.
 */
async function setClock(page: Page, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(GAME)
  await expect(selects.nth(1)).toBeEnabled()
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  await expect(page.locator('.dev-clock-now')).toBeVisible()
}

test('My Tips hides knockout rounds until their teams are known', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  // As-of just before the first kickoff: nothing is played, so the bracket is
  // entirely unresolved — only Group Stage is ready.
  await setClock(page, 'before')

  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)

  // Only the Group Stage tab renders; the six knockout rounds are hidden.
  await expect(page.locator('.round-tab')).toHaveCount(1)
  await expect(
    page.locator('.round-tab.active', { hasText: /^Group Stage$/ }),
  ).toBeVisible()
  await expect(
    page.locator('.round-tab', { hasText: /^Round of 32$/ }),
  ).toHaveCount(0)
  await expect(
    page.locator('.round-tab', { hasText: /^Final$/ }),
  ).toHaveCount(0)

  // Group Stage still drills into a single group via the group pills.
  await expect(page.locator('.group-subnav')).toBeVisible()
  await expect(page.locator('.tip-form')).toHaveCount(1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
