import { describe, expect, it } from 'vitest'
import type { Team, TeamSlot } from '../graphql/types'
import {
  axesFromLegacy,
  composeDisplayMode,
  FLAG_MODES,
  TEXT_MODES,
  teamLabelParts,
  type ConcreteDisplayMode,
  type FlagMode,
  type TextMode,
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

describe('FLAG_MODES / TEXT_MODES', () => {
  it('lists the flag segments on then off', () => {
    expect(FLAG_MODES).toEqual(['on', 'off'])
  })
  it('lists the text segments auto, name, code, off', () => {
    expect(TEXT_MODES).toEqual(['auto', 'name', 'code', 'off'])
  })
})

describe('composeDisplayMode', () => {
  // Full (flag x text) x {desktop, mobile} mapping table. `auto` is the only
  // axis value that depends on the viewport, and it is flag-aware so the label
  // is never empty.
  const cases: Array<{
    flag: FlagMode
    text: TextMode
    desktop: ConcreteDisplayMode
    mobile: ConcreteDisplayMode
  }> = [
    { flag: 'on', text: 'auto', desktop: 'flag-name', mobile: 'flag' },
    { flag: 'on', text: 'name', desktop: 'flag-name', mobile: 'flag-name' },
    { flag: 'on', text: 'code', desktop: 'flag-code', mobile: 'flag-code' },
    { flag: 'on', text: 'off', desktop: 'flag', mobile: 'flag' },
    { flag: 'off', text: 'auto', desktop: 'name', mobile: 'code' },
    { flag: 'off', text: 'name', desktop: 'name', mobile: 'name' },
    { flag: 'off', text: 'code', desktop: 'code', mobile: 'code' },
    // off+off is guarded in the UI; compose stays total and never empty.
    { flag: 'off', text: 'off', desktop: 'code', mobile: 'code' },
  ]

  for (const c of cases) {
    it(`flag=${c.flag} text=${c.text} → ${c.desktop} (desktop) / ${c.mobile} (mobile)`, () => {
      expect(composeDisplayMode(c.flag, c.text, false)).toBe(c.desktop)
      expect(composeDisplayMode(c.flag, c.text, true)).toBe(c.mobile)
    })
  }

  it('default (on, auto) reproduces today’s `auto`: flag-name desktop, flag-only mobile', () => {
    expect(composeDisplayMode('on', 'auto', false)).toBe('flag-name')
    expect(composeDisplayMode('on', 'auto', true)).toBe('flag')
  })
})

describe('axesFromLegacy', () => {
  it('maps each legacy enum value to its (flag, text) axes', () => {
    expect(axesFromLegacy('auto')).toEqual({ flag: 'on', text: 'auto' })
    expect(axesFromLegacy('flag')).toEqual({ flag: 'on', text: 'off' })
    expect(axesFromLegacy('flag-name')).toEqual({ flag: 'on', text: 'name' })
    expect(axesFromLegacy('flag-code')).toEqual({ flag: 'on', text: 'code' })
    expect(axesFromLegacy('name')).toEqual({ flag: 'off', text: 'name' })
    expect(axesFromLegacy('code')).toEqual({ flag: 'off', text: 'code' })
  })
  it('returns null for an unknown legacy value', () => {
    expect(axesFromLegacy('bogus')).toBeNull()
    expect(axesFromLegacy('')).toBeNull()
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
