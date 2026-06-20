import { describe, expect, it } from 'vitest'
import { accessFor } from './routeAccess'

describe('accessFor', () => {
  it('treats a player page as player-access', () => {
    expect(accessFor('/player/demo-ada')).toBe('player')
    expect(accessFor('/player/demo-ada/')).toBe('player')
  })
  it('leaves existing routes unchanged', () => {
    expect(accessFor('/scoreboard')).toBe('public')
    expect(accessFor('/admin')).toBe('admin')
    expect(accessFor('/mytips')).toBe('player')
  })
})
