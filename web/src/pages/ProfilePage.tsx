import { useState } from 'react'
import { Link } from 'react-router-dom'
import { useMutation, useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, UPDATE_PROFILE_MUTATION } from '../graphql/queries'
import type { Me, Player } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { NameForm } from '../components/NameForm'

/** Profile / account settings (UC-4). */
export function ProfilePage() {
  const { t } = useI18n()
  const { label } = useAuth()
  const [meResult] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !label,
  })

  const meRaw = meResult.data?.me ?? null
  const me = meRaw?.__typename === 'Player' ? meRaw : null

  if (!label) return <NeedsLogin />
  if (meResult.fetching) return <Loading />
  if (meResult.error) return <ErrorView message={meResult.error.message} />
  if (!me) return <ErrorView />

  // `key` on the form means it re-initialises if the loaded player changes —
  // form state is then seeded by lazy `useState`, no effect needed.
  return (
    <section className="page">
      <h2>{t('profileTitle')}</h2>
      <ProfileForm key={me.id} me={me} />
      <p>
        <Link to={`/player/${me.id}`}>{t('playerPageOwnLink')}</Link>
      </p>
    </section>
  )
}

function ProfileForm({ me }: { me: Player }) {
  const { t } = useI18n()
  const [updateState, updateProfile] = useMutation(UPDATE_PROFILE_MUTATION)
  const [flash, setFlash] = useState<string | null>(null)
  return (
    <NameForm
      initialNick={me.nick}
      initialFullName={me.fullName}
      submitLabel={t('save')}
      busy={updateState.fetching}
      flash={flash}
      onSubmit={async (nick, fullName) => {
        setFlash(null)
        const res = await updateProfile({ nick, fullName })
        setFlash(res.error ? `${t('errorPrefix')}: ${res.error.message}` : t('profileSaved'))
      }}
    />
  )
}
