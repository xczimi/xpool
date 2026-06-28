import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Head-to-head end to end. Seeds the `balanced` scenario, logs in, clocks past
 * the Final (board materialised + tips revealed), then drives the direct route
 * and the player-centric entry points: the anchored opponent picker on the
 * viewer's own page (`/me`) and on another player's page (`/player/:id`, from
 * that player's POV). demo-ada's nick renders as "ada", demo-alan's as "alan";
 * the route params are the player handles.
 */
const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

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

test('direct route compares two players with an overlaid trajectory', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')
  await page.goto('/h2h/demo-ada/demo-alan')

  await expect(page.locator('.h2h-summary')).toBeVisible()
  // Both players overlaid → two polylines.
  await expect(page.locator('.points-timeline polyline')).toHaveCount(2)
  await expect(page.locator('.h2h-delta-table')).toBeVisible()
  // The per-match section renders either a diff table or the "no differences" note.
  await expect(
    page.locator('.h2h-match-table, .h2h-no-diffs'),
  ).toHaveCount(1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('own page picker compares me with the chosen opponent', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')
  await page.goto('/me')

  const picker = page.locator('.h2h-picker')
  await expect(picker).toBeVisible()
  // Anchored to the viewer (demo-ada); the anchor is excluded from the options.
  await expect(picker.locator('option[value="demo-ada"]')).toHaveCount(0)
  await picker.locator('select').selectOption('demo-alan')

  await expect(page).toHaveURL(/\/h2h\/demo-ada\/demo-alan$/)
  await expect(page.locator('.h2h-summary')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test("another player's page picker compares from that player's POV", async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')
  await page.goto('/player/demo-alan')

  const picker = page.locator('.h2h-picker')
  await expect(picker).toBeVisible()
  // Anchored to the page owner (demo-alan), not the viewer (demo-ada).
  await expect(picker.locator('option[value="demo-alan"]')).toHaveCount(0)
  await picker.locator('select').selectOption('demo-grace')

  await expect(page).toHaveURL(/\/h2h\/demo-alan\/demo-grace$/)
  await expect(page.locator('.h2h-summary')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
