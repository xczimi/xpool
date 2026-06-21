import { test, expect } from '@playwright/test'
import { devLogin, devLogout, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Nav "Me" (own-player-page-access). For a logged-in demo player a player-gated
 * nav item sits right after Home and routes to the clean `/me` alias (no UUID),
 * which renders their own player page. The static Profile nav link is removed,
 * but Profile stays reachable from the own player-detail page.
 */
test('Me nav item routes to the /me alias; Profile moved off the nav', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // Visitor (not logged in): the player-gated item is absent.
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'Me', exact: true }),
  ).toHaveCount(0)

  await devLogin(page, 'demo-ada')

  // Profile is no longer in the nav.
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'Profile', exact: true }),
  ).toHaveCount(0)

  // "Me" sits right after Home and routes to /me (no UUID), which renders
  // demo-ada's own player page.
  const navLinks = page.locator('.nav-bar .nav-link')
  await expect(navLinks.nth(0)).toHaveText('Home')
  await expect(navLinks.nth(1)).toHaveText('Me')
  const ownNav = page
    .locator('.nav-bar')
    .getByRole('link', { name: 'Me', exact: true })
  await ownNav.click()
  await expect(page).toHaveURL(/\/me$/)
  await expect(page.locator('.player-header')).toBeVisible()
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
    page.locator('.nav-bar').getByRole('link', { name: 'Me', exact: true }),
  ).toHaveCount(0)

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
