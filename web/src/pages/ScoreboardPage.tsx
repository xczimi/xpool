import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
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
import { readyRounds, roundLabel, ROUND_ORDER, STAGE_MULTIPLIERS } from '../lib/rounds'

/** Ranked leaderboard, overall + per stage, with pool selector (UC-8). */
export function ScoreboardPage() {
  const { t } = useI18n()
  const { label } = useAuth()
  // `undefined` = the user has not chosen yet → default to their first pool
  // (the pool board is the default view); `null` = the explicit "everyone"
  // global board; a string = a specific pool.
  const [poolId, setPoolId] = useState<string | null | undefined>(undefined)

  // `pools` requires authentication (API.md §8) — the scoreboard itself is
  // public, so the pool selector is only populated for a logged-in player.
  // Issuing `pools` as a visitor would surface an auth error on a public page.
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = poolsResult.data?.pools ?? []
  // Default to the first pool the player belongs to; global stays reachable.
  const effectivePool = poolId === undefined ? (pools[0]?.id ?? null) : poolId

  const [probe] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const interval = useMemo(
    () => pollIntervalMs(probe.data?.tournament?.games ?? []),
    [probe.data],
  )
  const [result, reexecute] = usePolledQuery<{
    scoreboard: ScoreEntry[]
  }>({ query: SCOREBOARD_QUERY, variables: { pool: effectivePool } }, interval)

  const scoreboard = result.data?.scoreboard ?? null

  // Only show round columns whose teams are known — a future round with no
  // game determined yet (knockouts before the bracket resolves) is hidden,
  // mirroring the My Tips / All Tips round tabs. GROUP_STAGE is always ready.
  const ready = readyRounds(
    probe.data?.tournament?.groups ?? [],
    probe.data?.tournament?.games ?? [],
  )
  const visibleRounds = ROUND_ORDER.filter((r) => ready.has(r))

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
          value={effectivePool ?? ''}
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
            {visibleRounds.map((r) => (
              <th key={r}>
                {roundLabel(r, t)}
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
                <td>
                  <Link to={`/player/${entry.playerId}`}>{entry.nick}</Link>
                </td>
                {visibleRounds.map((r) => (
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
