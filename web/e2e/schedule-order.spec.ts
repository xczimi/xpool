import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Schedule order — `/games` lists fixture groups chronologically: the group
 * stage before the knockout rounds, and each group's first kickoff no later
 * than the next group's. This exercises the real fwc26 tournament data over
 * the live GraphQL API.
 */
test('schedule groups render in chronological order', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/games')

  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  const groups = page.locator('.schedule-group')
  const count = await groups.count()
  expect(count, 'the fwc26 tournament has multiple fixture groups').toBeGreaterThan(1)

  // The first kickoff cell of each group, as a sortable timestamp.
  const firstKickoffs: number[] = []
  for (let i = 0; i < count; i++) {
    const firstCell = groups
      .nth(i)
      .locator('tbody tr')
      .first()
      .locator('td')
      .first()
    const text = (await firstCell.textContent())?.trim() ?? ''
    const ts = Date.parse(text)
    expect(Number.isNaN(ts), `group ${i} kickoff "${text}" parses`).toBe(false)
    firstKickoffs.push(ts)
  }

  // Each group's first kickoff is <= the next group's first kickoff.
  for (let i = 1; i < firstKickoffs.length; i++) {
    expect(
      firstKickoffs[i],
      `group ${i} starts on/after group ${i - 1}`,
    ).toBeGreaterThanOrEqual(firstKickoffs[i - 1])
  }

  // The group stage renders before any knockout round.
  const roundTags = await page.locator('.round-tag').allTextContents()
  const firstKnockout = roundTags.findIndex((r) => !/group/i.test(r))
  if (firstKnockout >= 0) {
    const groupStageAfter = roundTags
      .slice(firstKnockout)
      .some((r) => /group/i.test(r))
    expect(
      groupStageAfter,
      'no group-stage group appears after a knockout round',
    ).toBe(false)
  }

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

/**
 * The "Best third-placed teams" section belongs *between* the group stage and
 * the knockout rounds (it explains which thirds advance), not dangling after
 * the FINAL. Walk the page blocks in DOM order and assert the thirds section
 * sits after the last group-stage group and before the first knockout round.
 */
test('best third-placed teams render between group stage and knockouts', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/games')

  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Fixture groups and the thirds section, in document order.
  const blocks = page.locator('.schedule-group, .thirds-section')
  const count = await blocks.count()
  expect(
    count,
    'schedule renders fixture groups plus a thirds section',
  ).toBeGreaterThan(2)

  type Kind = 'group-stage' | 'knockout' | 'thirds'
  const kinds: Kind[] = []
  for (let i = 0; i < count; i++) {
    const block = blocks.nth(i)
    const cls = (await block.getAttribute('class')) ?? ''
    if (cls.includes('thirds-section')) {
      kinds.push('thirds')
      continue
    }
    const tag = (await block.locator('.round-tag').textContent())?.trim() ?? ''
    kinds.push(/group/i.test(tag) ? 'group-stage' : 'knockout')
  }

  const thirdsIdx = kinds.indexOf('thirds')
  expect(thirdsIdx, 'a thirds section is present').toBeGreaterThanOrEqual(0)
  const lastGroupStage = kinds.lastIndexOf('group-stage')
  const firstKnockout = kinds.indexOf('knockout')

  // After the whole group stage…
  expect(
    thirdsIdx,
    'thirds section after the last group-stage group',
  ).toBeGreaterThan(lastGroupStage)
  // …and before the first knockout round.
  if (firstKnockout >= 0) {
    expect(
      thirdsIdx,
      'thirds section before the first knockout round',
    ).toBeLessThan(firstKnockout)
  }

  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
