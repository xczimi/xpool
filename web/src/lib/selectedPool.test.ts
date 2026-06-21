import { afterEach, describe, expect, it } from 'vitest'
import {
  SELECTED_POOL_KEY,
  readSelectedPool,
  writeSelectedPool,
  effectiveSelectedPool,
} from './selectedPool'

afterEach(() => localStorage.clear())

describe('readSelectedPool', () => {
  it('returns undefined (not chosen) when nothing is stored', () => {
    expect(readSelectedPool()).toBeUndefined()
  })
  it('returns null (everyone) for the sentinel value', () => {
    localStorage.setItem(SELECTED_POOL_KEY, '__everyone__')
    expect(readSelectedPool()).toBeNull()
  })
  it('returns the stored pool id', () => {
    localStorage.setItem(SELECTED_POOL_KEY, 'pool-demo')
    expect(readSelectedPool()).toBe('pool-demo')
  })
})

describe('writeSelectedPool', () => {
  it('stores the sentinel for null (everyone)', () => {
    writeSelectedPool(null)
    expect(localStorage.getItem(SELECTED_POOL_KEY)).toBe('__everyone__')
    expect(readSelectedPool()).toBeNull()
  })
  it('stores a pool id verbatim', () => {
    writeSelectedPool('pool-demo')
    expect(localStorage.getItem(SELECTED_POOL_KEY)).toBe('pool-demo')
    expect(readSelectedPool()).toBe('pool-demo')
  })
})

describe('effectiveSelectedPool', () => {
  it('defers to the first pool id when not chosen', () => {
    expect(effectiveSelectedPool(undefined, ['p1', 'p2'])).toBe('p1')
  })
  it('is null (everyone) when not chosen and the viewer has no pools', () => {
    expect(effectiveSelectedPool(undefined, [])).toBeNull()
  })
  it('honours an explicit everyone choice over the first pool', () => {
    expect(effectiveSelectedPool(null, ['p1'])).toBeNull()
  })
  it('honours an explicit pool choice', () => {
    expect(effectiveSelectedPool('p2', ['p1', 'p2'])).toBe('p2')
  })
})
