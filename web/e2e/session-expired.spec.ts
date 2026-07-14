import { expect, test, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView } from './helpers'

/**
 * The production bug: a player's token stopped being accepted (an expired
 * Auth0 refresh token), but the SPA kept the cached label and rendered a
 * signed-in shell over an anonymous session — so every player page showed a
 * bare "Something went wrong." with no way forward.
 *
 * Reproduced here by keeping the dev label (`xpool.devPlayer`) while swapping
 * the credential (`xpool.jwt`) for a token the real auth seam rejects — a
 * `401` from `crates/api/src/auth/seam.rs` (detector 2 in `sessionState.ts`).
 * That is the whole point of driving this against the real API rather than a
 * mock: it proves the seam really rejects the token and the real SPA really
 * renders `SessionExpired`, not a bare `ErrorView`.
 *
 * These tests deliberately do NOT use `watchNetwork`'s
 * `assertNoGraphqlErrors` — the 401 they provoke IS the point, and that
 * helper treats any non-200 GraphQL response as a failure.
 */

// No `iss` claim, so the seam's issuer-dispatch in `verify_token` falls
// through every trusted issuer and rejects it — a real 401, not a shape the
// server merely can't parse.
const REJECTED_JWT =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJnb25lIn0.this-signature-is-not-valid'

/**
 * Keep `xpool.devPlayer` — that is what makes the SPA still believe it has a
 * session (`label` stays truthy). Only the credential is replaced.
 */
async function poisonTheToken(page: Page): Promise<void> {
  await page.evaluate((jwt) => {
    localStorage.setItem('xpool.jwt', jwt)
  }, REJECTED_JWT)
}

test('a rejected token shows the session-expired view, not a bare error', async ({
  page,
}) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await poisonTheToken(page)
  await page.goto('/mytips')

  await expect(
    page.getByRole('heading', { name: 'Your session has expired' }),
  ).toBeVisible()
  await expect(
    page.getByRole('button', { name: 'Log in again' }),
  ).toBeVisible()

  // The symptom that started this must be gone.
  await expectNoErrorView(page)
})

test('public pages stay reachable with a dead session', async ({ page }) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await poisonTheToken(page)
  await page.goto('/rules')

  await expect(page.getByText('Your session has expired')).toHaveCount(0)
  // RulesPage's own root element — the stable selector for "the real page
  // rendered", not the SessionExpired dead-end.
  await expect(page.locator('section.page')).toBeVisible()
  await expect(
    page.getByRole('heading', { name: 'Rules & Scoring' }),
  ).toBeVisible()
})

test('"Log in again" clears the dead session and offers a fresh login', async ({
  page,
}) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await poisonTheToken(page)
  await page.goto('/mytips')

  await page.getByRole('button', { name: 'Log in again' }).click()

  // In dev-stub mode `reauthenticate` is `dropSession`: it clears the token,
  // clears the sticky flag, and drops the label — so the way back in is the
  // auth-bar player picker reappearing.
  await expect(page.locator('.auth-bar')).toContainText('You are outside.')
  await expect(page.locator('.auth-picker select')).toBeVisible()
  expect(await page.evaluate(() => localStorage.getItem('xpool.jwt'))).toBeNull()
})
