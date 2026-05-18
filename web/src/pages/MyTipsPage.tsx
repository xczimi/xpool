import { useEffect, useMemo, useRef, useState } from 'react'
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
  GroupGame,
  MatchPrediction,
  Player,
  Round,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { RoundNav } from '../components/RoundNav'
import { currentRoundNode, leafGroupsOfRound, roundNodes } from '../lib/rounds'
import { GroupTipForm } from './mytips/GroupTipForm'

/**
 * My Tips (UC-5/6) — a group-level prediction form. Round tabs pick a round;
 * the Group Stage round drills into one leaf group, a knockout round shows all
 * its one-match groups stacked. Save draft / Lock submits a group via
 * `submitGroup` (API.md §6).
 */
export function MyTipsPage() {
  const { t } = useI18n()
  const { playerId } = useAuth()
  const [selectedRound, setSelectedRound] = useState<Round | null>(null)
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null)

  const [tournamentResult, refetchTournament] = useQuery<{
    tournament: Tournament | null
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

  const rounds = useMemo(
    () => roundNodes(tournament?.groups ?? []),
    [tournament?.groups],
  )
  const activeRound =
    selectedRound ?? currentRoundNode(rounds)?.round ?? rounds[0]?.round ?? null

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

  const activeRoundNode = rounds.find((r) => r.round === activeRound) ?? null
  const roundLeaves = activeRoundNode
    ? leafGroupsOfRound(activeRoundNode, tournament.groups)
    : []

  // Group Stage drills into one selected group; knockout rounds show every
  // one-match group stacked.
  const isGroupStage = activeRound === 'GROUP_STAGE'
  const activeGroupId = selectedGroupId ?? roundLeaves[0]?.id ?? null
  const shownGroups: GroupGame[] = isGroupStage
    ? roundLeaves.filter((g) => g.id === activeGroupId)
    : roundLeaves

  return (
    <section className="page">
      <h2>{t('myTipsTitle')}</h2>
      <RoundNav
        groups={tournament.groups}
        selectedRound={activeRound}
        onSelectRound={(round) => {
          setSelectedRound(round)
          setSelectedGroupId(null)
        }}
        selectedGroupId={activeGroupId}
        onSelectGroup={setSelectedGroupId}
      />
      {shownGroups.length > 0 ? (
        shownGroups.map((group) => (
          <GroupTipForm
            key={group.id}
            tournament={tournament}
            group={group}
            me={me}
            results={results}
            onSubmit={async (predictions, standings, lock) => {
              const res = await submitGroup({
                groupId: group.id,
                predictions,
                standings,
                lock,
              })
              await refetchMe({ requestPolicy: 'network-only' })
              return res
            }}
          />
        ))
      ) : (
        <p>{t('selectGroup')}</p>
      )}
    </section>
  )
}
