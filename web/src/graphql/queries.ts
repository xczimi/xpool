/** GraphQL query & mutation documents — the agreed reconciled schema. */

export const TOURNAMENT_QUERY = `
  query Tournament {
    tournament {
      root
      groups {
        id name parent round lockMode carriesStandings
        childGroupIds childGameIds deadline deadlinePassed
      }
      games {
        id kickoff venue groupId
        home { teamId description }
        away { teamId description }
        resultPending withinTodayWindow isToday
      }
      teams { id name shortCode flag externalId }
    }
    now
  }
`

/**
 * Slim games+teams list for the dev clock's game-relative presets. Dev-only UI
 * (the `DevAuthBar` path) — never mounts in production, so no prod cost. urql
 * caches it.
 */
export const DEV_CLOCK_GAMES_QUERY = `
  query DevClockGames {
    tournament {
      games {
        id kickoff
        home { teamId description }
        away { teamId description }
      }
      teams { id shortCode }
    }
  }
`

export const ME_QUERY = `
  query Me {
    me {
      __typename
      ... on Player {
        id nick fullName isResultUser mayCreatePool version
        matchPredictions { gameId homeScore awayScore locked }
        standingsPredictions { groupId ordering drawOrder locked }
      }
      ... on UnclaimedViewer {
        email
        phone
        linkCandidate { personId provider }
      }
    }
  }
`

/** The result user's locked match predictions — official scores. */
export const RESULTS_QUERY = `
  query Results {
    results { gameId homeScore awayScore locked }
  }
`

export const SCOREBOARD_QUERY = `
  query Scoreboard($pool: ID) {
    scoreboard(pool: $pool) {
      playerId nick total
      stages { round points }
    }
  }
`

/**
 * Knockout-only scoreboard — re-sums points from knockout-stage matches only
 * (re-engagement view). The field is aliased back to `scoreboard` so the page
 * reads the same `data.scoreboard` shape in both modes.
 */
export const KNOCKOUT_SCOREBOARD_QUERY = `
  query KnockoutScoreboard($pool: ID) {
    scoreboard: knockoutScoreboard(pool: $pool) {
      playerId nick total
      stages { round points }
    }
  }
`

/**
 * Per-game cumulative points over time — one series per competitor. The x-axis
 * is the schedule walked game by game (kickoff order), so each line climbs as
 * matches are scored. Pool-scoped like the scoreboard.
 */
export const POINTS_TIMELINE_QUERY = `
  query PointsTimeline($pool: ID) {
    pointsTimeline(pool: $pool) {
      playerId
      nick
      points { gameId kickoff points cumulative }
    }
  }
`

export const POOLS_QUERY = `
  query Pools {
    pools { id name owner members prefix }
  }
`

export const TIPS_QUERY = `
  query Tips($groupId: ID!, $pool: ID) {
    tips(groupId: $groupId, pool: $pool) {
      playerId nick gameId
      prediction { gameId homeScore awayScore locked }
      points isPerfect
      breakdown { exactHome exactAway outcome base multiplier points }
    }
  }
`

/** Per-player standings (group-table) bonus for the leaf groups under a node. */
export const STANDINGS_QUERY = `
  query Standings($groupId: ID!) {
    standings(groupId: $groupId) {
      playerId nick groupId pairsCorrect pairsTotal bonus multiplier points
    }
  }
`

export const PERFECTS_QUERY = `
  query Perfects($pool: ID) {
    perfects(pool: $pool) {
      playerId nick gameId points
      breakdown { exactHome exactAway outcome base multiplier points }
    }
  }
`

/** All players — for the dev-login picker and the admin player list. */
export const PLAYERS_QUERY = `
  query Players {
    players { id nick fullName isResultUser }
  }
`

export const SUBMIT_GROUP_MUTATION = `
  mutation SubmitGroup(
    $groupId: ID!
    $predictions: [MatchPredictionInput!]!
    $standings: StandingsInput
    $lock: Boolean!
  ) {
    submitGroup(
      groupId: $groupId
      predictions: $predictions
      standings: $standings
      lock: $lock
    ) {
      id version
      matchPredictions { gameId homeScore awayScore locked }
      standingsPredictions { groupId ordering drawOrder locked }
    }
  }
`

export const UPDATE_PROFILE_MUTATION = `
  mutation UpdateProfile($nick: String, $fullName: String) {
    updateProfile(nick: $nick, fullName: $fullName) {
      id nick fullName
    }
  }
`

const POOL_FIELDS = 'id name owner members prefix'

export const CREATE_POOL_MUTATION = `
  mutation CreatePool($id: ID!, $name: String!) {
    createPool(id: $id, name: $name) { ${POOL_FIELDS} }
  }
`

export const UPDATE_POOL_MUTATION = `
  mutation UpdatePool($id: ID!, $name: String!) {
    updatePool(id: $id, name: $name) { ${POOL_FIELDS} }
  }
`

/** Accept an invite (lenient: full link, bare suffix, or bare prefix). */
export const JOIN_MUTATION = `
  mutation Join($code: String!) {
    join(code: $code) { ${POOL_FIELDS} }
  }
`

export const LEAVE_POOL_MUTATION = `
  mutation LeavePool($id: ID!) {
    leavePool(id: $id) { ${POOL_FIELDS} }
  }
`

export const REMOVE_MEMBER_MUTATION = `
  mutation RemoveMember($poolId: ID!, $memberId: ID!) {
    removeMember(poolId: $poolId, memberId: $memberId) { ${POOL_FIELDS} }
  }
`

/** Hand a pool over to one of its members (owner-only). */
export const TRANSFER_OWNERSHIP_MUTATION = `
  mutation TransferOwnership($poolId: ID!, $newOwner: ID!) {
    transferOwnership(poolId: $poolId, newOwner: $newOwner) { ${POOL_FIELDS} }
  }
`

/** Mint or reuse the current member's invite into a pool. */
export const CREATE_INVITE_MUTATION = `
  mutation CreateInvite($pool: ID!) {
    createInvite(pool: $pool) { code link }
  }
`

/** Revoke one of your invites (rotation = revoke + re-mint). */
export const REVOKE_INVITE_MUTATION = `
  mutation RevokeInvite($code: String!) {
    revokeInvite(code: $code)
  }
`

export const DELETE_POOL_MUTATION = `
  mutation DeletePool($id: ID!) {
    deletePool(id: $id)
  }
`

/** Dev-only: rebuild the scoreboard + bracket as-of the current dev clock. */
export const REMATERIALIZE_MUTATION = `
  mutation DevRematerialize {
    devRematerialize
  }
`

/**
 * Admin-gated: fetch SportsDB reported scores for result-pending games in a
 * group. Returns [] for non-admins and when SportsDB is unconfigured.
 */
export const REPORTED_RESULTS_QUERY = `
  query ReportedResults($groupId: String!) {
    reportedResults(groupId: $groupId) {
      gameId
      homeScore
      awayScore
      source
      sourceStatus
      ninetyMinuteUncertain
    }
  }
`

export const MATCH_QUERY = `
  query Match($gameId: ID!, $pool: ID) {
    match(gameId: $gameId, pool: $pool) {
      game {
        id kickoff venue groupId
        home { teamId description }
        away { teamId description }
        resultPending withinTodayWindow isToday
      }
      actual {
        homeScore awayScore provisional source sourceStatus ninetyMinuteUncertain
      }
      rows {
        playerId nick gameId
        prediction { gameId homeScore awayScore locked }
        points isPerfect maxReachable
        breakdown { exactHome exactAway outcome base multiplier points }
      }
    }
  }
`

/** The best third-placed-teams ranking (FWC26_RULES §3). `player: null` →
 *  official; a player id → that player's predicted ranking. */
export const THIRD_PLACE_QUERY = `
  query ThirdPlaceRanking($player: ID) {
    thirdPlaceRanking(player: $player) {
      complete
      entries {
        group
        team { id name shortCode flag externalId }
        points
        goalDiff
        goalsFor
        rank
        qualifies
        facesWinnerGroup
        # facesGame: reserved for a future match-detail link; not yet rendered
        facesGame
      }
    }
  }
`

