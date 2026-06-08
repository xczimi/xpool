export type Access = 'public' | 'player' | 'admin'

const PLAYER_PATHS = new Set(['/mytips', '/alltips', '/pools', '/profile', '/invite'])

/**
 * Canonical per-route access level — the single source shared by `NavBar`
 * (which links to show) and `Layout` (whether an authenticated-but-unclaimed
 * viewer hits the "you need an invite" dead-end instead of the page).
 *
 * The invite *claim* page (`/invite/:code`) is deliberately **public**: it is
 * the way OUT of the dead-end, so an unclaimed viewer must be able to reach it.
 * The invite *share* page (`/invite`, exact) is player-only.
 */
export function accessFor(pathname: string): Access {
  const path = pathname.replace(/\/+$/, '') || '/'
  if (path === '/admin' || path.startsWith('/admin/')) return 'admin'
  if (path.startsWith('/invite/')) return 'public' // the claim page (/invite/:code)
  if (PLAYER_PATHS.has(path)) return 'player'
  return 'public'
}
