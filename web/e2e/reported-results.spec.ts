import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Proves the SportsDB pre-fill wiring end-to-end, hermetically (no live
 * TheSportsDB calls — the e2e stack forces THESPORTSDB_API_KEY="" so the API
 * uses NullSource and returns [] from reportedResults).
 *
 * Two behaviours under test:
 *   1. The result user (admin) opens a group → the SPA auto-issues a
 *      `reportedResults` GraphQL query. The admin gate on the server ACCEPTS it
 *      and returns []. The empty response does NOT break the page.
 *   2. A non-admin (demo-ada) opens the same group → the SPA does NOT issue the
 *      query at all (it is paused for non-result-users).
 */

/** Collect the names of GraphQL operations POSTed to /api/graphql. */
function watchGraphqlOps(page: Page): string[] {
  const ops: string[] = []
  page.on('request', (req) => {
    if (req.method() !== 'POST' || !req.url().includes('/api/graphql')) return
    const body = req.postData() ?? ''
    if (body.includes('reportedResults') || body.includes('ReportedResults')) {
      ops.push('reportedResults')
    }
  })
  return ops
}

/** Open Group A in My Tips — mirrors result-entry.spec.ts. */
async function openGroupA(page: Page) {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await page.locator('.round-tabs button', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: /^Group A$/ }).click()
  await expect(page.locator('.tip-form h3')).toContainText('Group A')
}

test('result-user issues reportedResults query and page survives empty response', async ({ page }) => {
  const net = watchNetwork(page)
  const ops = watchGraphqlOps(page)
  await page.goto('/')

  await devLogin(page, 'result-user')
  await openGroupA(page)

  // The result user's tip form is always editable (deadline-exempt).
  await expect(page.getByRole('button', { name: 'Save draft' })).toBeVisible()

  // The SPA must have auto-issued the reportedResults query.
  await expect.poll(() => ops.length, {
    message: 'expected reportedResults query to be issued for the result user',
    timeout: 5_000,
  }).toBeGreaterThan(0)

  // The admin gate accepted the query (no GraphQL error) and the empty []
  // response didn't crash the page.
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('non-admin (demo-ada) does NOT issue reportedResults query', async ({ page }) => {
  const ops = watchGraphqlOps(page)
  await page.goto('/')

  await devLogin(page, 'demo-ada')
  await openGroupA(page)

  // Give the SPA a beat in case it would lazily fire the query.
  await page.waitForTimeout(500)

  // The query must NOT have been sent — it is paused for non-result-users.
  expect(ops, 'reportedResults must not be issued for a non-admin').toHaveLength(0)

  await expectNoErrorView(page)
})
