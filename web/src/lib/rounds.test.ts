import { describe, expect, it } from 'vitest'
import type { GroupGame, Round, SingleGame } from '../graphql/types'
import {
  ROUND_ORDER,
  STAGE_MULTIPLIERS,
  chronologicalLeafGroups,
  currentRoundNode,
  leafGroupsOfRound,
  readyRounds,
  roundLabel,
  roundLabelKey,
  roundNodes,
  visibleRoundNodes,
} from './rounds'
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

describe('chronologicalLeafGroups', () => {
  const group = (
    id: string,
    deadline: string | null,
    childGameIds: string[],
  ): GroupGame => ({
    id,
    name: id,
    parent: null,
    round: 'GROUP_STAGE',
    lockMode: 'LOCK_TOGETHER',
    carriesStandings: true,
    childGroupIds: [],
    childGameIds,
    deadline,
    deadlinePassed: false,
  })

  it('orders leaf groups by deadline ascending', () => {
    const groups = [
      group('c', '2026-06-15T12:00:00Z', ['g3']),
      group('a', '2026-06-11T12:00:00Z', ['g1']),
      group('b', '2026-06-13T12:00:00Z', ['g2']),
    ]
    expect(chronologicalLeafGroups(groups).map((g) => g.id)).toEqual(['a', 'b', 'c'])
  })

  it('drops non-leaf groups (no child games)', () => {
    const groups = [
      group('leaf', '2026-06-11T12:00:00Z', ['g1']),
      group('internal', '2026-06-10T12:00:00Z', []),
    ]
    expect(chronologicalLeafGroups(groups).map((g) => g.id)).toEqual(['leaf'])
  })

  it('sorts a null-deadline group last', () => {
    const groups = [
      group('tbd', null, ['g2']),
      group('scheduled', '2026-06-11T12:00:00Z', ['g1']),
    ]
    expect(chronologicalLeafGroups(groups).map((g) => g.id)).toEqual(['scheduled', 'tbd'])
  })

  it('does not mutate the input array', () => {
    const groups = [
      group('c', '2026-06-15T12:00:00Z', ['g3']),
      group('a', '2026-06-11T12:00:00Z', ['g1']),
    ]
    const before = groups.map((g) => g.id)
    chronologicalLeafGroups(groups)
    expect(groups.map((g) => g.id)).toEqual(before)
  })
})

describe('roundNodes / leafGroupsOfRound / currentRoundNode', () => {
  const node = (
    id: string,
    round: Round,
    opts: {
      childGroupIds?: string[]
      childGameIds?: string[]
      deadline?: string | null
      deadlinePassed?: boolean
    },
  ): GroupGame => ({
    id,
    name: id,
    parent: null,
    round,
    lockMode: 'LOCK_TOGETHER',
    carriesStandings: false,
    childGroupIds: opts.childGroupIds ?? [],
    childGameIds: opts.childGameIds ?? [],
    deadline: opts.deadline ?? null,
    deadlinePassed: opts.deadlinePassed ?? false,
  })

  // ROOT -> GROUPSTAGE -> {A,B}; ROOT -> KNOCKOUT -> {R32 -> {M1,M2}, FINAL -> {M3}}
  const tree: GroupGame[] = [
    node('ROOT', 'GROUP_STAGE', { childGroupIds: ['GROUPSTAGE', 'KNOCKOUT'] }),
    node('GROUPSTAGE', 'GROUP_STAGE', { childGroupIds: ['A', 'B'] }),
    node('KNOCKOUT', 'R32', { childGroupIds: ['R32', 'FINAL'] }),
    node('R32', 'R32', { childGroupIds: ['M1', 'M2'] }),
    node('FINAL', 'FINAL', { childGroupIds: ['M3'] }),
    node('A', 'GROUP_STAGE', { childGameIds: ['g1'], deadline: '2026-06-11T12:00:00Z' }),
    node('B', 'GROUP_STAGE', { childGameIds: ['g2'], deadline: '2026-06-12T12:00:00Z' }),
    node('M1', 'R32', { childGameIds: ['g3'], deadline: '2026-07-01T12:00:00Z' }),
    node('M2', 'R32', { childGameIds: ['g4'], deadline: '2026-06-30T12:00:00Z' }),
    node('M3', 'FINAL', { childGameIds: ['g5'], deadline: '2026-07-19T12:00:00Z' }),
  ]

  it('returns exactly the round nodes, ordered, excluding root and container', () => {
    expect(roundNodes(tree).map((g) => g.id)).toEqual(['GROUPSTAGE', 'R32', 'FINAL'])
  })

  it('lists a round node’s leaf groups chronologically', () => {
    const r32 = roundNodes(tree).find((g) => g.id === 'R32')!
    // M2 (Jun 30) before M1 (Jul 1)
    expect(leafGroupsOfRound(r32, tree).map((g) => g.id)).toEqual(['M2', 'M1'])
  })

  it('currentRoundNode picks the first round whose deadline has not passed', () => {
    const rounds = [
      node('GROUPSTAGE', 'GROUP_STAGE', { childGroupIds: ['A'], deadlinePassed: true }),
      node('R32', 'R32', { childGroupIds: ['M1'], deadlinePassed: false }),
      node('FINAL', 'FINAL', { childGroupIds: ['M3'], deadlinePassed: false }),
    ]
    expect(currentRoundNode(rounds)?.id).toBe('R32')
  })

  it('currentRoundNode falls back to the last round when all have passed', () => {
    const rounds = [
      node('GROUPSTAGE', 'GROUP_STAGE', { childGroupIds: ['A'], deadlinePassed: true }),
      node('FINAL', 'FINAL', { childGroupIds: ['M3'], deadlinePassed: true }),
    ]
    expect(currentRoundNode(rounds)?.id).toBe('FINAL')
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

describe('readyRounds / visibleRoundNodes', () => {
  const node = (
    id: string,
    round: Round,
    opts: { childGroupIds?: string[]; childGameIds?: string[] },
  ): GroupGame => ({
    id,
    name: id,
    parent: null,
    round,
    lockMode: 'LOCK_TOGETHER',
    carriesStandings: false,
    childGroupIds: opts.childGroupIds ?? [],
    childGameIds: opts.childGameIds ?? [],
    deadline: null,
    deadlinePassed: false,
  })

  // ROOT -> GROUPSTAGE -> {A,B}; ROOT -> KNOCKOUT -> {R32 -> {M1,M2}, FINAL -> {M3}}
  const tree: GroupGame[] = [
    node('ROOT', 'GROUP_STAGE', { childGroupIds: ['GROUPSTAGE', 'KNOCKOUT'] }),
    node('GROUPSTAGE', 'GROUP_STAGE', { childGroupIds: ['A', 'B'] }),
    node('KNOCKOUT', 'R32', { childGroupIds: ['R32', 'FINAL'] }),
    node('R32', 'R32', { childGroupIds: ['M1', 'M2'] }),
    node('FINAL', 'FINAL', { childGroupIds: ['M3'] }),
    node('A', 'GROUP_STAGE', { childGameIds: ['g1'] }),
    node('B', 'GROUP_STAGE', { childGameIds: ['g2'] }),
    node('M1', 'R32', { childGameIds: ['g3'] }),
    node('M2', 'R32', { childGameIds: ['g4'] }),
    node('M3', 'FINAL', { childGameIds: ['g5'] }),
  ]

  const game = (
    id: string,
    groupId: string,
    homeTeam: string | null,
    awayTeam: string | null,
  ): SingleGame => ({
    id,
    kickoff: '2026-06-11T12:00:00Z',
    venue: null,
    groupId,
    home: { teamId: homeTeam, description: homeTeam ?? 'TBD' },
    away: { teamId: awayTeam, description: awayTeam ?? 'TBD' },
    resultPending: false,
    withinTodayWindow: false,
    isToday: false,
  })

  // Group games always carry real teams; knockout slots start unresolved.
  const groupOnly: SingleGame[] = [
    game('g1', 'A', 'ARG', 'BRA'),
    game('g2', 'B', 'FRA', 'GER'),
    game('g3', 'M1', null, null),
    game('g4', 'M2', null, null),
    game('g5', 'M3', null, null),
  ]

  it('readyRounds is just GROUP_STAGE when every knockout slot is a placeholder', () => {
    expect(readyRounds(tree, groupOnly)).toEqual(new Set(['GROUP_STAGE']))
  })

  it('readyRounds includes a round once one of its games has BOTH teams', () => {
    const withR32 = groupOnly.map((g) =>
      g.id === 'g3' ? game('g3', 'M1', 'ARG', 'FRA') : g,
    )
    const ready = readyRounds(tree, withR32)
    expect(ready.has('R32')).toBe(true)
    // FINAL still has only a placeholder game -> excluded.
    expect(ready.has('FINAL')).toBe(false)
  })

  it('readyRounds excludes a round when only ONE slot of its game is known', () => {
    const halfR32 = groupOnly.map((g) =>
      g.id === 'g3' ? game('g3', 'M1', 'ARG', null) : g,
    )
    expect(readyRounds(tree, halfR32).has('R32')).toBe(false)
  })

  it('visibleRoundNodes drops not-yet-ready round nodes', () => {
    expect(visibleRoundNodes(tree, groupOnly).map((n) => n.id)).toEqual([
      'GROUPSTAGE',
    ])
  })

  it('visibleRoundNodes reveals a round once it is ready, keeping ROUND_ORDER', () => {
    const withR32 = groupOnly.map((g) =>
      g.id === 'g3' ? game('g3', 'M1', 'ARG', 'FRA') : g,
    )
    expect(visibleRoundNodes(tree, withR32).map((n) => n.id)).toEqual([
      'GROUPSTAGE',
      'R32',
    ])
  })
})
