import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Round-aware navigation on My Tips. The match hierarchy's round nodes
 * (Group Stage / Round of 32 / …) drive a two-level nav: round tabs, then a
 * round-dependent body — Group Stage drills into one group, a knockout round
 * shows every match stacked.
 */

// The e2e API clock defaults to 2026-06-20 — the group stage is over, so the
// "current round" is Round of 32. Pin it for a deterministic default tab.
const MID_GROUP_STAGE = '2026-06-20T12:00:00Z'

test('My Tips round tabs: knockout shows all matches, Group Stage drills into one', async ({
  page,
}) => {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, MID_GROUP_STAGE)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)

  // All seven round tabs render.
  await expect(page.locator('.round-tab')).toHaveCount(7)
  await expect(
    page.locator('.round-tab', { hasText: /^Group Stage$/ }),
  ).toBeVisible()

  // Default tab is the current round (R32 at this clock) — many match forms,
  // and no group-pill row for a knockout round.
  await expect(
    page.locator('.round-tab.active', { hasText: /^Round of 32$/ }),
  ).toBeVisible()
  await expect(page.locator('.group-subnav')).toHaveCount(0)
  const knockoutForms = await page.locator('.tip-form').count()
  expect(knockoutForms, 'a knockout round stacks every match').toBeGreaterThan(1)

  // Switching to Group Stage reveals the group pills and drills into one group.
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await expect(page.locator('.group-subnav')).toBeVisible()
  await expect(page.locator('.tip-form')).toHaveCount(1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
