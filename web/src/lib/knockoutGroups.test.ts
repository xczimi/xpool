import { describe, it, expect } from 'vitest'
import type { GroupGame, SingleGame } from '../graphql/types'
import { knockoutGroupIds } from './knockoutGroups'

const game = (id: string, groupId: string): SingleGame =>
  ({ id, groupId } as SingleGame)

const group = (id: string, childGameIds: string[]): GroupGame =>
  ({ id, childGameIds } as GroupGame)

describe('knockoutGroupIds', () => {
  it('flags single-game leaf groups as knockout', () => {
    const groups = [group('KO-M73', ['M73'])]
    const games = [game('M73', 'KO-M73')]
    expect(knockoutGroupIds(groups, games)).toEqual(new Set(['KO-M73']))
  })

  it('does not flag a multi-game group-stage leaf', () => {
    const groups = [group('A', ['M1', 'M2', 'M3'])]
    const games = [
      game('M1', 'A'),
      game('M2', 'A'),
      game('M3', 'A'),
    ]
    expect(knockoutGroupIds(groups, games)).toEqual(new Set())
  })

  it('does not flag internal/parent groups that hold no games', () => {
    const groups = [group('R32', [])]
    expect(knockoutGroupIds(groups, [])).toEqual(new Set())
  })

  it('selects only the knockout leaves from a mixed tournament', () => {
    const groups = [
      group('A', ['M1', 'M2']),
      group('R32', []),
      group('KO-M73', ['M73']),
      group('KO-M104', ['M104']),
    ]
    const games = [
      game('M1', 'A'),
      game('M2', 'A'),
      game('M73', 'KO-M73'),
      game('M104', 'KO-M104'),
    ]
    expect(knockoutGroupIds(groups, games)).toEqual(
      new Set(['KO-M73', 'KO-M104']),
    )
  })
})
