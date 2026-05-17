import { useState } from 'react'
import { useMutation } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { INVITE_MUTATION } from '../graphql/queries'
import { NeedsLogin } from '../components/StatusViews'

/** Referral invitation (UC-3). */
export function InvitePage() {
  const { t } = useI18n()
  const { playerId } = useAuth()
  const [inviteState, invite] = useMutation(INVITE_MUTATION)

  const [email, setEmail] = useState('')
  const [nick, setNick] = useState('')
  const [fullName, setFullName] = useState('')
  const [flash, setFlash] = useState<string | null>(null)

  if (!playerId) return <NeedsLogin />

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setFlash(null)
    const res = await invite({ input: { email, nick, fullName } })
    if (res.error) {
      const msg = res.error.message
      setFlash(
        /already/i.test(msg) ? t('inviteExists') : `${t('errorPrefix')}: ${msg}`,
      )
    } else {
      setFlash(t('inviteSent'))
      setEmail('')
      setNick('')
      setFullName('')
    }
  }

  return (
    <section className="page">
      <h2>{t('inviteTitle')}</h2>
      <p>{t('inviteIntro')}</p>
      {flash && <p className="flash-bar">{flash}</p>}
      <form className="form" onSubmit={submit}>
        <label>
          {t('email')}
          <input
            type="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>
        <label>
          {t('nick')}
          <input
            required
            value={nick}
            onChange={(e) => setNick(e.target.value)}
          />
        </label>
        <label>
          {t('fullName')}
          <input
            required
            value={fullName}
            onChange={(e) => setFullName(e.target.value)}
          />
        </label>
        <button type="submit" className="primary" disabled={inviteState.fetching}>
          {t('sendInvite')}
        </button>
      </form>
    </section>
  )
}
