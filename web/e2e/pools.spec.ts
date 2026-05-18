import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Custom pools (SCENARIOS.md §5) end to end: a member sees their pool, a
 * player creates one, and a second player joins it with the join code —
 * exercising the createPool / joinPool / pools GraphQL surface on the wire.
 *
 * The e2e DynamoDB persists across runs, so each test uses a unique pool name
 * and scopes every locator to that named card rather than to `.pool-card`.
 */

test('pools page lists a pool the player belongs to', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await page.getByRole('link', { name: 'Pools' }).click()
  await expect(page).toHaveURL(/\/pools$/)
  await expect(page.locator('h2')).toHaveText('Pools')
  // The seeded "Demo Pool" is visible to its members.
  await expect(
    page.locator('.pool-card', { hasText: 'Demo Pool' }),
  ).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('create a pool and it appears in the list as owner', async ({ page }) => {
  const net = watchNetwork(page)
  const name = `Grace Cup ${Date.now()}`
  await page.goto('/')
  await devLogin(page, 'demo-grace')
  await page.goto('/pools')

  await page.getByPlaceholder('e.g. Office League').fill(name)
  await page.getByRole('button', { name: 'Create pool' }).click()

  const card = page.locator('.pool-card', { hasText: name })
  await expect(card).toBeVisible()
  await expect(card.locator('.owner-tag')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('a player joins a pool with its join code', async ({ page }) => {
  const net = watchNetwork(page)
  const name = `Linus Invite ${Date.now()}`
  await page.goto('/')

  // demo-ada creates a fresh pool; read its generated join code from the card.
  await devLogin(page, 'demo-ada')
  await page.goto('/pools')
  await page.getByPlaceholder('e.g. Office League').fill(name)
  await page.getByRole('button', { name: 'Create pool' }).click()
  const card = page.locator('.pool-card', { hasText: name })
  await expect(card).toBeVisible()
  const code = (await card.locator('.join-code code').textContent())?.trim()
  expect(code).toBeTruthy()

  // demo-linus joins it with that code and then sees the pool.
  await devLogin(page, 'demo-linus')
  await page.goto('/pools')
  await page.getByPlaceholder('paste a join code').fill(code!)
  await page.getByRole('button', { name: 'Join pool' }).click()
  await expect(page.locator('.pool-card', { hasText: name })).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
