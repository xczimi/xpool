import { describe, it, expect } from 'vitest'
import { scoreMatchBase, scoreMatchPoints } from './matchScoring'

describe('scoreMatchBase', () => {
  it('awards 4 for an exact correct scoreline (1+1+2)', () => {
    expect(scoreMatchBase({ homeScore: 1, awayScore: 0 }, { homeScore: 1, awayScore: 0 })).toBe(4)
  })

  it('awards 2 for the right outcome only', () => {
    // predicted 2-0 (home win), actual 3-1 (home win): neither side exact, outcome only
    expect(scoreMatchBase({ homeScore: 2, awayScore: 0 }, { homeScore: 3, awayScore: 1 })).toBe(2)
  })

  it('awards 3 for one exact side + outcome', () => {
    // predicted 1-0, actual 1-1 → exact home (1), wrong away, wrong outcome (win vs draw)
    expect(scoreMatchBase({ homeScore: 1, awayScore: 0 }, { homeScore: 1, awayScore: 1 })).toBe(1)
    // predicted 1-0, actual 2-0 → exact away (1) + outcome (2)
    expect(scoreMatchBase({ homeScore: 1, awayScore: 0 }, { homeScore: 2, awayScore: 0 })).toBe(3)
  })

  it('awards 0 for a wrong outcome and no exact side', () => {
    expect(scoreMatchBase({ homeScore: 0, awayScore: 1 }, { homeScore: 2, awayScore: 0 })).toBe(0)
  })

  it('applies the symmetric 4-goal rule per side', () => {
    // predicted 5-0, actual 4-0 → home counts as exact (both >= 4), away exact, outcome
    expect(scoreMatchBase({ homeScore: 5, awayScore: 0 }, { homeScore: 4, awayScore: 0 })).toBe(4)
  })

  it('scores a draw outcome', () => {
    expect(scoreMatchBase({ homeScore: 2, awayScore: 2 }, { homeScore: 0, awayScore: 0 })).toBe(2)
  })
})

describe('scoreMatchPoints', () => {
  it('multiplies the base by the round multiplier', () => {
    expect(scoreMatchPoints({ homeScore: 1, awayScore: 0 }, { homeScore: 1, awayScore: 0 }, 3)).toBe(12)
  })

  it('group-stage multiplier of 1 returns the base unchanged', () => {
    // outcome-only base (2), multiplier 1 → 2
    expect(scoreMatchPoints({ homeScore: 2, awayScore: 0 }, { homeScore: 3, awayScore: 1 }, 1)).toBe(2)
  })
})
