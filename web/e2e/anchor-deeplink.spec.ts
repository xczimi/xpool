import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * My Tips hash anchors (`/mytips/<round>#<group.id>`) smooth-scroll the target
 * section into view, without adding a second tab level. The round tab is the
 * only routed level; the hash is client-side scroll.
 *
 * Setup: result-user seeds Group C + F so the R32 round becomes visible
 * (M75/M76 get both teams), then we deep-link to a section LOW in the stacked
 * R32 list (KO-M80) and assert the page scrolled to it.
 */
const CLOCK = '2026-06-26T12:00:00Z'
const TARGET = 'KO-M80'

async function openGroupStageGroup(page: Page, groupName: string): Promise<void> {
  await page.locator('.nav-bar').getByRole('link', { name: 'My Tips' }).click()
  await expect(page).toHaveURL(/\/mytips(\/|$)/)
  await page.locator('.round-tab', { hasText: /^Group Stage$/ }).click()
  await page
    .locator('.group-subnav button', { hasText: new RegExp(`^${groupName}$`) })
    .click()
  await expect(page.locator('.tip-form h3').first()).toContainText(groupName)
}

async function fillAllAndSave(page: Page): Promise<void> {
  const rows = page.locator('.tip-form table.data-table').first().locator('tbody tr')
  const count = await rows.count()
  expect(count, 'group has matches').toBeGreaterThan(0)
  for (let i = 0; i < count; i++) {
    const selects = rows.nth(i).locator('.score-cell select')
    await selects.nth(0).selectOption('2')
    await selects.nth(1).selectOption('1')
  }
  await page.getByRole('button', { name: 'Save draft' }).click()
  await expect(page.locator('.tip-form .flash-bar')).toContainText('Saved')
}

test('My Tips: deep-link scrolls to a knockout sub-section', async ({ page }) => {
  await page.addInitScript((value: string) => {
    localStorage.setItem('xpool.devNow', value)
  }, CLOCK)

  const net = watchNetwork(page)
  await page.goto('/')

  // Make R32 visible by resolving its feeding groups.
  await devLogin(page, 'result-user')
  await openGroupStageGroup(page, 'Group C')
  await fillAllAndSave(page)
  await openGroupStageGroup(page, 'Group F')
  await fillAllAndSave(page)

  // A regular player deep-links straight to a low R32 sub-section.
  await devLogin(page, 'demo-margaret')
  await page.goto(`/mytips/R32#${TARGET}`)

  // The R32 round tab is active and the target section exists.
  await expect(
    page.locator('.round-tab.active', { hasText: /^Round of 32$/ }),
  ).toBeVisible()
  const target = page.locator(`#${TARGET}`)
  await expect(target).toBeVisible()

  // The page scrolled (not still pinned at the top) and the target is in view.
  await expect
    .poll(async () => page.evaluate(() => window.scrollY), { timeout: 10_000 })
    .toBeGreaterThan(0)
  await expect
    .poll(async () =>
      target.evaluate((el) => {
        const r = el.getBoundingClientRect()
        return r.top >= 0 && r.top < window.innerHeight
      }),
    )
    .toBe(true)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('Schedule: "Open this KO match" link carries the #hash and scrolls there', async ({
  page,
}) => {
  await page.addInitScript((value: string) => {
    localStorage.setItem('xpool.devNow', value)
  }, CLOCK)

  const net = watchNetwork(page)
  await page.goto('/')

  // Make R32 visible by resolving its feeding groups (as above).
  await devLogin(page, 'result-user')
  await openGroupStageGroup(page, 'Group C')
  await fillAllAndSave(page)
  await openGroupStageGroup(page, 'Group F')
  await fillAllAndSave(page)

  // A regular player browses the schedule and follows a knockout match's link.
  // Knockout leaf groups (single-game ties) render "Open this KO match"; group
  // stage rows keep "Open this group". The link now carries `#<group.id>`.
  // Target KO-M80 specifically — it sits LOW in the stacked R32 list (the
  // sibling test relies on the same) so the scroll is unambiguous.
  await devLogin(page, 'demo-margaret')
  await page.goto('/games')

  const koLink = page.locator(
    `.open-group-link[href="/mytips/${TARGET}#${TARGET}"]`,
  )
  await expect(koLink).toHaveText('Open this KO match')

  await koLink.click()

  // The hash lands in the URL and the target match section exists.
  await expect(page).toHaveURL(new RegExp(`#${TARGET}$`))
  const target = page.locator(`#${TARGET}`)
  await expect(target).toBeVisible()

  // The page scrolled to bring the match section into view.
  await expect
    .poll(async () => page.evaluate(() => window.scrollY), { timeout: 10_000 })
    .toBeGreaterThan(0)
  await expect
    .poll(async () =>
      target.evaluate((el) => {
        const r = el.getBoundingClientRect()
        return r.top >= 0 && r.top < window.innerHeight
      }),
    )
    .toBe(true)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
