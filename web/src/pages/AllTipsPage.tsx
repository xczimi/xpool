import { useMemo, useState } from 'react'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { TIPS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { Tip, Tournament } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { GroupSubNav } from '../components/GroupSubNav'
import { byKickoff, slotCode, teamIndex } from '../lib/format'

/**
 * All Tips (UC-9) — a grid of every player's predictions for a group. The API
 * already applies hidden-until-locked visibility; a tip whose nested
 * `prediction` is null shows as "hidden".
 */
export function AllTipsPage() {
  const { t } = useI18n()
  const { playerId } = useAuth()
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null)

  const [tournamentResult] = useQuery<{
    tournament: Tournament | null
    motd: string | null
  }>({ query: TOURNAMENT_QUERY })

  const tournament = tournamentResult.data?.tournament ?? null
  const leafGroups = useMemo(
    () => (tournament?.groups ?? []).filter((g) => g.childGameIds.length > 0),
    [tournament],
  )
  const activeGroupId = selectedGroup ?? leafGroups[0]?.id ?? null

  const [tipsResult, refetchTips] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId: activeGroupId },
    pause: !activeGroupId,
  })

  if (!playerId) return <NeedsLogin />
  if (tournamentResult.fetching) return <Loading />
  if (!tournament) return <ErrorView />

  const activeGroup = leafGroups.find((g) => g.id === activeGroupId) ?? null
  const teams = teamIndex(tournament.teams)
  const games = activeGroup
    ? tournament.games
        .filter((g) => activeGroup.childGameIds.includes(g.id))
        .sort(byKickoff)
    : []

  const tips = tipsResult.data?.tips ?? []
  // playerId -> nick
  const players = [
    ...new Map(tips.map((tip) => [tip.playerId, tip.nick])).entries(),
  ]
  // (playerId, gameId) -> tip
  const tipKey = (p: string, g: string) => `${p}::${g}`
  const tipMap = new Map(tips.map((tip) => [tipKey(tip.playerId, tip.gameId), tip]))

  return (
    <section className="page">
      <h2>{t('allTipsTitle')}</h2>
      <GroupSubNav
        groups={tournament.groups}
        selectedId={activeGroupId}
        onSelect={setSelectedGroup}
      />

      {tipsResult.fetching && <Loading />}
      {tipsResult.error && (
        <ErrorView
          message={tipsResult.error.message}
          onRetry={() => refetchTips({ requestPolicy: 'network-only' })}
        />
      )}

      {activeGroup && !tipsResult.fetching && (
        <div className="grid-scroll">
          <table className="data-table compact">
            <thead>
              <tr>
                <th>{t('player')}</th>
                {games.map((g) => (
                  <th key={g.id}>
                    {slotCode(g.home, teams)}–{slotCode(g.away, teams)}
                  </th>
                ))}
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
                        {tip?.prediction
                          ? `${tip.prediction.homeScore}–${tip.prediction.awayScore}`
                          : tip
                            ? t('hiddenTip')
                            : '—'}
                      </td>
                    )
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}
