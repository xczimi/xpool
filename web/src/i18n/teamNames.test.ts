import { describe, expect, it } from 'vitest'
import type { Team } from '../graphql/types'
import { teamDisplayName } from './teamNames'

function team(partial: Partial<Team>): Team {
  return {
    id: 'X',
    name: 'English Name',
    shortCode: 'XXX',
    flag: null,
    ...partial,
  } as Team
}

describe('teamDisplayName', () => {
  it('returns the Hungarian name for a known team in hu', () => {
    const cro = team({ shortCode: 'CRO', name: 'Croatia' })
    expect(teamDisplayName(cro, 'hu')).toBe('Horvátország')
  })

  it('falls back to the English name in en (no en catalogue)', () => {
    const cro = team({ shortCode: 'CRO', name: 'Croatia' })
    expect(teamDisplayName(cro, 'en')).toBe('Croatia')
  })

  it('falls back to the English name for a team absent from the catalogue', () => {
    const unknown = team({ shortCode: 'ZZZ', name: 'Atlantis' })
    expect(teamDisplayName(unknown, 'hu')).toBe('Atlantis')
  })
})
