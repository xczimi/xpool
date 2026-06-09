import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Team names follow the UI language. `/games` is public; we force the
 * name-showing display mode (Text = Name), then toggle the language picker and
 * assert a known team (Mexico — the host, always on the schedule) renders in
 * Hungarian, then back in English.
 */
test('team names render in the selected UI language', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Show full names (not flags/codes) so the text is assertable.
  await page
    .getByRole('radiogroup', { name: 'Text' })
    .getByRole('radio', { name: 'Name' })
    .click()

  // Language picker is its own segmented toggle; each segment's accessible name
  // is the full language name (English / Magyar), constant across locales — so
  // we target the radios directly. (The group's own aria-label IS translated,
  // so scoping to a group named "Language" would break once the UI is in HU.)
  const langRadio = (name: string) => page.getByRole('radio', { name })

  // English baseline.
  await langRadio('English').click()
  await expect(
    page.locator('.team-label-text', { hasText: 'Mexico' }).first(),
  ).toBeVisible()

  // Hungarian: the same team renders localised; the English name is gone.
  await langRadio('Magyar').click()
  await expect(
    page.locator('.team-label-text', { hasText: 'Mexikó' }).first(),
  ).toBeVisible()
  await expect(
    page.locator('.team-label-text', { hasText: /^Mexico$/ }),
  ).toHaveCount(0)

  // Back to English.
  await langRadio('English').click()
  await expect(
    page.locator('.team-label-text', { hasText: 'Mexico' }).first(),
  ).toBeVisible()

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})
