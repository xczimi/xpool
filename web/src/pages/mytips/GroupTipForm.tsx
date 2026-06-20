import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import type { OperationResult } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import type {
  GroupGame,
  MatchPrediction,
  Player,
  PointsBreakdown,
  ReportedResult,
  StandingsScore,
  Tournament,
} from '../../graphql/types'
import { byKickoff, teamIndex } from '../../lib/format'
import {
  applyDrawOrder,
  computeStandings,
} from '../../lib/standings'
import { Matchup } from '../../components/TeamLabel'
import { Countdown } from '../../components/Countdown'
import { PointsBadge } from '../../components/PointsBadge'
import { InlineConfirm } from '../../components/InlineConfirm'
import { StandingsTable, PredictedStandingsEditor } from './StandingsTables'

interface PredictionInput {
  gameId: string
  homeScore: number
  awayScore: number
}

interface StandingsInput {
  ordering: string[]
  drawOrder: string[]
}

interface DraftMatch {
  homeScore: string
  awayScore: string
  locked: boolean
}

const SCORE_OPTIONS = Array.from({ length: 10 }, (_, i) => i) // 0–9 (legacy)

/**
 * The group-level prediction form (API.md §6). Edits all matches in one leaf
 * group; Save draft / Lock submits the whole group. Locked predictions render
 * read-only. Shows predicted vs actual standings with manual tie ordering.
 */
export function GroupTipForm({
  tournament,
  group,
  me,
  results,
  pointsByGame,
  standings,
  serverNowMs,
  onExpire,
  onSubmit,
  reported,
}: {
  tournament: Tournament
  group: GroupGame
  me: Player
  /** The result user's locked match predictions — official scores. */
  results: MatchPrediction[]
  /** gameId → earned-points breakdown for the current player (server-computed). */
  pointsByGame?: Map<
    string,
    { breakdown: PointsBreakdown | null; isPerfect: boolean }
  >
  /** This player's standings bonus for the group, once scoreable. */
  standings?: StandingsScore | null
  /** Estimated server-now in ms (from `useServerClock`) for the countdown. */
  serverNowMs: number
  /** Fired when this group's countdown crosses zero — triggers a refetch. */
  onExpire?: () => void
  onSubmit: (
    predictions: PredictionInput[],
    standings: StandingsInput | null,
    lock: boolean,
  ) => Promise<OperationResult>
  /** SportsDB reported scores to pre-fill empty inputs (result user only). */
  reported?: Map<string, ReportedResult>
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
    for (const r of results) {
      map.set(r.gameId, r)
    }
    return map
  }, [results])

  // Seed the form from the player's existing predictions for this group.
  // If there is no existing prediction and a reported score is available
  // (result user only), pre-fill from it.
  const initialMatches = useMemo(() => {
    const map: Record<string, DraftMatch> = {}
    for (const game of games) {
      const existing = me.matchPredictions.find((p) => p.gameId === game.id)
      const fill = !existing ? reported?.get(game.id) : undefined
      map[game.id] = {
        homeScore: existing
          ? String(existing.homeScore)
          : fill
            ? String(fill.homeScore)
            : '',
        awayScore: existing
          ? String(existing.awayScore)
          : fill
            ? String(fill.awayScore)
            : '',
        locked: existing?.locked ?? false,
      }
    }
    return map
  }, [games, me, reported])

  const initialDrawOrder = useMemo(
    () =>
      me.standingsPredictions.find((s) => s.groupId === group.id)?.drawOrder ??
      [],
    [me, group],
  )

  const [matches, setMatches] = useState<Record<string, DraftMatch>>(
    initialMatches,
  )
  const [drawOrder, setDrawOrder] = useState<string[]>(initialDrawOrder)
  const [flash, setFlash] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  // Merge reported pre-fills into the visible match state without touching the
  // stored `matches` state. Reported fills are shown only for games where the
  // admin has not yet typed anything and the slot is not locked — so the admin
  // always remains in control of what they see and submit.
  const displayMatches = useMemo(() => {
    const merged: Record<string, DraftMatch> = { ...matches }
    for (const game of games) {
      const cur = matches[game.id]
      const r = reported?.get(game.id)
      if (r && cur && cur.homeScore === '' && cur.awayScore === '' && !cur.locked) {
        merged[game.id] = { homeScore: String(r.homeScore), awayScore: String(r.awayScore), locked: false }
      }
    }
    return merged
  }, [matches, reported, games])

  // The result user enters official results and is never locked out — not by
  // the deadline (results arrive after kickoff) nor by a prior lock (they can
  // always re-correct). For everyone else the deadline freezes the group (UC-7).
  const isResultUser = me.isResultUser
  const deadlinePassed = group.deadlinePassed
  const groupLocked =
    deadlinePassed ||
    (games.length > 0 && games.every((g) => matches[g.id]?.locked))
  const readOnly = groupLocked && !isResultUser

  const setScore = (
    gameId: string,
    side: 'homeScore' | 'awayScore',
    value: string,
  ) => {
    setMatches((prev) => ({
      ...prev,
      [gameId]: { ...prev[gameId], [side]: value },
    }))
  }

  // Predicted standings derived from the current draft scores.
  const predicted = useMemo(() => {
    const ranked = computeStandings(games, (gameId) => {
      const m = displayMatches[gameId]
      if (!m || m.homeScore === '' || m.awayScore === '') return null
      return { home: Number(m.homeScore), away: Number(m.awayScore) }
    })
    return applyDrawOrder(ranked, drawOrder)
  }, [games, displayMatches, drawOrder])

  // Actual standings from official results.
  const actual = useMemo(
    () =>
      computeStandings(games, (gameId) => {
        const r = resultsByGame.get(gameId)
        return r ? { home: r.homeScore, away: r.awayScore } : null
      }),
    [games, resultsByGame],
  )

  const allComplete = games.every((g) => {
    const m = displayMatches[g.id]
    return m && m.homeScore !== '' && m.awayScore !== ''
  })

  const toPredictionInputs = (): PredictionInput[] =>
    games
      .filter((g) => {
        const m = displayMatches[g.id]
        return m && m.homeScore !== '' && m.awayScore !== ''
      })
      .map((g) => ({
        gameId: g.id,
        homeScore: Number(displayMatches[g.id].homeScore),
        awayScore: Number(displayMatches[g.id].awayScore),
      }))

  const submit = async (lock: boolean) => {
    setBusy(true)
    setFlash(null)
    try {
      const standings: StandingsInput | null = group.carriesStandings
        ? { ordering: predicted.map((s) => s.teamId), drawOrder }
        : null
      const res = await onSubmit(toPredictionInputs(), standings, lock)
      if (res.error) {
        setFlash(`${t('errorPrefix')}: ${res.error.message}`)
      } else {
        setFlash(t('saved'))
      }
    } catch (err) {
      setFlash(
        `${t('errorPrefix')}: ${
          err instanceof Error ? err.message : String(err)
        }`,
      )
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="tip-form">
      <h3>
        {group.name}{' '}
        <span className={groupLocked ? 'state-locked' : 'state-draft'}>
          {groupLocked ? t('locked') : t('draft')}
        </span>
        {!groupLocked && group.deadline && (
          <span className="finalize-countdown">
            {' · '}
            {t('finalizeBy')}{' '}
            <Countdown
              deadline={group.deadline}
              serverNowMs={serverNowMs}
              onExpire={onExpire}
            />
          </span>
        )}
      </h3>

      {!groupLocked && group.deadline && (
        <p className="hint">{t('enterAllGamesHint')}</p>
      )}

      {readOnly && <p className="flash-bar">{t('lockedNotice')}</p>}
      {flash && <p className="flash-bar">{flash}</p>}

      <table className="data-table">
        <thead>
          <tr>
            <th className="col-match">{t('match')}</th>
            <th>{t('prediction')}</th>
            <th>{t('result')}</th>
            <th>{t('points')}</th>
          </tr>
        </thead>
        <tbody>
          {games.map((game) => {
            const m = displayMatches[game.id]
            const matchLocked = readOnly || (m.locked && !isResultUser)
            return (
              <tr key={game.id}>
                <td>
                  <Link to={`/match/${game.id}`}>
                    <Matchup home={game.home} away={game.away} teams={teams} />
                  </Link>
                </td>
                <td className="score-cell">
                  {matchLocked ? (
                    // A locked prediction is settled — show it as plain text, not
                    // dead form controls.
                    <span className="score-locked">
                      {m.homeScore === '' ? '–' : m.homeScore} :{' '}
                      {m.awayScore === '' ? '–' : m.awayScore}
                    </span>
                  ) : (
                    <>
                      <ScoreInput
                        value={m.homeScore}
                        onChange={(v) => setScore(game.id, 'homeScore', v)}
                      />
                      <span>:</span>
                      <ScoreInput
                        value={m.awayScore}
                        onChange={(v) => setScore(game.id, 'awayScore', v)}
                      />
                    </>
                  )}
                </td>
                <td>
                  {(() => {
                    const r = resultsByGame.get(game.id)
                    return r ? `${r.homeScore}–${r.awayScore}` : '—'
                  })()}
                </td>
                <td>
                  {(() => {
                    const pt = pointsByGame?.get(game.id)
                    return pt?.breakdown ? (
                      <PointsBadge
                        breakdown={pt.breakdown}
                        isPerfect={pt.isPerfect}
                      />
                    ) : (
                      '—'
                    )
                  })()}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>

      <div className="standings-pair">
        {group.carriesStandings && (
          <PredictedStandingsEditor
            rows={predicted}
            teams={teams}
            readOnly={readOnly}
            onReorder={setDrawOrder}
          />
        )}
        <StandingsTable
          title={t('actualStandings')}
          rows={actual}
          teams={teams}
        />
      </div>

      {group.carriesStandings && standings && (
        <p className="standings-bonus">
          <strong>{t('standingsBonus')}:</strong>{' '}
          {standings.pairsCorrect}/{standings.pairsTotal} {t('pairsCorrect')} —{' '}
          {standings.bonus} × {standings.multiplier} = <strong>{standings.points}</strong>
        </p>
      )}

      {!readOnly && (
        <div className="tip-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() => submit(false)}
          >
            {t('saveDraft')}
          </button>
          <InlineConfirm
            className="primary"
            confirmClassName="primary"
            disabled={busy || !allComplete}
            title={!allComplete ? 'All matches must have scores to lock' : ''}
            question={t('lockConfirm')}
            onConfirm={() => void submit(true)}
          >
            {t('lockGroup')}
          </InlineConfirm>
        </div>
      )}
    </div>
  )
}

function ScoreInput({
  value,
  onChange,
}: {
  value: string
  onChange: (v: string) => void
}) {
  return (
    <select value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="">–</option>
      {SCORE_OPTIONS.map((n) => (
        <option key={n} value={n}>
          {n}
        </option>
      ))}
    </select>
  )
}
