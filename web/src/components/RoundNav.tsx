import type { GroupGame, Round } from '../graphql/types'
import { useI18n } from '../i18n/useI18n'
import { leafGroupsOfRound, roundLabel, roundNodes } from '../lib/rounds'
import { GroupSubNav } from './GroupSubNav'

/**
 * Two-level tournament navigation for My Tips / All Tips. Row 1 is a tab per
 * round node (Group Stage / R32 / … / Final). Row 2 is the group pills — only
 * the Group Stage round has one; knockout rounds show all their matches in the
 * page body instead.
 */
export function RoundNav({
  groups,
  selectedRound,
  onSelectRound,
  selectedGroupId,
  onSelectGroup,
}: {
  groups: GroupGame[]
  selectedRound: Round | null
  onSelectRound: (round: Round) => void
  selectedGroupId: string | null
  onSelectGroup: (groupId: string) => void
}) {
  const { t } = useI18n()
  const rounds = roundNodes(groups)
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
