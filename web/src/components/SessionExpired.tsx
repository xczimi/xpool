import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'

/**
 * The dead-end for a viewer whose session the server no longer accepts: an
 * expired/revoked Auth0 refresh token, a 401 from the auth seam, or `me`
 * resolving to null while the SPA still shows a login.
 *
 * Rendered in the content area in place of a player- or admin-only page, the
 * same way `NeedsInvite` is; public pages stay reachable (see `contentGate`).
 * Before this existed, that state rendered a bare `ErrorView` — a contentless
 * "Something went wrong." that told the player nothing and left them stuck.
 */
export function SessionExpired() {
  const { t } = useI18n()
  const { reauthenticate } = useAuth()

  return (
    <div className="status session-expired">
      <h2>{t('sessionExpiredTitle')}</h2>
      <p>{t('sessionExpiredBody')}</p>
      <button type="button" onClick={reauthenticate}>
        {t('logInAgain')}
      </button>
    </div>
  )
}
