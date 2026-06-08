import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Dev-login + auth-gating flows. The dev auth bar is backed by the `players`
 * GraphQL query — exactly the resolver Task B adds an API test for. A schema
 * mismatch there breaks the picker; this test exercises it end to end.
 */

test('dev login picks a seeded player and unlocks player-only nav', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // As a visitor: player-only nav links are absent.
  await expect(page.locator('.auth-bar')).toContainText('You are outside.')
  await expect(page.getByRole('link', { name: 'My Tips' })).toHaveCount(0)
  await expect(page.getByRole('link', { name: 'Profile' })).toHaveCount(0)

  // Pick demo-ada from the auth-bar picker.
  await devLogin(page, 'demo-ada')
  await expect(page.locator('.auth-bar')).toContainText('ada')

  // Player-only nav becomes usable.
  await expect(page.getByRole('link', { name: 'My Tips' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Profile' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'All Tips' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Invite' })).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('profile loads for a logged-in player with no auth error', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await page.getByRole('link', { name: 'Profile' }).click()
  await expect(page).toHaveURL(/\/profile$/)
  await expect(page.locator('h2')).toHaveText('Profile')

  // The profile form is seeded from the `me` query — no auth/error view.
  await expect(page.locator('form.form input').first()).toBeVisible()
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('visiting /profile as a visitor shows the auth-required state, not a crash', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/profile')

  // The NeedsLogin component, not an ErrorView and not a blank page.
  await expect(page.getByText('Login required')).toBeVisible()
  await expect(page.locator('main.content')).toContainText(
    'This screen is for players only',
  )
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('an invalid Bearer token gates player-only routes', async ({ page }) => {
  await page.goto('/')
  // Plant a bad JWT in localStorage where the urql client reads it.
  await page.evaluate(() => localStorage.setItem('xpool.jwt', 'not.a.jwt'))
  await page.reload()

  // Hit a player-only route — the SPA should render the auth-required state,
  // not a crash or a bare error view.
  await page.goto('/profile')
  await expect(page.getByText('Login required')).toBeVisible()
})

test('an invite link establishes a new player with referrer set', async ({ page, context }) => {
  // Inviter (demo-ada) shares her invite into a pool via the InvitePage. She is
  // a member of the seeded Demo Pool, so the pool selector is pre-populated.
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.getByRole('link', { name: 'Invite' }).click()
  await page.getByRole('button', { name: 'Share invite' }).click()
  const linkBox = page.locator('.invite-link textarea')
  await expect(linkBox).toBeVisible()
  const link = await linkBox.inputValue()
  expect(link).toMatch(/\/invite\//)
  const code = link.split('/invite/')[1]
  expect(code.length).toBeGreaterThan(0)

  // A fresh browser context — no shared localStorage with `page`.
  const fresh = await context.browser()!.newContext()
  const newbie = await fresh.newPage()

  // Mint a JWT for an arbitrary unclaimed identity via the extended dev-login.
  const res = await newbie.request.post('/api/dev/login', {
    data: { sub: 'auth0|newbie-e2e', email: 'newbie-e2e@example.com' },
  })
  const { token } = (await res.json()) as { token: string }

  // Plant the token in the fresh context's localStorage and open the link.
  await newbie.goto('/')
  await newbie.evaluate((t) => localStorage.setItem('xpool.jwt', t), token)
  await newbie.goto(`/invite/${code}`)

  // Fill the form (the not-yet-a-Player branch of InviteClaimPage).
  await newbie.getByPlaceholder('Nick').fill('Newbie')
  await newbie.getByPlaceholder('Full name').fill('New B.')
  await newbie.getByRole('button', { name: 'Join' }).click()

  // Lands on /profile.
  await expect(newbie).toHaveURL(/\/profile$/)

  await fresh.close()
})

test('an authenticated-but-uninvited viewer hits the invite dead-end; public pages stay open', async ({
  context,
}) => {
  // A fresh context so we control localStorage from a clean slate.
  const fresh = await context.browser()!.newContext()
  const page = await fresh.newPage()
  const net = watchNetwork(page)

  // Mint a JWT for an identity that has NOT claimed an invite — the API
  // resolves it to an UnclaimedViewer (authenticated, not yet a Player).
  const res = await page.request.post('/api/dev/login', {
    data: { sub: 'auth0|uninvited-e2e', email: 'uninvited-e2e@example.com' },
  })
  const { token } = (await res.json()) as { token: string }

  await page.goto('/')
  // Plant the bearer (urql sends it) AND the dev-session label. The label is a
  // client-side boolean gate ("a session exists") that unpauses Layout's `me`
  // query; its value is irrelevant — the API resolves the viewer from the JWT.
  await page.evaluate((t) => {
    localStorage.setItem('xpool.jwt', t)
    localStorage.setItem('xpool.devPlayer', 'auth0|uninvited-e2e')
  }, token)

  // A player-only route shows the invite dead-end in the content area, not the
  // page and not an error view.
  await page.goto('/mytips')
  await expect(
    page.getByRole('heading', { name: 'You need an invite' }),
  ).toBeVisible()

  // Player-only nav stays hidden — an unclaimed viewer is not a Player.
  await expect(page.getByRole('link', { name: 'My Tips' })).toHaveCount(0)
  await expect(page.getByRole('link', { name: 'Profile' })).toHaveCount(0)
  await expect(page.getByRole('link', { name: 'Invite', exact: true })).toHaveCount(0)

  // Public pages remain reachable — no dead-end on Home or Rules.
  await page.goto('/rules')
  await expect(
    page.getByRole('heading', { name: 'You need an invite' }),
  ).toHaveCount(0)
  await expect(page.getByRole('link', { name: 'Rules' })).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()

  await fresh.close()
})
