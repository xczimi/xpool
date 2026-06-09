import type { GroupGame, Round, SingleGame } from '../graphql/types'
import { useI18n } from '../i18n/useI18n'
import { leafGroupsOfRound, roundLabel, visibleRoundNodes } from '../lib/rounds'
import { GroupSubNav } from './GroupSubNav'

/**
 * Two-level tournament navigation for My Tips / All Tips. Row 1 is a tab per
 * round node (Group Stage / R32 / … / Final). Row 2 is the group pills — only
 * the Group Stage round has one; knockout rounds show all their matches in the
 * page body instead.
 *
 * Only rounds that are ready for predictions are shown — a future round whose
 * teams are still unknown (no game with both teams determined) is hidden until
 * the official results resolve it. See `visibleRoundNodes`.
 */
export function RoundNav({
  groups,
  games,
  selectedRound,
  onSelectRound,
  selectedGroupId,
  onSelectGroup,
}: {
  groups: GroupGame[]
  games: SingleGame[]
  selectedRound: Round | null
  onSelectRound: (round: Round) => void
  selectedGroupId: string | null
  onSelectGroup: (groupId: string) => void
}) {
  const { t } = useI18n()
  const rounds = visibleRoundNodes(groups, games)
  const activeNode = rounds.find((r) => r.round === selectedRound) ?? null

  return (
    <div className="round-nav">
      <div className="round-tabs">
        {rounds.map((node) => (
          <button
            key={node.id}
            type="button"
            className={node.round === selectedRound ? 'round-tab active' : 'round-tab'}
            onClick={() => onSelectRound(node.round)}
          >
            {roundLabel(node.round, t)}
          </button>
        ))}
      </div>
      {activeNode?.round === 'GROUP_STAGE' && (
        <GroupSubNav
          groups={leafGroupsOfRound(activeNode, groups)}
          selectedId={selectedGroupId}
          onSelect={onSelectGroup}
        />
      )}
    </div>
  )
}
