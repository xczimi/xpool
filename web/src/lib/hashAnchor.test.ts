import { describe, expect, it } from 'vitest'
import { hashToId } from './hashAnchor'

describe('hashToId', () => {
  it('strips the leading # from a knockout group id', () => {
    expect(hashToId('#KO-M76')).toBe('KO-M76')
  })

  it('returns a group-stage id unchanged', () => {
    expect(hashToId('#E')).toBe('E')
  })

  it('returns empty string for an empty hash', () => {
    expect(hashToId('')).toBe('')
  })

  it('returns empty string for a bare #', () => {
    expect(hashToId('#')).toBe('')
  })

  it('decodes a percent-encoded hash', () => {
    expect(hashToId('#KO%2DM76')).toBe('KO-M76')
  })

  it('falls back to the raw value when decoding throws', () => {
    expect(hashToId('#%E0%A4%A')).toBe('%E0%A4%A')
  })
})
