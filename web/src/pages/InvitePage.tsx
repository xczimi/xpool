import { useState } from 'react'
import { useMutation, useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'
import { CREATE_INVITE_MUTATION, POOLS_QUERY } from '../graphql/queries'
import type { Pool } from '../graphql/types'
import { Loading, NeedsLogin } from '../components/StatusViews'

/**
 * Share your invite into a pool you belong to. Every invite is pool-bound and
 * reusable, so re-generating returns the same link. (The same action lives
 * per-pool on the Pools page; this is the standalone entry point.)
 */
export function InvitePage() {
  const { t } = useI18n()
  const { label } = useAuth()
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const [result, run] = useMutation(CREATE_INVITE_MUTATION)
  const [poolId, setPoolId] = useState('')

  if (!label) return <NeedsLogin />
  if (poolsResult.fetching && !poolsResult.data) return <Loading />

  const pools = poolsResult.data?.pools ?? []
  const selected = poolId || pools[0]?.id || ''
  const link = result.data?.createInvite?.link as string | undefined

  return (
    <main className="content">
      <h2>{t('shareInvite')}</h2>
      {pools.length === 0 ? (
        <p>{t('noPools')}</p>
      ) : (
        <>
          <label className="pool-selector">
            {t('poolsTitle')}:{' '}
            <select value={selected} onChange={(e) => setPoolId(e.target.value)}>
              {pools.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <button onClick={() => void run({ pool: selected })}>
            {t('shareInvite')}
          </button>
          {result.error && (
            <p className="flash-bar">
              {t('errorPrefix')}: {result.error.message}
            </p>
          )}
          {link && (
            <div className="invite-link">
              <label>
                {t('inviteLinkLabel')}
                <textarea
                  readOnly
                  value={link}
                  onFocus={(e) => e.currentTarget.select()}
                />
              </label>
              <button onClick={() => void navigator.clipboard.writeText(link)}>
                {t('copyLink')}
              </button>
            </div>
          )}
        </>
      )}
    </main>
  )
}
