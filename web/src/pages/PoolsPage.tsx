import { useState } from 'react'
import { useMutation, useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  CREATE_INVITE_MUTATION,
  CREATE_POOL_MUTATION,
  DELETE_POOL_MUTATION,
  JOIN_MUTATION,
  LEAVE_POOL_MUTATION,
  ME_QUERY,
  POOLS_QUERY,
  REMOVE_MEMBER_MUTATION,
  REVOKE_INVITE_MUTATION,
  TRANSFER_OWNERSHIP_MUTATION,
  UPDATE_POOL_MUTATION,
} from '../graphql/queries'
import type { Me, Pool } from '../graphql/types'
import { Loading, NeedsLogin } from '../components/StatusViews'
import { InlineConfirm } from '../components/InlineConfirm'
import { ShareTemplates } from '../components/ShareTemplates'
import { useDisplayName } from '../hooks/useDisplayName'

/**
 * Custom pools (SCENARIOS.md §5) — scoreboard-scoping groups. A member shares a
 * reusable invite (code/link); a player joins by pasting it. A player sees only
 * the pools they own or belong to.
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
  // Ownership is a player-id relation, so compare against the resolved player
  // id — NOT `label`, which is the e-mail under Auth0 and would never match.
  const [meResult] = useQuery<{ me: Me }>({ query: ME_QUERY, pause: !label })
  const me = meResult.data?.me
  const myId = me?.__typename === 'Player' ? me.id : null
  const canCreatePool = me?.__typename === 'Player' && me.mayCreatePool
  // Pool members/owner are bare player ids; resolve them to nicks for display.
  const displayName = useDisplayName()

  const [, createPool] = useMutation(CREATE_POOL_MUTATION)
  const [, join] = useMutation(JOIN_MUTATION)
  const [, leavePool] = useMutation(LEAVE_POOL_MUTATION)
  const [, removeMember] = useMutation(REMOVE_MEMBER_MUTATION)
  const [, createInvite] = useMutation(CREATE_INVITE_MUTATION)
  const [, revokeInvite] = useMutation(REVOKE_INVITE_MUTATION)
  const [, deletePool] = useMutation(DELETE_POOL_MUTATION)
  const [, updatePool] = useMutation(UPDATE_POOL_MUTATION)
  const [, transferOwnership] = useMutation(TRANSFER_OWNERSHIP_MUTATION)

  const [newName, setNewName] = useState('')
  const [joinCode, setJoinCode] = useState('')
  const [flash, setFlash] = useState<string | null>(null)
  // The current member's invite link per pool, revealed by "Share invite".
  const [invites, setInvites] = useState<Record<string, { code: string; link: string }>>({})
  // Inline rename: the pool being renamed (id) and its draft name.
  const [renaming, setRenaming] = useState<string | null>(null)
  const [renameDraft, setRenameDraft] = useState('')
  // Inline hand-over confirmation: the member picked from the select, pending
  // confirmation, per pool.
  const [pendingOwner, setPendingOwner] = useState<{ poolId: string; memberId: string } | null>(null)

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
    act(join({ code }), 'poolJoined')
    setJoinCode('')
  }

  /** Open the inline rename editor for a pool, seeded with its current name. */
  const startRename = (pool: Pool) => {
    setRenaming(pool.id)
    setRenameDraft(pool.name)
  }

  /** Commit the inline rename, if the draft is a non-empty change. */
  const saveRename = (poolId: string) => {
    const name = renameDraft.trim()
    setRenaming(null)
    if (name) {
      act(updatePool({ id: poolId, name }), 'poolRenamed')
    }
  }

  /** Mint/reveal the current member's invite link for a pool. */
  const onShareInvite = async (poolId: string) => {
    setFlash(null)
    const res = await createInvite({ pool: poolId })
    if (res.error) {
      setFlash(`${t('errorPrefix')}: ${res.error.message}`)
      return
    }
    const link = res.data?.createInvite?.link as string | undefined
    const code = res.data?.createInvite?.code as string | undefined
    if (link && code) {
      setInvites((prev) => ({ ...prev, [poolId]: { code, link } }))
      setFlash(t('inviteShared'))
    }
  }

  /** Revoke the revealed invite for a pool, then hide it. */
  const onRevokeInvite = async (poolId: string, code: string) => {
    await act(revokeInvite({ code }), 'inviteRevoked')
    setInvites((prev) => {
      const next = { ...prev }
      delete next[poolId]
      return next
    })
  }

  return (
    <section className="page">
      <h2>{t('poolsTitle')}</h2>
      <p>{t('poolsIntro')}</p>
      {flash && <p className="flash-bar">{flash}</p>}

      <div className="pool-forms">
        {canCreatePool && (
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
        )}

        <form className="form" onSubmit={onJoin}>
          <label>
            {t('inviteCodeLabel')}
            <input
              required
              placeholder={t('inviteCodePlaceholder')}
              value={joinCode}
              onChange={(e) => setJoinCode(e.target.value)}
            />
          </label>
          <button type="submit" className="primary" disabled={!joinCode.trim()}>
            {t('joinAction')}
          </button>
        </form>
      </div>

      {pools.length === 0 ? (
        <p>{t('noPools')}</p>
      ) : (
        <ul className="pool-list">
          {pools.map((pool) => {
            const isOwner = pool.owner === myId
            return (
              <li key={pool.id} className="pool-card">
                <h3>
                  {renaming === pool.id ? (
                    <span className="pool-rename">
                      <input
                        autoFocus
                        aria-label={t('renamePrompt')}
                        value={renameDraft}
                        onChange={(e) => setRenameDraft(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') saveRename(pool.id)
                          if (e.key === 'Escape') setRenaming(null)
                        }}
                      />
                      <button
                        type="button"
                        className="primary"
                        disabled={!renameDraft.trim()}
                        onClick={() => saveRename(pool.id)}
                      >
                        {t('save')}
                      </button>
                      <button
                        type="button"
                        className="link-button"
                        onClick={() => setRenaming(null)}
                      >
                        {t('cancel')}
                      </button>
                    </span>
                  ) : (
                    <>
                      {pool.name}
                      {isOwner && (
                        <span className="owner-tag"> · {t('ownerTag')}</span>
                      )}
                    </>
                  )}
                </h3>
                <ul className="pool-members">
                  {pool.members.map((m) => (
                    <li key={m}>
                      {m === pool.owner ? (
                        <strong>{displayName(m)}</strong>
                      ) : (
                        displayName(m)
                      )}
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
                  <button type="button" onClick={() => void onShareInvite(pool.id)}>
                    {t('shareInvite')}
                  </button>
                  {isOwner ? (
                    <>
                      <button
                        type="button"
                        disabled={renaming === pool.id}
                        onClick={() => startRename(pool)}
                      >
                        {t('renamePool')}
                      </button>
                      {pendingOwner?.poolId === pool.id ? (
                        <span className="inline-confirm" role="group">
                          <span className="inline-confirm-q">
                            {t('handOverConfirm')}{' '}
                            <strong>{displayName(pendingOwner.memberId)}</strong>?
                          </span>
                          <button
                            type="button"
                            className="danger"
                            onClick={() => {
                              act(
                                transferOwnership({
                                  poolId: pool.id,
                                  newOwner: pendingOwner.memberId,
                                }),
                                'ownershipTransferred',
                              )
                              setPendingOwner(null)
                            }}
                          >
                            {t('confirmAction')}
                          </button>
                          <button
                            type="button"
                            className="link-button"
                            onClick={() => setPendingOwner(null)}
                          >
                            {t('cancel')}
                          </button>
                        </span>
                      ) : (
                        pool.members.some((m) => m !== pool.owner) && (
                          <select
                            className="handover-select"
                            aria-label={t('handOverTo')}
                            value=""
                            onChange={(e) => {
                              if (e.target.value) {
                                setPendingOwner({
                                  poolId: pool.id,
                                  memberId: e.target.value,
                                })
                              }
                              e.target.value = ''
                            }}
                          >
                            <option value="" disabled>
                              {t('transferOwnership')}…
                            </option>
                            {pool.members
                              .filter((m) => m !== pool.owner)
                              .map((m) => (
                                <option key={m} value={m}>
                                  {displayName(m)}
                                </option>
                              ))}
                          </select>
                        )
                      )}
                      <InlineConfirm
                        className="danger"
                        question={t('deleteConfirm')}
                        onConfirm={() =>
                          act(deletePool({ id: pool.id }), 'poolDeleted')
                        }
                      >
                        {t('deletePool')}
                      </InlineConfirm>
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
                {invites[pool.id] && (
                  <div className="invite-link">
                    <label>
                      {t('inviteLinkLabel')}
                      <input
                        readOnly
                        value={invites[pool.id].link}
                        onFocus={(e) => e.currentTarget.select()}
                      />
                    </label>
                    <button
                      type="button"
                      onClick={() => {
                        void navigator.clipboard.writeText(invites[pool.id].link)
                        setFlash(t('linkCopied'))
                      }}
                    >
                      {t('copyLink')}
                    </button>
                    <button
                      type="button"
                      className="link-button"
                      onClick={() =>
                        void onRevokeInvite(pool.id, invites[pool.id].code)
                      }
                    >
                      {t('revokeInvite')}
                    </button>
                  </div>
                )}
              </li>
            )
          })}
        </ul>
      )}

      <ShareTemplates />
    </section>
  )
}
