import { describe, expect, it } from 'vitest'
import { clockSkewMs, formatCountdown, remainingMs } from './countdown'

describe('formatCountdown', () => {
  it('formats sub-minute durations as HH:MM:SS', () => {
    expect(formatCountdown(45_000)).toBe('00:00:45')
  })

  it('formats hours/minutes/seconds without a day part', () => {
    // 1h 59m 48s
    expect(formatCountdown((1 * 3600 + 59 * 60 + 48) * 1000)).toBe('01:59:48')
  })

  it('prefixes a day count when >= 1 day remains', () => {
    // 3d 4h 11m 22s
    expect(
      formatCountdown((3 * 86400 + 4 * 3600 + 11 * 60 + 22) * 1000),
    ).toBe('3d 04:11:22')
  })

  it('renders exactly zero as 00:00:00', () => {
    expect(formatCountdown(0)).toBe('00:00:00')
  })

  it('clamps negative remaining to 00:00:00', () => {
    expect(formatCountdown(-5_000)).toBe('00:00:00')
  })
})

describe('clockSkewMs', () => {
  it('is the signed offset of server now from the client clock', () => {
    const serverNow = '2026-06-11T12:00:00Z'
    const clientNow = Date.parse('2026-06-11T11:59:58Z') // client 2s behind
    expect(clockSkewMs(serverNow, clientNow)).toBe(2_000)
  })
})

describe('remainingMs', () => {
  it('is the deadline minus the estimated server now', () => {
    const deadline = '2026-06-11T12:00:00Z'
    const estNow = Date.parse('2026-06-11T11:00:00Z')
    expect(remainingMs(deadline, estNow)).toBe(3_600_000)
  })

  it('goes negative once the deadline has passed', () => {
    const deadline = '2026-06-11T12:00:00Z'
    const estNow = Date.parse('2026-06-11T12:00:10Z')
    expect(remainingMs(deadline, estNow)).toBe(-10_000)
  })
})
