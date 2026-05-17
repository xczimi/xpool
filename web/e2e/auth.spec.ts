import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Dev-login + auth-gating flows. The dev auth bar is backed by the `players`
 * GraphQL query — exactly the resolver Task B adds an API test for. A schema
 * mismatch there breaks the picker; this test exercises it end to end.
 */

test('dev login picks a seeded player and unlocks player-only nav', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // As a visitor: player-only nav links are absent.
  await expect(page.locator('.auth-bar')).toContainText('You are outside.')
  await expect(page.getByRole('link', { name: 'My Tips' })).toHaveCount(0)
  await expect(page.getByRole('link', { name: 'Profile' })).toHaveCount(0)

  // Pick demo-ada from the auth-bar picker.
  await devLogin(page, 'demo-ada')
  await expect(page.locator('.auth-bar')).toContainText('ada')

  // Player-only nav becomes usable.
  await expect(page.getByRole('link', { name: 'My Tips' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Profile' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'All Tips' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Invite' })).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('profile loads for a logged-in player with no auth error', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await page.getByRole('link', { name: 'Profile' }).click()
  await expect(page).toHaveURL(/\/profile$/)
  await expect(page.locator('h2')).toHaveText('Profile')

  // The profile form is seeded from the `me` query — no auth/error view.
  await expect(page.locator('form.form input').first()).toBeVisible()
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('visiting /profile as a visitor shows the auth-required state, not a crash', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/profile')

  // The NeedsLogin component, not an ErrorView and not a blank page.
  await expect(page.getByText('Login required')).toBeVisible()
  await expect(page.locator('main.content')).toContainText(
    'This screen is for players only',
  )
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
