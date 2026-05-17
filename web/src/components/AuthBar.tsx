import { useState } from 'react'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, SCOREBOARD_QUERY } from '../graphql/queries'
import type { Player, Scoreboard } from '../graphql/types'

/**
 * Dev auth bar. A visitor sees a player picker (sourced from the public
 * scoreboard's entries) plus a free-text id input. Choosing one stores the id
 * and sends it as `X-Dev-Player` on every request. "Log out" clears it.
 */
export function AuthBar() {
  const { playerId, login, logout } = useAuth()
  const { t } = useI18n()
  const [manualId, setManualId] = useState('')

  const [meResult] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !playerId,
  })
  const [sbResult] = useQuery<{ scoreboard: Scoreboard | null }>({
    query: SCOREBOARD_QUERY,
    variables: { pool: null },
  })

  const me = meResult.data?.me
  const candidates = sbResult.data?.scoreboard?.entries ?? []

  if (playerId) {
    return (
      <div className="auth-bar">
        <span>
          {t('loggedInAs')} <strong>{me?.nick ?? playerId}</strong>
          {me?.isAdmin ? ' (admin)' : ''}
        </span>
        <button type="button" onClick={logout}>
          {t('logOut')}
        </button>
      </div>
    )
  }

  return (
    <div className="auth-bar">
      <span>{t('visitor')}</span>
      <span className="auth-picker">
        {candidates.length > 0 && (
          <select
            defaultValue=""
            onChange={(e) => e.target.value && login(e.target.value)}
          >
            <option value="" disabled>
              {t('logIn')}…
            </option>
            {candidates.map((c) => (
              <option key={c.playerId} value={c.playerId}>
                {c.nick} ({c.playerId})
              </option>
            ))}
          </select>
        )}
        <input
          placeholder="player id"
          value={manualId}
          onChange={(e) => setManualId(e.target.value)}
        />
        <button
          type="button"
          disabled={!manualId.trim()}
          onClick={() => login(manualId.trim())}
        >
          {t('logIn')}
        </button>
      </span>
    </div>
  )
}
