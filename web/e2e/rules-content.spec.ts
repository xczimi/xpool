import { test, expect } from '@playwright/test'
import { expectNoErrorView, openSettings, watchNetwork } from './helpers'

/**
 * Rules content (`.scratch/rules-content/PRD.md`): the Home page carries a
 * "how it works" summary that links through to the full Rules page, and every
 * rules string is i18n'd (EN + HU) — the page used to be hardcoded English.
 * `/` and `/rules` are both public, so a logged-out newcomer is the audience.
 */

test('Home "how it works" links through to the full Rules page', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  const home = page.locator('.page')
  await expect(home.getByRole('heading', { name: 'How it works' })).toBeVisible()
  await expect(
    home.getByText('Predict the exact score of every match.'),
  ).toBeVisible()

  await home.getByRole('link', { name: 'See full rules & scoring' }).click()
  await expect(page).toHaveURL(/\/rules$/)
  await expect(page.getByRole('heading', { name: 'Rules & Scoring' })).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('the Rules page renders its content in the selected UI language', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/rules')

  // English baseline — the section headings the i18n keys drive.
  await expect(page.getByRole('heading', { name: 'Per match' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Per group' })).toBeVisible()
  await expect(
    page.getByRole('heading', { name: 'Stage multipliers' }),
  ).toBeVisible()

  // Switch to Hungarian; the same sections render localised (was English-only).
  await openSettings(page)
  await page.getByRole('radio', { name: 'Magyar' }).click()
  await expect(page.getByRole('heading', { name: 'Meccsenként' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Csoportonként' })).toBeVisible()
  await expect(
    page.getByRole('heading', { name: 'Szakasz-szorzók' }),
  ).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Per match' })).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
