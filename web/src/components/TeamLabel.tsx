import type { Team, TeamSlot } from '../graphql/types'
import { compactMode, teamLabelParts } from '../lib/displayMode'
import { useResolvedDisplayMode } from '../display/useResolvedDisplayMode'

/**
 * An 8-bit flag image. The ISO code drives the bundled asset path. `className`
 * defaults to `team-flag` (the schedule/scoreboard flags); the language picker
 * passes `lang-flag` so the two are distinguishable in markup and tests.
 */
export function Flag({
  iso,
  name,
  className = 'team-flag',
}: {
  iso: string
  name: string
  className?: string
}) {
  return (
    <img
      className={className}
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
 *
 * `compact` forces names down to short codes (see `compactMode`) for dense,
 * space-constrained contexts — the flag/flag-only modes are unaffected.
 */
export function TeamLabel({
  slot,
  teams,
  side,
  compact,
}: {
  slot: TeamSlot
  teams: Map<string, Team>
  side?: 'home' | 'away'
  compact?: boolean
}) {
  const resolved = useResolvedDisplayMode()
  const mode = compact ? compactMode(resolved) : resolved
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
 *
 * `compact` is forwarded to both team labels (names → codes) for dense layouts
 * such as the All Tips column headers.
 */
export function Matchup({
  home,
  away,
  teams,
  compact,
}: {
  home: TeamSlot
  away: TeamSlot
  teams: Map<string, Team>
  compact?: boolean
}) {
  return (
    <span className="matchup">
      <span className="matchup-home">
        <TeamLabel slot={home} teams={teams} side="home" compact={compact} />
      </span>
      <span className="matchup-sep">–</span>
      <span className="matchup-away">
        <TeamLabel slot={away} teams={teams} side="away" compact={compact} />
      </span>
    </span>
  )
}
