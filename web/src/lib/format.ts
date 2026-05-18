import type { SingleGame, Team, TeamSlot } from '../graphql/types'

/** Index teams by id for quick lookup. */
export function teamIndex(teams: Team[]): Map<string, Team> {
  return new Map(teams.map((t) => [t.id, t]))
}

/** Display label for a team slot — the team name, or the placeholder. */
export function slotLabel(slot: TeamSlot, teams: Map<string, Team>): string {
  if (slot.teamId) {
    return teams.get(slot.teamId)?.name ?? slot.teamId
  }
  return slot.description || 'TBD'
}

/** Short code for a team slot (used in dense grids). */
export function slotCode(slot: TeamSlot, teams: Map<string, Team>): string {
  if (slot.teamId) {
    return teams.get(slot.teamId)?.shortCode ?? slot.teamId
  }
  return slot.description || 'TBD'
}

/** Locale-aware kickoff formatting. */
export function formatKickoff(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString(locale, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

/** Sort matches by kickoff ascending. */
export function byKickoff(a: SingleGame, b: SingleGame): number {
  return Date.parse(a.kickoff) - Date.parse(b.kickoff)
}
