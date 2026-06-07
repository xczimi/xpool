import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Team display-mode toggle. The selector is a global preference (localStorage);
 * `/games` is public, so no login is needed. We assert real flag PNGs load
 * (naturalWidth > 0 — a 404 would be 0), that name text shows/hides per mode,
 * and that the choice survives a reload.
 */
test('display-mode selector switches flags/names and persists', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  const selector = page.locator('.header-controls select').first()

  // Flag + name: both a flag image and the country name are present.
  await selector.selectOption('flag-name')
  const firstFlag = page.locator('img.team-flag').first()
  await expect(firstFlag).toBeVisible()
  // The bundled PNG actually resolves (not a broken 404 image).
  await expect
    .poll(async () => firstFlag.evaluate((img: HTMLImageElement) => img.naturalWidth))
    .toBeGreaterThan(0)
  await expect(page.locator('.team-label-text').first()).toBeVisible()

  // Flag only: flag images are shown; resolved teams show NO text alongside the
  // flag (unresolved knockout slots still render a placeholder description but
  // never a flag, so flags and text labels never coexist in the same cell).
  await selector.selectOption('flag')
  await expect(page.locator('img.team-flag').first()).toBeVisible()
  // No .team-label should contain both a flag image AND a text label.
  await expect(page.locator('.team-label:has(img.team-flag):has(.team-label-text)')).toHaveCount(0)

  // Name only: text returns for all resolved teams, flags disappear entirely.
  await selector.selectOption('name')
  await expect(page.locator('.team-label-text').first()).toBeVisible()
  await expect(page.locator('img.team-flag')).toHaveCount(0)

  // Persists across reload.
  await page.reload()
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expect(page.locator('img.team-flag')).toHaveCount(0)
  await expect(page.locator('.team-label-text').first()).toBeVisible()
  await expect(selector).toHaveValue('name')

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})
