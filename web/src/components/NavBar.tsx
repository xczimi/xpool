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
// dynamic "My player page" item below, and remains reachable from the own
// player-detail page (and its /profile route still exists).
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
 * `me` (when present) supplies the id for the dynamic "My player page" item,
 * which targets `/player/<me.id>`. It is shown only for a real Player who is
 * not the result user (the result user has no participant page).
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

  return (
    <nav className="nav-bar">
      {visible.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.to === '/'}
          className={({ isActive }) => (isActive ? 'nav-link active' : 'nav-link')}
        >
          {t(item.label)}
        </NavLink>
      ))}
      {showOwnPlayer && (
        <NavLink
          key="own-player"
          to={`/player/${me.id}`}
          className={({ isActive }) => (isActive ? 'nav-link active' : 'nav-link')}
        >
          {t('playerPageOwnLink')}
        </NavLink>
      )}
    </nav>
  )
}
