import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Per-prediction earned points, end to end (prediction-points-on-tips). We seed
 * the `balanced` scenario (full results, ~12 players) and assert the server
 * computes points via the real scoring engine and the SPA renders them:
 *  - /perfect (public): every perfect prediction shows its round-multiplied
 *    points with the star marker.
 *  - /alltips (logged in, clock past the Final): visible tips carry a points
 *    badge once their match is played.
 * A schema mismatch on the new `points` / `isPerfect` fields would surface as a
 * GraphQL `errors` array (watchNetwork) — the same class of bug the suite guards.
 */

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

/** A clock past the Final so every match is played and visible. */
const AFTER_TOURNAMENT = '2026-07-20T00:00:00Z'

test.beforeAll(() => {
  // Seed the `balanced` scenario into the table the live stack booted (its name
  // is written by scripts/e2e-stack.sh to web/.e2e-table).
  const table = readFileSync(resolve(repoRoot, 'web/.e2e-table'), 'utf8').trim()
  execFileSync('cargo', ['run', '-p', 'xtask', '--', 'scenario', 'balanced'], {
    cwd: repoRoot,
    stdio: 'inherit',
    env: {
      ...process.env,
      XPOOL_TABLE: table,
      DYNAMO_ENDPOINT: 'http://localhost:8001',
    },
  })
})

test('Perfect Predictions shows each perfect tip its earned points', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/perfect')
  await expect(page.locator('main.content')).toContainText('Perfect Predictions')

  // The balanced scenario has perfects, so the new Points column is populated.
  const badge = page.locator('.points-badge').first()
  await expect(badge).toBeVisible()
  // Every row on this page is a perfect, so the badge carries the star + a
  // positive points value.
  await expect(badge).toContainText('★')
  const value = Number((await badge.innerText()).replace(/[^\d]/g, ''))
  expect(value).toBeGreaterThan(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('All Tips shows earned points once a match is played', async ({ page }) => {
  const net = watchNetwork(page)
  // Pin the clock past the Final so every group-stage match is played → visible
  // tips are scored. localStorage is read before the app loads on every nav.
  await page.addInitScript(
    (v) => localStorage.setItem('xpool.devNow', v),
    AFTER_TOURNAMENT,
  )
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/alltips')
  await expect(page.locator('main.content')).toContainText('All Tips')

  // Once all deadlines pass the page falls back to the Group Stage round; its
  // matches are played, so visible tips carry an earned-points badge.
  await expect(page.locator('.points-badge').first()).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
