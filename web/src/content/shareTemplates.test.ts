import { describe, expect, it } from 'vitest'
import { SHARE_TEMPLATES } from './shareTemplates'

describe('SHARE_TEMPLATES', () => {
  it('offers the curated four', () => {
    expect(SHARE_TEMPLATES.map((t) => t.id)).toEqual([
      'short',
      'oneLiner',
      'email',
      'hungarian',
    ])
  })

  it('has unique ids', () => {
    const ids = SHARE_TEMPLATES.map((t) => t.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it('keeps the {LINK} placeholder in every body', () => {
    for (const tpl of SHARE_TEMPLATES) {
      expect(tpl.body).toContain('{LINK}')
    }
  })

  it('carries an i18n label key per template', () => {
    for (const tpl of SHARE_TEMPLATES) {
      expect(tpl.labelKey).toMatch(/^shareTemplate/)
    }
  })
})
