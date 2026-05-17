/**
 * TypeScript shapes mirroring the GraphQL schema.
 *
 * The schema is derived from `crates/domain/src/model.rs` and `API.md`. As of
 * P6 the `api` crate (P5) is still a stub, so field names below are the
 * frontend's assumed contract — see `web/README.md` "GraphQL assumptions".
 */

export type Round =
  | 'GROUP_STAGE'
  | 'R32'
  | 'R16'
  | 'QF'
  | 'SF'
  | 'THIRD_PLACE'
  | 'FINAL'

export type LockMode = 'LOCK_TOGETHER' | 'LOCK_PER_MATCH'

export interface Team {
  id: string
  name: string
  shortCode: string
  flag: string | null
  externalId: string | null
}

export interface TeamSlot {
  teamId: string | null
  description: string
}

export interface SingleGame {
  id: string
  kickoff: string // ISO-8601
  venue: string | null
  groupId: string
  home: TeamSlot
  away: TeamSlot
  /** Official 90-minute result, present once the admin has entered it. */
  result: MatchResult | null
}

export interface MatchResult {
  homeScore: number
  awayScore: number
  locked: boolean
}

export interface GroupGame {
  id: string
  name: string
  parent: string | null
  round: Round
  lockMode: LockMode
  carriesStandings: boolean
  /** Child group ids (internal node). Empty for a leaf group. */
  childGroupIds: string[]
  /** Match ids (leaf group). Empty for an internal node. */
  gameIds: string[]
  /** Earliest kickoff in the subtree — the prediction deadline. */
  deadline: string | null
}

export interface Tournament {
  root: string
  groups: GroupGame[]
  games: SingleGame[]
  teams: Team[]
}

export interface MatchPrediction {
  gameId: string
  homeScore: number
  awayScore: number
  locked: boolean
}

export interface StandingsPrediction {
  groupId: string
  ordering: string[]
  drawOrder: string[]
  locked: boolean
}

export interface Player {
  id: string
  nick: string
  fullName: string
  email: string | null
  isAdmin: boolean
  isResultUser: boolean
  matchPredictions: MatchPrediction[]
  standingsPredictions: StandingsPrediction[]
}

export interface ScoreboardEntry {
  playerId: string
  nick: string
  total: number
  /** Per-round breakdown, multipliers already applied. */
  byRound: { round: Round; points: number }[]
}

export interface Scoreboard {
  poolId: string | null
  poolName: string | null
  entries: ScoreboardEntry[]
  /** The multiplier in effect for each round (display only). */
  multipliers: { round: Round; multiplier: number }[]
}

export interface Pool {
  id: string
  name: string
  ownerId: string
  memberIds: string[]
}

export interface PlayerTip {
  playerId: string
  nick: string
  gameId: string
  homeScore: number
  awayScore: number
  /** True once the tip is locked or the match kicked off (UC-9 visibility). */
  visible: boolean
}

export interface Perfect {
  playerId: string
  nick: string
  gameId: string
}

export interface Motd {
  text: string
}
