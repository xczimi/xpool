import { useEffect, useMemo, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { STANDINGS_QUERY, TIPS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { Round, StandingsScore, Tip, Tournament } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { RoundNav } from '../components/RoundNav'
import { byKickoff, teamIndex } from '../lib/format'
import { Matchup } from '../components/TeamLabel'
import { PointsBadge } from '../components/PointsBadge'
import { StandingsBadge } from '../components/StandingsBadge'
import { currentRoundNode, leafGroupsOfRound, visibleRoundNodes } from '../lib/rounds'

/**
 * All Tips (UC-9) — a grid of every player's predictions. Round tabs pick a
 * round; the Group Stage round shows one group, a knockout round shows every
 * match in the round (the tips query takes the round node id — its `games_in`
 * is recursive). The API already applies hidden-until-locked visibility.
 */
/** Composite key for the (player, game) -> tip lookup map. */
const tipKey = (playerId: string, gameId: string) => `${playerId}::${gameId}`

export function AllTipsPage() {
  const { t, locale } = useI18n()
  const { label } = useAuth()
  const [selectedRound, setSelectedRound] = useState<Round | null>(null)
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null)

  const [tournamentResult] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })

  const tournament = tournamentResult.data?.tournament ?? null
  const rounds = useMemo(
    () => visibleRoundNodes(tournament?.groups ?? [], tournament?.games ?? []),
    [tournament?.groups, tournament?.games],
  )
  const activeRound =
    selectedRound ?? currentRoundNode(rounds)?.round ?? rounds[0]?.round ?? null
  const activeRoundNode = rounds.find((r) => r.round === activeRound) ?? null
  const roundLeaves = activeRoundNode
    ? leafGroupsOfRound(activeRoundNode, tournament?.groups ?? [])
    : []
  const isGroupStage = activeRound === 'GROUP_STAGE'
  const activeGroupId = selectedGroupId ?? roundLeaves[0]?.id ?? null

  // Keep group selection coherent with the active round: if the derived round
  // flips (e.g. a `tournament` refetch moves `currentRoundNode`), a group from
  // the old round would strand the page in the empty "select a group" state.
  const prevActiveRound = useRef(activeRound)
  useEffect(() => {
    if (prevActiveRound.current !== activeRound) {
      prevActiveRound.current = activeRound
      setSelectedGroupId(null)
    }
  }, [activeRound])

  // Group Stage queries one leaf group; a knockout round queries the round
  // node — the `tips` resolver walks its subtree.
  const tipsGroupId = isGroupStage ? activeGroupId : (activeRoundNode?.id ?? null)

  const [tipsResult, refetchTips] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId: tipsGroupId },
    pause: !label || !tipsGroupId,
  })
  const [standingsResult] = useQuery<{ standings: StandingsScore[] }>({
    query: STANDINGS_QUERY,
    variables: { groupId: tipsGroupId },
    pause: !label || !tipsGroupId,
  })

  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament?.teams, locale],
  )
  const tips = useMemo(
    () => tipsResult.data?.tips ?? [],
    [tipsResult.data],
  )
  // playerId -> nick
  const players = useMemo(
    () => [...new Map(tips.map((tip) => [tip.playerId, tip.nick])).entries()],
    [tips],
  )
  // (playerId, gameId) -> tip
  const tipMap = useMemo(
    () => new Map(tips.map((tip) => [tipKey(tip.playerId, tip.gameId), tip])),
    [tips],
  )
  // playerId -> standings scores (one per scoreable group in the round).
  const standingsByPlayer = useMemo(() => {
    const map = new Map<string, StandingsScore[]>()
    for (const s of standingsResult.data?.standings ?? []) {
      const list = map.get(s.playerId) ?? []
      list.push(s)
      map.set(s.playerId, list)
    }
    return map
  }, [standingsResult.data])
  const showStandings = (standingsResult.data?.standings?.length ?? 0) > 0
  // groupId -> display name, for the per-group standings tooltip lines.
  const groupName = useMemo(() => {
    const map = new Map((tournament?.groups ?? []).map((g) => [g.id, g.name]))
    return (id: string) => map.get(id) ?? id
  }, [tournament?.groups])

  if (!label) return <NeedsLogin />
  if (tournamentResult.fetching) return <Loading />
  if (!tournament) return <ErrorView />

  const shownGameIds = new Set(
    isGroupStage
      ? (roundLeaves.find((g) => g.id === activeGroupId)?.childGameIds ?? [])
      : roundLeaves.flatMap((g) => g.childGameIds),
  )
  const games = tournament.games
    .filter((g) => shownGameIds.has(g.id))
    .sort(byKickoff)

  return (
    <section className="page">
      <h2>{t('allTipsTitle')}</h2>
      <RoundNav
        groups={tournament.groups}
        games={tournament.games}
        selectedRound={activeRound}
        onSelectRound={(round) => {
          setSelectedRound(round)
          setSelectedGroupId(null)
        }}
        selectedGroupId={activeGroupId}
        onSelectGroup={setSelectedGroupId}
      />

      {tipsResult.fetching && <Loading />}
      {tipsResult.error && (
        <ErrorView
          message={tipsResult.error.message}
          onRetry={() => refetchTips({ requestPolicy: 'network-only' })}
        />
      )}

      {!tipsResult.fetching && games.length > 0 && (
        <div className="grid-scroll">
          <table className="data-table compact">
            <thead>
              <tr>
                <th>{t('player')}</th>
                {games.map((g) => (
                  <th key={g.id}>
                    <Link to={`/match/${g.id}`}>
                      <Matchup home={g.home} away={g.away} teams={teams} compact />
                    </Link>
                  </th>
                ))}
                {showStandings && <th>{t('standingsCol')}</th>}
              </tr>
            </thead>
            <tbody>
              {players.map(([pid, nick]) => (
                <tr key={pid}>
                  <td>{nick}</td>
                  {games.map((g) => {
                    const tip = tipMap.get(tipKey(pid, g.id))
                    return (
                      <td key={g.id}>
                        {tip?.prediction ? (
                          <span className="tip-cell">
                            <span>
                              {tip.prediction.homeScore}–{tip.prediction.awayScore}
                            </span>
                            <PointsBadge
                              breakdown={tip.breakdown}
                              isPerfect={tip.isPerfect}
                            />
                          </span>
                        ) : tip ? (
                          t('hiddenTip')
                        ) : (
                          '—'
                        )}
                      </td>
                    )
                  })}
                  {showStandings && (
                    <td>
                      <StandingsBadge
                        scores={standingsByPlayer.get(pid) ?? []}
                        groupLabel={groupName}
                      />
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}
