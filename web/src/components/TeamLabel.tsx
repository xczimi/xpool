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
 *
 * `side` controls flag placement for the scoreboard matchup layout: the home
 * team puts its flag on the right (next to the centre dash), the away team on
 * the left. Omit `side` (single-team contexts) to keep the flag on the left.
 */
export function TeamLabel({
  slot,
  teams,
  side,
}: {
  slot: TeamSlot
  teams: Map<string, Team>
  side?: 'home' | 'away'
}) {
  const mode = useResolvedDisplayMode()
  const { flag, text } = teamLabelParts(slot, teams, mode)
  const flagEl = flag && <Flag iso={flag.iso} name={flag.name} />
  const textEl = text && <span className="team-label-text">{text}</span>
  return (
    <span className="team-label">
      {side === 'home' ? (
        <>
          {textEl}
          {flagEl}
        </>
      ) : (
        <>
          {flagEl}
          {textEl}
        </>
      )}
    </span>
  )
}

/**
 * A home–away matchup, scoreboard style: the home team hugs the centre dash
 * from the right, the away team from the left, and the dash sits in a fixed
 * centre column so it lines up vertically across rows (`1fr auto 1fr`).
 */
export function Matchup({
  home,
  away,
  teams,
}: {
  home: TeamSlot
  away: TeamSlot
  teams: Map<string, Team>
}) {
  return (
    <span className="matchup">
      <span className="matchup-home">
        <TeamLabel slot={home} teams={teams} side="home" />
      </span>
      <span className="matchup-sep">–</span>
      <span className="matchup-away">
        <TeamLabel slot={away} teams={teams} side="away" />
      </span>
    </span>
  )
}
