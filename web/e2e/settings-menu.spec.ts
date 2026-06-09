import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Settings menu — the chrome preferences (language, display flag/text, theme
 * accent/mode) are collapsed behind a gear in the header and revealed in a
 * popover with a visible text label on every row. `/games` is public, so no
 * login is needed.
 */
test('gear reveals a labelled settings panel and closes on Escape / outside click', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Collapsed by default: the toggles are not on screen.
  await expect(page.getByRole('radiogroup', { name: 'Flag' })).toBeHidden()

  // Gear opens the panel.
  const gear = page.getByRole('button', { name: 'Settings' })
  await expect(gear).toHaveAttribute('aria-expanded', 'false')
  await gear.click()
  const panel = page.getByRole('dialog', { name: 'Settings' })
  await expect(panel).toBeVisible()
  await expect(gear).toHaveAttribute('aria-expanded', 'true')

  // Every row carries a visible label (the old confusion was unlabelled toggles).
  for (const label of ['Language', 'Flag', 'Text', 'Theme', 'Mode']) {
    await expect(panel.getByText(label, { exact: true })).toBeVisible()
  }
  // And the toggles themselves are now reachable.
  await expect(page.getByRole('radiogroup', { name: 'Flag' })).toBeVisible()

  // Escape closes it.
  await page.keyboard.press('Escape')
  await expect(panel).toBeHidden()
  await expect(gear).toHaveAttribute('aria-expanded', 'false')

  // Reopen, then an outside click closes it.
  await gear.click()
  await expect(panel).toBeVisible()
  await page.locator('h2').click()
  await expect(panel).toBeHidden()

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})
