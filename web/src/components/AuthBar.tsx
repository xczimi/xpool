import type { ChangeEvent } from 'react'
import { useQuery } from 'urql'
import { useAuth0 } from '@auth0/auth0-react'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, PLAYERS_QUERY } from '../graphql/queries'
import type { Me, PlayerSummary } from '../graphql/types'
import { clearToken, getDevNow, setDevNow, clearDevNow } from '../auth/devAuth'
import { auth0Enabled } from '../auth/auth0Provider'

export function AuthBar() {
  if (auth0Enabled) return <ProdAuthBar />
  return <DevAuthBar />
}

/**
 * Production auth bar. Uses Auth0 redirect flow.
 * i18n: strings are hardcoded English for now; localisation deferred to a
 * later UI-polish phase (the plan puts prod-mode string polish out of scope).
 */
function ProdAuthBar() {
  const { isAuthenticated, loginWithRedirect, logout, user } = useAuth0()
  if (!isAuthenticated) {
    return (
      <div className="auth-bar">
        <span>You are outside.</span>{' '}
        <button onClick={() => void loginWithRedirect()}>Log in</button>
      </div>
    )
  }
  return (
    <div className="auth-bar">
      <span>Logged in as {user?.name ?? user?.email ?? 'player'}</span>{' '}
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

function DevClock() {
  const { t } = useI18n()
  const current = getDevNow()
  // datetime-local wants 'YYYY-MM-DDTHH:mm'; store/send full RFC3339 (UTC).
  const value = current ? current.slice(0, 16) : ''
  // The dev clock input is interpreted as UTC — consistent with the XPOOL_NOW
  // env var (RFC3339 UTC). Appending ':00Z' avoids the local-time shift that
  // new Date(localString).toISOString() would introduce on non-UTC machines.
  const onChange = (e: ChangeEvent<HTMLInputElement>) => {
    if (e.target.value) {
      // datetime-local yields 'YYYY-MM-DDTHH:MM'; ':00Z' makes valid RFC3339 UTC.
      setDevNow(e.target.value + ':00Z')
    } else {
      clearDevNow()
    }
    location.reload() // simplest correct cache reset for the new clock
  }
  return (
    <span className="dev-clock">
      <label>
        {t('devClock')}
        <input type="datetime-local" value={value} onChange={onChange} />
      </label>
      {current && (
        <button type="button" onClick={() => { clearDevNow(); location.reload() }}>
          {t('devClockReset')}
        </button>
      )}
    </span>
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
