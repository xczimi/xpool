import { NavLink } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import type { StringKey } from '../i18n/strings'
import type { Player } from '../graphql/types'
import { accessFor } from '../auth/routeAccess'

interface NavItem {
  to: string
  label: StringKey
}

// Profile is intentionally absent here — it is replaced in the nav by the
// dynamic "Me" item (inserted right after Home below), and remains reachable
// from the own player-detail page (and its /profile route still exists).
const ITEMS: NavItem[] = [
  { to: '/', label: 'navHome' },
  { to: '/today', label: 'navToday' },
  { to: '/games', label: 'navGames' },
  { to: '/mytips', label: 'navMyTips' },
  { to: '/alltips', label: 'navAllTips' },
  { to: '/scoreboard', label: 'navScoreboard' },
  { to: '/perfect', label: 'navPerfect' },
  { to: '/pools', label: 'navPools' },
  { to: '/rules', label: 'navRules' },
  { to: '/admin', label: 'navAdmin' },
]

/**
 * `isPlayer` is true only for a real linked Player — NOT merely "a session
 * exists". An authenticated-but-unclaimed viewer is not a player, so the
 * player-only links stay hidden and they see the invite dead-end instead.
 * Access per route comes from the shared `accessFor` map (single source with
 * `Layout`'s dead-end gating).
 *
 * The dynamic "Me" item (shown only for a real Player who is not the result
 * user) is inserted right after Home and targets the `/me` alias — a clean
 * route that renders the viewer's own player page without a UUID in the URL.
 */
export function NavBar({
  isPlayer,
  isAdmin,
  me,
}: {
  isPlayer: boolean
  isAdmin: boolean
  me: Player | null
}) {
  const { t } = useI18n()

  const visible = ITEMS.filter((item) => {
    const access = accessFor(item.to)
    if (access === 'player') return isPlayer
    if (access === 'admin') return isAdmin
    return true
  })

  const showOwnPlayer = me !== null && !me.isResultUser
  // "Me" sits immediately after Home. It is the /me alias, not /player/<uuid>.
  const links: NavItem[] = showOwnPlayer
    ? visible.flatMap((item) =>
        item.to === '/' ? [item, { to: '/me', label: 'navMe' }] : [item],
      )
    : visible

  return (
    <nav className="nav-bar">
      {links.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.to === '/'}
          className={({ isActive }) => (isActive ? 'nav-link active' : 'nav-link')}
        >
          {t(item.label)}
        </NavLink>
      ))}
    </nav>
  )
}
