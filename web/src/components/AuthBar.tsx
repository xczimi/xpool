import { useQuery } from 'urql'
import { useAuth0 } from '@auth0/auth0-react'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, PLAYERS_QUERY } from '../graphql/queries'
import type { Me, PlayerSummary } from '../graphql/types'
import { clearToken } from '../auth/devAuth'
import { auth0Enabled } from '../auth/auth0Provider'
import { DevClock } from './DevClock'

export function AuthBar() {
  if (auth0Enabled) return <ProdAuthBar />
  return <DevAuthBar />
}

/**
 * Production auth bar. Uses Auth0 redirect flow.
 *
 * Invite-only front door (invite-only-hardening): a signed-out visitor sees an
 * invite-oriented lead ("Got an invite? Open your link") as the primary
 * message; "Members: log in" is present but secondary. Login passes
 * `screen_hint: 'login'` so a returning member lands on the login screen, not a
 * signup-flavoured one — open self-signup is discouraged (membership follows
 * invitations). Self-signup is not hard-blocked here; that is a documented
 * Auth0-Action fallback (see `.scratch/invite-only-hardening/PRD.md`).
 */
function ProdAuthBar() {
  const { isAuthenticated, loginWithRedirect, logout, user } = useAuth0()
  const { t } = useI18n()
  // Prefer the xPool nick (canonical display name everywhere) over the Auth0
  // profile name; fall back to the Auth0 e-mail only while the viewer is
  // authenticated but not yet a Player (no nick to show).
  const [meResult] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !isAuthenticated,
  })
  const meRaw = meResult.data?.me
  const me = meRaw?.__typename === 'Player' ? meRaw : null
  if (!isAuthenticated) {
    return (
      <div className="auth-bar">
        <span className="front-door-lead">{t('frontDoorLead')}</span>{' '}
        <button
          className="front-door-login"
          onClick={() =>
            void loginWithRedirect({
              appState: { returnTo: window.location.pathname + window.location.search },
              authorizationParams: { screen_hint: 'login' },
            })
          }
        >
          {t('frontDoorMembers')}
        </button>
      </div>
    )
  }
  return (
    <div className="auth-bar">
      <span>Logged in as {me?.nick ?? user?.email ?? 'player'}</span>{' '}
      <button onClick={() => {
        // Belt-and-suspenders: drop the localStorage JWT before the Auth0
        // redirect, in case TokenBridge's effect-driven cleanup races with
        // the urql client's next request.
        clearToken()
        logout({ logoutParams: { returnTo: window.location.origin } })
      }}>
        Log out
      </button>
    </div>
  )
}

/**
 * Dev auth bar. A visitor picks a seeded player from the `players` list; the
 * choice triggers `POST /api/dev/login` which mints a JWT stored in
 * localStorage. The urql client sends `Authorization: Bearer <jwt>` on every
 * request. "Log out" clears the token.
 */
function DevAuthBar() {
  const { label, login, logout } = useAuth()
  const { t } = useI18n()

  const [meResult] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !label,
  })
  const [playersResult] = useQuery<{ players: PlayerSummary[] }>({
    query: PLAYERS_QUERY,
  })

  const meRaw = meResult.data?.me
  const me = meRaw?.__typename === 'Player' ? meRaw : null
  const players = playersResult.data?.players ?? []

  if (label) {
    // An id that resolves to no player (e.g. a stale localStorage value).
    const unknown = !meResult.fetching && !me
    return (
      <div className="auth-bar">
        <span>
          {t('loggedInAs')} <strong>{me?.nick ?? label}</strong>
          {me?.isResultUser ? ' (admin)' : ''}
          {unknown && (
            <em className="auth-warn"> — unknown player id, pick one below</em>
          )}
        </span>
        <button type="button" onClick={logout}>
          {t('logOut')}
        </button>
        <DevClock />
      </div>
    )
  }

  return (
    <div className="auth-bar">
      <span>{t('visitor')}</span>
      <span className="auth-picker">
        <select
          defaultValue=""
          onChange={(e) => { if (e.target.value) { void login(e.target.value) } }}
        >
          <option value="" disabled>
            {t('logIn')}…
          </option>
          {players.map((p) => (
            <option key={p.id} value={p.id}>
              {p.nick}
              {p.isResultUser ? ' (admin / results)' : ''} — {p.id}
            </option>
          ))}
        </select>
      </span>
      <DevClock />
    </div>
  )
}
