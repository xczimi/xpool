import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Points-timeline chart end to end. Seeds the `balanced` scenario, logs in,
 * clocks past the Final (so the board is materialised) and asserts the player
 * page renders a single trajectory <polyline>.
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

test('player page renders the points trajectory chart', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await setClock(page, await lastGameIndex(page), 'after')

  await page.locator('.auth-player-link').click()
  await expect(page).toHaveURL(/\/me$/)

  await expect(page.locator('.points-timeline svg')).toBeVisible()
  await expect(page.locator('.points-timeline polyline')).toHaveCount(1)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
