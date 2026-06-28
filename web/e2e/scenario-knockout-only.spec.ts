import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Knockout-only scoreboard, end to end. Seed the `balanced` scenario (full
 * results, ~12 players), advance the dev clock past the Final so every round is
 * scored, then assert the Overall ⇄ Knockout-only toggle and the standalone
 * `/scoreboard/knockout` route render a board that drops the Group Stage column
 * and totals strictly less than the overall board (group-stage points excluded
 * — everyone starts the knockouts from zero).
 */

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

/** Sum of the visible per-player scoreboard totals (the `<strong>` cells). */
async function scoreboardTotal(page: Page): Promise<number> {
  const totals = page.locator('.data-table tbody strong')
  await expect(totals.first()).toBeVisible()
  const texts = await totals.allInnerTexts()
  return texts.reduce((sum, text) => {
    const n = Number(text.replace(/[^\d-]/g, ''))
    return Number.isNaN(n) ? sum : sum + n
  }, 0)
}

/**
 * Pick a game + phase in the auth-bar dev clock; it applies, fires
 * devRematerialize, then reloads. Read totals from the RELOADED board.
 */
async function setClock(page: Page, gameId: string, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  await expect(page.locator('.dev-clock-now')).toBeVisible()
  await expect(page.locator('.data-table tbody tr').first()).toBeVisible()
}

test.beforeAll(() => {
  // Seed the `balanced` scenario into the same table the live stack booted
  // (its name is written by the e2e stack script to web/.e2e-table).
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

test('knockout-only board: toggle + route drop the group stage and total less', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/scoreboard')

  // Advance to just after the Final so every round is scored.
  await setClock(page, 'M104', 'after')

  // Overall board: group + knockout columns present.
  await expect(page.locator('.data-table thead')).toContainText('Group Stage')
  await expect(page.locator('.data-table thead')).toContainText('Round of 32')
  const overall = await scoreboardTotal(page)

  // Switch to knockout-only via the toggle.
  await page.locator('.scoreboard-toggle').getByText('Knockout only').click()
  await expect(page).toHaveURL(/\/scoreboard\/knockout$/)
  await expect(page.locator('.data-table tbody tr').first()).toBeVisible()

  // Group Stage column is dropped; knockout rounds remain.
  await expect(page.locator('.data-table thead')).not.toContainText('Group Stage')
  await expect(page.locator('.data-table thead')).toContainText('Round of 32')
  await expect(page.locator('.data-table thead')).toContainText('Final')

  // Knockout-only totals exclude group-stage points → strictly smaller.
  const knockout = await scoreboardTotal(page)
  expect(knockout).toBeGreaterThan(0)
  expect(knockout).toBeLessThan(overall)

  // The route is independently linkable (deep-link, not just via the toggle).
  await page.goto('/scoreboard/knockout')
  await expect(page.locator('.data-table tbody tr').first()).toBeVisible()
  await expect(page.locator('.data-table thead')).not.toContainText('Group Stage')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
