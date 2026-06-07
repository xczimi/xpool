import type { Team, TeamSlot } from '../graphql/types'
import { teamLabelParts } from '../lib/displayMode'
import { useResolvedDisplayMode } from '../display/useResolvedDisplayMode'

/** An 8-bit flag image. The ISO code drives the bundled asset path. */
export function Flag({ iso, name }: { iso: string; name: string }) {
  return (
    <img
      className="team-flag"
      src={`/flags/${iso}.png`}
      alt={name}
      title={name}
      loading="lazy"
      width={20}
      height={15}
    />
  )
}

/**
 * Render a team slot per the current display mode (flag / code / name / combo).
 * Falls back to text when a flag asset is unavailable, and shows the
 * placeholder description for unresolved knockout slots.
 */
export function TeamLabel({
  slot,
  teams,
}: {
  slot: TeamSlot
  teams: Map<string, Team>
}) {
  const mode = useResolvedDisplayMode()
  const { flag, text } = teamLabelParts(slot, teams, mode)
  return (
    <span className="team-label">
      {flag && <Flag iso={flag.iso} name={flag.name} />}
      {text && <span className="team-label-text">{text}</span>}
    </span>
  )
}
