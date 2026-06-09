import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * The dev clock's game-relative presets (DevClock.tsx) set X-Dev-Now to an
 * instant relative to a chosen game, landing on a time-dependent state without
 * hand-typing a timestamp. This drives the new two-`<select>` control end to
 * end and proves the picked instant reaches the server clock: the same Group A
 * is editable at the `before` preset and locked at the `during` preset.
 *
 * Group A's earliest kickoff is M1 (2026-06-11T19:00Z), so it is the group's
 * prediction deadline. `before` = K−10m (18:50Z, open); `during` = K+60m
 * (20:00Z, deadline passed → locked).
 */

const GAME = 'M1' // Group A, MEX v RSA, first kickoff of the tournament

/** Pick a game + phase in the auth-bar dev clock; it applies and reloads. */
async function setPreset(page: Page, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(GAME) // game — enables the phase select
  await expect(selects.nth(1)).toBeEnabled()
  await selects.nth(1).selectOption(phase) // phase — applies + reloads
}

/** Open Group A in the My Tips sub-navigation. */
async function openGroupA(page: Page) {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  // Row 1: pick the Group Stage round — only then do the group pills appear.
  await page
    .locator('.round-tabs button', { hasText: /^Group Stage$/ })
    .click()
  // Row 2: pick Group A (its earliest kickoff M1 is the group's deadline).
  await page.locator('.group-subnav button', { hasText: /^Group A$/ }).click()
  await expect(page.locator('.tip-form h3')).toContainText('Group A')
}

test('dev clock presets: `during` locks the chosen game’s group, `before` keeps it open', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  // `before` M1 → Group A's deadline is in the future → predictions open.
  await setPreset(page, 'before')
  await openGroupA(page)
  await expect(page.getByRole('button', { name: 'Save draft' })).toBeVisible()
  await expect(page.locator('.tip-form .flash-bar')).toHaveCount(0)

  // `during` M1 → deadline passed → the same group is now read-only.
  await setPreset(page, 'during')
  await openGroupA(page)
  await expect(page.locator('.tip-form .flash-bar')).toContainText(
    'This group is locked',
  )
  await expect(page.getByRole('button', { name: 'Save draft' })).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
