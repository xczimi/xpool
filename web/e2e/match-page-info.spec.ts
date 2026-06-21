import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Match page (#2) — venue render + prediction-stats visibility gate.
 *
 *   1. Venue: the match card shows a `.match-card-venue` line (the fixture
 *      carries a "Stadium, City" venue for every game — see schedule-venue.spec).
 *   2. Stats hidden BEFORE the gate: with the clock before the group deadline,
 *      another player's tip is still hidden, so `.prediction-stats` is absent and
 *      the `.prediction-stats-hidden` note shows instead.
 *   3. Stats shown AFTER the gate: once kickoff passes, others' tips are revealed
 *      and `.prediction-stats` renders with the most-common scoreline.
 *
 * Uses Group D / M4 for isolation (same convention as match-page.spec.ts). Two
 * tippers (demo-ada + demo-grace) are seeded so the viewer is not the only one
 * with a visible prediction — the gate signal is a NON-own visible tip.
 */

const TEST_GROUP = 'Group D'
const FIRST_GAME = 'M4'

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

async function openGroupD(page: Page) {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips$/)
  await page.locator('.round-tabs button', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: new RegExp(`^${TEST_GROUP}$`) }).click()
  await expect(page.locator('.tip-form h3')).toContainText(TEST_GROUP)
}

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

test('match page: venue renders and prediction stats are gated by visibility', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')

  // ── ada tips BEFORE kickoff so there IS another player's tip to reveal ──────
  await devLogin(page, 'demo-ada')
  await setPreset(page, FIRST_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '2', '1')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // ── grace tips BEFORE kickoff; grace is the viewer for the assertions ───────
  await devLogin(page, 'demo-grace')
  await setPreset(page, FIRST_GAME, 'before')
  await openGroupD(page)
  await fillScores(page, '2', '1')
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.flash-bar')).toContainText('Saved')

  // ── Match page BEFORE the gate: venue shows; stats are hidden ───────────────
  await page.goto(`/match/${FIRST_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${FIRST_GAME}`))

  // Venue line renders with the fixture's stadium text.
  const venue = page.locator('.match-card-venue')
  await expect(venue).toBeVisible()
  await expect(venue).toContainText('Venue:')

  // Gate closed (before kickoff, grace not committed to seeing ada's tip):
  // the stats panel is absent and the "hidden until kickoff" note shows.
  await expect(page.locator('.prediction-stats')).toHaveCount(0)
  await expect(page.locator('.prediction-stats-hidden')).toBeVisible()

  // ── Move the clock AFTER kickoff: others' tips reveal, stats appear ─────────
  // Use the dev clock on the match page itself (the auth bar persists).
  await setPreset(page, FIRST_GAME, 'after')
  await page.goto(`/match/${FIRST_GAME}`)

  // Gate open: the stats panel renders with the most-common scoreline (2–1,
  // both ada and grace tipped it) and the hidden note is gone.
  const stats = page.locator('.prediction-stats')
  await expect(stats).toBeVisible()
  await expect(stats).toContainText('What everyone predicted')
  await expect(stats.locator('.stats-scoreline').first()).toContainText('2')
  await expect(page.locator('.prediction-stats-hidden')).toHaveCount(0)

  // Venue still renders after the gate opens.
  await expect(page.locator('.match-card-venue')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
