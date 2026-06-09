import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * The Home page is identity-aware (design:
 * docs/superpowers/specs/2026-06-09-home-identity-aware-welcome-design.md).
 * A non-player can paste an invite code on Home and be routed to the claim
 * page; a Player sees quick-action links and no invite entry. Locators are
 * scoped to `.page` (the HomePage section) so they never match the NavBar.
 */

test('a logged-out visitor enters an invite code on Home and is routed to the claim page', async ({
  page,
}) => {
  const net = watchNetwork(page)
  // A fresh test context is logged out.
  await page.goto('/')

  const home = page.locator('.page')
  const box = home.getByPlaceholder('Paste your invite link or code')
  await expect(box).toBeVisible()
  await box.fill('ABC123XYZ0')
  await home.getByRole('button', { name: 'Open' }).click()

  await expect(page).toHaveURL(/\/invite\/ABC123XYZ0$/)
  await expect(page.getByText('Log in to claim this invite.')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('a logged-in player sees action links on Home and no invite entry', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/')

  const home = page.locator('.page')
  await expect(home.getByRole('link', { name: 'My Tips' })).toBeVisible()
  await expect(home.getByRole('link', { name: 'Pools' })).toBeVisible()
  await expect(
    home.getByPlaceholder('Paste your invite link or code'),
  ).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
