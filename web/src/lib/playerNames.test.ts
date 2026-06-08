import { describe, expect, it } from 'vitest'
import type { PlayerSummary } from '../graphql/types'
import { displayNick, nickIndex } from './playerNames'

const roster: PlayerSummary[] = [
  { id: 'demo-ada', nick: 'ada', fullName: 'Ada Lovelace', isResultUser: false },
  { id: 'result-user', nick: 'official', fullName: 'Official Result', isResultUser: true },
]

describe('nickIndex', () => {
  it('maps player id to nick', () => {
    const index = nickIndex(roster)
    expect(index.get('demo-ada')).toBe('ada')
    expect(index.get('result-user')).toBe('official')
  })
})

describe('displayNick', () => {
  const index = nickIndex(roster)

  it('returns the nick for a known id', () => {
    expect(displayNick(index, 'demo-ada', '(unknown)')).toBe('ada')
  })

  it('returns the fallback for an unknown id', () => {
    expect(displayNick(index, 'f4617bcf-deadbeef', '(unknown)')).toBe('(unknown)')
  })
})
