import { useMemo, useState } from 'react'
import { useQuery } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import { STANDINGS_QUERY, TIPS_QUERY } from '../../graphql/queries'
import type {
  GroupGame,
  MatchPrediction,
  StandingsScore,
  Tip,
  Tournament,
} from '../../graphql/types'
import type { Locale } from '../../i18n/strings'
import { ErrorView, Loading } from '../../components/StatusViews'
import { GroupSubNav } from '../../components/GroupSubNav'
import { Matchup } from '../../components/TeamLabel'
import { PointsBadge } from '../../components/PointsBadge'
import { StandingsBadge } from '../../components/StandingsBadge'
import { byKickoff, teamIndex } from '../../lib/format'
import { leafGroupsOfRound } from '../../lib/rounds'

/**
 * One expanded round for a single player. Group Stage gets a group sub-nav and
 * loads one leaf group at a time (`tips` + `standings`); a knockout round loads
 * the round node id once (`tips` walks the subtree). Predictions are filtered
 * to `playerId`; a tip whose `prediction` is null is gated-hidden and renders
 * as a placeholder. Each fetch is lazy — this component only mounts on expand.
 */
export function PlayerRoundDetail({
  playerId,
  roundNode,
  tournament,
  resultByGame,
  locale,
}: {
  playerId: string
  roundNode: GroupGame
  tournament: Tournament
  resultByGame: Map<string, MatchPrediction>
  locale: Locale
}) {
  const { t } = useI18n()
  const isGroupStage = roundNode.round === 'GROUP_STAGE'
  const leaves = useMemo(
    () => leafGroupsOfRound(roundNode, tournament.groups),
    [roundNode, tournament.groups],
  )
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(
    () => (isGroupStage ? (leaves[0]?.id ?? null) : null),
  )

  // Group Stage queries the selected leaf group; a knockout queries the round
  // node id (its recursive `games_in` is walked server-side).
  const queryGroupId = isGroupStage ? selectedGroupId : roundNode.id

  const [tipsResult] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId: queryGroupId },
    pause: !queryGroupId,
  })
  const [standingsResult] = useQuery<{ standings: StandingsScore[] }>({
    query: STANDINGS_QUERY,
    variables: { groupId: queryGroupId },
    pause: !queryGroupId || !isGroupStage,
  })

  const teams = useMemo(
    () => teamIndex(tournament.teams, locale),
    [tournament.teams, locale],
  )
  const groupName = useMemo(() => {
    const map = new Map(tournament.groups.map((g) => [g.id, g.name]))
    return (gid: string) => map.get(gid) ?? gid
  }, [tournament.groups])

  // Only this player's tips, keyed by game.
  const tipByGame = useMemo(() => {
    const map = new Map<string, Tip>()
    for (const tip of tipsResult.data?.tips ?? []) {
      if (tip.playerId === playerId) map.set(tip.gameId, tip)
    }
    return map
  }, [tipsResult.data, playerId])
  const standings = useMemo(
    () =>
      (standingsResult.data?.standings ?? []).filter(
        (s) => s.playerId === playerId,
      ),
    [standingsResult.data, playerId],
  )

  // Which games to show: the selected group's children (group stage), or every
  // leaf game in the round (knockout), in kickoff order.
  const shownGameIds = useMemo(() => {
    const ids = isGroupStage
      ? (leaves.find((g) => g.id === selectedGroupId)?.childGameIds ?? [])
      : leaves.flatMap((g) => g.childGameIds)
    return new Set(ids)
  }, [isGroupStage, leaves, selectedGroupId])
  const games = useMemo(
    () => tournament.games.filter((g) => shownGameIds.has(g.id)).sort(byKickoff),
    [tournament.games, shownGameIds],
  )

  return (
    <div className="player-round-detail">
      {isGroupStage && (
        <GroupSubNav
          groups={leaves}
          selectedId={selectedGroupId}
          onSelect={setSelectedGroupId}
        />
      )}

      {tipsResult.fetching && <Loading />}
      {tipsResult.error && <ErrorView message={tipsResult.error.message} />}

      {!tipsResult.fetching && (
        <table className="data-table compact player-round-table">
          <thead>
            <tr>
              <th className="col-match">{t('match')}</th>
              <th>{t('player')}</th>
              <th>{t('result')}</th>
              <th>{t('points')}</th>
            </tr>
          </thead>
          <tbody>
            {games.map((g) => {
              const tip = tipByGame.get(g.id)
              const result = resultByGame.get(g.id)
              return (
                <tr key={g.id}>
                  <td>
                    <Matchup home={g.home} away={g.away} teams={teams} compact />
                  </td>
                  <td>
                    {tip?.prediction
                      ? `${tip.prediction.homeScore}–${tip.prediction.awayScore}`
                      : tip
                        ? t('hiddenTip')
                        : '—'}
                  </td>
                  <td>
                    {result ? `${result.homeScore}–${result.awayScore}` : '—'}
                  </td>
                  <td>
                    <PointsBadge
                      breakdown={tip?.breakdown}
                      isPerfect={tip?.isPerfect}
                    />
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      )}

      {isGroupStage && standings.length > 0 && (
        <div className="player-round-standings">
          <span>{t('standingsCol')}: </span>
          <StandingsBadge scores={standings} groupLabel={groupName} />
        </div>
      )}
    </div>
  )
}
