import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Custom pools (SCENARIOS.md §5) end to end: a member sees their pool, a
 * player creates one, and a second player joins it via the inviter's reusable
 * link — exercising the createPool / createInvite / join / pools GraphQL
 * surface on the wire.
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

test('a player joins a pool via the inviter’s link', async ({ page }) => {
  const net = watchNetwork(page)
  const name = `Linus Invite ${Date.now()}`
  await page.goto('/')

  // demo-ada creates a fresh pool, then shares her invite and reads the link.
  await devLogin(page, 'demo-ada')
  await page.goto('/pools')
  await page.getByPlaceholder('e.g. Office League').fill(name)
  await page.getByRole('button', { name: 'Create pool' }).click()
  const card = page.locator('.pool-card', { hasText: name })
  await expect(card).toBeVisible()
  await card.getByRole('button', { name: 'Share invite' }).click()
  const link = await card.locator('.invite-link input').inputValue()
  expect(link).toContain('/invite/')

  // demo-linus pastes the full link into the join box and then sees the pool.
  await devLogin(page, 'demo-linus')
  await page.goto('/pools')
  await page.getByPlaceholder('paste a link or type a code').fill(link)
  await page.getByRole('button', { name: 'Join', exact: true }).click()
  await expect(page.locator('.pool-card', { hasText: name })).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
