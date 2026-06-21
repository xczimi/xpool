import { test, expect } from '@playwright/test'
import { devLogin, devLogout, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Nav "My player page" (own-player-page-access). For a logged-in demo player a
 * player-gated nav item routes to their /player/:id. The static Profile nav
 * link is removed, but Profile stays reachable from the own player-detail page.
 */
test('My player page nav item routes to own /player/:id; Profile moved off the nav', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // Visitor (not logged in): the player-gated item is absent.
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'My player page' }),
  ).toHaveCount(0)

  await devLogin(page, 'demo-ada')

  // Profile is no longer in the nav.
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'Profile', exact: true }),
  ).toHaveCount(0)

  // The "My player page" nav item is present and routes to demo-ada's page.
  const ownNav = page
    .locator('.nav-bar')
    .getByRole('link', { name: 'My player page' })
  await expect(ownNav).toBeVisible()
  await ownNav.click()
  await expect(page).toHaveURL(/\/player\/demo-ada$/)
  await expectNoErrorView(page)

  // Profile is reachable from the own player page via the new link.
  const profileLink = page
    .locator('.player-profile-link')
    .getByRole('link', { name: 'Profile & settings' })
  await expect(profileLink).toBeVisible()
  await profileLink.click()
  await expect(page).toHaveURL(/\/profile$/)
  await expectNoErrorView(page)

  // The result user has no own-player nav item (they are excluded).
  await devLogout(page)
  await devLogin(page, 'result-user')
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'My player page' }),
  ).toHaveCount(0)

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
