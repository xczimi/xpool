import { useMemo, useState } from 'react'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  POOLS_QUERY,
  SCOREBOARD_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type {
  Pool,
  ScoreEntry,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { usePolledQuery } from '../lib/usePolledQuery'
import { pollIntervalMs } from '../lib/polling'
import { ROUND_LABELS, ROUND_ORDER, STAGE_MULTIPLIERS } from '../lib/rounds'

/** Ranked leaderboard, overall + per stage, with pool selector (UC-8). */
export function ScoreboardPage() {
  const { t } = useI18n()
  const { playerId } = useAuth()
  const [poolId, setPoolId] = useState<string | null>(null)

  // `pools` requires authentication (API.md §8) — the scoreboard itself is
  // public, so the pool selector is only populated for a logged-in player.
  // Issuing `pools` as a visitor would surface an auth error on a public page.
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !playerId,
  })
  const [probe] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const interval = useMemo(
    () => pollIntervalMs(probe.data?.tournament?.games ?? []),
    [probe.data],
  )
  const [result, reexecute] = usePolledQuery<{
    scoreboard: ScoreEntry[]
  }>({ query: SCOREBOARD_QUERY, variables: { pool: poolId } }, interval)

  const scoreboard = result.data?.scoreboard ?? null
  const pools = poolsResult.data?.pools ?? []

  if (result.fetching && !scoreboard) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!scoreboard) return <ErrorView />

  const ranked = [...scoreboard].sort((a, b) => b.total - a.total)

  return (
    <section className="page">
      <h2>{t('scoreboardTitle')}</h2>
      {interval > 0 && <p className="poll-note">● live</p>}

      <label className="pool-selector">
        {t('pool')}:{' '}
        <select
          value={poolId ?? ''}
          onChange={(e) => setPoolId(e.target.value || null)}
        >
          <option value="">{t('everyone')}</option>
          {pools.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>

      <table className="data-table">
        <thead>
          <tr>
            <th>{t('rank')}</th>
            <th>{t('player')}</th>
            {ROUND_ORDER.map((r) => (
              <th key={r}>
                {ROUND_LABELS[r]}
                <br />
                <small>
                  {t('multiplier')} ×{STAGE_MULTIPLIERS[r]}
                </small>
              </th>
            ))}
            <th>{t('total')}</th>
          </tr>
        </thead>
        <tbody>
          {ranked.map((entry, i) => {
            const byRound = new Map(
              entry.stages.map((s) => [s.round, s.points]),
            )
            return (
              <tr key={entry.playerId}>
                <td>{i + 1}</td>
                <td>{entry.nick}</td>
                {ROUND_ORDER.map((r) => (
                  <td key={r}>{byRound.get(r) ?? 0}</td>
                ))}
                <td>
                  <strong>{entry.total}</strong>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </section>
  )
}
