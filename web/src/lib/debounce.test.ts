import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { debounce } from './debounce'

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

describe('debounce', () => {
  it('fires once after the delay with the latest args', () => {
    const spy = vi.fn()
    const d = debounce(spy, 200)
    d.call('a')
    d.call('b')
    expect(spy).not.toHaveBeenCalled()
    vi.advanceTimersByTime(200)
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledWith('b')
  })

  it('cancel() prevents a pending call', () => {
    const spy = vi.fn()
    const d = debounce(spy, 200)
    d.call('x')
    d.cancel()
    vi.advanceTimersByTime(200)
    expect(spy).not.toHaveBeenCalled()
  })
})
