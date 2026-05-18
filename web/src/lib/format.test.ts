import { describe, expect, it } from 'vitest'
import type { SingleGame, Team, TeamSlot } from '../graphql/types'
import {
  byKickoff,
  formatKickoff,
  slotCode,
  slotLabel,
  teamIndex,
} from './format'

function team(id: string, name: string, shortCode: string): Team {
  return { id, name, shortCode } as Team
}

function slot(over: Partial<TeamSlot>): TeamSlot {
  return { teamId: null, description: null, ...over } as TeamSlot
}

describe('teamIndex', () => {
  it('indexes teams by id', () => {
    const idx = teamIndex([team('t1', 'Alpha', 'ALP'), team('t2', 'Beta', 'BET')])
    expect(idx.get('t1')?.name).toBe('Alpha')
    expect(idx.get('t2')?.shortCode).toBe('BET')
  })

  it('is empty for no teams', () => {
    expect(teamIndex([]).size).toBe(0)
  })
})

describe('slotLabel', () => {
  const teams = teamIndex([team('t1', 'Alpha', 'ALP')])

  it('returns the team name when the slot has a known team', () => {
    expect(slotLabel(slot({ teamId: 't1' }), teams)).toBe('Alpha')
  })

  it('falls back to the team id when the team is unknown', () => {
    expect(slotLabel(slot({ teamId: 'missing' }), teams)).toBe('missing')
  })

  it('uses the description when no team is assigned', () => {
    expect(slotLabel(slot({ description: 'Winner Group A' }), teams)).toBe(
      'Winner Group A',
    )
  })

  it('falls back to TBD when there is no team and no description', () => {
    expect(slotLabel(slot({}), teams)).toBe('TBD')
  })
})

describe('slotCode', () => {
  const teams = teamIndex([team('t1', 'Alpha', 'ALP')])

  it('returns the short code for a known team', () => {
    expect(slotCode(slot({ teamId: 't1' }), teams)).toBe('ALP')
  })

  it('falls back to the team id for an unknown team', () => {
    expect(slotCode(slot({ teamId: 'missing' }), teams)).toBe('missing')
  })

  it('uses the description when no team is assigned', () => {
    expect(slotCode(slot({ description: 'W1' }), teams)).toBe('W1')
  })

  it('falls back to TBD when nothing is available', () => {
    expect(slotCode(slot({}), teams)).toBe('TBD')
  })
})

describe('formatKickoff', () => {
  it('returns the raw string for an unparseable date', () => {
    expect(formatKickoff('not-a-date', 'en')).toBe('not-a-date')
  })

  it('produces a non-empty locale-formatted string for a valid date', () => {
    const out = formatKickoff('2026-06-20T12:00:00Z', 'en')
    expect(out).not.toBe('2026-06-20T12:00:00Z')
    expect(out.length).toBeGreaterThan(0)
  })
})

describe('byKickoff', () => {
  const game = (id: string, kickoff: string): SingleGame =>
    ({ id, kickoff }) as SingleGame

  it('sorts matches by kickoff ascending', () => {
    const games = [
      game('b', '2026-06-20T15:00:00Z'),
      game('a', '2026-06-20T12:00:00Z'),
      game('c', '2026-06-21T12:00:00Z'),
    ]
    const sorted = [...games].sort(byKickoff)
    expect(sorted.map((g) => g.id)).toEqual(['a', 'b', 'c'])
  })

  it('returns 0 for equal kickoffs', () => {
    expect(
      byKickoff(
        game('a', '2026-06-20T12:00:00Z'),
        game('b', '2026-06-20T12:00:00Z'),
      ),
    ).toBe(0)
  })
})
