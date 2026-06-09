import type { Team, TeamSlot } from '../graphql/types'

/**
 * Team display is two orthogonal axes the user controls independently:
 *
 * - `FlagMode` — whether a flag is shown at all.
 * - `TextMode` — what text label accompanies it (`auto` is responsive).
 *
 * `ConcreteDisplayMode` is the resolved rendering the label layer consumes;
 * `composeDisplayMode` collapses the two axes (plus the viewport) into one.
 */
export type FlagMode = 'on' | 'off'
export type TextMode = 'auto' | 'name' | 'code' | 'off'

/** A fully-resolved rendering — what `teamLabelParts` knows how to draw. */
export type ConcreteDisplayMode =
  | 'flag'
  | 'code'
  | 'name'
  | 'flag-name'
  | 'flag-code'

/** The paired axes, as stored and passed around. */
export interface DisplayAxes {
  flag: FlagMode
  text: TextMode
}

/** Flag segments, in display order. */
export const FLAG_MODES: readonly FlagMode[] = ['on', 'off']

/** Text segments, in display order. */
export const TEXT_MODES: readonly TextMode[] = ['auto', 'name', 'code', 'off']

/**
 * Collapse the two axes (and the viewport) into a concrete rendering.
 *
 * `text: 'auto'` is the only viewport-dependent value, and it is flag-aware so
 * the label is never empty: on a narrow phone it shows nothing when a flag is
 * present (compact flag-only), otherwise the short code. The one nonsensical
 * combination — flag off, text off — is guarded in the UI; here it falls back
 * to the code so the function stays total.
 */
export function composeDisplayMode(
  flag: FlagMode,
  text: TextMode,
  isMobile: boolean,
): ConcreteDisplayMode {
  // Resolve the text axis to a concrete choice first.
  let resolvedText: 'name' | 'code' | 'none'
  if (text === 'name') resolvedText = 'name'
  else if (text === 'code') resolvedText = 'code'
  else if (text === 'off') resolvedText = 'none'
  else {
    // auto
    if (!isMobile) resolvedText = 'name'
    else resolvedText = flag === 'on' ? 'none' : 'code'
  }

  if (flag === 'on') {
    if (resolvedText === 'name') return 'flag-name'
    if (resolvedText === 'code') return 'flag-code'
    return 'flag'
  }
  // flag off — text must carry the label; 'none' would be empty, so use code.
  if (resolvedText === 'name') return 'name'
  return 'code'
}

const LEGACY_AXES: Readonly<Record<string, DisplayAxes>> = {
  auto: { flag: 'on', text: 'auto' },
  flag: { flag: 'on', text: 'off' },
  'flag-name': { flag: 'on', text: 'name' },
  'flag-code': { flag: 'on', text: 'code' },
  name: { flag: 'off', text: 'name' },
  code: { flag: 'off', text: 'code' },
}

/**
 * Translate a legacy single-enum `xpool.displayMode` value into the two axes,
 * for the one-time storage migration. Returns null for anything unrecognised.
 */
export function axesFromLegacy(legacy: string): DisplayAxes | null {
  return LEGACY_AXES[legacy] ?? null
}

/**
 * Downgrade a mode so it never shows a full team name — names become short
 * codes. Used where horizontal space is tight (e.g. All Tips column headers,
 * one per match) so headers stay compact regardless of the global preference.
 */
export function compactMode(mode: ConcreteDisplayMode): ConcreteDisplayMode {
  if (mode === 'name') return 'code'
  if (mode === 'flag-name') return 'flag-code'
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

const RENDERS_FLAG: ReadonlySet<ConcreteDisplayMode> = new Set([
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

  const wantsFlag = RENDERS_FLAG.has(mode)
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
