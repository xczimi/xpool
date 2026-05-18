import { NavLink } from 'react-router-dom'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import type { StringKey } from '../i18n/strings'

interface NavItem {
  to: string
  label: StringKey
  access: 'public' | 'player' | 'admin'
}

const ITEMS: NavItem[] = [
  { to: '/', label: 'navHome', access: 'public' },
  { to: '/today', label: 'navToday', access: 'public' },
  { to: '/games', label: 'navGames', access: 'public' },
  { to: '/mytips', label: 'navMyTips', access: 'player' },
  { to: '/alltips', label: 'navAllTips', access: 'player' },
  { to: '/scoreboard', label: 'navScoreboard', access: 'public' },
  { to: '/perfect', label: 'navPerfect', access: 'public' },
  { to: '/pools', label: 'navPools', access: 'player' },
  { to: '/profile', label: 'navProfile', access: 'player' },
  { to: '/invite', label: 'navInvite', access: 'player' },
  { to: '/rules', label: 'navRules', access: 'public' },
  { to: '/admin', label: 'navAdmin', access: 'admin' },
]

export function NavBar({ isAdmin }: { isAdmin: boolean }) {
  const { playerId } = useAuth()
  const { t } = useI18n()
  const isPlayer = Boolean(playerId)

  const visible = ITEMS.filter((item) => {
    if (item.access === 'player') return isPlayer
    if (item.access === 'admin') return isAdmin
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
