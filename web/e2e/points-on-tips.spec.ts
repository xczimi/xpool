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
  // Every row on this page is a perfect: star + a positive points value, and
  // all three component marks scored (transparency — how it was earned).
  await expect(badge).toContainText('★')
  const value = Number(
    (await badge.locator('.pts-value').innerText()).replace(/[^\d]/g, ''),
  )
  expect(value).toBeGreaterThan(0)
  await expect(badge.locator('.pts-marks .pts-mark.on')).toHaveCount(3)

  // Focusing the badge reveals the labelled breakdown tooltip (base × mult).
  await badge.focus()
  const tip = badge.locator('.pts-tip')
  await expect(tip).toBeVisible()
  await expect(tip).toContainText('×')

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
  // matches are played, so visible tips carry an earned-points badge with the
  // three component marks visible inline.
  const badge = page.locator('.points-badge').first()
  await expect(badge).toBeVisible()
  await expect(badge.locator('.pts-marks')).toBeVisible()

  // The per-player Standings column appears (every group carries standings),
  // proving the standings bonus is no longer hidden.
  await expect(page.getByRole('columnheader', { name: 'Standings' })).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('My Tips shows the standings bonus and per-match breakdown', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.addInitScript(
    (v) => localStorage.setItem('xpool.devNow', v),
    AFTER_TOURNAMENT,
  )
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/mytips')
  await expect(page.locator('main.content')).toContainText('My Tips')

  // The group is fully played → a points breakdown badge per match and the
  // standings-bonus line under the standings tables.
  await expect(page.locator('.points-badge').first()).toBeVisible()
  await expect(page.locator('.standings-bonus').first()).toBeVisible()
  await expect(page.locator('.standings-bonus').first()).toContainText('=')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
