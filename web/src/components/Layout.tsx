import { Outlet } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY } from '../graphql/queries'
import type { Me } from '../graphql/types'
import { AuthBar } from './AuthBar'
import { BrandIcon } from './BrandIcon'
import { DisplayModeSelector } from './DisplayModeSelector'
import { LanguageSelector } from './LanguageSelector'
import { ThemeSelector } from './ThemeSelector'
import { NavBar } from './NavBar'
import { UnclaimedBanner } from './UnclaimedBanner'

/**
 * Persistent chrome (REWRITE_USE_CASES §4): header (tagline + language),
 * auth bar, horizontal nav, content area, footer.
 */
export function Layout() {
  const { label } = useAuth()
  const { t } = useI18n()

  const [meResult] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !label,
  })

  const meRaw = meResult.data?.me ?? null
  const me = meRaw?.__typename === 'Player' ? meRaw : null

  return (
    <div className="app">
      <header className="app-header">
        <div className="brand">
          <BrandIcon />
          <div className="brand-text">
            <h1>xPool</h1>
            <p className="tagline">{t('tagline')}</p>
          </div>
        </div>
        <div className="header-controls">
          <DisplayModeSelector />
          <LanguageSelector />
          <ThemeSelector />
        </div>
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
