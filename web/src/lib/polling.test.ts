import { describe, expect, it } from 'vitest'
import type { SingleGame } from '../graphql/types'
import { pollIntervalMs } from './polling'

const game = (resultPending: boolean): SingleGame =>
  ({ resultPending }) as SingleGame

describe('pollIntervalMs', () => {
  it('returns 0 when no games are loaded', () => {
    expect(pollIntervalMs([])).toBe(0)
  })

  it('returns 0 when no game is result-pending', () => {
    expect(pollIntervalMs([game(false), game(false)])).toBe(0)
  })

  it('polls every 30s when at least one game is result-pending', () => {
    expect(pollIntervalMs([game(false), game(true)])).toBe(30_000)
  })

  it('polls when every game is result-pending', () => {
    expect(pollIntervalMs([game(true)])).toBe(30_000)
  })
})
