import { useRef } from 'react'
import type { OperationResult } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import type {
  GroupGame,
  MatchPrediction,
  Player,
  PointsBreakdown,
  StandingsScore,
  Tournament,
} from '../../graphql/types'
import { MobileGroupCard } from './MobileGroupCard'
import type { PredictionInput, StandingsInput } from './types'

/**
 * Swipe one-group-per-screen mobile prediction flow. Shows a progress line
 * ("Group C · 3 of 12"), Prev/Next controls and left/right swipe, and renders
 * the active group's `MobileGroupCard`. Group selection is driven through
 * `onSelectGroup` (which navigates `/mytips/<id>`), so the URL stays the source
 * of truth and the desktop nav stays consistent.
 */
export function MobileGroupEntry({
  tournament,
  groups,
  activeGroupId,
  onSelectGroup,
  me,
  results,
  pointsByGame,
  standingsByGroup,
  serverNowMs,
  onExpire,
  onAutosave,
  onFinalize,
}: {
  tournament: Tournament
  /** All group-stage leaf groups, in display order. */
  groups: GroupGame[]
  activeGroupId: string | null
  onSelectGroup: (groupId: string) => void
  me: Player
  results: MatchPrediction[]
  pointsByGame?: Map<
    string,
    { breakdown: PointsBreakdown | null; isPerfect: boolean }
  >
  standingsByGroup: Map<string, StandingsScore>
  serverNowMs: number
  onExpire?: () => void
  onAutosave: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
  onFinalize: (
    groupId: string,
    predictions: PredictionInput[],
    standings: StandingsInput | null,
  ) => Promise<OperationResult>
}) {
  const { t } = useI18n()
  const startX = useRef<number | null>(null)

  const rawIndex = groups.findIndex((g) => g.id === activeGroupId)
  const index = rawIndex >= 0 ? rawIndex : 0
  const active = groups[index]
  const total = groups.length

  const goto = (i: number) => {
    if (i >= 0 && i < total) onSelectGroup(groups[i].id)
  }

  const onTouchStart = (e: React.TouchEvent) => {
    startX.current = e.changedTouches[0].clientX
  }
  const onTouchEnd = (e: React.TouchEvent) => {
    if (startX.current === null) return
    const dx = e.changedTouches[0].clientX - startX.current
    startX.current = null
    if (Math.abs(dx) < 50) return
    goto(dx < 0 ? index + 1 : index - 1)
  }

  if (!active) return null

  return (
    <div className="mobile-entry" onTouchStart={onTouchStart} onTouchEnd={onTouchEnd}>
      <div className="mobile-entry-progress">
        <span className="mobile-entry-label">
          {active.name} · {index + 1} {t('mobileOf')} {total}
        </span>
        <span className="mobile-entry-nav">
          <button
            type="button"
            className="mobile-entry-prev"
            disabled={index === 0}
            onClick={() => goto(index - 1)}
          >
            {t('prevGroup')}
          </button>
          <button
            type="button"
            className="mobile-entry-next"
            disabled={index === total - 1}
            onClick={() => goto(index + 1)}
          >
            {t('nextGroup')}
          </button>
        </span>
      </div>
      <MobileGroupCard
        key={active.id}
        tournament={tournament}
        group={active}
        me={me}
        results={results}
        pointsByGame={pointsByGame}
        standings={standingsByGroup.get(active.id) ?? null}
        serverNowMs={serverNowMs}
        onExpire={onExpire}
        onAutosave={onAutosave}
        onFinalize={onFinalize}
      />
    </div>
  )
}
