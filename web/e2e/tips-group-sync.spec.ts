import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView } from './helpers'

/**
 * The selected tip group is shared between My Tips and All Tips (persisted in
 * localStorage), so switching pages via the nav lands on the same group.
 */
test('group selection carries between My Tips and All Tips', async ({ page }) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  const activeGroup = page.locator('.group-subnav .subnav-item.active')

  // Pick a non-default group on My Tips (deep-link straight to Group D).
  await page.goto('/mytips/D')
  await expect(activeGroup).toHaveText('Group D')

  // Switch to All Tips via the nav → the same group is selected.
  await page.locator('.nav-bar').getByRole('link', { name: 'All Tips' }).click()
  await expect(page).toHaveURL(/\/alltips$/)
  await expect(activeGroup).toHaveText('Group D')

  // Change the group on All Tips → Group F.
  await page
    .locator('.group-subnav .subnav-item', { hasText: /^Group F$/ })
    .click()
  await expect(activeGroup).toHaveText('Group F')

  // Switch back to My Tips via the nav → Group F is preserved.
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips/)
  await expect(activeGroup).toHaveText('Group F')

  await expectNoErrorView(page)
})
