import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'
import { InviteCodeEntry } from './InviteCodeEntry'

/**
 * The dead-end for an authenticated viewer who is not yet a Player and has no
 * link candidate (invite-only-hardening). Shown in the content area in place of
 * a player-only page, and also rendered at the public `/invite` route. Public
 * pages stay reachable — see `accessFor` in `auth/routeAccess.ts`.
 *
 * The way out is `InviteCodeEntry`: it extracts the code from a pasted link or
 * bare code and routes to the public claim page (`/invite/:code`).
 */
export function NeedsInvite() {
  const { t } = useI18n()
  const { logout } = useAuth()

  return (
    <div className="status needs-invite">
      <h2>{t('inviteOnlyTitle')}</h2>
      <p>{t('inviteOnlyBody')}</p>

      <InviteCodeEntry />

      <button type="button" onClick={logout}>
        {t('logOut')}
      </button>
    </div>
  )
}
