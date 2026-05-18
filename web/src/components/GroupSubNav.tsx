import type { GroupGame } from '../graphql/types'
import { chronologicalLeafGroups } from '../lib/rounds'

/**
 * Sub-navigation for tournament groups — leaf groups only (those holding
 * matches). Used by My Tips and All Tips.
 */
export function GroupSubNav({
  groups,
  selectedId,
  onSelect,
}: {
  groups: GroupGame[]
  selectedId: string | null
  onSelect: (groupId: string) => void
}) {
  const leaves = chronologicalLeafGroups(groups)
  return (
    <div className="group-subnav">
      {leaves.map((g) => (
        <button
          key={g.id}
          type="button"
          className={g.id === selectedId ? 'subnav-item active' : 'subnav-item'}
          onClick={() => onSelect(g.id)}
        >
          {g.name}
        </button>
      ))}
    </div>
  )
}
