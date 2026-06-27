import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useMutation, useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  ME_QUERY,
  REPORTED_RESULTS_QUERY,
  RESULTS_QUERY,
  STANDINGS_QUERY,
  SUBMIT_GROUP_MUTATION,
  THIRD_PLACE_QUERY,
  TIPS_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type {
  GroupGame,
  MatchPrediction,
  Me,
  PointsBreakdown,
  ReportedResult,
  Round,
  StandingsScore,
  ThirdPlaceRanking,
  Tip,
  Tournament,
} from '../graphql/types'
import { teamIndex } from '../lib/format'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { ThirdPlaceTable } from '../components/ThirdPlaceTable'
import { RoundNav } from '../components/RoundNav'
import { Countdown } from '../components/Countdown'
import { currentRoundNode, leafGroupsOfRound, visibleRoundNodes } from '../lib/rounds'
import { resolveGroupParam, roundNodeIdFor } from '../lib/groupRoute'
import { readTipsGroup, writeTipsGroup } from '../lib/tipsGroup'
import { useServerClock } from '../lib/useServerClock'
import { GroupTipForm } from './mytips/GroupTipForm'

/**
 * My Tips (UC-5/6) — a group-level prediction form. Round tabs pick a round;
 * the Group Stage round drills into one leaf group, a knockout round shows all
 * its one-match groups stacked. Save draft / Lock submits a group via
 * `submitGroup` (API.md §6).
 */
export function MyTipsPage() {
  const { t, locale } = useI18n()
  const { label } = useAuth()
  // The URL is the source of truth for which round/group is open
  // (`/mytips/:groupId`). Local state is only the fallback when no param is
  // present (`/mytips`) — the default-round/group behaviour.
  const { groupId: groupParam } = useParams<{ groupId?: string }>()
  const navigate = useNavigate()
  const [selectedRound, setSelectedRound] = useState<Round | null>(null)
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null)

  const [tournamentResult, refetchTournament] = useQuery<{
    tournament: Tournament | null
    now: string
  }>({ query: TOURNAMENT_QUERY })
  const [meResult, refetchMe] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !label,
  })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })
  const [, submitGroup] = useMutation(SUBMIT_GROUP_MUTATION)

  // Estimated server-now, ticking once a second, anchored to the GraphQL `now`
  // — drives the finalize countdowns without ever gating locking on the
  // browser clock (CLAUDE.md server-authoritative clock).
  const serverNowMs = useServerClock(tournamentResult.data?.now)

  const tournament = tournamentResult.data?.tournament ?? null
  const meRaw = meResult.data?.me ?? null
  const me = meRaw?.__typename === 'Player' ? meRaw : null

  // Predicted ranking (this player) + official ranking, shown side by side.
  const [myThirdsResult] = useQuery<{ thirdPlaceRanking: ThirdPlaceRanking }>({
    query: THIRD_PLACE_QUERY,
    variables: { player: me?.id ?? null },
    pause: !me,
  })
  const [officialThirdsResult] = useQuery<{ thirdPlaceRanking: ThirdPlaceRanking }>({
    query: THIRD_PLACE_QUERY,
    variables: { player: null },
  })
  const myThirds = myThirdsResult.data?.thirdPlaceRanking ?? null
  const officialThirds = officialThirdsResult.data?.thirdPlaceRanking ?? null

  const results = resultsResult.data?.results ?? []

  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )

  const rounds = useMemo(
    () => visibleRoundNodes(tournament?.groups ?? [], tournament?.games ?? []),
    [tournament?.groups, tournament?.games],
  )

  // The URL param, resolved against the loaded group tree, is authoritative
  // when it names a real group/round node. `/mytips` (no param) → null, which
  // falls through to the default round/group state below.
  const paramResolved = useMemo(
    () => resolveGroupParam(tournament?.groups ?? [], groupParam),
    [tournament?.groups, groupParam],
  )

  // With no URL group, fall back to the last group the viewer looked at (shared
  // with All Tips via localStorage) so switching pages lands on the same group.
  // Honoured only when its round is still visible.
  const storedResolved = useMemo(() => {
    const r = resolveGroupParam(
      tournament?.groups ?? [],
      readTipsGroup() ?? undefined,
    )
    return r && rounds.some((n) => n.round === r.round) ? r : null
  }, [tournament?.groups, rounds])
  const effectiveResolved = paramResolved ?? storedResolved

  const activeRound =
    effectiveResolved?.round ??
    selectedRound ??
    currentRoundNode(rounds)?.round ??
    rounds[0]?.round ??
    null

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

  // Round/group selection — derived above the early returns so the tips query
  // (for per-game earned points) can key off it without a conditional hook.
  // Group Stage drills into one selected leaf group; knockout rounds show every
  // one-match group stacked. The `tips` query takes the round node id for
  // knockout (its resolver walks the subtree) or the leaf group id otherwise.
  const activeRoundNode = rounds.find((r) => r.round === activeRound) ?? null
  const roundLeaves = activeRoundNode
    ? leafGroupsOfRound(activeRoundNode, tournament?.groups ?? [])
    : []
  const isGroupStage = activeRound === 'GROUP_STAGE'
  // A leaf-group param pins the group; a round-node param (groupId === null)
  // and the no-param case both fall back to local state then the first leaf.
  const activeGroupId =
    effectiveResolved?.groupId ?? selectedGroupId ?? roundLeaves[0]?.id ?? null
  const tipsGroupId = isGroupStage ? activeGroupId : (activeRoundNode?.id ?? null)

  // Remember the group across My Tips ⇄ All Tips (and reloads).
  useEffect(() => {
    if (tipsGroupId) writeTipsGroup(tipsGroupId)
  }, [tipsGroupId])

  // The server computes per-(player, game) earned points + breakdown on the tip
  // grid, and the per-group standings bonus on the standings query; we reuse
  // both (same source as All Tips) rather than re-deriving scoring on the
  // client. Keep only the current player's rows.
  const myId = meRaw?.__typename === 'Player' ? meRaw.id : null
  const [tipsResult] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId: tipsGroupId },
    pause: !label || !tipsGroupId,
  })
  const [standingsResult] = useQuery<{ standings: StandingsScore[] }>({
    query: STANDINGS_QUERY,
    variables: { groupId: tipsGroupId },
    pause: !label || !tipsGroupId,
  })

  // The result user (official-results admin) gets SportsDB pre-fill: when they
  // open a group with result-pending games, auto-fetch reported scores. The
  // query is admin-gated server-side and returns [] when SportsDB is absent, so
  // this is a no-op for everyone else / when unconfigured.
  const isResultUser = meRaw?.__typename === 'Player' && meRaw.isResultUser
  const [reportedResult] = useQuery<{ reportedResults: ReportedResult[] }>({
    query: REPORTED_RESULTS_QUERY,
    variables: { groupId: tipsGroupId },
    pause: !label || !tipsGroupId || !isResultUser,
  })
  const reportedByGame = useMemo(() => {
    const map = new Map<string, ReportedResult>()
    for (const r of reportedResult.data?.reportedResults ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [reportedResult.data])

  const pointsByGame = useMemo(() => {
    const map = new Map<
      string,
      { breakdown: PointsBreakdown | null; isPerfect: boolean }
    >()
    if (!myId) return map
    for (const tip of tipsResult.data?.tips ?? []) {
      if (tip.playerId === myId) {
        map.set(tip.gameId, { breakdown: tip.breakdown, isPerfect: tip.isPerfect })
      }
    }
    return map
  }, [tipsResult.data, myId])
  const standingsByGroup = useMemo(() => {
    const map = new Map<string, StandingsScore>()
    if (!myId) return map
    for (const s of standingsResult.data?.standings ?? []) {
      if (s.playerId === myId) map.set(s.groupId, s)
    }
    return map
  }, [standingsResult.data, myId])

  if (!label) return <NeedsLogin />
  if (tournamentResult.fetching || meResult.fetching) return <Loading />
  if (tournamentResult.error)
    return (
      <ErrorView
        message={tournamentResult.error.message}
        onRetry={() => refetchTournament({ requestPolicy: 'network-only' })}
      />
    )
  if (!tournament || !me) return <ErrorView />

  const shownGroups: GroupGame[] = isGroupStage
    ? roundLeaves.filter((g) => g.id === activeGroupId)
    : roundLeaves

  // A leaf group this player has finalized — every child game locked. Mirrors
  // the per-group `groupLocked` signature below.
  const playerFinalized = (g: GroupGame): boolean =>
    g.childGameIds.length > 0 &&
    g.childGameIds.every(
      (id) => me.matchPredictions.find((p) => p.gameId === id)?.locked,
    )

  // The soonest deadline still open to finalize, across the visible rounds'
  // leaf groups — the page-level "next to finalize" nudge. The result user is
  // never bound by deadlines, so they get no banner.
  const nextFinalize = me.isResultUser
    ? null
    : (rounds
        .flatMap((r) => leafGroupsOfRound(r, tournament.groups))
        .filter(
          (g) => g.deadline && !g.deadlinePassed && !playerFinalized(g),
        )
        .sort(
          (a, b) => Date.parse(a.deadline!) - Date.parse(b.deadline!),
        )[0] ?? null)

  const refetchAll = () => {
    refetchTournament({ requestPolicy: 'network-only' })
    refetchMe({ requestPolicy: 'network-only' })
  }

  return (
    <section className="page">
      <h2>{t('myTipsTitle')}</h2>
      {nextFinalize && (
        <p className="finalize-banner">
          ⏰ {t('nextToFinalize')}: {nextFinalize.name}
          {' · '}
          <Countdown
            deadline={nextFinalize.deadline}
            serverNowMs={serverNowMs}
            onExpire={refetchAll}
          />
        </p>
      )}
      <RoundNav
        groups={tournament.groups}
        games={tournament.games}
        selectedRound={activeRound}
        onSelectRound={(round) => {
          // Local state for immediate feedback; the URL is authoritative once
          // the navigation lands. A round tab carries no group, so we navigate
          // to the round-node id (`/mytips/R32`); group stage falls back to its
          // first group there.
          setSelectedRound(round)
          setSelectedGroupId(null)
          const nodeId = roundNodeIdFor(tournament.groups, round)
          navigate(nodeId ? `/mytips/${nodeId}` : '/mytips')
        }}
        selectedGroupId={activeGroupId}
        onSelectGroup={(groupId) => {
          setSelectedGroupId(groupId)
          navigate(`/mytips/${groupId}`)
        }}
      />
      {shownGroups.length > 0 ? (
        shownGroups.map((group) => {
          // Remount the form when this group's locked state flips, so a
          // successful Lock re-seeds the form from the refetched `me` (the
          // server is the source of truth). GroupTipForm seeds its match state
          // via useState (init runs once); without a key change the locked
          // flags from the refetch never resynced and the group kept rendering
          // "Draft" with the Lock button live — inviting a second, rejected
          // submit. The signature is per-group, so only the locked group
          // remounts (other groups keep their in-progress drafts).
          const groupLocked =
            group.childGameIds.length > 0 &&
            group.childGameIds.every(
              (id) => me.matchPredictions.find((p) => p.gameId === id)?.locked,
            )
          return (
            <GroupTipForm
              key={`${group.id}:${groupLocked ? 'locked' : 'draft'}`}
              tournament={tournament}
              group={group}
              me={me}
              results={results}
              pointsByGame={pointsByGame}
              standings={standingsByGroup.get(group.id) ?? null}
              serverNowMs={serverNowMs}
              onExpire={refetchAll}
              reported={reportedByGame}
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
          )
        })
      ) : (
        <p>{t('selectGroup')}</p>
      )}
      <div className="thirds-section" data-testid="third-place-section">
        <h3>{t('thirdsTitle')}</h3>
        <p className="hint">{t('thirdsBlurb')}</p>
        <div className="standings-pair">
          <ThirdPlaceTable title={t('thirdsPredicted')} ranking={myThirds} teams={teams} />
          <ThirdPlaceTable title={t('thirdsOfficial')} ranking={officialThirds} teams={teams} />
        </div>
      </div>
    </section>
  )
}
