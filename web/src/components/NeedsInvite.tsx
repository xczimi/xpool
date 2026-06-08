import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'
import { extractCode } from './inviteCode'

/**
 * The dead-end for an authenticated viewer who is not yet a Player and has no
 * link candidate (invite-only-hardening). Shown in the content area in place of
 * a player-only page, so the viewer gets a clear explanation instead of an
 * erroring screen. Public pages (Home/Rules/…) stay reachable — see
 * `accessFor` in `auth/routeAccess.ts`.
 *
 * The "paste your invite link" input is the way out: it extracts the code and
 * routes to the public claim page (`/invite/:code`).
 */
export function NeedsInvite() {
  const { t } = useI18n()
  const { logout } = useAuth()
  const navigate = useNavigate()
  const [entry, setEntry] = useState('')
  const [bad, setBad] = useState(false)

  const open = () => {
    const code = extractCode(entry)
    if (!code) {
      setBad(true)
      return
    }
    navigate(`/invite/${code}`)
  }

  return (
    <div className="status needs-invite">
      <h2>{t('inviteOnlyTitle')}</h2>
      <p>{t('inviteOnlyBody')}</p>

      <div className="needs-invite-link">
        <label>
          {t('inviteOnlyHaveLink')}
          <input
            type="text"
            value={entry}
            placeholder={t('inviteOnlyPastePlaceholder')}
            onChange={(e) => {
              setEntry(e.target.value)
              if (bad) setBad(false)
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') open()
            }}
          />
        </label>
        <button type="button" onClick={open}>
          {t('inviteOnlyOpen')}
        </button>
        {bad && <p className="auth-warn">{t('inviteOnlyBadLink')}</p>}
      </div>

      <button type="button" onClick={logout}>
        {t('logOut')}
      </button>
    </div>
  )
}
