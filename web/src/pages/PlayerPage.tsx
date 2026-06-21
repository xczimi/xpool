import { useMemo } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  ME_QUERY,
  PERFECTS_QUERY,
  POOLS_QUERY,
  RESULTS_QUERY,
  SCOREBOARD_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type {
  Me,
  MatchPrediction,
  Perfect,
  Pool,
  ScoreEntry,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import {
  perfectsOf,
  playerEntry,
  playerRank,
  sharedPoolWith,
} from '../lib/playerPage'
import { PlayerHeader } from './player/PlayerHeader'
import { PlayerTodaySlice } from './player/PlayerTodaySlice'
import { PlayerPerfects } from './player/PlayerPerfects'
import { PlayerRounds } from './player/PlayerRounds'

/**
 * One participant's complete tournament view (consumer #3). Read-only,
 * frontend-only aggregation. The header is served by `scoreboard` + `perfects`;
 * each round's predictions are lazily fetched in `PlayerRoundDetail` on expand,
 * inheriting the `tips` resolver's visibility gating. Pool-mate gating is soft:
 * a player absent from the viewer's pool scoreboard shows a "not in your pool"
 * notice instead of the page.
 */
export function PlayerPage() {
  const { id = '' } = useParams<{ id: string }>()
  const { t, locale } = useI18n()
  const { label } = useAuth()

  // The viewer's pools (they are a member of each). You may view a player's
  // page if you share AT LEAST ONE pool with them — checked across every pool
  // you belong to, not just the first. The header standing is scoped to that
  // shared pool (your common context); your own page resolves to your first
  // pool, and a pool-less viewer falls back to the global board.
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = useMemo(
    () => poolsResult.data?.pools ?? [],
    [poolsResult.data],
  )
  const sharedPool = useMemo(() => sharedPoolWith(pools, id), [pools, id])
  const effectivePool = sharedPool?.id ?? null

  const [meResult] = useQuery<{ me: Me }>({ query: ME_QUERY, pause: !label })
  const meRaw = meResult.data?.me ?? null
  const myId = meRaw?.__typename === 'Player' ? meRaw.id : null
  const isOwn = myId !== null && myId === id

  const [scoreboardResult] = useQuery<{ scoreboard: ScoreEntry[] }>({
    query: SCOREBOARD_QUERY,
    variables: { pool: effectivePool },
    pause: !label,
  })
  const [perfectsResult] = useQuery<{ perfects: Perfect[] }>({
    query: PERFECTS_QUERY,
  })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })
  const [tournamentResult] = useQuery<{
    tournament: Tournament | null
    now: string
  }>({
    query: TOURNAMENT_QUERY,
  })

  const scoreboard = useMemo(
    () => scoreboardResult.data?.scoreboard ?? [],
    [scoreboardResult.data],
  )
  const entry = useMemo(() => playerEntry(scoreboard, id), [scoreboard, id])
  const rank = useMemo(() => playerRank(scoreboard, id), [scoreboard, id])
  const perfects = useMemo(
    () => perfectsOf(perfectsResult.data?.perfects ?? [], id),
    [perfectsResult.data, id],
  )
  const tournament = tournamentResult.data?.tournament ?? null
  const now = tournamentResult.data?.now ?? ''
  const resultByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsResult.data?.results ?? []) map.set(r.gameId, r)
    return map
  }, [resultsResult.data])

  if (!label) return <NeedsLogin />
  if (
    poolsResult.fetching ||
    meResult.fetching ||
    scoreboardResult.fetching ||
    tournamentResult.fetching
  )
    return <Loading />
  if (scoreboardResult.error)
    return <ErrorView message={scoreboardResult.error.message} />
  if (!tournament) return <ErrorView />

  // Soft pool-mate gate: you may view a player's page if it's your own or you
  // share at least one pool with them. A shared pool-mate who has not yet
  // predicted has no scoreboard entry either — both fall through to the same
  // notice rather than crash on a missing entry.
  if ((!isOwn && !sharedPool) || !entry) {
    return (
      <section className="page">
        <p>{t('playerNotInPool')}</p>
      </section>
    )
  }

  return (
    <section className="page player-page">
      <h2>{entry.nick}</h2>
      {isOwn && (
        <p className="player-profile-link">
          <Link to="/profile">{t('playerProfileLink')}</Link>
        </p>
      )}
      <PlayerHeader entry={entry} rank={rank} />
      <PlayerTodaySlice
        playerId={id}
        tournament={tournament}
        resultByGame={resultByGame}
        now={now}
        locale={locale}
      />
      <PlayerPerfects
        perfects={perfects}
        tournament={tournament}
        resultByGame={resultByGame}
        locale={locale}
      />
      <PlayerRounds
        playerId={id}
        isOwn={isOwn}
        entry={entry}
        tournament={tournament}
        resultByGame={resultByGame}
        locale={locale}
      />
      {/* Per-round detail shows '—' for any match without a pick, so a
          participant who never predicted reads honestly without a special
          empty state. A reliable "no predictions" signal isn't available from
          the materialised totals alone (every participant scores 0 before any
          result), so there is deliberately no zero-total short-circuit here. */}
    </section>
  )
}
