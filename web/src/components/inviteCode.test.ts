import { describe, expect, it } from 'vitest'
import { extractCode } from './inviteCode'

describe('extractCode', () => {
  it('returns a bare code as-is', () => {
    expect(extractCode('ABC123')).toBe('ABC123')
  })

  it('pulls the code out of a full invite URL', () => {
    expect(extractCode('https://xpool.example/invite/ABC123')).toBe('ABC123')
  })

  it('pulls the code out of a path-only link', () => {
    expect(extractCode('/invite/ABC123')).toBe('ABC123')
  })

  it('drops a trailing query/hash', () => {
    expect(extractCode('https://xpool.example/invite/ABC123?ref=x#top')).toBe('ABC123')
    expect(extractCode('ABC123/')).toBe('ABC123')
  })

  it('trims surrounding whitespace', () => {
    expect(extractCode('  ABC123  ')).toBe('ABC123')
  })

  it('returns null for empty or whitespace-only input', () => {
    expect(extractCode('')).toBeNull()
    expect(extractCode('   ')).toBeNull()
  })

  it('returns null when a link ends at the marker with no code', () => {
    expect(extractCode('https://xpool.example/invite/')).toBeNull()
  })
})
