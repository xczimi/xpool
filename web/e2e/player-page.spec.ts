import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Player-detail page (#3) end to end. Seeds the `balanced` scenario (full demo
 * roster + official results, everyone in `pool-demo`) and drives the real stack.
 *
 * The materialised scoreboard the page header reads is built lazily by the
 * dev-clock picker's re-materialise (the scenario seeds raw predictions/results
 * but no board), so each test sets the clock through the auth-bar picker — never
 * raw localStorage — exactly like scenario-scoreboard.spec. The picked instant
 * also drives tip-visibility: after the final every match is locked-and-complete
 * → another player's picks are revealed; before the first kickoff neither the
 * mutual-lock nor match-opened condition holds → another's picks stay hidden.
 */
const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

type Phase = 'before' | 'during' | 'after'

/**
 * Drive the auth-bar dev-clock picker: pick a game (by option index, skipping
 * the placeholder at 0) and a phase. Selecting the phase applies the instant,
 * fires `devRematerialize`, and reloads — so we tie it to the navigation. The
 * picked clock persists via localStorage, surviving later `goto`s.
 */
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

/** Index of the last game option (the Final) in the picker. */
async function lastGameIndex(page: Page): Promise<number> {
  const count = await page.locator('.dev-clock select').nth(0).locator('option').count()
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

test('own page shows totals and drill-down with points', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  // Clock past the Final → every match scored; board materialised as-of it.
  // setClock reloads back to '/', where the auth-bar name is now a link.
  await setClock(page, await lastGameIndex(page), 'after')

  // Reach the own page via the auth-bar name link → the clean /me alias.
  await page.locator('.auth-player-link').click()
  await expect(page).toHaveURL(/\/me$/)

  // Header: total + rank stat cards rendered.
  await expect(page.locator('.player-stats')).toBeVisible()
  await expect(page.locator('.player-stat-value').first()).toBeVisible()

  // The "around now" slice renders for the window around the (just-played) Final.
  await expect(page.locator('.player-today')).toBeVisible()
  await expect(page.locator('.player-today .data-table tbody tr').first()).toBeVisible()

  // Rounds start collapsed; expanding the Group Stage row reveals its detail
  // (lazy fetch), with a group sub-nav and scored predictions.
  const firstRow = page.locator('.player-round-row').first()
  await expect(firstRow).toHaveAttribute('aria-expanded', 'false')
  await firstRow.click()
  await expect(page.locator('.player-round-detail')).toBeVisible()
  await expect(page.locator('.group-subnav')).toBeVisible()
  await expect(
    page.locator('.player-round-detail .points-badge').first(),
  ).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('reaches a pool-mate page from a scoreboard link; picks visible after kickoff', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')
  await page.goto('/scoreboard')

  // The scoreboard shows each player's nick; demo-alan's nick is "alan".
  // Clicking it navigates to that player's page (route uses the player id).
  await page.getByRole('link', { name: 'alan', exact: true }).first().click()
  await expect(page).toHaveURL(/\/player\/demo-alan$/)
  await expect(page.locator('.player-header')).toBeVisible()

  // After the tournament every match opened → their group-stage picks are
  // revealed (a real scoreline, not the hidden placeholder).
  await page.locator('.player-round-row').first().click()
  await expect(page.locator('.player-round-detail')).toBeVisible()
  await expect(
    page.locator('.player-round-table tbody tr').first(),
  ).not.toContainText('hidden')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('another player’s un-revealable picks are hidden before kickoff', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  // Clock before the first kickoff → no match opened, nothing mutually locked.
  await setClock(page, 1, 'before')
  await page.goto('/player/demo-alan')

  // demo-alan is a pool-mate, so the page renders; but the tips resolver gates
  // their picks → placeholder cells in the group-stage drill-down.
  await page.locator('.player-round-row').first().click()
  await expect(page.locator('.player-round-detail')).toBeVisible()
  await expect(page.locator('.player-round-table')).toContainText('hidden')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
