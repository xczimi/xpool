import { describe, expect, it } from 'vitest'
import type { Team, TeamSlot } from '../graphql/types'
import {
  DISPLAY_MODES,
  resolveDisplayMode,
  teamLabelParts,
} from './displayMode'

function team(over: Partial<Team>): Team {
  return {
    id: 'BRA',
    name: 'Brazil',
    shortCode: 'BRA',
    flag: 'br',
    externalId: null,
    ...over,
  }
}

function slot(over: Partial<TeamSlot>): TeamSlot {
  return { teamId: null, description: '', ...over }
}

const teams = new Map<string, Team>([['BRA', team({})]])
const noFlag = new Map<string, Team>([['BRA', team({ flag: null })]])

describe('DISPLAY_MODES', () => {
  it('lists auto first, then the five explicit modes', () => {
    expect(DISPLAY_MODES).toEqual([
      'auto',
      'flag',
      'code',
      'name',
      'flag-name',
      'flag-code',
    ])
  })
})

describe('resolveDisplayMode', () => {
  it('resolves auto to flag on mobile', () => {
    expect(resolveDisplayMode('auto', true)).toBe('flag')
  })
  it('resolves auto to flag-name on larger screens', () => {
    expect(resolveDisplayMode('auto', false)).toBe('flag-name')
  })
  it('passes explicit modes through unchanged', () => {
    for (const m of ['flag', 'code', 'name', 'flag-name', 'flag-code'] as const) {
      expect(resolveDisplayMode(m, true)).toBe(m)
      expect(resolveDisplayMode(m, false)).toBe(m)
    }
  })
})

describe('teamLabelParts', () => {
  const braSlot = slot({ teamId: 'BRA' })

  it('name mode: full name, no flag', () => {
    expect(teamLabelParts(braSlot, teams, 'name')).toEqual({
      flag: null,
      text: 'Brazil',
    })
  })
  it('code mode: short code, no flag', () => {
    expect(teamLabelParts(braSlot, teams, 'code')).toEqual({
      flag: null,
      text: 'BRA',
    })
  })
  it('flag mode: flag only, no text', () => {
    expect(teamLabelParts(braSlot, teams, 'flag')).toEqual({
      flag: { iso: 'br', name: 'Brazil' },
      text: null,
    })
  })
  it('flag-name mode: flag and name', () => {
    expect(teamLabelParts(braSlot, teams, 'flag-name')).toEqual({
      flag: { iso: 'br', name: 'Brazil' },
      text: 'Brazil',
    })
  })
  it('flag-code mode: flag and code', () => {
    expect(teamLabelParts(braSlot, teams, 'flag-code')).toEqual({
      flag: { iso: 'br', name: 'Brazil' },
      text: 'BRA',
    })
  })
  it('flag mode with no flag asset falls back to the code', () => {
    expect(teamLabelParts(braSlot, noFlag, 'flag')).toEqual({
      flag: null,
      text: 'BRA',
    })
  })
  it('flag-name mode with no flag asset still shows the name', () => {
    expect(teamLabelParts(braSlot, noFlag, 'flag-name')).toEqual({
      flag: null,
      text: 'Brazil',
    })
  })
  it('flag-code mode with no flag asset still shows the code', () => {
    expect(teamLabelParts(braSlot, noFlag, 'flag-code')).toEqual({
      flag: null,
      text: 'BRA',
    })
  })
  it('unresolved slot shows its placeholder description in every mode', () => {
    const ph = slot({ teamId: null, description: '2A' })
    expect(teamLabelParts(ph, teams, 'flag')).toEqual({ flag: null, text: '2A' })
    expect(teamLabelParts(ph, teams, 'name')).toEqual({ flag: null, text: '2A' })
  })
  it('unresolved slot with empty description falls back to TBD', () => {
    const empty = slot({ teamId: null, description: '' })
    expect(teamLabelParts(empty, teams, 'flag')).toEqual({
      flag: null,
      text: 'TBD',
    })
  })
  it('unknown team id falls back to the id text', () => {
    const unknown = slot({ teamId: 'ZZZ' })
    expect(teamLabelParts(unknown, teams, 'flag-name')).toEqual({
      flag: null,
      text: 'ZZZ',
    })
  })
})
