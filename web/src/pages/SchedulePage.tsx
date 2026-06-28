import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import type { StringKey } from '../i18n/strings'
import { RESULTS_QUERY, THIRD_PLACE_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type {
  MatchPrediction,
  SingleGame,
  Team,
  ThirdPlaceRanking,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { ThirdPlaceTable } from '../components/ThirdPlaceTable'
import { byKickoff, formatKickoff, teamIndex } from '../lib/format'
import { groupByDay } from '../lib/scheduleByDate'
import { knockoutGroupIds } from '../lib/knockoutGroups'
import { Matchup } from '../components/TeamLabel'
import { roundLabel } from '../lib/rounds'

type ScheduleView = 'group' | 'date'

const VIEW_STORAGE_KEY = 'xpool.scheduleView'

/** Read the persisted view, defaulting to 'group'. Tolerates SSR/no-storage. */
function readView(): ScheduleView {
  try {
    return localStorage.getItem(VIEW_STORAGE_KEY) === 'date' ? 'date' : 'group'
  } catch {
    return 'group'
  }
}

/** Persist the chosen view per-user. Swallows storage failures (private mode). */
function persistView(view: ScheduleView): void {
  try {
    localStorage.setItem(VIEW_STORAGE_KEY, view)
  } catch {
    // ignore — persistence is best-effort
  }
}

/** Full fixture list (UC-12). Public, read-only. Toggle: by group ⇄ by date. */
export function SchedulePage() {
  const { t, locale } = useI18n()
  const [view, setView] = useState<ScheduleView>(readView)
  const [result, reexecute] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })
  const [thirdsResult] = useQuery<{ thirdPlaceRanking: ThirdPlaceRanking }>({
    query: THIRD_PLACE_QUERY,
    variables: { player: null },
  })
  const officialThirds = thirdsResult.data?.thirdPlaceRanking ?? null

  const tournament = result.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )
  const resultsByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsResult.data?.results ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [resultsResult.data])
  // Leaf groups that are single-game knockout ties — their "open" link reads
  // "Open this KO match" instead of "Open this group".
  const koGroupIds = useMemo(
    () => knockoutGroupIds(tournament?.groups ?? [], tournament?.games ?? []),
    [tournament],
  )

  function chooseView(next: ScheduleView) {
    setView(next)
    persistView(next)
  }

  if (result.fetching) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!tournament) return <ErrorView />

  // Leaf groups (those holding matches), ordered chronologically by their
  // deadline — the earliest kickoff in the group — so the schedule reads in
  // time order: group stage A–L, then each knockout round.
  const leafGroups = tournament.groups
    .filter((g) => g.childGameIds.length > 0)
    .sort((a, b) => {
      const da = a.deadline ? Date.parse(a.deadline) : Number.POSITIVE_INFINITY
      const db = b.deadline ? Date.parse(b.deadline) : Number.POSITIVE_INFINITY
      return da - db
    })

  // By-date sections — all games bucketed by the viewer's local calendar day.
  const daySections = groupByDay(tournament.games, locale)

  return (
    <section className="page">
      <h2>{t('scheduleTitle')}</h2>

      <div
        className="schedule-view-toggle"
        role="group"
        aria-label={t('scheduleTitle')}
      >
        <button
          type="button"
          className={view === 'group' ? 'view-tab active' : 'view-tab'}
          aria-pressed={view === 'group'}
          onClick={() => chooseView('group')}
        >
          {t('scheduleViewByGroup')}
        </button>
        <button
          type="button"
          className={view === 'date' ? 'view-tab active' : 'view-tab'}
          aria-pressed={view === 'date'}
          onClick={() => chooseView('date')}
        >
          {t('scheduleViewByDate')}
        </button>
      </div>

      {view === 'group'
        ? leafGroups.map((group) => {
            const games = tournament.games
              .filter((m) => group.childGameIds.includes(m.id))
              .sort(byKickoff)
            return (
              <div key={group.id} className="schedule-group">
                <h3>
                  {group.name}{' '}
                  <span className="round-tag">
                    {roundLabel(group.round, t)}
                  </span>
                </h3>
                <ScheduleTable
                  games={games}
                  teams={teams}
                  resultsByGame={resultsByGame}
                  koGroupIds={koGroupIds}
                  locale={locale}
                  t={t}
                />
              </div>
            )
          })
        : daySections.map((section) => (
            <div key={section.key} className="schedule-day">
              <h3>{section.label}</h3>
              <ScheduleTable
                games={section.games}
                teams={teams}
                resultsByGame={resultsByGame}
                koGroupIds={koGroupIds}
                locale={locale}
                t={t}
              />
            </div>
          ))}

      <div className="thirds-section" data-testid="third-place-section">
        <h3>{t('thirdsScheduleTitle')}</h3>
        <ThirdPlaceTable title={t('thirdsOfficial')} ranking={officialThirds} teams={teams} />
      </div>
    </section>
  )
}

/** Shared fixture table — identical rows for both the group and date views. */
function ScheduleTable({
  games,
  teams,
  resultsByGame,
  koGroupIds,
  locale,
  t,
}: {
  games: SingleGame[]
  teams: Map<string, Team>
  resultsByGame: Map<string, MatchPrediction>
  koGroupIds: Set<string>
  locale: string
  t: (key: StringKey) => string
}) {
  return (
    <table className="data-table">
      <thead>
        <tr>
          <th>{t('kickoff')}</th>
          <th className="col-match">{t('match')}</th>
          <th>{t('venue')}</th>
          <th>{t('result')}</th>
        </tr>
      </thead>
      <tbody>
        {games.map((m) => {
          const r = resultsByGame.get(m.id)
          return (
            <tr key={m.id}>
              <td>{formatKickoff(m.kickoff, locale)}</td>
              <td>
                <Link to={`/match/${m.id}`}>
                  <Matchup home={m.home} away={m.away} teams={teams} />
                </Link>
                {' '}
                <Link
                  to={`/mytips/${m.groupId}#${m.groupId}`}
                  className="open-group-link"
                >
                  {koGroupIds.has(m.groupId)
                    ? t('openKoMatch')
                    : t('openGroup')}
                </Link>
              </td>
              <td>{m.venue ?? '—'}</td>
              <td>{r ? `${r.homeScore}–${r.awayScore}` : '—'}</td>
            </tr>
          )
        })}
      </tbody>
    </table>
  )
}
