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

  await page.locator('.nav-bar').getByRole('link', { name: 'Pools' }).click()
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

test('owner renames a pool inline and deletes it via inline confirm (no native popups)', async ({
  page,
}) => {
  const net = watchNetwork(page)
  const name = `Rename Cup ${Date.now()}`
  const renamed = `${name} (v2)`
  await page.goto('/')
  await devLogin(page, 'demo-grace')
  await page.goto('/pools')

  await page.getByPlaceholder('e.g. Office League').fill(name)
  await page.getByRole('button', { name: 'Create pool' }).click()
  const card = page.locator('.pool-card', { hasText: name })
  await expect(card).toBeVisible()

  // Inline rename: the title turns into an editor, save commits. Once editing,
  // the name is an <input value> (not text), so the `hasText` card no longer
  // matches — locate the (single) open editor directly.
  await card.getByRole('button', { name: 'Rename' }).click()
  const renameEditor = page.locator('.pool-rename')
  const editor = renameEditor.locator('input')
  await expect(editor).toBeVisible()
  await editor.fill(renamed)
  await renameEditor.getByRole('button', { name: 'Save' }).click()
  const renamedCard = page.locator('.pool-card', { hasText: renamed })
  await expect(renamedCard).toBeVisible()

  // Inline delete: the Delete button arms a confirm; Confirm removes the card.
  await renamedCard.getByRole('button', { name: 'Delete' }).click()
  await renamedCard.getByRole('button', { name: 'Confirm' }).click()
  await expect(page.locator('.pool-card', { hasText: renamed })).toHaveCount(0)

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
  // Picking a member arms an inline confirm (no native dialog); confirm it.
  await owned.locator('.handover-select').selectOption('demo-margaret')
  await owned.getByRole('button', { name: 'Confirm' }).click()

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

test('share-templates panel offers copyable invite messages with the {LINK} placeholder', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/pools')

  // Collapsed by default; the summary expands it.
  const panel = page.locator('.share-templates')
  await panel.getByText('Ready-to-send invite messages').click()

  // A template body renders with the literal placeholder for the inviter to swap.
  const firstBody = panel.locator('.share-template-body').first()
  await expect(firstBody).toBeVisible()
  await expect(firstBody).toContainText('{LINK}')

  // The Hungarian variant is present regardless of the (English) UI locale —
  // message language follows the recipient, not the toggle.
  await expect(panel.getByText('Hungarian')).toBeVisible()
  // Each template carries a Copy button.
  await expect(
    panel.locator('.share-template').first().getByRole('button', { name: 'Copy' }),
  ).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('an authorized player sees the create-pool form', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')
  await page.goto('/pools')

  await expect(
    page.getByRole('button', { name: 'Create pool' }),
  ).toBeVisible()

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
