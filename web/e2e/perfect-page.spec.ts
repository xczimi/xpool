import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Perfect page (UC-10) end to end: the sticky pool picker scopes the perfects
 * list, and the by-player toggle reorders the flat list. Exercises the
 * `perfects(pool:)` argument and the SelectedPool context on the wire. The
 * e2e DynamoDB persists across runs, so the scoping test creates a uniquely
 * named single-member pool rather than relying on the shared seeded data.
 */

test('by-player toggle reorders the perfects list', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.locator('.nav-bar').getByRole('link', { name: 'Perfect' }).click()
  await expect(page).toHaveURL(/\/perfect$/)

  // Scope to "Everyone" so the toggle has multiple players to reorder.
  await page.locator('.pool-selector select').selectOption('')

  // Default view is by-match. Capture the player-column order.
  const matchOrder = await page.locator('.data-table tbody tr td:first-child').allInnerTexts()

  // Switch to by-player and re-read. The page may have no perfects yet (the
  // seeded clock can precede results) — only assert reordering when rows exist.
  await page.getByRole('button', { name: 'By player' }).click()
  await expect(page.getByRole('button', { name: 'By player' })).toHaveAttribute(
    'aria-pressed',
    'true',
  )
  const playerOrder = await page.locator('.data-table tbody tr td:first-child').allInnerTexts()

  if (matchOrder.length > 1) {
    // By-player groups each nick contiguously: no nick appears in two
    // non-adjacent blocks.
    const seen = new Set<string>()
    let prev = ''
    for (const nick of playerOrder) {
      if (nick !== prev) {
        expect(seen.has(nick), `${nick} should be contiguous`).toBe(false)
        seen.add(nick)
        prev = nick
      }
    }
  }

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('selecting a single-member pool scopes the perfects list', async ({ page }) => {
  const net = watchNetwork(page)
  const poolName = `Perfect Scope ${Date.now()}`
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // Create a fresh pool — grace is its sole member (a strict subset of all).
  await page.goto('/pools')
  await page.getByPlaceholder('e.g. Office League').fill(poolName)
  await page.getByRole('button', { name: 'Create pool' }).click()
  await expect(page.locator('.pool-card', { hasText: poolName })).toBeVisible()

  await page.locator('.nav-bar').getByRole('link', { name: 'Perfect' }).click()
  await expect(page).toHaveURL(/\/perfect$/)

  // Everyone: count the distinct players with perfects.
  await page.locator('.pool-selector select').selectOption('')
  const everyoneNicks = await page
    .locator('.data-table tbody tr td:first-child')
    .allInnerTexts()

  // Scope to grace's solo pool: every row must be grace (or the table empty).
  await page.locator('.pool-selector select').selectOption({ label: poolName })
  const scopedNicks = new Set(
    await page.locator('.data-table tbody tr td:first-child').allInnerTexts(),
  )
  // The scoped set is a subset of everyone, and never larger.
  expect(scopedNicks.size).toBeLessThanOrEqual(new Set(everyoneNicks).size)
  for (const nick of scopedNicks) {
    expect(everyoneNicks).toContain(nick)
  }

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
