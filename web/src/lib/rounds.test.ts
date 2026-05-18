import { describe, expect, it } from 'vitest'
import type { Round } from '../graphql/types'
import { ROUND_ORDER, STAGE_MULTIPLIERS, roundLabel, roundLabelKey } from './rounds'
import { catalogues } from '../i18n/strings'
import type { StringKey } from '../i18n/strings'

const ALL_ROUNDS: Round[] = [
  'GROUP_STAGE',
  'R32',
  'R16',
  'QF',
  'SF',
  'THIRD_PLACE',
  'FINAL',
]

describe('ROUND_ORDER', () => {
  it('lists every round once, in chronological order', () => {
    expect([...ROUND_ORDER].sort()).toEqual([...ALL_ROUNDS].sort())
    expect(ROUND_ORDER[0]).toBe('GROUP_STAGE')
    expect(ROUND_ORDER[ROUND_ORDER.length - 1]).toBe('FINAL')
  })
})

describe('STAGE_MULTIPLIERS', () => {
  it('weights later rounds at least as heavily as earlier ones', () => {
    expect(STAGE_MULTIPLIERS.GROUP_STAGE).toBe(1)
    expect(STAGE_MULTIPLIERS.FINAL).toBe(6)
  })

  it('has an entry for every round', () => {
    for (const r of ALL_ROUNDS) {
      expect(STAGE_MULTIPLIERS[r]).toBeGreaterThan(0)
    }
  })
})

describe('roundLabelKey', () => {
  it('maps every round to a key present in both locale catalogues', () => {
    for (const r of ALL_ROUNDS) {
      const key = roundLabelKey(r)
      expect(catalogues.en[key]).toBeTruthy()
      expect(catalogues.hu[key]).toBeTruthy()
    }
  })

  it('resolves distinct labels per round', () => {
    const keys = ALL_ROUNDS.map(roundLabelKey)
    expect(new Set(keys).size).toBe(ALL_ROUNDS.length)
  })

  it('translates the group stage to Hungarian', () => {
    expect(catalogues.hu[roundLabelKey('GROUP_STAGE')]).toBe('Csoportkör')
  })
})

describe('roundLabel', () => {
  const enT = (key: StringKey) => catalogues.en[key]
  const huT = (key: StringKey) => catalogues.hu[key]

  it('resolves the English label via the translator', () => {
    expect(roundLabel('FINAL', enT)).toBe('Final')
    expect(roundLabel('R32', enT)).toBe('Round of 32')
  })

  it('resolves the Hungarian label when given the hu translator', () => {
    expect(roundLabel('FINAL', huT)).toBe('Döntő')
    expect(roundLabel('GROUP_STAGE', huT)).toBe('Csoportkör')
  })
})
