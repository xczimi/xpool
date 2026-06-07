import { describe, expect, it } from 'vitest'
import { devClockInstant } from './devClockTimes'

describe('devClockInstant', () => {
  const K = '2026-06-11T19:00:00Z'

  it('before: kickoff − 10 min (predictions still open)', () => {
    expect(devClockInstant(K, 'before')).toBe('2026-06-11T18:50:00.000Z')
  })

  it('during: kickoff + 60 min (result pending)', () => {
    expect(devClockInstant(K, 'during')).toBe('2026-06-11T20:00:00.000Z')
  })

  it('after: kickoff + 135 min (~match over + 15 min)', () => {
    expect(devClockInstant(K, 'after')).toBe('2026-06-11T21:15:00.000Z')
  })

  it('rolls the date forward when the offset crosses midnight UTC', () => {
    // Kickoff at 23:30Z, +135m lands on the next UTC day.
    expect(devClockInstant('2026-06-11T23:30:00Z', 'after')).toBe(
      '2026-06-12T01:45:00.000Z',
    )
  })

  it('yields the correct UTC instant regardless of the local DST boundary', () => {
    // 2026-03-29 01:30Z is during Europe's spring-forward; the result is a
    // plain UTC arithmetic — no local-time shift creeps in.
    expect(devClockInstant('2026-03-29T01:30:00Z', 'before')).toBe(
      '2026-03-29T01:20:00.000Z',
    )
  })
})
