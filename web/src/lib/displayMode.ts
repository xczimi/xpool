import type { Team, TeamSlot } from '../graphql/types'

/** How a team is displayed. `auto` resolves responsively (see resolveDisplayMode). */
export type DisplayMode =
  | 'auto'
  | 'flag'
  | 'code'
  | 'name'
  | 'flag-name'
  | 'flag-code'

/** A display mode with `auto` already resolved to a concrete rendering. */
export type ConcreteDisplayMode = Exclude<DisplayMode, 'auto'>

/** Selector options, in display order. */
export const DISPLAY_MODES: readonly DisplayMode[] = [
  'auto',
  'flag',
  'code',
  'name',
  'flag-name',
  'flag-code',
]

/**
 * Resolve `auto` against the viewport: flag-only on mobile, flag + name on
 * larger screens. Every explicit mode passes through unchanged.
 */
export function resolveDisplayMode(
  mode: DisplayMode,
  isMobile: boolean,
): ConcreteDisplayMode {
  if (mode === 'auto') {
    return isMobile ? 'flag' : 'flag-name'
  }
  return mode
}

/** A flag image reference — the ISO code drives the asset path, name is alt text. */
export interface FlagPart {
  iso: string
  name: string
}

/** What to render for one team slot, already resolved for the current mode. */
export interface TeamLabelParts {
  flag: FlagPart | null
  text: string | null
}

const FLAG_MODES: ReadonlySet<ConcreteDisplayMode> = new Set([
  'flag',
  'flag-name',
  'flag-code',
])

/**
 * Decide the flag + text to show for a slot under a concrete mode.
 *
 * - Unresolved slots (no team yet) show their placeholder description in every
 *   mode, never a flag.
 * - When a flag is wanted but the team has no ISO code, fall back to text so a
 *   slot never renders empty.
 */
export function teamLabelParts(
  slot: TeamSlot,
  teams: Map<string, Team>,
  mode: ConcreteDisplayMode,
): TeamLabelParts {
  const team = slot.teamId ? teams.get(slot.teamId) : undefined

  // Unresolved slot, or an id we don't know — placeholder/text only.
  if (!team) {
    // First non-empty of: team id, placeholder description, then 'TBD'.
    return { flag: null, text: slot.teamId || slot.description || 'TBD' }
  }

  const wantsFlag = FLAG_MODES.has(mode)
  const flag =
    wantsFlag && team.flag ? { iso: team.flag, name: team.name } : null

  let text: string | null = null
  if (mode === 'name' || mode === 'flag-name') {
    text = team.name
  } else if (mode === 'code' || mode === 'flag-code') {
    text = team.shortCode
  } else if (mode === 'flag') {
    // Flag-only — but if the flag asset is missing, fall back to the code.
    text = flag ? null : team.shortCode
  }

  return { flag, text }
}
