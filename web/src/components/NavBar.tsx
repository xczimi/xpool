import { NavLink } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import type { StringKey } from '../i18n/strings'
import { accessFor } from '../auth/routeAccess'

interface NavItem {
  to: string
  label: StringKey
}

const ITEMS: NavItem[] = [
  { to: '/', label: 'navHome' },
  { to: '/today', label: 'navToday' },
  { to: '/games', label: 'navGames' },
  { to: '/mytips', label: 'navMyTips' },
  { to: '/alltips', label: 'navAllTips' },
  { to: '/scoreboard', label: 'navScoreboard' },
  { to: '/perfect', label: 'navPerfect' },
  { to: '/pools', label: 'navPools' },
  { to: '/profile', label: 'navProfile' },
  { to: '/invite', label: 'navInvite' },
  { to: '/rules', label: 'navRules' },
  { to: '/admin', label: 'navAdmin' },
]

/**
 * `isPlayer` is true only for a real linked Player — NOT merely "a session
 * exists". An authenticated-but-unclaimed viewer is not a player, so the
 * player-only links stay hidden and they see the invite dead-end instead.
 * Access per route comes from the shared `accessFor` map (single source with
 * `Layout`'s dead-end gating).
 */
export function NavBar({ isPlayer, isAdmin }: { isPlayer: boolean; isAdmin: boolean }) {
  const { t } = useI18n()

  const visible = ITEMS.filter((item) => {
    const access = accessFor(item.to)
    if (access === 'player') return isPlayer
    if (access === 'admin') return isAdmin
    return true
  })

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
    </nav>
  )
}
