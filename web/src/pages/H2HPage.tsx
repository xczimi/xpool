import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  POOLS_QUERY,
  SCOREBOARD_QUERY,
  TIPS_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type { Pool, Round, ScoreEntry, Tip, Tournament } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { PoolSelector } from '../pools/PoolSelector'
import { useSelectedPool } from '../pools/useSelectedPool'
import { effectiveSelectedPool } from '../lib/selectedPool'
import {
  ROUND_ORDER,
  currentRoundNode,
  leafGroupsOfRound,
  readyRounds,
  roundLabel,
  visibleRoundNodes,
} from '../lib/rounds'
import { cumulativeSeries } from '../lib/cumulativePoints'
import { h2hSummary, matchDiffs, roundDeltas } from '../lib/headToHead'
import type { ScoreCell } from '../lib/headToHead'
import { PointsTimelineChart } from '../components/PointsTimelineChart'
import { TIMELINE_COLORS } from '../components/timelineColors'

/**
 * Head-to-head: two players compared within the selected pool. All data is
 * reused client-side — SCOREBOARD_QUERY for totals/positions and the per-round
 * trajectory, TIPS_QUERY for the per-match breakdown (its hidden-until-
 * revealable gating is server-applied). No new resolver. The route params are
 * the same clean player handles the scoreboard links use.
 */
export function H2HPage() {
  const { a = '', b = '' } = useParams<{ a: string; b: string }>()
  const { t } = useI18n()
  const { label } = useAuth()
  const { selected } = useSelectedPool()
  const [selectedRound, setSelectedRound] = useState<Round | null>(null)

  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = poolsResult.data?.pools ?? []
  const effectivePool = effectiveSelectedPool(
    selected,
    pools.map((p) => p.id),
  )

  const [scoreboardResult] = useQuery<{ scoreboard: ScoreEntry[] }>({
    query: SCOREBOARD_QUERY,
    variables: { pool: effectivePool },
  })
  const [tournamentResult] = useQuery<{ tournament: Tournament | null }>({
    query: TOURNAMENT_QUERY,
  })

  const scoreboard = scoreboardResult.data?.scoreboard ?? []
  const tournament = tournamentResult.data?.tournament ?? null

  // Per-match round selector (mirrors All Tips): group stage queries one leaf
  // group; a knockout round queries the round node — the tips resolver walks
  // its subtree. Default to the current round.
  const roundNodes = visibleRoundNodes(
    tournament?.groups ?? [],
    tournament?.games ?? [],
  )
  const activeRound =
    selectedRound ??
    currentRoundNode(roundNodes)?.round ??
    roundNodes[0]?.round ??
    null
  const activeRoundNode = roundNodes.find((r) => r.round === activeRound) ?? null
  const isGroupStage = activeRound === 'GROUP_STAGE'
  const roundLeaves = activeRoundNode
    ? leafGroupsOfRound(activeRoundNode, tournament?.groups ?? [])
    : []
  const tipsGroupId = isGroupStage
    ? (roundLeaves[0]?.id ?? null)
    : (activeRoundNode?.id ?? null)

  const [tipsResult] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId: tipsGroupId, pool: effectivePool },
    pause: !label || !tipsGroupId,
  })

  if (!label) return <NeedsLogin />
  if (scoreboardResult.fetching || tournamentResult.fetching) return <Loading />
  if (scoreboardResult.error)
    return <ErrorView message={scoreboardResult.error.message} />
  if (!tournament) return <ErrorView />

  const summary = h2hSummary(scoreboard, a, b)
  if (!summary) {
    return (
      <section className="page">
        <p>{t('playerNotInPool')}</p>
      </section>
    )
  }

  const ready = readyRounds(tournament.groups, tournament.games)
  const rounds = ROUND_ORDER.filter((r) => ready.has(r))
  const series = [
    {
      label: summary.a.nick,
      color: TIMELINE_COLORS[0],
      points: cumulativeSeries(summary.a, rounds),
    },
    {
      label: summary.b.nick,
      color: TIMELINE_COLORS[1],
      points: cumulativeSeries(summary.b, rounds),
    },
  ]
  const deltas = roundDeltas(summary.a, summary.b, rounds)
  const diffs = matchDiffs(tipsResult.data?.tips ?? [], a, b)

  const cell = (pred: ScoreCell | null, hidden: boolean) =>
    hidden ? t('hiddenTip') : pred ? `${pred.homeScore}–${pred.awayScore}` : '—'

  return (
    <section className="page h2h-page">
      <h2>{t('h2hTitle')}</h2>
      <PoolSelector pools={pools} />

      <div className="h2h-summary">
        <div className="h2h-stat">
          <Link to={`/player/${summary.a.playerId}`}>{summary.a.nick}</Link>
          <span className="h2h-stat-value">{summary.a.total}</span>
          <span className="h2h-stat-rank">#{summary.rankA ?? '—'}</span>
        </div>
        <div className="h2h-stat h2h-delta">
          <span className="h2h-stat-label">{t('h2hTotalDelta')}</span>
          <span className="h2h-stat-value">
            {summary.totalDelta > 0 ? '+' : ''}
            {summary.totalDelta}
          </span>
        </div>
        <div className="h2h-stat">
          <Link to={`/player/${summary.b.playerId}`}>{summary.b.nick}</Link>
          <span className="h2h-stat-value">{summary.b.total}</span>
          <span className="h2h-stat-rank">#{summary.rankB ?? '—'}</span>
        </div>
      </div>

      <PointsTimelineChart
        title={t('timelineTitle')}
        xLabels={rounds.map((r) => roundLabel(r, t))}
        series={series}
      />

      <table className="data-table h2h-delta-table">
        <thead>
          <tr>
            <th>{t('h2hRoundLabel')}</th>
            <th>{summary.a.nick}</th>
            <th>{summary.b.nick}</th>
            <th>Δ</th>
          </tr>
        </thead>
        <tbody>
          {deltas.map((d) => (
            <tr key={d.round}>
              <td>{roundLabel(d.round, t)}</td>
              <td>{d.pointsA}</td>
              <td>{d.pointsB}</td>
              <td>
                {d.delta > 0 ? '+' : ''}
                {d.delta}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>{t('h2hPerMatch')}</h3>
      <label className="h2h-round-select">
        {t('h2hRoundLabel')}{' '}
        <select
          value={activeRound ?? ''}
          onChange={(e) => setSelectedRound(e.target.value as Round)}
        >
          {roundNodes.map((node) => (
            <option key={node.round} value={node.round}>
              {roundLabel(node.round, t)}
            </option>
          ))}
        </select>
      </label>
      {diffs.length === 0 ? (
        <p className="h2h-no-diffs">{t('h2hNoDiffs')}</p>
      ) : (
        <table className="data-table h2h-match-table">
          <thead>
            <tr>
              <th>{t('h2hMatch')}</th>
              <th>{summary.a.nick}</th>
              <th>{summary.b.nick}</th>
            </tr>
          </thead>
          <tbody>
            {diffs.map((m) => (
              <tr key={m.gameId}>
                <td>{m.gameId}</td>
                <td className={m.hiddenA ? 'h2h-hidden' : undefined}>
                  {cell(m.predA, m.hiddenA)}
                  {m.pointsA != null && ` (${m.pointsA})`}
                </td>
                <td className={m.hiddenB ? 'h2h-hidden' : undefined}>
                  {cell(m.predB, m.hiddenB)}
                  {m.pointsB != null && ` (${m.pointsB})`}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  )
}
