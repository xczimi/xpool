import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Live-scoring cluster e2e. Proves, hermetically (no real SportsDB — the stack
 * injects XPOOL_LIVE_SCORES="E2E1=1:0:2H" and M8 carries externalId E2E1):
 *   A. Match page during the live window shows the live (provisional) score,
 *      a working "Refresh now" button (re-issues the match query on the wire),
 *      and a last-updated indicator.
 *   B. Scoreboard shows the "Max" (still-reachable) column while M8 is live,
 *      with grace's ceiling reflecting her 1–0 tip vs the 1–0 live score.
 *
 * M8 is the live game (Group D's second game — distinct from M4, which
 * match-page.spec.ts uses for its official-result assertions, so the live stub
 * on M8 never perturbs that spec). Group D is LockTogether, so tips are entered
 * in M4's `before` window (the group is still editable then); the live preset
 * moves the clock into M8's own kickoff window.
 */

const TEST_GROUP = 'Group D'
// Group D's first game — the group's deadline reference; `before` here keeps
// the whole LockTogether group editable so grace can save her tips.
const ENTRY_GAME = 'M4'
// The live game carrying externalId E2E1 (the stub's target).
const LIVE_GAME = 'M8'

/**
 * Pick a game + phase in the auth-bar dev clock. Selecting the phase applies the
 * clock and triggers a full page reload; tag the document first and wait for the
 * tag to disappear so the reload is complete before the caller interacts.
 */
async function setPreset(page: Page, gameId: string, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  await page.evaluate(() =>
    document.documentElement.setAttribute('data-pre-reload', '1'),
  )
  await selects.nth(1).selectOption(phase)
  await page.waitForFunction(
    () => !document.documentElement.hasAttribute('data-pre-reload'),
  )
}

/** Open TEST_GROUP in the My Tips sub-navigation. */
async function openGroupD(page: Page) {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await page.locator('.round-tabs button', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: new RegExp(`^${TEST_GROUP}$`) }).click()
  await expect(page.locator('.tip-form h3')).toContainText(TEST_GROUP)
}

/** Fill every match-prediction <select> in the tip form with a score. */
async function fillScores(page: Page, home: string, away: string) {
  const rows = page.locator('.tip-form table.data-table').first().locator('tbody tr')
  const count = await rows.count()
  expect(count, 'the group has matches').toBeGreaterThan(0)
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption(home)
    await selects.nth(1).selectOption(away)
  }
}

/** Count match-query POSTs to /api/graphql. */
function watchMatchOps(page: Page): { count: () => number } {
  let n = 0
  page.on('request', (req) => {
    if (req.method() !== 'POST' || !req.url().includes('/api/graphql')) return
    const body = req.postData() ?? ''
    if (body.includes('query Match') || /\bmatch\(gameId/.test(body)) n++
  })
  return { count: () => n }
}

test('live match page: provisional score, refresh re-issues query, last-updated shows', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // Enter grace's 1–0 tips for Group D while the group is editable (M4 before).
  await setPreset(page, ENTRY_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '1', '0')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // Move the clock into M8's live window.
  await setPreset(page, LIVE_GAME, 'during')

  // Go to the match page; the stub live score 1–0 must render as provisional.
  const matchOps = watchMatchOps(page)
  await page.goto(`/match/${LIVE_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${LIVE_GAME}`))

  const liveScore = page.locator('.match-scoreline.is-live')
  await expect(liveScore).toBeVisible()
  await expect(liveScore.locator('.match-scoreline-value')).toContainText('1')
  await expect(page.locator('.match-provisional')).toBeVisible()

  // The refresh button is present and re-issues the match query on click.
  const before = matchOps.count()
  const refresh = page.getByRole('button', { name: 'Refresh now' })
  await expect(refresh).toBeVisible()
  await refresh.click()
  await expect
    .poll(() => matchOps.count(), {
      message: 'clicking Refresh now must re-issue the match query',
      timeout: 5_000,
    })
    .toBeGreaterThan(before)

  // Last-updated indicator renders.
  await expect(page.locator('.last-updated')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('scoreboard: Max column appears with grace ceiling while M8 is live', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // Ensure grace has a 1–0 tip for Group D (re-entering is idempotent), entered
  // while the LockTogether group is still editable (M4 before).
  await setPreset(page, ENTRY_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '1', '0')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // Move into M8's live window so the server sees a live match.
  await setPreset(page, LIVE_GAME, 'during')

  await page.locator('.nav-bar').getByRole('link', { name: 'Scoreboard' }).click()
  await expect(page).toHaveURL(/\/scoreboard$/)

  // The "Max" column appears only while something is live.
  const ceiling = page.locator('.score-ceiling').first()
  await expect(ceiling).toBeVisible()

  // grace's row shows a ceiling of ≤ 4 (1–0 vs live 1–0 → base 4, group ×1).
  const graceRow = page.locator('table.data-table tbody tr').filter({ hasText: 'grace' })
  await expect(graceRow).toBeVisible()
  await expect(graceRow.locator('.score-ceiling')).toContainText('4')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
