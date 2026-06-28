import { useMemo } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  KNOCKOUT_SCOREBOARD_QUERY,
  POINTS_TIMELINE_QUERY,
  POOLS_QUERY,
  SCOREBOARD_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type {
  PlayerTimeline,
  Pool,
  Round,
  ScoreEntry,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { ScoreboardModeToggle } from '../components/ScoreboardModeToggle'
import { PointsTimelineChart } from '../components/PointsTimelineChart'
import { buildSeries } from '../lib/timeline'
import { usePolledQuery } from '../lib/usePolledQuery'
import { pollIntervalMs } from '../lib/polling'
import {
  readyRounds,
  roundLabel,
  ROUND_ORDER,
  STAGE_MULTIPLIERS,
} from '../lib/rounds'
import { PoolSelector } from '../pools/PoolSelector'
import { useSelectedPool } from '../pools/useSelectedPool'
import { effectiveSelectedPool } from '../lib/selectedPool'

type ScoreboardMode = 'overall' | 'knockout'

/**
 * The board itself: one query (overall or knockout, the two aliasing the
 * `scoreboard` field so the render is mode-agnostic) plus the ranked table.
 *
 * Rendered with `key={mode}` by the page so switching Overall ⇄ Knockout-only
 * REMOUNTS it. Without a fresh mount the same urql `useQuery` keeps the previous
 * board's data while the new query is in flight, so the OVERALL totals would
 * render under the KNOCKOUT-only headers for a tick (group-stage points still
 * counted). Remounting drops straight to the loading state until the correct
 * board arrives. (A `key` on the route element itself does not work — react
 * router supplies its own element key, so it never remounts.)
 */
function ScoreboardBoard({
  mode,
  pool,
  interval,
  ready,
}: {
  mode: ScoreboardMode
  pool: string | null
  interval: number
  ready: Set<Round>
}) {
  const { t } = useI18n()
  // Both queries return `{ scoreboard: ScoreEntry[] }` — the knockout query
  // aliases the field — so the rest of the component is mode-agnostic.
  const query = mode === 'knockout' ? KNOCKOUT_SCOREBOARD_QUERY : SCOREBOARD_QUERY
  const [result, reexecute] = usePolledQuery<{
    scoreboard: ScoreEntry[]
  }>({ query, variables: { pool } }, interval)

  const scoreboard = result.data?.scoreboard ?? null

  // Only show round columns whose teams are known — a future round with no
  // game determined yet (knockouts before the bracket resolves) is hidden,
  // mirroring the My Tips / All Tips round tabs. GROUP_STAGE is always ready,
  // but it is dropped in knockout mode (it never contributes there).
  const visibleRounds = ROUND_ORDER.filter(
    (r) => ready.has(r) && (mode === 'overall' || r !== 'GROUP_STAGE'),
  )

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
          const byRound = new Map(entry.stages.map((s) => [s.round, s.points]))
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
  )
}

/**
 * Ranked leaderboard, overall + per stage, with pool selector (UC-8).
 * `mode` picks the board: `overall` (group + knockout) or `knockout` (knockout
 * matches only, summed fresh from zero — a re-engagement view). The mode is
 * route-driven (`/scoreboard` vs `/scoreboard/knockout`) so the knockout board
 * is directly linkable.
 */
export function ScoreboardPage({ mode = 'overall' }: { mode?: ScoreboardMode }) {
  const { t, locale } = useI18n()
  const { label } = useAuth()
  // Sticky, cross-page pool selection (see SelectedPoolProvider): `undefined`
  // = not chosen → default to the first pool; `null` = explicit "everyone";
  // a string = a specific pool.
  const { selected } = useSelectedPool()

  // `pools` requires authentication (API.md §8) — the scoreboard itself is
  // public, so the pool selector is only populated for a logged-in player.
  // Issuing `pools` as a visitor would surface an auth error on a public page.
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = poolsResult.data?.pools ?? []
  // Default to the first pool the player belongs to; global stays reachable.
  const effectivePool = effectiveSelectedPool(
    selected,
    pools.map((p) => p.id),
  )

  // The all-pool points trajectory — one line per member of the selected pool,
  // pool-scoped like the board. Mode-independent (group + knockout points), so
  // it lives on the page, not inside the remounting board.
  const [timelineResult] = useQuery<{ pointsTimeline: PlayerTimeline[] }>({
    query: POINTS_TIMELINE_QUERY,
    variables: { pool: effectivePool },
  })
  const trajectory = useMemo(
    () => buildSeries(timelineResult.data?.pointsTimeline ?? [], null),
    [timelineResult.data],
  )

  const [probe] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const interval = useMemo(
    () => pollIntervalMs(probe.data?.tournament?.games ?? []),
    [probe.data],
  )
  const ready = useMemo(
    () =>
      readyRounds(
        probe.data?.tournament?.groups ?? [],
        probe.data?.tournament?.games ?? [],
      ),
    [probe.data],
  )

  const title =
    mode === 'knockout' ? t('scoreboardKnockoutTitle') : t('scoreboardTitle')

  return (
    <section className="page">
      <h2>{title}</h2>
      {interval > 0 && <p className="poll-note">● live</p>}

      <ScoreboardModeToggle />
      <PoolSelector pools={pools} />

      {/* `key={mode}` → a clean remount on Overall ⇄ Knockout-only (see
          `ScoreboardBoard`), so stale totals never flash under the new header. */}
      <ScoreboardBoard
        key={mode}
        mode={mode}
        pool={effectivePool}
        interval={interval}
        ready={ready}
      />

      <PointsTimelineChart
        title={t('timelineTitle')}
        locale={locale}
        emptyLabel={t('timelineEmpty')}
        series={trajectory}
      />
    </section>
  )
}
