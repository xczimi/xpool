import { useMemo, useState } from 'react'
import type { OperationResult } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import type {
  GroupGame,
  MatchPrediction,
  Player,
  Tournament,
} from '../../graphql/types'
import { byKickoff, teamIndex } from '../../lib/format'
import {
  applyDrawOrder,
  computeStandings,
} from '../../lib/standings'
import { Matchup } from '../../components/TeamLabel'
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
  onSubmit,
}: {
  tournament: Tournament
  group: GroupGame
  me: Player
  /** The result user's locked match predictions — official scores. */
  results: MatchPrediction[]
  onSubmit: (
    predictions: PredictionInput[],
    standings: StandingsInput | null,
    lock: boolean,
  ) => Promise<OperationResult>
}) {
  const { t } = useI18n()
  const teams = useMemo(() => teamIndex(tournament.teams), [tournament])

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
  const initialMatches = useMemo(() => {
    const map: Record<string, DraftMatch> = {}
    for (const game of games) {
      const existing = me.matchPredictions.find((p) => p.gameId === game.id)
      map[game.id] = {
        homeScore: existing ? String(existing.homeScore) : '',
        awayScore: existing ? String(existing.awayScore) : '',
        locked: existing?.locked ?? false,
      }
    }
    return map
  }, [games, me])

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
      const m = matches[gameId]
      if (!m || m.homeScore === '' || m.awayScore === '') return null
      return { home: Number(m.homeScore), away: Number(m.awayScore) }
    })
    return applyDrawOrder(ranked, drawOrder)
  }, [games, matches, drawOrder])

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
    const m = matches[g.id]
    return m && m.homeScore !== '' && m.awayScore !== ''
  })

  const toPredictionInputs = (): PredictionInput[] =>
    games
      .filter((g) => {
        const m = matches[g.id]
        return m && m.homeScore !== '' && m.awayScore !== ''
      })
      .map((g) => ({
        gameId: g.id,
        homeScore: Number(matches[g.id].homeScore),
        awayScore: Number(matches[g.id].awayScore),
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
      </h3>

      {readOnly && <p className="flash-bar">{t('lockedNotice')}</p>}
      {flash && <p className="flash-bar">{flash}</p>}

      <table className="data-table">
        <thead>
          <tr>
            <th className="col-match">{t('match')}</th>
            <th>{t('prediction')}</th>
            <th>{t('result')}</th>
          </tr>
        </thead>
        <tbody>
          {games.map((game) => {
            const m = matches[game.id]
            const matchLocked = readOnly || (m.locked && !isResultUser)
            return (
              <tr key={game.id}>
                <td>
                  <Matchup home={game.home} away={game.away} teams={teams} />
                </td>
                <td className="score-cell">
                  <ScoreInput
                    value={m.homeScore}
                    disabled={matchLocked}
                    onChange={(v) => setScore(game.id, 'homeScore', v)}
                  />
                  <span>:</span>
                  <ScoreInput
                    value={m.awayScore}
                    disabled={matchLocked}
                    onChange={(v) => setScore(game.id, 'awayScore', v)}
                  />
                </td>
                <td>
                  {(() => {
                    const r = resultsByGame.get(game.id)
                    return r ? `${r.homeScore}–${r.awayScore}` : '—'
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

      {!readOnly && (
        <div className="tip-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() => submit(false)}
          >
            {t('saveDraft')}
          </button>
          <button
            type="button"
            className="primary"
            disabled={busy || !allComplete}
            title={!allComplete ? 'All matches must have scores to lock' : ''}
            onClick={() => submit(true)}
          >
            {t('lockGroup')}
          </button>
        </div>
      )}
    </div>
  )
}

function ScoreInput({
  value,
  disabled,
  onChange,
}: {
  value: string
  disabled: boolean
  onChange: (v: string) => void
}) {
  return (
    <select
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
    >
      <option value="">–</option>
      {SCORE_OPTIONS.map((n) => (
        <option key={n} value={n}>
          {n}
        </option>
      ))}
    </select>
  )
}
