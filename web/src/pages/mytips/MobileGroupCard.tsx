import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import type { OperationResult } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import type {
  GroupGame,
  MatchPrediction,
  Player,
  PointsBreakdown,
  StandingsScore,
  Tournament,
} from '../../graphql/types'
import { byKickoff, teamIndex } from '../../lib/format'
import { computeStandings, applyDrawOrder } from '../../lib/standings'
import { predictedCount } from '../../lib/score'
import { Matchup } from '../../components/TeamLabel'
import { Countdown } from '../../components/Countdown'
import { PointsBadge } from '../../components/PointsBadge'
import { InlineConfirm } from '../../components/InlineConfirm'
import { ScoreStepper } from './ScoreStepper'
import { useDebouncedCallback } from '../../hooks/useDebouncedCallback'
import type { PredictionInput, StandingsInput } from './types'

interface Cell {
  home: number | null
  away: number | null
}

type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

/**
 * One group's mobile prediction card: big steppers, debounced autosave, a
 * per-group "N of M predicted" + save status, server-driven read-only, and a
 * Finalize action. Group-stage score entry only.
 */
export function MobileGroupCard({
  tournament,
  group,
  me,
  results,
  pointsByGame,
  serverNowMs,
  onExpire,
  onAutosave,
  onFinalize,
}: {
  tournament: Tournament
  group: GroupGame
  me: Player
  results: MatchPrediction[]
  pointsByGame?: Map<
    string,
    { breakdown: PointsBreakdown | null; isPerfect: boolean }
  >
  /** Reserved for future per-group bonus display; unused for now. */
  standings?: StandingsScore | null
  serverNowMs: number
  onExpire?: () => void
  onAutosave: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
  onFinalize: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
}) {
  const { t, locale } = useI18n()
  const teams = useMemo(() => teamIndex(tournament.teams, locale), [tournament, locale])

  const games = useMemo(
    () =>
      tournament.games
        .filter((g) => group.childGameIds.includes(g.id))
        .sort(byKickoff),
    [tournament, group],
  )

  const resultsByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of results) map.set(r.gameId, r)
    return map
  }, [results])

  const initial = useMemo(() => {
    const map: Record<string, Cell> = {}
    for (const g of games) {
      const p = me.matchPredictions.find((mp) => mp.gameId === g.id)
      map[g.id] = { home: p ? p.homeScore : null, away: p ? p.awayScore : null }
    }
    return map
  }, [games, me])

  // Seeded once; the parent keys this card by `group.id`, so switching groups
  // remounts and reseeds from the freshest `me`.
  const [cells, setCells] = useState<Record<string, Cell>>(initial)
  const [status, setStatus] = useState<SaveStatus>('idle')
  const [busy, setBusy] = useState(false)

  const deadlinePassed = group.deadlinePassed
  const allLocked =
    games.length > 0 &&
    games.every((g) => me.matchPredictions.find((p) => p.gameId === g.id)?.locked)
  const readOnly = (deadlinePassed || allLocked) && !me.isResultUser

  const total = games.length
  const done = predictedCount(games.map((g) => cells[g.id]))
  const allComplete = total > 0 && done === total

  const buildPredictions = (state: Record<string, Cell>): PredictionInput[] =>
    games
      .filter((g) => state[g.id].home !== null && state[g.id].away !== null)
      .map((g) => ({
        gameId: g.id,
        homeScore: state[g.id].home as number,
        awayScore: state[g.id].away as number,
      }))

  const buildStandings = (state: Record<string, Cell>): StandingsInput | null => {
    if (!group.carriesStandings) return null
    const ranked = applyDrawOrder(
      computeStandings(games, (gid) => {
        const c = state[gid]
        return c.home !== null && c.away !== null
          ? { home: c.home, away: c.away }
          : null
      }),
      [],
    )
    return { ordering: ranked.map((s) => s.teamId), drawOrder: [] }
  }

  const persist = useDebouncedCallback((state: Record<string, Cell>) => {
    void (async () => {
      try {
        const res = await onAutosave(
          group.id,
          buildPredictions(state),
          buildStandings(state),
        )
        setStatus(res.error ? 'error' : 'saved')
      } catch {
        setStatus('error')
      }
    })()
  }, 800)

  const setScore = (gameId: string, side: 'home' | 'away', value: number | null) => {
    const next = { ...cells, [gameId]: { ...cells[gameId], [side]: value } }
    setCells(next)
    setStatus('saving')
    persist(next)
  }

  const finalize = async () => {
    setBusy(true)
    try {
      await onFinalize(group.id, buildPredictions(cells), buildStandings(cells))
    } finally {
      setBusy(false)
    }
  }

  const statusText =
    status === 'saving'
      ? t('mobileSaving')
      : status === 'saved'
        ? t('saved')
        : status === 'error'
          ? t('mobileSaveError')
          : ''

  return (
    <div className="mobile-group-card">
      <div className="mobile-group-head">
        <h3>
          {group.name}{' '}
          <span className={readOnly ? 'state-locked' : 'state-draft'}>
            {readOnly ? t('locked') : t('draft')}
          </span>
        </h3>
        {!readOnly && group.deadline && (
          <span className="finalize-countdown">
            {t('finalizeBy')}{' '}
            <Countdown
              deadline={group.deadline}
              serverNowMs={serverNowMs}
              onExpire={onExpire}
            />
          </span>
        )}
      </div>

      <div className="mobile-group-status">
        <span>
          {done} {t('mobileOf')} {total} {t('mobilePredicted')}
        </span>
        {statusText && (
          <span className={`mobile-save-status${status === 'saved' ? ' saved' : ''}`}>
            {statusText}
          </span>
        )}
      </div>

      {readOnly && <p className="flash-bar">{t('lockedNotice')}</p>}

      <div className="mobile-matches">
        {games.map((game) => {
          const c = cells[game.id]
          const teamsPlaced = !!game.home.teamId && !!game.away.teamId
          const result = resultsByGame.get(game.id)
          const pt = pointsByGame?.get(game.id)
          return (
            <div className="mobile-match" key={game.id}>
              <Link to={`/match/${game.id}`}>
                <Matchup home={game.home} away={game.away} teams={teams} />
              </Link>
              {!teamsPlaced ? (
                <span className="hint">{t('teamsNotDetermined')}</span>
              ) : readOnly ? (
                <div className="mobile-match-scores">
                  <span className="score-locked">
                    {c.home === null ? '–' : c.home} :{' '}
                    {c.away === null ? '–' : c.away}
                  </span>
                </div>
              ) : (
                <div className="mobile-match-scores">
                  <ScoreStepper
                    value={c.home}
                    onChange={(v) => setScore(game.id, 'home', v)}
                  />
                  <span className="mobile-match-sep">:</span>
                  <ScoreStepper
                    value={c.away}
                    onChange={(v) => setScore(game.id, 'away', v)}
                  />
                </div>
              )}
              {result && (
                <span className="mobile-match-result">
                  {t('result')}: {result.homeScore}–{result.awayScore}
                </span>
              )}
              {pt?.breakdown && (
                <PointsBadge breakdown={pt.breakdown} isPerfect={pt.isPerfect} />
              )}
            </div>
          )
        })}
      </div>

      {!readOnly && (
        <div className="tip-actions">
          <InlineConfirm
            className="primary"
            confirmClassName="primary"
            disabled={busy || !allComplete}
            title={!allComplete ? t('enterAllGamesHint') : ''}
            question={t('lockConfirm')}
            onConfirm={() => void finalize()}
          >
            {t('lockGroup')}
          </InlineConfirm>
        </div>
      )}
    </div>
  )
}
