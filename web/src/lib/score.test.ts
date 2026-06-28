import { describe, expect, it } from 'vitest'
import { clampScore, predictedCount, SCORE_MAX, stepScore } from './score'

describe('clampScore', () => {
  it('floors at 0', () => expect(clampScore(-3)).toBe(0))
  it('caps at SCORE_MAX', () => expect(clampScore(99)).toBe(SCORE_MAX))
  it('truncates fractional input', () => expect(clampScore(3.9)).toBe(3))
  it('treats NaN as 0', () => expect(clampScore(Number.NaN)).toBe(0))
})

describe('stepScore', () => {
  it('first + from unset commits 0', () => expect(stepScore(null, 1)).toBe(0))
  it('- from unset stays unset', () => expect(stepScore(null, -1)).toBeNull())
  it('+ increments within range', () => expect(stepScore(2, 1)).toBe(3))
  it('- decrements within range', () => expect(stepScore(2, -1)).toBe(1))
  it('- below 0 unsets the value', () => expect(stepScore(0, -1)).toBeNull())
  it('+ cannot exceed SCORE_MAX', () => expect(stepScore(SCORE_MAX, 1)).toBe(SCORE_MAX))
})

describe('predictedCount', () => {
  it('counts only fully-entered matches', () => {
    expect(
      predictedCount([
        { home: 1, away: 0 },
        { home: 2, away: null },
        { home: null, away: null },
        { home: 0, away: 0 },
      ]),
    ).toBe(2)
  })
})
