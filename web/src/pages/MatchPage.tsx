import { useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { MATCH_QUERY, ME_QUERY, POOLS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { MatchDetail, Me, Pool, Tournament } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { Matchup } from '../components/TeamLabel'
import { PointsBadge } from '../components/PointsBadge'
import { PredictionStats } from '../components/PredictionStats'
import { teamIndex, formatKickoff } from '../lib/format'
import {
  sortRows,
  nextSort,
  readMatchSort,
  writeMatchSort,
  type MatchSort,
  type MatchSortColumn,
} from '../lib/matchSort'
import { STAGE_MULTIPLIERS } from '../lib/rounds'
import { computeWhatIf } from '../lib/whatIf'
import { WhatIfCell } from '../components/WhatIfCell'

/**
 * Match page (#2). The all-players tip grid is the spine in every state; the
 * live/official score and provisional points are an overlay on top. Polls
 * every 60s only while the match is live (`actual.provisional`).
 */
export function MatchPage() {
  const { gameId = '' } = useParams()
  const { t, locale } = useI18n()
  const { label } = useAuth()

  // Pool scoping mirrors the scoreboard: `undefined` = default to the player's
  // first pool, `null` = the explicit "everyone" global view, a string = a pool.
  const [poolId, setPoolId] = useState<string | null | undefined>(undefined)
  const [sort, setSort] = useState<MatchSort>(() => readMatchSort())
  const applySort = (column: MatchSortColumn) => {
    const next = nextSort(sort, column)
    setSort(next)
    writeMatchSort(next)
  }
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = poolsResult.data?.pools ?? []
  // integration: swap for shared sticky pool context
  const effectivePool = poolId === undefined ? (pools[0]?.id ?? null) : poolId

  // The viewer's own player id — needed to exclude the always-visible own row
  // from the stats visibility gate. Sourced from `me` (useAuth exposes only a
  // display label, not the player id).
  const [meResult] = useQuery<{ me: Me }>({ query: ME_QUERY, pause: !label })
  const meRaw = meResult.data?.me ?? null
  const viewerId = meRaw?.__typename === 'Player' ? meRaw.id : null

  const [tournamentResult] = useQuery<{ tournament: Tournament | null }>({
    query: TOURNAMENT_QUERY,
  })
  const [matchResult, reexecuteMatch] = useQuery<{ match: MatchDetail | null }>({
    query: MATCH_QUERY,
    variables: { gameId, pool: effectivePool },
    pause: !gameId,
  })

  const tournament = tournamentResult.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament?.teams, locale],
  )
  const match = matchResult.data?.match ?? null
  const isLive = match?.actual?.provisional ?? false

  // Last-updated stamp — display only (formatting is allowed to read the wall
  // clock per .specs/TESTING.md §3.3; no behavioural branch reads Date.now()).
  // Adjust-during-render (React's "storing previous value" pattern) so we stamp
  // a fresh time whenever a new `match` response settles, without a setState in
  // an effect. The stamp tracks the data object identity from urql.
  const [lastSeen, setLastSeen] = useState<{
    data: typeof matchResult.data
    at: Date
  } | null>(null)
  if (
    matchResult.data &&
    !matchResult.fetching &&
    matchResult.data !== lastSeen?.data
  ) {
    setLastSeen({ data: matchResult.data, at: new Date() })
  }
  const lastUpdated = lastSeen?.at ?? null

  // Poll only while live. 60s matches the server cache floor — polling faster
  // would only re-read the cache, never hit SportsDB more often.
  useEffect(() => {
    if (!isLive) return
    const id = setInterval(
      () => reexecuteMatch({ requestPolicy: 'network-only' }),
      60_000,
    )
    return () => clearInterval(id)
  }, [isLive, reexecuteMatch])

  if (!label) return <NeedsLogin />
  if (matchResult.fetching || tournamentResult.fetching) return <Loading />
  if (matchResult.error)
    return (
      <ErrorView
        message={matchResult.error.message}
        onRetry={() => reexecuteMatch({ requestPolicy: 'network-only' })}
      />
    )
  if (!match) return <ErrorView message="match not found" />

  const { game, actual, rows } = match

  // The stats gate mirrors the server: a NON-own row with a visible prediction
  // means the server has revealed others' tips (deadline passed / kickoff). The
  // viewer's own prediction is always visible, so it is excluded here. No
  // Date.now() — the gate is entirely server-derived (the row's `prediction`).
  const gateOpen = rows.some(
    (r) => r.playerId !== viewerId && r.prediction != null,
  )

  const sortedRows = sortRows(rows, sort)
  // Points are only sortable once at least one row has been scored.
  const pointsSortable = rows.some((r) => r.points != null)
  const ariaSort = (
    column: MatchSortColumn,
  ): 'ascending' | 'descending' | 'none' =>
    sort.column === column
      ? sort.direction === 'asc'
        ? 'ascending'
        : 'descending'
      : 'none'

  // What-if is live-only and gated: show it once tips are revealable AND the
  // match is live (provisional). `liveActual` narrows `actual` to non-null so
  // the re-scoring below is type-safe.
  const liveActual = isLive && actual ? actual : null
  const showWhatIf = liveActual != null && gateOpen
  // The round multiplier for this match comes from its leaf group's round.
  const group = tournament?.groups.find((g) => g.id === game.groupId) ?? null
  const multiplier = group ? STAGE_MULTIPLIERS[group.round] : 1

  return (
    <section className="page match-page">
      <h2>{t('match')}</h2>

      {pools.length > 0 && (
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
      )}

      <div className="match-card">
        <div className="match-card-teams">
          <Matchup home={game.home} away={game.away} teams={teams} />
        </div>
        <div className="match-card-kickoff">
          {formatKickoff(game.kickoff, locale)}
        </div>
        {game.venue && (
          <div className="match-card-venue">
            {t('venue')}: {game.venue}
          </div>
        )}
        <div className="match-card-open-group">
          <Link to={`/mytips/${game.groupId}`}>{t('openGroup')}</Link>
        </div>

        <div className="match-refresh">
          <button
            type="button"
            className="refresh-btn"
            onClick={() => reexecuteMatch({ requestPolicy: 'network-only' })}
            disabled={matchResult.fetching}
          >
            {matchResult.fetching ? t('refreshing') : t('refreshNow')}
          </button>
          {matchResult.fetching && (
            <span className="refresh-spinner" aria-hidden="true" />
          )}
          {lastUpdated && (
            <span className="last-updated">
              {t('lastUpdated')} {lastUpdated.toLocaleTimeString(locale)}
            </span>
          )}
        </div>

        {actual ? (
          <>
            <div
              className={`match-scoreline ${actual.provisional ? 'is-live' : 'is-final'}`}
            >
              <span className="match-scoreline-value">
                {actual.homeScore}–{actual.awayScore}
              </span>
              <span className="match-scoreline-label">
                {actual.provisional
                  ? `${t('liveLabel')}${actual.sourceStatus ? ` · ${actual.sourceStatus}` : ''}`
                  : t('finalLabel')}
              </span>
            </div>
            {actual.provisional && (
              <p className="match-note match-provisional">{t('provisionalLabel')}</p>
            )}
            {actual.provisional && actual.ninetyMinuteUncertain && (
              <p className="match-note match-warn">{t('ninetyMinuteNote')}</p>
            )}
          </>
        ) : (
          game.resultPending && (
            <p className="match-note match-muted">{t('awaitingResult')}</p>
          )
        )}
      </div>

      {gateOpen ? (
        <PredictionStats rows={rows} actual={actual} />
      ) : (
        <p className="match-note match-muted prediction-stats-hidden">
          {t('predictionStatsHidden')}
        </p>
      )}

      <table className="data-table compact match-grid">
        <thead>
          <tr>
            <th
              className={`sortable${sort.column === 'player' ? ' active' : ''}`}
              aria-sort={ariaSort('player')}
              onClick={() => applySort('player')}
            >
              {t('player')}
            </th>
            <th
              className={`sortable${sort.column === 'prediction' ? ' active' : ''}`}
              aria-sort={ariaSort('prediction')}
              onClick={() => applySort('prediction')}
            >
              {t('prediction')}
            </th>
            <th
              className={`num sortable${pointsSortable ? '' : ' disabled'}${
                sort.column === 'points' ? ' active' : ''
              }`}
              aria-sort={pointsSortable ? ariaSort('points') : 'none'}
              onClick={pointsSortable ? () => applySort('points') : undefined}
            >
              {t('points')}
            </th>
            {showWhatIf && (
              <>
                <th className="num what-if-col" title={t('whatIfHint')}>
                  {t('ifHomeScores')}
                </th>
                <th className="num what-if-col" title={t('whatIfHint')}>
                  {t('ifAwayScores')}
                </th>
              </>
            )}
          </tr>
        </thead>
        <tbody>
          {sortedRows.map((row) => {
            const whatIf =
              liveActual && row.prediction
                ? computeWhatIf(row.prediction, liveActual, multiplier)
                : null
            return (
              <tr
                key={row.playerId}
                className={row.playerId === viewerId ? 'is-self' : undefined}
              >
                <td className="nick">
                  {row.nick}
                  {row.playerId === viewerId && (
                    <span className="you-badge">{t('youBadge')}</span>
                  )}
                </td>
                <td className="pred">
                  {row.prediction ? (
                    `${row.prediction.homeScore}–${row.prediction.awayScore}`
                  ) : (
                    <span className="match-hidden">{t('hiddenTip')}</span>
                  )}
                </td>
                <td className="pts num">
                  {row.points != null ? (
                    <PointsBadge
                      breakdown={row.breakdown}
                      points={row.points}
                      isPerfect={row.isPerfect}
                    />
                  ) : (
                    '—'
                  )}
                  {row.maxReachable != null && (
                    <span
                      className="max-reachable"
                      title={t('maxReachableTooltip')}
                    >
                      {t('maxReachableShort')} ≤ {row.maxReachable}
                    </span>
                  )}
                </td>
                {showWhatIf && (
                  <>
                    <td className="num what-if-cell">
                      {whatIf ? (
                        <WhatIfCell outcome={whatIf.ifHome} />
                      ) : (
                        <span className="match-hidden">{t('hiddenTip')}</span>
                      )}
                    </td>
                    <td className="num what-if-cell">
                      {whatIf ? (
                        <WhatIfCell outcome={whatIf.ifAway} />
                      ) : (
                        <span className="match-hidden">{t('hiddenTip')}</span>
                      )}
                    </td>
                  </>
                )}
              </tr>
            )
          })}
        </tbody>
      </table>
    </section>
  )
}
