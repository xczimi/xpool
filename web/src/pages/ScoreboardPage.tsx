import { useMemo, useState } from 'react'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import {
  POOLS_QUERY,
  SCOREBOARD_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type { Motd, Pool, Round, Scoreboard, Tournament } from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { usePolledQuery } from '../lib/usePolledQuery'
import { pollIntervalMs } from '../lib/polling'
import {
  DEFAULT_MULTIPLIERS,
  ROUND_LABELS,
  ROUND_ORDER,
} from '../lib/rounds'

/** Ranked leaderboard, overall + per stage, with pool selector (UC-8). */
export function ScoreboardPage() {
  const { t } = useI18n()
  const [poolId, setPoolId] = useState<string | null>(null)

  const [poolsResult] = useQuery<{ pools: Pool[] }>({ query: POOLS_QUERY })
  const [probe] = useQuery<{ tournament: Tournament | null; motd: Motd | null }>(
    { query: TOURNAMENT_QUERY },
  )
  const interval = useMemo(
    () => pollIntervalMs(probe.data?.tournament?.games ?? []),
    [probe.data],
  )
  const [result, reexecute] = usePolledQuery<{
    scoreboard: Scoreboard | null
  }>({ query: SCOREBOARD_QUERY, variables: { pool: poolId } }, interval)

  const scoreboard = result.data?.scoreboard ?? null
  const pools = poolsResult.data?.pools ?? []

  const multiplierFor = useMemo(() => {
    const map = new Map<Round, number>()
    for (const m of scoreboard?.multipliers ?? []) {
      map.set(m.round, m.multiplier)
    }
    return (r: Round) => map.get(r) ?? DEFAULT_MULTIPLIERS[r]
  }, [scoreboard])

  if (result.fetching && !scoreboard) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!scoreboard) return <ErrorView />

  const ranked = [...scoreboard.entries].sort((a, b) => b.total - a.total)

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
                  {t('multiplier')} ×{multiplierFor(r)}
                </small>
              </th>
            ))}
            <th>{t('total')}</th>
          </tr>
        </thead>
        <tbody>
          {ranked.map((entry, i) => {
            const byRound = new Map(
              entry.byRound.map((b) => [b.round, b.points]),
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
