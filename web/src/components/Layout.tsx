import { Link, Outlet, useLocation } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY } from '../graphql/queries'
import type { Me } from '../graphql/types'
import { accessFor } from '../auth/routeAccess'
import { contentGate, type ViewerState } from '../auth/contentGate'
import { detectEnv, envSuffix } from '../lib/env'
import { AuthBar } from './AuthBar'
import { BrandIcon } from './BrandIcon'
import { SettingsMenu } from './SettingsMenu'
import { NavBar } from './NavBar'
import { NeedsInvite } from './NeedsInvite'
import { SessionExpired } from './SessionExpired'

/**
 * Persistent chrome (REWRITE_USE_CASES §4): header (tagline + language),
 * auth bar, horizontal nav, content area, footer.
 *
 * Invite-only dead-end: an authenticated viewer who is not yet a Player and has
 * no link candidate sees the `NeedsInvite` explainer in place of any player- or
 * admin-only page. Public pages stay reachable (incl. the `/invite/:code` claim
 * page, the way out) — see `accessFor`. A viewer WITH a link candidate is mid
 * link/claim flow (handled on the invite page), so they are not dead-ended.
 *
 * Dead session: when the server no longer accepts the token (or `me` comes back
 * null while the SPA still shows a login), `SessionExpired` replaces the page on
 * every non-public route. See `auth/contentGate.ts`.
 */
export function Layout() {
  const { label, sessionExpired, hasSession } = useAuth()
  const { t } = useI18n()
  const location = useLocation()
  const envTag = envSuffix(detectEnv())

  const [meResult] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !label,
  })

  const meRaw = meResult.data?.me ?? null
  const me = meRaw?.__typename === 'Player' ? meRaw : null

  // `data === undefined` means the query is still in flight or paused — NOT
  // that the server said Visitor. Only an explicit null `me` is anonymous;
  // conflating the two would flash the session-expired view on every load.
  const viewer: ViewerState =
    meResult.data === undefined
      ? 'loading'
      : meRaw === null
        ? 'anonymous'
        : meRaw.__typename === 'Player'
          ? 'player'
          : meRaw.linkCandidate
            ? 'unclaimed-linkable'
            : 'unclaimed'

  const gate = contentGate({
    access: accessFor(location.pathname),
    sessionExpired,
    // NOT `Boolean(label)` — the label is nulled on expiry, which is exactly
    // when the dead-end must render. See `AuthState.hasSession`.
    hasSession,
    viewer,
  })

  // Optimistic player-nav signal: show player links as soon as a session
  // exists, hiding them only once the viewer is *confirmed* unclaimed. This
  // keeps nav synchronous for a real player (no flash-of-hidden-nav while the
  // `me` query is in flight) and still hides it at the invite dead-end.
  const showPlayerNav = Boolean(label) && viewer !== 'unclaimed'

  return (
    <div className="app">
      <header className="app-header">
        <div className="brand">
          <BrandIcon />
          <div className="brand-text">
            <div className="wordmark">
              <h1>xPool</h1>
              {envTag && <span className="env-tag">{envTag}</span>}
            </div>
            <p className="tagline">{t('tagline')}</p>
          </div>
        </div>
        <div className="header-controls">
          <SettingsMenu />
        </div>
      </header>

      <AuthBar />
      <NavBar isPlayer={showPlayerNav} isAdmin={Boolean(me?.isResultUser)} me={me} />

      <main className="content">
        {gate === 'session-expired' ? (
          <SessionExpired />
        ) : gate === 'needs-invite' ? (
          <NeedsInvite />
        ) : (
          <Outlet />
        )}
      </main>

      <footer className="app-footer">
        <span className="footer-tagline">{t('footer')}</span>
        <span className="footer-meta">
          ©{' '}
          <a
            href="https://xczimi.com/"
            target="_blank"
            rel="noopener noreferrer"
          >
            xczimi
          </a>{' '}
          ·{' '}
          <Link to="/privacy">{t('privacy')}</Link>{' '}
          ·{' '}
          <a
            href="https://github.com/xczimi/xpool"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
        </span>
      </footer>
    </div>
  )
}
