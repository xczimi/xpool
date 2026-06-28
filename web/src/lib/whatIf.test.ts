import { describe, it, expect } from 'vitest'
import { computeWhatIf } from './whatIf'

describe('computeWhatIf', () => {
  it('computes current + ifHome/ifAway totals and deltas (group ×1)', () => {
    // grace 1-0 vs live 1-0, group multiplier 1.
    // current: exact home + exact away + outcome = 4.
    // if home scores → 2-0: exact away + outcome = 3 (delta -1).
    // if away scores → 1-1: exact home only = 1 (delta -3).
    const result = computeWhatIf(
      { gameId: 'M8', homeScore: 1, awayScore: 0, locked: true },
      { homeScore: 1, awayScore: 0, provisional: true, source: null, sourceStatus: null, ninetyMinuteUncertain: false },
      1,
    )
    expect(result.current).toBe(4)
    expect(result.ifHome).toEqual({ total: 3, delta: -1 })
    expect(result.ifAway).toEqual({ total: 1, delta: -3 })
  })

  it('applies the round multiplier to totals and deltas', () => {
    // Same as above but R16 (×3): current 12, ifHome 9 (delta -3), ifAway 3 (delta -9).
    const result = computeWhatIf(
      { gameId: 'X', homeScore: 1, awayScore: 0, locked: true },
      { homeScore: 1, awayScore: 0, provisional: true, source: null, sourceStatus: null, ninetyMinuteUncertain: false },
      3,
    )
    expect(result.current).toBe(12)
    expect(result.ifHome).toEqual({ total: 9, delta: -3 })
    expect(result.ifAway).toEqual({ total: 3, delta: -9 })
  })

  it('shows a positive delta when the next goal helps', () => {
    // predicted 2-1 vs live 1-1; if home scores → 2-1 exact (4), big jump from current.
    const result = computeWhatIf(
      { gameId: 'X', homeScore: 2, awayScore: 1, locked: true },
      { homeScore: 1, awayScore: 1, provisional: true, source: null, sourceStatus: null, ninetyMinuteUncertain: false },
      1,
    )
    // current: 2-1 vs 1-1 → exact away (1), wrong home, wrong outcome (win vs draw) = 1.
    expect(result.current).toBe(1)
    // if home → 2-1: exact home + exact away + outcome = 4 (delta +3).
    expect(result.ifHome).toEqual({ total: 4, delta: 3 })
  })
})
