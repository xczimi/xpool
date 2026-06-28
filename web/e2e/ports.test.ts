import { describe, it, expect } from 'vitest'
import { e2ePorts, e2eTableFile, DEFAULT_WEB_PORT, DEFAULT_API_PORT } from './ports'

describe('e2ePorts', () => {
  it('falls back to the legacy fixed ports when env is unset', () => {
    expect(e2ePorts({})).toEqual({ web: DEFAULT_WEB_PORT, api: DEFAULT_API_PORT })
  })

  it('reads dynamic ports from env', () => {
    expect(
      e2ePorts({ XPOOL_E2E_WEB_PORT: '51111', XPOOL_E2E_API_PORT: '52222' }),
    ).toEqual({ web: 51111, api: 52222 })
  })

  it('falls back per-var when only one is set', () => {
    expect(e2ePorts({ XPOOL_E2E_API_PORT: '40000' })).toEqual({
      web: DEFAULT_WEB_PORT,
      api: 40000,
    })
  })

  it('treats an empty string as unset', () => {
    expect(e2ePorts({ XPOOL_E2E_WEB_PORT: '' }).web).toBe(DEFAULT_WEB_PORT)
  })

  it('rejects a non-numeric port', () => {
    expect(() => e2ePorts({ XPOOL_E2E_WEB_PORT: 'abc' })).toThrow()
  })

  it('rejects an out-of-range port', () => {
    expect(() => e2ePorts({ XPOOL_E2E_API_PORT: '70000' })).toThrow()
  })

  it('namespaces the table file by the dynamic api port', () => {
    expect(e2eTableFile({ XPOOL_E2E_API_PORT: '54321' })).toBe('web/.e2e-table.54321')
  })

  it('namespaces the table file by the fallback api port when env is unset', () => {
    expect(e2eTableFile({})).toBe(`web/.e2e-table.${DEFAULT_API_PORT}`)
  })
})
