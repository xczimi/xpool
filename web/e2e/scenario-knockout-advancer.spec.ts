import { test, expect, type Locator, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Knockout draw → the RESULT panel must show who actually ADVANCED on
 * ET/penalties, not the home team. A level knockout tie (1-1) leaves both teams
 * equal on points/GD/goals, so the score-derived order alone falls back to
 * home-first; the official advancer is the result user's `ordering[0]` for the
 * one-match group (the same field `resolve_bracket` reads). Regression for the
 * "Paraguay won on penalties but Germany looks like the winner" report: the
 * RESULT ✓ landed on the home team because the official ordering was never
 * applied to the actual-standings table.
 *
 * NAMING IS load-bearing: the `scenario-` prefix runs this in the late
 * "scenario zone". It seeds Group C + F and writes an official R32 result as the
 * result user — global mutations to the shared e2e table — so it must run AFTER
 * the minimal-seed specs (`mytips-*`, `match-page-*`, …) that assume an
 * untouched M75. It runs before `scenario-knockout-only`, which reseeds the
 * whole tournament. Do not rename this to an early-sorting name.
 */

const CLOCK = '2026-06-26T12:00:00Z' // Group C/F done; R32 open.

async function openGroupStageGroup(page: Page, groupName: string): Promise<void> {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await page.locator('.group-subnav button', { hasText: new RegExp(`^${groupName}$`) }).click()
  await expect(page.locator('.tip-form h3').first()).toContainText(groupName)
}

async function fillAllAndSave(page: Page): Promise<void> {
  const rows = page.locator('.tip-form table.data-table').first().locator('tbody tr')
  const count = await rows.count()
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption('2')
    await selects.nth(1).selectOption('1')
  }
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.tip-form .flash-bar')).toContainText('Saved')
}

async function openR32(page: Page): Promise<void> {
  // A fresh navigation refetches the tournament so the newly-resolved R32 teams
  // (placed by the result user's Group C/F seed → recompute) surface in the
  // round nav. The dev-login token lives in localStorage, so this keeps the
  // current player logged in. Clicking My Tips alone leaves the round nav stale.
  await page.goto('/mytips')
  const r32 = page.locator('.round-tab', { hasText: /^Round of 32$/ })
  await expect(r32).toBeVisible()
  await r32.click()
}

/** R32 stacks all 16 one-match forms; only matches whose groups are seeded have
 *  concrete teams (a score `<select>`). The rest show "Teams not yet
 *  determined". Return the first *placed* form. */
function firstPlacedForm(page: Page): Locator {
  return page
    .locator('.tip-form')
    .filter({ has: page.locator('.score-cell select') })
    .first()
}

/** The team name shown in a standings row (`.team-label-text`, desktop default). */
async function rowTeam(row: Locator): Promise<string> {
  return (await row.locator('.team-label-text').first().innerText()).trim()
}

test('knockout draw: RESULT advancer follows the official ordering, not home', async ({
  page,
}) => {
  await page.addInitScript((value: string) => {
    localStorage.setItem('xpool.devNow', value)
  }, CLOCK)

  const net = watchNetwork(page)
  await page.goto('/')

  // ── result user seeds Group C + F so R32 (M75/M76) gets concrete teams ──────
  await devLogin(page, 'result-user')
  await openGroupStageGroup(page, 'Group C')
  await fillAllAndSave(page)
  await openGroupStageGroup(page, 'Group F')
  await fillAllAndSave(page)

  // ── enter the first *placed* R32 match as a 1-1 draw, advancer = AWAY ───────
  await openR32(page)
  const form = firstPlacedForm(page)
  await expect(form).toBeVisible()
  const formId = await form.getAttribute('id') // e.g. "KO-M75" — pin it across logins
  const matchRow = form.locator('table.data-table').first().locator('tbody tr').first()
  await matchRow.locator('.score-cell select').nth(0).selectOption('1')
  await matchRow.locator('.score-cell select').nth(1).selectOption('1')

  // YOUR PICK editor: on a 1-1 tie the default order is home-first. Capture both
  // teams, then move the away team (row 1) up so it advances on penalties.
  const pick = form.locator('.standings', {
    has: page.locator('h4', { hasText: /^Your pick$/ }),
  })
  const homeName = await rowTeam(pick.locator('tbody tr').nth(0))
  const awayName = await rowTeam(pick.locator('tbody tr').nth(1))
  expect(awayName, 'two distinct knockout teams').not.toBe(homeName)

  await pick.locator('tbody tr').nth(1).getByRole('button', { name: 'Move up' }).click()
  // The away team now leads the YOUR PICK order (advancer).
  await expect(pick.locator('tbody tr').nth(0).locator('.team-label-text')).toHaveText(awayName)

  // A draft save is enough — `results`/`resultStandings` ignore `locked`.
  await form.getByRole('button', { name: 'Save draft' }).click()
  await expect(form.locator('.flash-bar')).toContainText('Saved')

  // ── a regular player sees the same match — exactly the reported scenario ────
  await devLogin(page, 'demo-ada')
  await openR32(page)
  const form2 = page.locator(`.tip-form[id="${formId}"]`)
  await expect(form2).toBeVisible()
  const result = form2.locator('.standings', {
    has: page.locator('h4', { hasText: /^Result$/ }),
  })
  const resultRows = result.locator('tbody tr')

  // The ✓ (row 0) is the away team that advanced — NOT the home team (the bug).
  await expect(resultRows.nth(0).locator('td').nth(0)).toHaveText('✓')
  await expect(resultRows.nth(0).locator('.team-label-text')).toHaveText(awayName)
  await expect(resultRows.nth(1).locator('.team-label-text')).toHaveText(homeName)
  await expect(resultRows.nth(1).locator('td').nth(0)).toHaveText('')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
