import { Outlet } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { Player, Tournament } from '../graphql/types'
import { AuthBar } from './AuthBar'
import { LanguageSelector } from './LanguageSelector'
import { NavBar } from './NavBar'

/**
 * Persistent chrome (REWRITE_USE_CASES §4): header (tagline + language),
 * auth bar, horizontal nav, content area with an optional flash bar, footer.
 */
export function Layout() {
  const { playerId } = useAuth()
  const { t } = useI18n()

  const [meResult] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !playerId,
  })
  const [tournamentResult] = useQuery<{
    tournament: Tournament | null
    motd: string | null
  }>({ query: TOURNAMENT_QUERY })

  const me = meResult.data?.me ?? null
  const motd = tournamentResult.data?.motd

  return (
    <div className="app">
      <header className="app-header">
        <div className="brand">
          <h1>xPool</h1>
          <p className="tagline">{t('tagline')}</p>
        </div>
        <LanguageSelector />
      </header>

      <AuthBar />
      <NavBar isAdmin={Boolean(me?.isResultUser)} />

      {motd && <div className="flash-bar">{motd}</div>}

      <main className="content">
        <Outlet />
      </main>

      <footer className="app-footer">{t('footer')}</footer>
    </div>
  )
}
