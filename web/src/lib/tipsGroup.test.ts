import { afterEach, describe, expect, it } from 'vitest'
import { readTipsGroup, writeTipsGroup } from './tipsGroup'

afterEach(() => localStorage.clear())

describe('tipsGroup persistence', () => {
  it('returns null when nothing is stored', () => {
    expect(readTipsGroup()).toBeNull()
  })

  it('round-trips a leaf group id', () => {
    writeTipsGroup('A')
    expect(readTipsGroup()).toBe('A')
  })

  it('round-trips a round-node id and overwrites the previous value', () => {
    writeTipsGroup('A')
    writeTipsGroup('R16')
    expect(readTipsGroup()).toBe('R16')
  })
})
