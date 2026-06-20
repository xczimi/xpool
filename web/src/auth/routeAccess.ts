export type Access = 'public' | 'player' | 'admin'

const PLAYER_PATHS = new Set(['/mytips', '/alltips', '/pools', '/profile'])

/**
 * Canonical per-route access level — the single source shared by `NavBar`
 * (which links to show) and `Layout` (whether an authenticated-but-unclaimed
 * viewer hits the "you need an invite" dead-end instead of the page).
 *
 * The whole `/invite*` namespace is **public** — both the recipient-side
 * code-entry page (`/invite`, exact, renders `NeedsInvite`) and the claim page
 * (`/invite/:code`). They are the way OUT of the dead-end, so an unclaimed (or
 * logged-out) viewer must reach them. Sharing an invite lives on the
 * player-only Pools page; there is no separate share page.
 */
export function accessFor(pathname: string): Access {
  const path = pathname.replace(/\/+$/, '') || '/'
  if (path === '/admin' || path.startsWith('/admin/')) return 'admin'
  if (path === '/invite' || path.startsWith('/invite/')) return 'public'
  if (path.startsWith('/player/')) return 'player'
  if (PLAYER_PATHS.has(path)) return 'player'
  return 'public'
}
