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
  /** Kicks off on the current (server) calendar day. */
  isToday: boolean
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
  __typename: 'Player'
  id: string
  nick: string
  fullName: string
  /** The result user IS the admin — gate admin features on this flag. */
  isResultUser: boolean
  /** Whether this viewer may create pools (result user or a direct referral). */
  mayCreatePool: boolean
  version: number
  matchPredictions: MatchPrediction[]
  standingsPredictions: StandingsPrediction[]
}

/** Authenticated but not yet linked to a Player — invite / claim flow. */
export interface UnclaimedViewer {
  __typename: 'UnclaimedViewer'
  email?: string | null
  phone?: string | null
  linkCandidate?: { personId: string; provider: string } | null
}

/** The result of the `me` query — a Player, an unclaimed viewer, or null for a visitor. */
export type Me = Player | UnclaimedViewer | null

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
  /** Cosmetic invite-link prefix; a bare prefix resolves to the owner invite. */
  prefix: string
}

/** How a single prediction earned its points, component by component. */
export interface PointsBreakdown {
  exactHome: boolean
  exactAway: boolean
  outcome: boolean
  /** Base points before the round multiplier (0–4). */
  base: number
  multiplier: number
  /** Final points (base × multiplier). */
  points: number
}

export interface Tip {
  playerId: string
  nick: string
  gameId: string
  /** Null when the prediction is still hidden from others (UC-9 visibility). */
  prediction: MatchPrediction | null
  /** Round-multiplied points earned; null until the game has an official result. */
  points: number | null
  /** Whether the prediction scored a perfect (max base points). */
  isPerfect: boolean
  /** Component breakdown of `points` — null whenever `points` is. */
  breakdown: PointsBreakdown | null
  /** Best still-reachable points for THIS match while it is live; null otherwise. */
  maxReachable: number | null
}

export interface Perfect {
  playerId: string
  nick: string
  gameId: string
  /** Round-multiplied points earned (a perfect always has a result). */
  points: number
  breakdown: PointsBreakdown
}

/** One player's standings (group-table) bonus for one group. */
export interface StandingsScore {
  playerId: string
  nick: string
  groupId: string
  pairsCorrect: number
  pairsTotal: number
  /** Raw bonus before the round multiplier. */
  bonus: number
  multiplier: number
  /** Final standings points (bonus × multiplier). */
  points: number
}

/** Lightweight player listing — dev-login picker, admin player list. */
export interface PlayerSummary {
  id: string
  nick: string
  fullName: string
  isResultUser: boolean
}

/** One game's score as reported by SportsDB (admin-gated, pre-fill only). */
export interface ReportedResult {
  gameId: string
  homeScore: number
  awayScore: number
  source: string
  sourceStatus: string
  ninetyMinuteUncertain: boolean
}

export interface MatchScore {
  homeScore: number
  awayScore: number
  /** true = live "if it ended now"; false = official entered result. */
  provisional: boolean
  /** "thesportsdb" when provisional; null for an official result. */
  source: string | null
  /** SportsDB status (e.g. "2H") when provisional; null otherwise. */
  sourceStatus: string | null
  ninetyMinuteUncertain: boolean
}

export interface MatchDetail {
  game: SingleGame
  /** Null until there is a score to show (upcoming, or source absent). */
  actual: MatchScore | null
  rows: Tip[]
}

export interface ThirdPlaceEntry {
  group: string
  team: Team
  points: number
  goalDiff: number
  goalsFor: number
  /** 1-based ranking position (best = 1). */
  rank: number
  /** Top-8 → advances to the R32. */
  qualifies: boolean
  /** The group-winner faced in the R32 (e.g. "E"), once known. */
  facesWinnerGroup: string | null
  /** The R32 game id this third plays in, once known. */
  facesGame: string | null
}

export interface ThirdPlaceRanking {
  entries: ThirdPlaceEntry[]
  /** True once all 12 groups' thirds are final. */
  complete: boolean
}
