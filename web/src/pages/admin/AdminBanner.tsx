import { useState } from 'react'
import { useMutation, useQuery } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import { SET_MOTD_MUTATION, TOURNAMENT_QUERY } from '../../graphql/queries'
import type { Motd, Tournament } from '../../graphql/types'
import { Loading } from '../../components/StatusViews'

/** Admin banner / message-of-the-day editor (UC-16). */
export function AdminBanner() {
  const { t } = useI18n()
  const [result] = useQuery<{ tournament: Tournament | null; motd: Motd | null }>(
    { query: TOURNAMENT_QUERY },
  )

  if (result.fetching && !result.data) return <Loading />

  // `key` re-initialises the editor when the loaded banner changes; the
  // textarea state is seeded by lazy `useState`, no effect needed.
  const initial = result.data?.motd?.text ?? ''
  return (
    <div>
      <h3>{t('adminBanner')}</h3>
      <BannerEditor key={initial} initialText={initial} />
    </div>
  )
}

function BannerEditor({ initialText }: { initialText: string }) {
  const { t } = useI18n()
  const [setState, setMotd] = useMutation(SET_MOTD_MUTATION)
  const [text, setText] = useState(initialText)
  const [flash, setFlash] = useState<string | null>(null)

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setFlash(null)
    const res = await setMotd({ text })
    setFlash(
      res.error ? `${t('errorPrefix')}: ${res.error.message}` : t('bannerSet'),
    )
  }

  return (
    <>
      {flash && <p className="flash-bar">{flash}</p>}
      <form className="form" onSubmit={submit}>
        <label>
          {t('bannerText')}
          <textarea
            value={text}
            rows={3}
            onChange={(e) => setText(e.target.value)}
          />
        </label>
        <button type="submit" className="primary" disabled={setState.fetching}>
          {t('setBanner')}
        </button>
      </form>
    </>
  )
}
