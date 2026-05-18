/**
 * TypeScript shapes mirroring the GraphQL schema.
 *
 * The schema is the agreed reconciled contract — see `web/README.md`
 * "GraphQL assumptions". It mirrors the `api` crate's `gql` types exactly.
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
  resultPending: boolean
  withinTodayWindow: boolean
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
  childGameIds: string[]
  /** Earliest kickoff in the subtree — the prediction deadline. */
  deadline: string | null
  deadlinePassed: boolean
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
  /** The result user IS the admin — gate admin features on this flag. */
  isResultUser: boolean
  version: number
  matchPredictions: MatchPrediction[]
  standingsPredictions: StandingsPrediction[]
}

/** One row of the materialised scoreboard. `scoreboard` returns these directly. */
export interface ScoreEntry {
  playerId: string
  nick: string
  total: number
  /** Per-round point breakdown (multipliers already applied server-side). */
  stages: StageScore[]
}

export interface StageScore {
  round: Round
  points: number
}

export interface Pool {
  id: string
  name: string
  owner: string
  members: string[]
  joinCode: string
}

export interface Tip {
  playerId: string
  nick: string
  gameId: string
  /** Null when the prediction is still hidden from others (UC-9 visibility). */
  prediction: MatchPrediction | null
}

export interface Perfect {
  playerId: string
  nick: string
  gameId: string
}

/** Lightweight player listing — dev-login picker, admin player list. */
export interface PlayerSummary {
  id: string
  nick: string
  fullName: string
  isResultUser: boolean
}
