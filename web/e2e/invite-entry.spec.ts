import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * The standalone Invite *share* page was removed (sharing lives on Pools). The
 * bare `/invite` route is repurposed into a **public** recipient-side entry: it
 * renders `NeedsInvite`, so anyone holding a code — even logged out — can paste
 * it and be routed to the claim page (`/invite/:code`). Real invite links
 * (`/invite/:code`) and the claim flow are unchanged.
 * See .scratch/merge-pools-invite-pages/PRD.md.
 */

test('a logged-out visitor can open /invite and paste a bare code to reach the claim page', async ({
  page,
}) => {
  const net = watchNetwork(page)
  // A fresh test context is logged out; /invite must be reachable anyway.
  await page.goto('/invite')

  const box = page.getByPlaceholder('Paste your invite link or code')
  await expect(box).toBeVisible()
  await box.fill('ABC123XYZ0')
  await page.getByRole('button', { name: 'Open' }).click()

  await expect(page).toHaveURL(/\/invite\/ABC123XYZ0$/)
  // On the claim page a logged-out viewer is invited to establish identity.
  await expect(
    page.getByRole('button', { name: 'Continue to join' }),
  ).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('/invite extracts the code from a full pasted link', async ({ page }) => {
  await page.goto('/invite')

  const box = page.getByPlaceholder('Paste your invite link or code')
  await box.fill('https://pool.example.com/invite/SOUTH7K-AD9XK3P7QT')
  await page.getByRole('button', { name: 'Open' }).click()

  await expect(page).toHaveURL(/\/invite\/SOUTH7K-AD9XK3P7QT$/)
})

test('the standalone Invite nav item is gone; sharing lives on Pools', async ({
  page,
}) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  // No "Invite" nav link anywhere (the share page is gone).
  await expect(
    page.getByRole('link', { name: 'Invite', exact: true }),
  ).toHaveCount(0)
  // Sharing lives on Pools — assert the nav link specifically. (The
  // identity-aware Home also renders a "Pools" action link, so scope to nav.)
  await expect(
    page.locator('.nav-bar').getByRole('link', { name: 'Pools' }),
  ).toBeVisible()
})
