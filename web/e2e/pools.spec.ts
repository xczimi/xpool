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

test('result user bootstraps a pool, invites a player, then hands it over', async ({
  page,
}) => {
  const net = watchNetwork(page)
  const name = `Bootstrap Cup ${Date.now()}`
  await page.goto('/')

  // The result user (admin / official) creates the pool as a transient owner —
  // it owns but is not a member — and shares its invite.
  await devLogin(page, 'result-user')
  await page.goto('/pools')
  await page.getByPlaceholder('e.g. Office League').fill(name)
  await page.getByRole('button', { name: 'Create pool' }).click()
  const card = page.locator('.pool-card', { hasText: name })
  await expect(card).toBeVisible()
  await expect(card.locator('.owner-tag')).toBeVisible()
  await card.getByRole('button', { name: 'Share invite' }).click()
  const link = await card.locator('.invite-link input').inputValue()
  expect(link).toContain('/invite/')

  // demo-margaret joins via the invite → becomes a member of the pool.
  await devLogin(page, 'demo-margaret')
  await page.goto('/pools')
  await page.getByPlaceholder('paste a link or type a code').fill(link)
  await page.getByRole('button', { name: 'Join', exact: true }).click()
  await expect(page.locator('.pool-card', { hasText: name })).toBeVisible()

  // Back as the result user, hand the pool over to the member.
  await devLogin(page, 'result-user')
  await page.goto('/pools')
  const owned = page.locator('.pool-card', { hasText: name })
  await expect(owned).toBeVisible()
  page.on('dialog', (d) => void d.accept())
  await owned.locator('.handover-select').selectOption('demo-margaret')

  // The result user is now detached: the pool drops off its list.
  await expect(page.locator('.pool-card', { hasText: name })).toHaveCount(0)

  // The new owner sees the pool with owner controls.
  await devLogin(page, 'demo-margaret')
  await page.goto('/pools')
  const handedOver = page.locator('.pool-card', { hasText: name })
  await expect(handedOver).toBeVisible()
  await expect(handedOver.locator('.owner-tag')).toBeVisible()

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
  const joined = page.locator('.pool-card', { hasText: name })
  await expect(joined).toBeVisible()

  // The member list shows nicks (ada, linus), NOT raw player ids (demo-ada…).
  const members = joined.locator('.pool-members')
  await expect(members).toContainText('ada')
  await expect(members).toContainText('linus')
  await expect(members).not.toContainText('demo-')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
