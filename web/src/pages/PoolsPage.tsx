import { useState } from 'react'
import { useMutation, useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  CREATE_POOL_MUTATION,
  DELETE_POOL_MUTATION,
  JOIN_POOL_MUTATION,
  LEAVE_POOL_MUTATION,
  POOLS_QUERY,
  REMOVE_MEMBER_MUTATION,
  ROTATE_JOIN_CODE_MUTATION,
  UPDATE_POOL_MUTATION,
} from '../graphql/queries'
import type { Pool } from '../graphql/types'
import { Loading, NeedsLogin } from '../components/StatusViews'

/**
 * Custom pools (SCENARIOS.md §5) — scoreboard-scoping groups joined by an
 * opaque join code. A player sees only the pools they own or belong to.
 */
export function PoolsPage() {
  const { t } = useI18n()
  const { label } = useAuth()
  // `pools` requires authentication — pause it for a visitor so a logged-out
  // render does not fire an auth-error query (mirrors ScoreboardPage).
  const [poolsResult, refetchPools] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })

  const [, createPool] = useMutation(CREATE_POOL_MUTATION)
  const [, joinPool] = useMutation(JOIN_POOL_MUTATION)
  const [, leavePool] = useMutation(LEAVE_POOL_MUTATION)
  const [, removeMember] = useMutation(REMOVE_MEMBER_MUTATION)
  const [, rotateCode] = useMutation(ROTATE_JOIN_CODE_MUTATION)
  const [, deletePool] = useMutation(DELETE_POOL_MUTATION)
  const [, updatePool] = useMutation(UPDATE_POOL_MUTATION)

  const [newName, setNewName] = useState('')
  const [joinCode, setJoinCode] = useState('')
  const [flash, setFlash] = useState<string | null>(null)

  if (!label) return <NeedsLogin />
  if (poolsResult.fetching && !poolsResult.data) return <Loading />

  const pools = poolsResult.data?.pools ?? []
  const refresh = () => refetchPools({ requestPolicy: 'network-only' })

  /** Run a mutation, surface success/failure, and refresh the list. */
  const act = async (
    op: Promise<{ error?: { message: string } }>,
    okKey: Parameters<typeof t>[0],
  ) => {
    setFlash(null)
    const res = await op
    if (res.error) {
      setFlash(`${t('errorPrefix')}: ${res.error.message}`)
    } else {
      setFlash(t(okKey))
      refresh()
    }
  }

  const onCreate = (e: React.FormEvent) => {
    e.preventDefault()
    const name = newName.trim()
    if (!name) return
    act(createPool({ id: crypto.randomUUID(), name }), 'poolCreated')
    setNewName('')
  }

  const onJoin = (e: React.FormEvent) => {
    e.preventDefault()
    const code = joinCode.trim()
    if (!code) return
    act(joinPool({ joinCode: code }), 'poolJoined')
    setJoinCode('')
  }

  return (
    <section className="page">
      <h2>{t('poolsTitle')}</h2>
      <p>{t('poolsIntro')}</p>
      {flash && <p className="flash-bar">{flash}</p>}

      <div className="pool-forms">
        <form className="form" onSubmit={onCreate}>
          <label>
            {t('poolName')}
            <input
              required
              placeholder={t('poolNamePlaceholder')}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
          </label>
          <button type="submit" className="primary" disabled={!newName.trim()}>
            {t('createPool')}
          </button>
        </form>

        <form className="form" onSubmit={onJoin}>
          <label>
            {t('joinCodeLabel')}
            <input
              required
              placeholder={t('joinCodePlaceholder')}
              value={joinCode}
              onChange={(e) => setJoinCode(e.target.value)}
            />
          </label>
          <button type="submit" className="primary" disabled={!joinCode.trim()}>
            {t('joinPool')}
          </button>
        </form>
      </div>

      {pools.length === 0 ? (
        <p>{t('noPools')}</p>
      ) : (
        <ul className="pool-list">
          {pools.map((pool) => {
            const isOwner = pool.owner === label
            return (
              <li key={pool.id} className="pool-card">
                <h3>
                  {pool.name}
                  {isOwner && <span className="owner-tag"> · {t('ownerTag')}</span>}
                </h3>
                <p className="join-code">
                  {t('joinCodeLabel')}: <code>{pool.joinCode}</code>
                </p>
                <ul className="pool-members">
                  {pool.members.map((m) => (
                    <li key={m}>
                      {m === pool.owner ? <strong>{m}</strong> : m}
                      {isOwner && m !== pool.owner && (
                        <button
                          type="button"
                          className="link-button"
                          onClick={() =>
                            act(
                              removeMember({ poolId: pool.id, memberId: m }),
                              'memberRemoved',
                            )
                          }
                        >
                          {t('removeMember')}
                        </button>
                      )}
                    </li>
                  ))}
                </ul>
                <div className="pool-actions">
                  {isOwner ? (
                    <>
                      <button
                        type="button"
                        onClick={() => {
                          const name = window.prompt(t('renamePrompt'), pool.name)
                          if (name?.trim()) {
                            act(
                              updatePool({ id: pool.id, name: name.trim() }),
                              'poolRenamed',
                            )
                          }
                        }}
                      >
                        {t('renamePool')}
                      </button>
                      <button
                        type="button"
                        onClick={() =>
                          act(rotateCode({ id: pool.id }), 'codeRotated')
                        }
                      >
                        {t('rotateCode')}
                      </button>
                      <button
                        type="button"
                        className="danger"
                        onClick={() => {
                          if (window.confirm(t('deleteConfirm'))) {
                            act(deletePool({ id: pool.id }), 'poolDeleted')
                          }
                        }}
                      >
                        {t('deletePool')}
                      </button>
                    </>
                  ) : (
                    <button
                      type="button"
                      className="danger"
                      onClick={() => act(leavePool({ id: pool.id }), 'poolLeft')}
                    >
                      {t('leavePool')}
                    </button>
                  )}
                </div>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
