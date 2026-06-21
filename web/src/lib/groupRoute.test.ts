import { describe, expect, it } from 'vitest'
import type { GroupGame, Round } from '../graphql/types'
import { resolveGroupParam, roundNodeIdFor } from './groupRoute'

/**
 * The group tree used by the deep-link resolver tests. Mirrors the real
 * shape: round-node groups (childGroupIds) parent the leaf groups
 * (childGameIds). Group-stage leaves are single letters; knockout leaves are
 * `KO-*`.
 */
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

const tree: GroupGame[] = [
  node('ROOT', 'GROUP_STAGE', { childGroupIds: ['GROUPSTAGE', 'KNOCKOUT'] }),
  node('GROUPSTAGE', 'GROUP_STAGE', { childGroupIds: ['A', 'B'] }),
  node('KNOCKOUT', 'R32', { childGroupIds: ['R32', 'FINAL'] }),
  node('R32', 'R32', { childGroupIds: ['KO-M73', 'KO-M74'] }),
  node('FINAL', 'FINAL', { childGroupIds: ['KO-M104'] }),
  node('A', 'GROUP_STAGE', { childGameIds: ['M1'] }),
  node('B', 'GROUP_STAGE', { childGameIds: ['M2'] }),
  node('KO-M73', 'R32', { childGameIds: ['M73'] }),
  node('KO-M74', 'R32', { childGameIds: ['M74'] }),
  node('KO-M104', 'FINAL', { childGameIds: ['M104'] }),
]

describe('resolveGroupParam', () => {
  it('returns null for an undefined nodeId (no deep link → use defaults)', () => {
    expect(resolveGroupParam(tree, undefined)).toBeNull()
  })

  it('returns null for an empty nodeId', () => {
    expect(resolveGroupParam(tree, '')).toBeNull()
  })

  it('resolves a group-stage leaf to its round + group id', () => {
    expect(resolveGroupParam(tree, 'A')).toEqual({
      round: 'GROUP_STAGE',
      groupId: 'A',
    })
  })

  it('resolves a knockout leaf to its round + group id', () => {
    expect(resolveGroupParam(tree, 'KO-M73')).toEqual({
      round: 'R32',
      groupId: 'KO-M73',
    })
    expect(resolveGroupParam(tree, 'KO-M104')).toEqual({
      round: 'FINAL',
      groupId: 'KO-M104',
    })
  })

  it('resolves a round-node id to its round with a null group (round-level select)', () => {
    expect(resolveGroupParam(tree, 'GROUPSTAGE')).toEqual({
      round: 'GROUP_STAGE',
      groupId: null,
    })
    expect(resolveGroupParam(tree, 'R32')).toEqual({
      round: 'R32',
      groupId: null,
    })
    expect(resolveGroupParam(tree, 'FINAL')).toEqual({
      round: 'FINAL',
      groupId: null,
    })
  })

  it('returns null for an unknown nodeId', () => {
    expect(resolveGroupParam(tree, 'NOPE')).toBeNull()
  })

  it('treats a node with both child arrays empty as no match', () => {
    const orphan = [node('EMPTY', 'GROUP_STAGE', {})]
    expect(resolveGroupParam(orphan, 'EMPTY')).toBeNull()
  })

  it('does not mutate the input array', () => {
    const before = tree.map((g) => g.id)
    resolveGroupParam(tree, 'A')
    expect(tree.map((g) => g.id)).toEqual(before)
  })
})

describe('roundNodeIdFor', () => {
  it('returns the round-node id whose round matches', () => {
    expect(roundNodeIdFor(tree, 'GROUP_STAGE')).toBe('GROUPSTAGE')
    expect(roundNodeIdFor(tree, 'R32')).toBe('R32')
    expect(roundNodeIdFor(tree, 'FINAL')).toBe('FINAL')
  })

  it('returns undefined for a round with no round-node group', () => {
    // No SF node in the tree.
    expect(roundNodeIdFor(tree, 'SF')).toBeUndefined()
  })
})
