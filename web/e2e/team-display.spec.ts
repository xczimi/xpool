import { test, expect, type Page } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Team display picker — now two segmented toggles (Flag on/off, Text
 * auto/name/code/off) replacing the old `<select>`. It is a global preference
 * (localStorage); `/games` is public, so no login is needed. We assert real
 * flag PNGs load (naturalWidth > 0 — a 404 would be 0), that name text
 * shows/hides per axis, that the Flag-off + Text-off combo is guarded, and that
 * the choice survives a reload.
 */
const flagRadio = (page: Page, name: string) =>
  page.getByRole('radiogroup', { name: 'Flag' }).getByRole('radio', { name })
const textRadio = (page: Page, name: string) =>
  page.getByRole('radiogroup', { name: 'Text' }).getByRole('radio', { name })

test('display toggles switch flags/names, guard the empty combo, and persist', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Flag On + Text Name: both a flag image and the country name are present.
  await flagRadio(page, 'On').click()
  await textRadio(page, 'Name').click()
  const firstFlag = page.locator('img.team-flag').first()
  await expect(firstFlag).toBeVisible()
  // The bundled PNG actually resolves (not a broken 404 image).
  await expect
    .poll(async () =>
      firstFlag.evaluate((img: HTMLImageElement) => img.naturalWidth),
    )
    .toBeGreaterThan(0)
  await expect(page.locator('.team-label-text').first()).toBeVisible()

  // Text Off (flag-only): flag images show; resolved teams show NO text. (An
  // unresolved knockout slot still renders a placeholder description but never a
  // flag, so flags and text labels never coexist in the same cell.)
  await textRadio(page, 'Off').click()
  await expect(page.locator('img.team-flag').first()).toBeVisible()
  await expect(
    page.locator('.team-label:has(img.team-flag):has(.team-label-text)'),
  ).toHaveCount(0)

  // Flag Off: the Text Off segment is guarded (would render nothing), so it is
  // disabled. Switch text back to Name → text returns, flags disappear.
  await textRadio(page, 'Name').click()
  await flagRadio(page, 'Off').click()
  await expect(textRadio(page, 'Off')).toBeDisabled()
  await expect(page.locator('.team-label-text').first()).toBeVisible()
  await expect(page.locator('img.team-flag')).toHaveCount(0)

  // Persists across reload: Flag Off + Text Name → names, no flags.
  await page.reload()
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expect(page.locator('img.team-flag')).toHaveCount(0)
  await expect(page.locator('.team-label-text').first()).toBeVisible()
  await expect(flagRadio(page, 'Off')).toHaveAttribute('aria-checked', 'true')
  await expect(textRadio(page, 'Name')).toHaveAttribute('aria-checked', 'true')

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})
