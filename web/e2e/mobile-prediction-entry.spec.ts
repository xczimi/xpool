import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Mobile prediction entry: on a phone viewport the group-stage tips render as a
 * swipe one-group-per-screen card with big +/− steppers. Entering a score
 * autosaves (a `submitGroup` POST) and persists across reload; "Next group"
 * advances the progress.
 *
 * Group G / demo-grace, pre-tournament so group-stage tips are editable, and
 * untouched by other specs (order-independent, mutates only Group G).
 */
const PRE_TOURNAMENT = '2026-01-01T12:00:00Z'

test.use({ viewport: { width: 390, height: 844 } })

test('Mobile My Tips: stepper entry autosaves and persists', async ({ page }) => {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, PRE_TOURNAMENT)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  await page.goto('/mytips/G')

  // The mobile flow is rendered (not the desktop table).
  const entry = page.locator('.mobile-entry')
  await expect(entry).toBeVisible()
  await expect(page.locator('.mobile-entry-label')).toContainText('Group G')

  // Enter a COMPLETE prediction on the first match (both sides). A match with
  // one side unset cannot be persisted (the server requires both scores), so
  // only a complete entry survives a reload.
  //
  // The stepper starts UNSET ("–"); the first `+` commits 0, so reaching a
  // displayed `3` takes FOUR taps (– → 0 → 1 → 2 → 3). The away side is the
  // same: one tap from unset commits 0. (See `lib/score.ts` `stepScore`.)
  const firstMatch = page.locator('.mobile-match').first()
  const homeStepper = firstMatch.locator('.score-stepper').first()
  const awayStepper = firstMatch.locator('.score-stepper').nth(1)
  await homeStepper.locator('.score-stepper-inc').click() // – → 0
  await homeStepper.locator('.score-stepper-inc').click() // 0 → 1
  await homeStepper.locator('.score-stepper-inc').click() // 1 → 2
  await homeStepper.locator('.score-stepper-inc').click() // 2 → 3
  await expect(homeStepper.locator('.score-stepper-value')).toHaveText('3')
  await awayStepper.locator('.score-stepper-inc').click() // – → 0
  await expect(awayStepper.locator('.score-stepper-value')).toHaveText('0')

  // Autosave fires (debounced) → status shows the saved string.
  await expect(page.locator('.mobile-save-status.saved')).toBeVisible()

  // Reload: the autosaved draft re-seeds both steppers.
  await page.goto('/mytips/G')
  const reloadedFirst = page.locator('.mobile-match').first()
  await expect(
    reloadedFirst.locator('.score-stepper').first().locator('.score-stepper-value'),
  ).toHaveText('3')
  await expect(
    reloadedFirst.locator('.score-stepper').nth(1).locator('.score-stepper-value'),
  ).toHaveText('0')

  // "Next group" advances the progress index.
  const label = page.locator('.mobile-entry-label')
  const before = (await label.textContent()) ?? ''
  const beforeIndex = Number(before.match(/·\s*(\d+)\s/)?.[1] ?? '0')
  await page.locator('.mobile-entry-next').click()
  await expect
    .poll(async () => {
      const text = (await label.textContent()) ?? ''
      return Number(text.match(/·\s*(\d+)\s/)?.[1] ?? '0')
    })
    .toBe(beforeIndex + 1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
