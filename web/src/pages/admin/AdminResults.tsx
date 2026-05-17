import { useMemo, useState } from 'react'
import { useMutation, useQuery } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import {
  ENTER_RESULT_MUTATION,
  RESULTS_QUERY,
  TOURNAMENT_QUERY,
} from '../../graphql/queries'
import type { MatchPrediction, Tournament } from '../../graphql/types'
import { ErrorView, Loading } from '../../components/StatusViews'
import { byKickoff, formatKickoff, slotLabel, teamIndex } from '../../lib/format'

/** Admin results entry — calls `enterResult` per match (UC-14). */
export function AdminResults() {
  const { t, locale } = useI18n()
  const [result, refetch] = useQuery<{
    tournament: Tournament | null
    motd: string | null
  }>({ query: TOURNAMENT_QUERY })
  const [resultsQuery, refetchResults] = useQuery<{
    results: MatchPrediction[]
  }>({ query: RESULTS_QUERY })
  const [, enterResult] = useMutation(ENTER_RESULT_MUTATION)

  const tournament = result.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )
  const resultsByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsQuery.data?.results ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [resultsQuery.data])

  if (result.fetching) return <Loading />
  if (result.error) return <ErrorView message={result.error.message} />
  if (!tournament) return <ErrorView />

  const games = [...tournament.games].sort(byKickoff)

  return (
    <div>
      <h3>{t('adminResults')}</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>{t('kickoff')}</th>
            <th>{t('match')}</th>
            <th>{t('result')}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {games.map((game) => {
            const official = resultsByGame.get(game.id) ?? null
            return (
              <ResultRow
                key={game.id}
                gameId={game.id}
                label={`${slotLabel(game.home, teams)} – ${slotLabel(
                  game.away,
                  teams,
                )}`}
                kickoff={formatKickoff(game.kickoff, locale)}
                initialHome={official?.homeScore ?? null}
                initialAway={official?.awayScore ?? null}
                locked={official?.locked ?? false}
                onSave={async (home, away, lock) => {
                  await enterResult({
                    gameId: game.id,
                    homeScore: home,
                    awayScore: away,
                    advancer: null,
                    lock,
                  })
                  refetch({ requestPolicy: 'network-only' })
                  refetchResults({ requestPolicy: 'network-only' })
                }}
              />
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

function ResultRow({
  label,
  kickoff,
  initialHome,
  initialAway,
  locked,
  onSave,
}: {
  gameId: string
  label: string
  kickoff: string
  initialHome: number | null
  initialAway: number | null
  locked: boolean
  onSave: (home: number, away: number, lock: boolean) => Promise<void>
}) {
  const { t } = useI18n()
  const [home, setHome] = useState(
    initialHome === null ? '' : String(initialHome),
  )
  const [away, setAway] = useState(
    initialAway === null ? '' : String(initialAway),
  )
  const [busy, setBusy] = useState(false)

  const save = async (lock: boolean) => {
    if (home === '' || away === '') return
    setBusy(true)
    try {
      await onSave(Number(home), Number(away), lock)
    } finally {
      setBusy(false)
    }
  }

  return (
    <tr>
      <td>{kickoff}</td>
      <td>{label}</td>
      <td className="score-cell">
        <input
          type="number"
          min={0}
          value={home}
          disabled={locked}
          onChange={(e) => setHome(e.target.value)}
        />
        <span>:</span>
        <input
          type="number"
          min={0}
          value={away}
          disabled={locked}
          onChange={(e) => setAway(e.target.value)}
        />
      </td>
      <td>
        {locked ? (
          <span className="state-locked">{t('locked')}</span>
        ) : (
          <>
            <button type="button" disabled={busy} onClick={() => save(false)}>
              {t('save')}
            </button>
            <button
              type="button"
              className="primary"
              disabled={busy}
              onClick={() => save(true)}
            >
              {t('enterResult')}
            </button>
          </>
        )}
      </td>
    </tr>
  )
}
