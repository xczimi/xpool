import { describe, expect, it } from 'vitest'
import {
  ACCENTS,
  THEME_MODES,
  coerceAccent,
  coerceThemeMode,
  resolveTheme,
} from './theme'

describe('ACCENTS', () => {
  it('lists the six accents, amber first', () => {
    expect(ACCENTS).toEqual([
      'amber',
      'green',
      'cyan',
      'magenta',
      'violet',
      'mono',
    ])
  })
})

describe('THEME_MODES', () => {
  it('lists system, dark, light', () => {
    expect(THEME_MODES).toEqual(['system', 'dark', 'light'])
  })
})

describe('coerceAccent', () => {
  it('passes through every valid accent', () => {
    for (const a of ACCENTS) expect(coerceAccent(a)).toBe(a)
  })
  it('falls back to amber on junk', () => {
    expect(coerceAccent('orange')).toBe('amber')
    expect(coerceAccent('')).toBe('amber')
    expect(coerceAccent(null)).toBe('amber')
    expect(coerceAccent(undefined)).toBe('amber')
  })
})

describe('coerceThemeMode', () => {
  it('passes through every valid mode', () => {
    for (const m of THEME_MODES) expect(coerceThemeMode(m)).toBe(m)
  })
  it('falls back to system on junk', () => {
    expect(coerceThemeMode('auto')).toBe('system')
    expect(coerceThemeMode('')).toBe('system')
    expect(coerceThemeMode(null)).toBe('system')
  })
})

describe('resolveTheme', () => {
  it('resolves system via the OS flag', () => {
    expect(resolveTheme('system', true)).toBe('dark')
    expect(resolveTheme('system', false)).toBe('light')
  })
  it('passes explicit modes through unchanged', () => {
    expect(resolveTheme('dark', false)).toBe('dark')
    expect(resolveTheme('light', true)).toBe('light')
  })
})
