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
 *
 * STATE ISOLATION — this spec sorts EARLY (position 5), which matters twice:
 *
 *  1. Seeding the `balanced` scenario writes OFFICIAL RESULTS for the whole
 *     tournament and ~5 extra ("whacky") players' predictions for every match —
 *     global, irreversible mutations on the shared e2e table. Specs that assume
 *     the MINIMAL seed (`live-scoring`, `match-page-*`, `mobile-prediction-entry`:
 *     M8 has no result; Group D/G hold only the tips they enter) sort LATER, so
 *     this spec would pollute them. The `afterAll` below RESETS the table to the
 *     minimal seed (drop + import + seed) so it leaves no trace — a full reset is
 *     the only way to remove the whacky players, which `seed` alone cannot delete.
 *  2. It must NOT move late either: `H2HPage` scopes the board to the viewer's
 *     default pool, and pool-creating specs (`pools`, invites) that sort later
 *     would shadow `pool-demo` with a pool missing one of the two players, so
 *     `h2hSummary` would return null ("playerNotInPool"). Running before any
 *     pool-creating spec keeps `pool-demo` (with all demo players) the default.
 *
 * Net: seed early, assert, then reset — self-contained and non-polluting.
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

/** y-coords of the nth <polyline>; inverted y means rising cumulative = last < first. */
async function polylineYs(page: Page, nth: number): Promise<number[]> {
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

function xtableEnv() {
  const table = readFileSync(resolve(repoRoot, 'web/.e2e-table'), 'utf8').trim()
  return {
    ...process.env,
    XPOOL_TABLE: table,
    DYNAMO_ENDPOINT: 'http://localhost:8001',
  }
}

function xtask(...args: string[]) {
  execFileSync('cargo', ['run', '-p', 'xtask', '--', ...args], {
    cwd: repoRoot,
    stdio: 'inherit',
    env: xtableEnv(),
  })
}

test.beforeAll(() => {
  xtask('scenario', 'balanced')
})

// Reset the shared e2e table to the MINIMAL seed so this early spec leaves no
// scenario residue for the later minimal-seed specs (see the header note). A
// full drop + import + seed is required: `seed` overwrites the demo players'
// predictions but cannot DELETE the whacky players the scenario created.
test.afterAll(() => {
  xtask('drop-table')
  xtask('import', 'tournaments/fwc26.json')
  xtask('seed')
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
  // Both players overlaid → two GAME-BY-GAME polylines, and each climbs
  // (last cumulative above the first — the chart is no longer flat).
  await expect(page.locator('.points-timeline polyline')).toHaveCount(2)
  for (const nth of [0, 1]) {
    const ys = await polylineYs(page, nth)
    expect(ys.length).toBeGreaterThan(1)
    expect(ys[ys.length - 1]).toBeLessThan(ys[0])
  }
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
