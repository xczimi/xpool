import { test, expect } from '@playwright/test'
import { expectNoErrorView, openSettings, watchNetwork } from './helpers'

/**
 * Theme switcher — accent presets + dark/light mode. Both are global
 * localStorage preferences available to logged-out visitors; `/games` is
 * public, so no login is needed. We assert the <html> data-attributes drive
 * the CSS custom properties and that choices survive a reload.
 */
test('accent + mode switch, drive CSS tokens, and persist', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)
  await openSettings(page)

  const html = page.locator('html')
  const cssVar = (name: string) =>
    page.evaluate(
      (n) =>
        getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
      name,
    )

  // Default accent is amber, and the token resolves to the amber base.
  await expect(html).toHaveAttribute('data-accent', 'amber')
  expect(await cssVar('--accent')).toBe('#ff8c00')

  // Pick the cyan accent → attribute + token both change.
  await page.getByRole('radio', { name: 'Cyan' }).click()
  await expect(html).toHaveAttribute('data-accent', 'cyan')
  expect(await cssVar('--accent')).toBe('#21d4fd')

  // Force Dark, capture the surface, then switch to Light → surface changes.
  await page.getByRole('radio', { name: 'Dark' }).click()
  await expect(html).toHaveAttribute('data-theme', 'dark')
  const darkBg = await cssVar('--bg-deep')

  await page.getByRole('radio', { name: 'Light' }).click()
  await expect(html).toHaveAttribute('data-theme', 'light')
  expect(await cssVar('--bg-deep')).not.toBe(darkBg)

  // Both choices persist across a reload.
  await page.reload()
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expect(html).toHaveAttribute('data-accent', 'cyan')
  await expect(html).toHaveAttribute('data-theme', 'light')

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})

/**
 * System mode follows the OS preference live, via the provider's matchMedia
 * subscription. Playwright's emulateMedia drives prefers-color-scheme.
 */
test('system mode follows prefers-color-scheme', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' })
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await openSettings(page)

  const html = page.locator('html')

  // Default mode is system; emulated light OS preference → resolved light.
  await page.getByRole('radio', { name: 'System' }).click()
  await expect(html).toHaveAttribute('data-theme', 'light')

  // Flip the OS preference → the resolved theme follows without a reload.
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(html).toHaveAttribute('data-theme', 'dark')
})
