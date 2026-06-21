import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Schedule By-date view. The schedule page offers a By-group ⇄ By-date toggle.
 * By-date buckets every fixture into local-calendar-day sections (reusing the
 * same match rows / /match/:id links), and the chosen view persists per-user
 * in localStorage across a reload. Public, read-only — no login required.
 */
test('By-date toggle sections matches by day and persists across reload', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/games')

  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Default view is By group.
  await expect(page.locator('.schedule-group').first()).toBeVisible()
  await expect(page.locator('.schedule-day')).toHaveCount(0)

  // Switch to By date → day sections appear, group sections disappear.
  await page.getByRole('button', { name: 'By date' }).click()
  const daySections = page.locator('.schedule-day')
  await expect(daySections.first()).toBeVisible()
  expect(
    await daySections.count(),
    'the tournament spans multiple calendar days',
  ).toBeGreaterThan(1)
  await expect(page.locator('.schedule-group')).toHaveCount(0)

  // Each day section still has a day heading and clickable /match/:id rows.
  await expect(daySections.first().locator('h3')).not.toBeEmpty()
  const firstMatchLink = daySections
    .first()
    .locator('tbody tr')
    .first()
    .locator('a[href^="/match/"]')
  await expect(firstMatchLink).toBeVisible()

  // Day sections are chronological: first section's first kickoff <= second's.
  const firstKickoffOf = async (i: number) => {
    const cell = daySections
      .nth(i)
      .locator('tbody tr')
      .first()
      .locator('td')
      .first()
    return Date.parse(((await cell.textContent()) ?? '').trim())
  }
  expect(await firstKickoffOf(1)).toBeGreaterThanOrEqual(
    await firstKickoffOf(0),
  )

  // Persistence: reload → By-date is still the active view.
  await page.reload()
  await expect(page.locator('.schedule-day').first()).toBeVisible()
  await expect(page.locator('.schedule-group')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'By date' })).toHaveAttribute(
    'aria-pressed',
    'true',
  )

  // Toggle back to By group → persisted again.
  await page.getByRole('button', { name: 'By group' }).click()
  await expect(page.locator('.schedule-group').first()).toBeVisible()
  await page.reload()
  await expect(page.locator('.schedule-group').first()).toBeVisible()
  await expect(page.locator('.schedule-day')).toHaveCount(0)

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
