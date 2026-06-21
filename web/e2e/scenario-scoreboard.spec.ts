import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * The scenario generator + as-of re-materialise loop, end to end. We seed the
 * `balanced` scenario into the e2e table (full results, ~12 players), then move
 * the dev-clock picker from the first match to the final and assert the
 * scoreboard total grows — proving the board re-materialises as-of the clock
 * from a single seed (the picker fires `devRematerialize`).
 */

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

/** Sum of the visible per-player scoreboard totals (the `<strong>` cells). */
async function scoreboardTotal(page: Page): Promise<number> {
  const totals = page.locator('.data-table tbody strong')
  await expect(totals.first()).toBeVisible()
  // Read every total in ONE atomic snapshot. The scoreboard is a polled,
  // re-materialising board; a per-element `nth(i).innerText()` loop races a
  // re-render (an element detaches between count and read → 30s timeout under
  // full-suite load). `allInnerTexts()` retrieves them in a single call.
  const texts = await totals.allInnerTexts()
  return texts.reduce((sum, text) => {
    const n = Number(text.replace(/[^\d-]/g, ''))
    return Number.isNaN(n) ? sum : sum + n
  }, 0)
}

/**
 * Pick a game + phase in the auth-bar dev clock; it applies, fires
 * devRematerialize, then `location.reload()`s. Crucially we must read totals
 * from the RELOADED board, not the stale pre-reload one — so we wait for the
 * actual navigation the picker triggers, then for the fresh table to render.
 */
async function setClock(page: Page, gameId: string, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  // The phase change applies the instant, awaits devRematerialize, then reloads.
  // Tie the select action to the reload navigation so we don't read the old DOM.
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'load' }),
    selects.nth(1).selectOption(phase),
  ])
  // The applied dev clock renders its `now` text only after the reload landed,
  // and the scoreboard re-fetches as-of that instant.
  await expect(page.locator('.dev-clock-now')).toBeVisible()
  await expect(page.locator('.data-table tbody tr').first()).toBeVisible()
}

test.beforeAll(() => {
  // Seed the `balanced` scenario into the same table the live stack booted
  // (its name is written by scripts/e2e-stack.sh to web/.e2e-table).
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

test('scoreboard re-materialises larger as the dev clock advances', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/scoreboard')

  // Early: clock just after the first match → few matches scored → small board.
  await setClock(page, 'M1', 'after')
  const early = await scoreboardTotal(page)

  // As-of M1 the bracket is unresolved — Group Stage column shows, knockout
  // columns are hidden (no game with both teams known yet).
  await expect(page.locator('.data-table thead')).toContainText('Group Stage')
  await expect(page.locator('.data-table thead')).not.toContainText(
    'Round of 32',
  )

  // Late: clock just after the Final → all matches scored → larger board.
  await setClock(page, 'M104', 'after')
  const late = await scoreboardTotal(page)

  // As-of the final every round is resolved — the knockout columns appear.
  await expect(page.locator('.data-table thead')).toContainText('Round of 32')
  await expect(page.locator('.data-table thead')).toContainText('Final')

  expect(late).toBeGreaterThan(early)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
