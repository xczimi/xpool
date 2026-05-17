import { NavLink, Navigate, Route, Routes } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY } from '../graphql/queries'
import type { Player } from '../graphql/types'
import { Loading, NeedsAdmin, NeedsLogin } from '../components/StatusViews'
import { AdminResults } from './admin/AdminResults'
import { AdminBanner } from './admin/AdminBanner'
import { AdminTeams } from './admin/AdminTeams'
import { AdminPlayers } from './admin/AdminPlayers'

/** Admin area (UC-13..16) with sub-routes under /admin. */
export function AdminPage() {
  const { t } = useI18n()
  const { playerId } = useAuth()
  const [meResult] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !playerId,
  })

  if (!playerId) return <NeedsLogin />
  if (meResult.fetching) return <Loading />
  if (!meResult.data?.me?.isAdmin) return <NeedsAdmin />

  return (
    <section className="page">
      <h2>{t('adminTitle')}</h2>
      <div className="group-subnav">
        <AdminTab to="results" label={t('adminResults')} />
        <AdminTab to="banner" label={t('adminBanner')} />
        <AdminTab to="teams" label={t('adminTeams')} />
        <AdminTab to="players" label={t('adminPlayers')} />
      </div>
      <Routes>
        <Route index element={<Navigate to="results" replace />} />
        <Route path="results" element={<AdminResults />} />
        <Route path="banner" element={<AdminBanner />} />
        <Route path="teams" element={<AdminTeams />} />
        <Route path="players" element={<AdminPlayers />} />
      </Routes>
    </section>
  )
}

function AdminTab({ to, label }: { to: string; label: string }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        isActive ? 'subnav-item active' : 'subnav-item'
      }
    >
      {label}
    </NavLink>
  )
}
