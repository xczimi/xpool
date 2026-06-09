import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * First-run UX: the invite link shows a clear "Continue to join" entry, the
 * settings gear is labelled, and the Profile form (now the shared NameForm)
 * still renders its labelled fields.
 */

test('the settings trigger shows a visible "Settings" label', async ({ page }) => {
  await page.goto('/')
  await expect(page.locator('.settings-gear')).toContainText('Settings')
})

test('the invite welcome shows "Continue to join" for a logged-out visitor', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/invite/SOUTH7K-AD9XK3P7QT')
  await expect(page.locator('main.content h2')).toBeVisible()
  await expect(
    page.getByRole('button', { name: 'Continue to join' }),
  ).toBeVisible()
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('the Profile form renders labelled name fields and Save (shared NameForm)', async ({
  page,
}) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/profile')
  const form = page.locator('form.form')
  await expect(form).toBeVisible()
  await expect(form.locator('label', { hasText: 'Nick' })).toBeVisible()
  await expect(form.locator('label', { hasText: 'Full name' })).toBeVisible()
  await expect(form.getByRole('button', { name: 'Save' })).toBeVisible()
})
