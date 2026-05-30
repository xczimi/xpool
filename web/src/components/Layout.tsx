import { Outlet } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY } from '../graphql/queries'
import type { Player } from '../graphql/types'
import { AuthBar } from './AuthBar'
import { LanguageSelector } from './LanguageSelector'
import { NavBar } from './NavBar'
import { UnclaimedBanner } from './UnclaimedBanner'

/**
 * Persistent chrome (REWRITE_USE_CASES §4): header (tagline + language),
 * auth bar, horizontal nav, content area, footer.
 */
export function Layout() {
  const { label } = useAuth()
  const { t } = useI18n()

  const [meResult] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !label,
  })

  const me = meResult.data?.me ?? null

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
      <UnclaimedBanner />

      <main className="content">
        <Outlet />
      </main>

      <footer className="app-footer">{t('footer')}</footer>
    </div>
  )
}
