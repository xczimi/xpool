import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Points-timeline chart end to end. Seeds the `balanced` scenario, logs in,
 * clocks past the Final (so every game is scored) and asserts the player page
 * renders a single trajectory <polyline> that RISES game by game — the bug was
 * a flat line (by-round x-axis), so a climbing line is the regression guard.
 */
const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

/**
 * The y-coordinates of an SVG <polyline>'s `points` attribute. The chart plots
 * cumulative points with the y-axis inverted (more points = smaller y), so a
 * rising trajectory means the last y is strictly below the first.
 */
async function polylineYs(page: Page, nth = 0): Promise<number[]> {
  const attr = await page
    .locator('.points-timeline polyline')
    .nth(nth)
    .getAttribute('points')
  return (attr ?? '')
    .trim()
    .split(/\s+/)
    .map((pair) => Number(pair.split(',')[1]))
    .filter((y) => !Number.isNaN(y))
}

type Phase = 'before' | 'during' | 'after'

async function setClock(page: Page, gameIndex: number, phase: Phase) {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption({ index: gameIndex })
  await expect(selects.nth(1)).toBeEnabled()
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  await expect(page.locator('.dev-clock-now')).toBeVisible()
}

async function lastGameIndex(page: Page): Promise<number> {
  const count = await page
    .locator('.dev-clock select')
    .nth(0)
    .locator('option')
    .count()
  return count - 1
}

test.beforeAll(() => {
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

test('player page renders the points trajectory chart', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')

  await page.locator('.auth-player-link').click()
  await expect(page).toHaveURL(/\/me$/)

  await expect(page.locator('.points-timeline svg')).toBeVisible()
  await expect(page.locator('.points-timeline polyline')).toHaveCount(1)

  // The trajectory must CLIMB: many games on the x-axis, last cumulative
  // strictly above the first (inverted y → last y < first y). This is the
  // direct guard against the old flat-line bug.
  const ys = await polylineYs(page)
  expect(ys.length).toBeGreaterThan(1)
  expect(ys[ys.length - 1]).toBeLessThan(ys[0])

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('scoreboard overlays every pool member as a separate line', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')
  await page.goto('/scoreboard')

  // The all-pool overlay: one <polyline> per member of the default pool, so
  // more than one line. (The board itself remains; this chart sits below it.)
  await expect(page.locator('.points-timeline svg')).toBeVisible()
  const lines = page.locator('.points-timeline polyline')
  expect(await lines.count()).toBeGreaterThan(1)
  // A legend appears once there are multiple overlaid players.
  await expect(page.locator('.points-timeline .pt-legend-item').first()).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
