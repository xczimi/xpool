import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Match page (#2) — navigation + all-players tip grid.
 *
 * The e2e stack forces THESPORTSDB_API_KEY="" so the API uses NullSource and
 * `match.actual` is always null here. The live/provisional path (score overlay,
 * polling) is covered by the Rust resolver tests, not e2e. What we exercise is:
 *   1. Navigation: Schedule → click a matchup link → URL becomes /match/:id.
 *   2. Grid: `table.tips-grid` renders with at least one player row after a
 *      prediction is entered.
 *   3. Own prediction: the logged-in player's nick and score appear in the grid.
 *   4. Official result / finished state: after the result user enters a result
 *      (post-kickoff), the match page shows the official score, earned points
 *      for the player, and NO provisional marker.
 *
 * Group D (M4 as first game) is used throughout — distinct from the groups
 * touched by other specs:
 *   - result-entry uses Group A (M1)
 *   - mytips uses Group B
 *   - mytips-lock uses Group C
 *   - dev-clock-presets uses Group A (M1)
 * so Group D / M4 is safe for isolated use here.
 */

const TEST_GROUP = 'Group D'
// M4 is Group D's first game — used as the clock preset reference.
const FIRST_GAME = 'M4'
// Pin before the tournament so group-stage deadlines have not passed.
const PRE_TOURNAMENT = '2026-01-01T12:00:00Z'

/** Pick a game + phase in the auth-bar dev clock; it applies and reloads. */
async function setPreset(page: Page, gameId: string, phase: 'before' | 'during' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  await selects.nth(1).selectOption(phase)
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

test('match page: navigate from Schedule and tip grid shows player after prediction is entered', async ({
  page,
}) => {
  // Pin the clock to before the tournament so group-stage tips are editable.
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, PRE_TOURNAMENT)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // ── 1. Enter and save a prediction for Group D so rows appear on match page ─
  await openGroupD(page)
  await fillScores(page, '1', '0')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // ── 2. Navigate to the Schedule and click the first Group D matchup link ────
  await page.locator('.nav-bar').getByRole('link', { name: 'Schedule' }).click()
  await expect(page).toHaveURL(/\/games$/)
  await expect(page.locator('h2')).toHaveText('Schedule')

  const groupDSection = page.locator('.schedule-group').filter({ hasText: TEST_GROUP })
  await expect(groupDSection).toBeVisible()
  const firstLink = groupDSection.locator('tbody tr').first().locator('td a').first()
  await expect(firstLink).toBeVisible()

  const href = await firstLink.getAttribute('href')
  expect(href, 'matchup link has /match/ href').toMatch(/^\/match\//)

  await firstLink.click()

  // The URL must have changed to /match/<gameId>.
  await expect(page).toHaveURL(/\/match\//)

  // ── 3. The tip grid renders with at least one player row ───────────────────
  const grid = page.locator('table.tips-grid')
  await expect(grid).toBeVisible()
  const rows = grid.locator('tbody tr')
  await expect(rows.first()).toBeVisible()
  const rowCount = await rows.count()
  expect(rowCount, 'tips-grid has at least one player row').toBeGreaterThan(0)

  // Each row has .nick, .pred, and .pts cells.
  const firstRow = rows.first()
  await expect(firstRow.locator('.nick')).toBeVisible()
  await expect(firstRow.locator('.pred')).toBeVisible()
  await expect(firstRow.locator('.pts')).toBeVisible()

  // ── 4. demo-grace's nick appears in the grid with the entered prediction ───
  const graceRow = grid.locator('tbody tr').filter({ hasText: 'grace' })
  await expect(graceRow).toBeVisible()
  // Prediction should show "1–0" (the score we entered).
  await expect(graceRow.locator('.pred')).toContainText('1')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('match page: shows official score and no provisional marker after result is entered', async ({
  page,
}) => {
  // NOTE: the live/provisional score path (match.actual.provisional = true) is
  // covered by Rust resolver tests, not e2e — the e2e stack uses NullSource so
  // `actual` is always either official (result-user draft) or null. This test
  // covers the official (finished) state: result user enters a result via My
  // Tips → match page shows the score block with `.score-final` and no
  // `.score-live` / `.provisional-note`.
  //
  // We use the dev-clock UI preset (no localStorage override) so the clock
  // survives all navigations without being reset by addInitScript.

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')

  // ── 1. Enter grace's prediction BEFORE Group D's kickoff ───────────────────
  // `before` M4 → Group D deadline is in the future → editable.
  await setPreset(page, FIRST_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '2', '1')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // ── 2. Result user enters the official result AFTER kickoff ────────────────
  await devLogin(page, 'result-user')
  // `after` M4 → Group D kickoff has passed → result-user can enter results.
  await setPreset(page, FIRST_GAME, 'after')
  await openGroupD(page)
  // Result-user is always editable regardless of deadline.
  await expect(page.getByRole('button', { name: 'Save draft' })).toBeVisible()
  await fillScores(page, '2', '1') // exact match of grace's prediction
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.tip-form .flash-bar')).toContainText('Saved')

  // ── 3. Navigate to the first Group D match page ─────────────────────────────
  // The clock is now after kickoff (M4 `after` preset), so time_open = true.
  await page.goto(`/match/${FIRST_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${FIRST_GAME}`))

  // ── 4. The official score block is visible (no provisional marker) ─────────
  const scoreFinal = page.locator('.score.score-final')
  await expect(scoreFinal).toBeVisible()
  await expect(scoreFinal.locator('.score-value')).toContainText('2')

  // No live/provisional markers — this is a final result.
  await expect(page.locator('.score-live')).toHaveCount(0)
  await expect(page.locator('.provisional-note')).toHaveCount(0)

  // ── 5. The tip grid renders with at least one row ──────────────────────────
  const grid = page.locator('table.tips-grid')
  await expect(grid).toBeVisible()
  const gridRows = grid.locator('tbody tr')
  const rowCount = await gridRows.count()
  expect(rowCount, 'tips-grid has at least one row post-result').toBeGreaterThan(0)

  // ── 6. grace's row shows a PointsBadge (points are scored post-kickoff) ────
  // The viewer is result-user; time_open = true → all tippers' predictions are
  // visible and scored against the official result. grace matched 2–1 exactly.
  const graceRow = grid.locator('tbody tr').filter({ hasText: 'grace' })
  await expect(graceRow).toBeVisible()
  // A PointsBadge renders as .points-badge when points are non-null.
  await expect(graceRow.locator('.pts .points-badge')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
