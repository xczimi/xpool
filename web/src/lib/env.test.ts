import { describe, it, expect } from 'vitest'
import { detectEnv, envSuffix } from './env'

describe('detectEnv', () => {
  it('treats localhost / 127.0.0.1 as local', () => {
    expect(detectEnv('localhost')).toBe('local')
    expect(detectEnv('127.0.0.1')).toBe('local')
  })

  it('treats a hostname containing "dev" as dev', () => {
    expect(detectEnv('pool-dev.xczimi.com')).toBe('dev')
  })

  it('treats everything else as prod', () => {
    expect(detectEnv('pool.xczimi.com')).toBe('prod')
  })

  it('is case-insensitive', () => {
    expect(detectEnv('POOL-DEV.xczimi.com')).toBe('dev')
    expect(detectEnv('LocalHost')).toBe('local')
  })
})

describe('envSuffix', () => {
  it('returns a ·-prefixed suffix for non-prod', () => {
    expect(envSuffix('local')).toBe('·local')
    expect(envSuffix('dev')).toBe('·dev')
  })

  it('returns null for prod (no suffix)', () => {
    expect(envSuffix('prod')).toBeNull()
  })
})
