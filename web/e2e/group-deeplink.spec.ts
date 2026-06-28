import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Deep-linkable My Tips groups (`/mytips/:groupId`) + the "Open this group"
 * links from match contexts.
 *
 * Two things are proven over the live stack:
 *   (a) Visiting `/mytips/E` directly (logged in as a player) lands on Group E's
 *       tip form — the URL param, not local state, drives the open group.
 *   (b) The Match page "Open this group" link navigates to `/mytips/<groupId>`
 *       and the right group's form is shown.
 *
 * Group E / M9 is used throughout — distinct from the groups other specs touch
 * (A result-entry, B mytips, C mytips-lock, D match-page), so this spec is
 * order-independent. It only reads the form (no save/lock), so it mutates no
 * shared backend state. Pinned before the tournament so Group E is editable.
 */

const TEST_GROUP = 'Group E'
const TEST_GROUP_ID = 'E'
// M9 is Group E's first game — used as a match-context entry point.
const FIRST_GAME = 'M9'
// Before the tournament so group-stage deadlines have not passed (editable).
const PRE_TOURNAMENT = '2026-01-01T12:00:00Z'

test('My Tips: visiting /mytips/E lands directly on Group E', async ({ page }) => {
  // Pin the clock so group-stage tips are editable (display only — the deep
  // link works regardless, but an editable form is the realistic case).
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, PRE_TOURNAMENT)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-linus')

  // Deep-link straight to Group E — no clicking through the default group.
  await page.goto(`/mytips/${TEST_GROUP_ID}`)
  await expect(page).toHaveURL(new RegExp(`/mytips/${TEST_GROUP_ID}$`))

  // The Group Stage round tab is active and the open group's form is Group E.
  await expect(
    page.locator('.round-tab.active', { hasText: /^Group Stage$/ }),
  ).toBeVisible()
  await expect(page.locator('.tip-form h3')).toContainText(TEST_GROUP)
  // The matching group pill is selected (the URL drove the sub-nav).
  await expect(
    page.locator('.group-subnav button.active', {
      hasText: new RegExp(`^${TEST_GROUP}$`),
    }),
  ).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('Match page: "Open this group" jumps to that match\'s group on My Tips', async ({
  page,
}) => {
  await page.addInitScript((value) => {
    localStorage.setItem('xpool.devNow', value)
  }, PRE_TOURNAMENT)

  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-linus')

  // Open the match page for M9 (Group E's first game).
  await page.goto(`/match/${FIRST_GAME}`)
  await expect(page).toHaveURL(new RegExp(`/match/${FIRST_GAME}$`))

  // The "Open this group" link points at the game's leaf group (E).
  const openLink = page.locator('.match-card-open-group a', {
    hasText: 'Open this group',
  })
  await expect(openLink).toBeVisible()
  // The link carries the leaf group as both the path and the #anchor, so the
  // target match scrolls into view (essential for stacked knockout rounds,
  // harmless for the group stage).
  await expect(openLink).toHaveAttribute(
    'href',
    `/mytips/${TEST_GROUP_ID}#${TEST_GROUP_ID}`,
  )

  // Clicking it navigates to /mytips/E#E and shows Group E's form.
  await openLink.click()
  await expect(page).toHaveURL(
    new RegExp(`/mytips/${TEST_GROUP_ID}#${TEST_GROUP_ID}$`),
  )
  await expect(page.locator('.tip-form h3')).toContainText(TEST_GROUP)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
