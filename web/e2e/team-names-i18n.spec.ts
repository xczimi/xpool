import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Team names follow the UI language. `/games` is public; we force the
 * name-showing display mode, then toggle the language picker and assert a known
 * team (Mexico — the host, always on the schedule) renders in Hungarian, then
 * back in English.
 */
test('team names render in the selected UI language', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Show full names (not flags/codes) so the text is assertable.
  await page.locator('.header-controls select').first().selectOption('name')

  // Both DisplayModeSelector and LanguageSelector share the `.lang-selector`
  // wrapper class; the language picker is the second one in the header.
  const lang = page.locator('.lang-selector select').last()

  // English baseline.
  await lang.selectOption('en')
  await expect(page.locator('.team-label-text', { hasText: 'Mexico' }).first()).toBeVisible()

  // Hungarian: the same team renders localised; the English name is gone.
  await lang.selectOption('hu')
  await expect(page.locator('.team-label-text', { hasText: 'Mexikó' }).first()).toBeVisible()
  await expect(page.locator('.team-label-text', { hasText: /^Mexico$/ })).toHaveCount(0)

  // Back to English.
  await lang.selectOption('en')
  await expect(page.locator('.team-label-text', { hasText: 'Mexico' }).first()).toBeVisible()

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})
