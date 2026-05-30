import { useState } from 'react'
import { useMutation } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { INVITE_MUTATION } from '../graphql/queries'
import { NeedsLogin } from '../components/StatusViews'

/**
 * Referral invitation (UC-3).
 *
 * The reconciled `invite(inviteeId: ID!)` mutation only records a referral
 * link to an *already-existing* player — there is no account creation here.
 * In this dev build the screen is a simple "refer an existing player by id"
 * action; see `web/README.md` "GraphQL assumptions" for the limitation.
 */
export function InvitePage() {
  const { t } = useI18n()
  const { label } = useAuth()
  const [inviteState, invite] = useMutation(INVITE_MUTATION)

  const [inviteeId, setInviteeId] = useState('')
  const [flash, setFlash] = useState<string | null>(null)

  if (!label) return <NeedsLogin />

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setFlash(null)
    const res = await invite({ inviteeId: inviteeId.trim() })
    if (res.error) {
      setFlash(`${t('errorPrefix')}: ${res.error.message}`)
    } else {
      setFlash(t('inviteSent'))
      setInviteeId('')
    }
  }

  return (
    <section className="page">
      <h2>{t('inviteTitle')}</h2>
      <p>{t('inviteIntro')}</p>
      {flash && <p className="flash-bar">{flash}</p>}
      <form className="form" onSubmit={submit}>
        <label>
          {t('player')}
          <input
            required
            placeholder="player id"
            value={inviteeId}
            onChange={(e) => setInviteeId(e.target.value)}
          />
        </label>
        <button
          type="submit"
          className="primary"
          disabled={inviteState.fetching || !inviteeId.trim()}
        >
          {t('sendInvite')}
        </button>
      </form>
    </section>
  )
}
