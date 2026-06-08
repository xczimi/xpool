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
        id nick fullName isResultUser version
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

export const POOLS_QUERY = `
  query Pools {
    pools { id name owner members joinCode }
  }
`

export const TIPS_QUERY = `
  query Tips($groupId: ID!) {
    tips(groupId: $groupId) {
      playerId nick gameId
      prediction { gameId homeScore awayScore locked }
      points isPerfect
    }
  }
`

export const PERFECTS_QUERY = `
  query Perfects {
    perfects { playerId nick gameId points }
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

export const INVITE_MUTATION = `
  mutation Invite($inviteeId: ID!) {
    invite(inviteeId: $inviteeId)
  }
`

const POOL_FIELDS = 'id name owner members joinCode'

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

export const JOIN_POOL_MUTATION = `
  mutation JoinPool($joinCode: String!) {
    joinPool(joinCode: $joinCode) { ${POOL_FIELDS} }
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

export const ROTATE_JOIN_CODE_MUTATION = `
  mutation RotateJoinCode($id: ID!) {
    rotateJoinCode(id: $id) { ${POOL_FIELDS} }
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

