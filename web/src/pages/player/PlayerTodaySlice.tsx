import { useCallback, useEffect, useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import { TIPS_QUERY } from '../../graphql/queries'
import type { MatchPrediction, Tip, Tournament } from '../../graphql/types'
import type { Locale } from '../../i18n/strings'
import { Matchup } from '../../components/TeamLabel'
import { PointsBadge } from '../../components/PointsBadge'
import { byKickoff, teamIndex } from '../../lib/format'

/**
 * Loads one leaf group's tips and reports just this player's up to the parent.
 * One of these mounts per distinct group in the window, so each fires a single
 * small `tips(groupId)` query — inheriting the resolver's visibility gating.
 */
function GroupTipsLoader({
  groupId,
  playerId,
  onLoaded,
}: {
  groupId: string
  playerId: string
  onLoaded: (groupId: string, tips: Tip[]) => void
}) {
  const [res] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId },
  })
  const data = res.data?.tips
  useEffect(() => {
    if (data) onLoaded(groupId, data.filter((tp) => tp.playerId === playerId))
  }, [data, groupId, playerId, onLoaded])
  return null
}

/**
 * A thin "around now" slice for one player: the last few played and the next
 * few upcoming matches (from the server's ±2-day today window), each with their
 * tip, the official result, and points. Upcoming picks on another player's page
 * obey the same gating as everywhere — un-revealable tips show as a placeholder.
 * Renders nothing when the window is empty (well before / after the tournament).
 */
export function PlayerTodaySlice({
  playerId,
  tournament,
  resultByGame,
  now,
  locale,
}: {
  playerId: string
  tournament: Tournament
  resultByGame: Map<string, MatchPrediction>
  now: string
  locale: Locale
}) {
  const { t } = useI18n()
  const teams = useMemo(
    () => teamIndex(tournament.teams, locale),
    [tournament.teams, locale],
  )

  // Up to the 3 most recent and 3 soonest matches around the server clock.
  const games = useMemo(() => {
    const win = tournament.games
      .filter((g) => g.withinTodayWindow)
      .sort(byKickoff)
    const nowMs = Date.parse(now)
    const past = win.filter((g) => Date.parse(g.kickoff) <= nowMs)
    const future = win.filter((g) => Date.parse(g.kickoff) > nowMs)
    return [...past.slice(-3), ...future.slice(0, 3)]
  }, [tournament.games, now])

  const groupIds = useMemo(
    () => [...new Set(games.map((g) => g.groupId))],
    [games],
  )

  const [tipsByGroup, setTipsByGroup] = useState<Map<string, Tip[]>>(new Map())
  const onLoaded = useCallback((groupId: string, tips: Tip[]) => {
    setTipsByGroup((prev) => {
      const next = new Map(prev)
      next.set(groupId, tips)
      return next
    })
  }, [])
  const tipByGame = useMemo(() => {
    const map = new Map<string, Tip>()
    for (const tips of tipsByGroup.values()) {
      for (const tp of tips) map.set(tp.gameId, tp)
    }
    return map
  }, [tipsByGroup])

  if (games.length === 0) return null

  return (
    <section className="player-today">
      <h3>{t('playerNowHeading')}</h3>
      {groupIds.map((gid) => (
        <GroupTipsLoader
          key={gid}
          groupId={gid}
          playerId={playerId}
          onLoaded={onLoaded}
        />
      ))}
      <div className="grid-scroll">
        <table className="data-table compact">
          <thead>
            <tr>
              <th className="col-match">{t('match')}</th>
              <th>{t('tipCol')}</th>
              <th>{t('result')}</th>
              <th>{t('points')}</th>
            </tr>
          </thead>
          <tbody>
            {games.map((g) => {
              const tip = tipByGame.get(g.id)
              const r = resultByGame.get(g.id)
              return (
                <tr key={g.id} className={g.isToday ? 'is-today' : undefined}>
                  <td>
                    <Link to={`/match/${g.id}`}>
                      <Matchup home={g.home} away={g.away} teams={teams} compact />
                    </Link>
                  </td>
                  <td>
                    {tip?.prediction
                      ? `${tip.prediction.homeScore}–${tip.prediction.awayScore}`
                      : tip
                        ? t('hiddenTip')
                        : '—'}
                  </td>
                  <td>{r ? `${r.homeScore}–${r.awayScore}` : '—'}</td>
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
      </div>
    </section>
  )
}
