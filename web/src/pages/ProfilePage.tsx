import { useState } from 'react'
import { useMutation, useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, UPDATE_PROFILE_MUTATION } from '../graphql/queries'
import type { Player } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'

/** Profile / account settings (UC-4). */
export function ProfilePage() {
  const { t } = useI18n()
  const { playerId } = useAuth()
  const [meResult] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !playerId,
  })

  const me = meResult.data?.me ?? null

  if (!playerId) return <NeedsLogin />
  if (meResult.fetching) return <Loading />
  if (meResult.error) return <ErrorView message={meResult.error.message} />
  if (!me) return <ErrorView />

  // `key` on the form means it re-initialises if the loaded player changes —
  // form state is then seeded by lazy `useState`, no effect needed.
  return (
    <section className="page">
      <h2>{t('profileTitle')}</h2>
      <ProfileForm key={me.id} me={me} />
    </section>
  )
}

function ProfileForm({ me }: { me: Player }) {
  const { t } = useI18n()
  const [updateState, updateProfile] = useMutation(UPDATE_PROFILE_MUTATION)

  const [nick, setNick] = useState(me.nick)
  const [fullName, setFullName] = useState(me.fullName)
  const [email, setEmail] = useState(me.email ?? '')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [flash, setFlash] = useState<string | null>(null)

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setFlash(null)
    if (password && password !== confirm) {
      setFlash(t('passwordMismatch'))
      return
    }
    const input: Record<string, string> = { nick, fullName, email }
    if (password) input.password = password
    const res = await updateProfile({ input })
    if (res.error) {
      setFlash(`${t('errorPrefix')}: ${res.error.message}`)
    } else {
      setFlash(t('profileSaved'))
      setPassword('')
      setConfirm('')
    }
  }

  return (
    <>
      {flash && <p className="flash-bar">{flash}</p>}
      <form className="form" onSubmit={submit}>
        <label>
          {t('nick')}
          <input value={nick} onChange={(e) => setNick(e.target.value)} />
        </label>
        <label>
          {t('fullName')}
          <input
            value={fullName}
            onChange={(e) => setFullName(e.target.value)}
          />
        </label>
        <label>
          {t('email')}
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>
        <label>
          {t('password')}
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        <label>
          {t('passwordConfirm')}
          <input
            type="password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
          />
        </label>
        <button type="submit" className="primary" disabled={updateState.fetching}>
          {t('save')}
        </button>
      </form>
    </>
  )
}
