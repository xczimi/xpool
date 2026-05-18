import type { ChangeEvent } from 'react'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, PLAYERS_QUERY } from '../graphql/queries'
import type { Player, PlayerSummary } from '../graphql/types'
import { getDevNow, setDevNow, clearDevNow } from '../auth/devAuth'

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
 * Dev auth bar. Auth is a stub (API.md §8) — there is no real login. A visitor
 * picks a seeded player from the `players` list; the choice is stored and sent
 * as the `X-Dev-Player` header on every request. "Log out" clears it.
 */
export function AuthBar() {
  const { playerId, login, logout } = useAuth()
  const { t } = useI18n()

  const [meResult] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !playerId,
  })
  const [playersResult] = useQuery<{ players: PlayerSummary[] }>({
    query: PLAYERS_QUERY,
  })

  const me = meResult.data?.me
  const players = playersResult.data?.players ?? []

  if (playerId) {
    // An id that resolves to no player (e.g. a stale localStorage value).
    const unknown = !meResult.fetching && !me
    return (
      <div className="auth-bar">
        <span>
          {t('loggedInAs')} <strong>{me?.nick ?? playerId}</strong>
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
          onChange={(e) => e.target.value && login(e.target.value)}
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
