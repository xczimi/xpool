import { useMemo, useState } from 'react'
import { useMutation, useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  ME_QUERY,
  RESULTS_QUERY,
  SUBMIT_GROUP_MUTATION,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type {
  MatchPrediction,
  Player,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { GroupSubNav } from '../components/GroupSubNav'
import { GroupTipForm } from './mytips/GroupTipForm'

/**
 * My Tips (UC-5/6) — a group-level prediction form. Picks a leaf group, edits
 * all matches, then Save draft / Lock submits the whole group via
 * `submitGroup` (API.md §6). Optimistic via urql's cache update on the
 * mutation result.
 */
export function MyTipsPage() {
  const { t } = useI18n()
  const { playerId } = useAuth()
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null)

  const [tournamentResult, refetchTournament] = useQuery<{
    tournament: Tournament | null
    motd: string | null
  }>({ query: TOURNAMENT_QUERY })
  const [meResult, refetchMe] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !playerId,
  })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })
  const [, submitGroup] = useMutation(SUBMIT_GROUP_MUTATION)

  const tournament = tournamentResult.data?.tournament ?? null
  const me = meResult.data?.me ?? null
  const results = resultsResult.data?.results ?? []

  const leafGroups = useMemo(
    () => (tournament?.groups ?? []).filter((g) => g.childGameIds.length > 0),
    [tournament],
  )

  if (!playerId) return <NeedsLogin />
  if (tournamentResult.fetching || meResult.fetching) return <Loading />
  if (tournamentResult.error)
    return (
      <ErrorView
        message={tournamentResult.error.message}
        onRetry={() => refetchTournament({ requestPolicy: 'network-only' })}
      />
    )
  if (!tournament || !me) return <ErrorView />

  const activeGroupId = selectedGroup ?? leafGroups[0]?.id ?? null
  const activeGroup =
    leafGroups.find((g) => g.id === activeGroupId) ?? null

  return (
    <section className="page">
      <h2>{t('myTipsTitle')}</h2>
      <GroupSubNav
        groups={tournament.groups}
        selectedId={activeGroupId}
        onSelect={setSelectedGroup}
      />
      {activeGroup ? (
        <GroupTipForm
          key={activeGroup.id}
          tournament={tournament}
          group={activeGroup}
          me={me}
          results={results}
          onSubmit={async (predictions, standings, lock) => {
            const res = await submitGroup({
              groupId: activeGroup.id,
              predictions,
              standings,
              lock,
            })
            await refetchMe({ requestPolicy: 'network-only' })
            return res
          }}
        />
      ) : (
        <p>{t('selectGroup')}</p>
      )}
    </section>
  )
}
