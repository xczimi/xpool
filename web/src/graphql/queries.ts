/** GraphQL query & mutation documents (API.md §4, §5). */

export const TOURNAMENT_QUERY = `
  query Tournament {
    tournament {
      root
      groups {
        id name parent round lockMode carriesStandings
        childGroupIds gameIds deadline
      }
      games {
        id kickoff venue groupId
        home { teamId description }
        away { teamId description }
        result { homeScore awayScore locked }
      }
      teams { id name shortCode flag externalId }
    }
    motd { text }
  }
`

export const ME_QUERY = `
  query Me {
    me {
      id nick fullName email isAdmin isResultUser
      matchPredictions { gameId homeScore awayScore locked }
      standingsPredictions { groupId ordering drawOrder locked }
    }
  }
`

export const SCOREBOARD_QUERY = `
  query Scoreboard($pool: ID) {
    scoreboard(pool: $pool) {
      poolId poolName
      multipliers { round multiplier }
      entries {
        playerId nick total
        byRound { round points }
      }
    }
  }
`

export const POOLS_QUERY = `
  query Pools {
    pools { id name ownerId memberIds }
  }
`

export const TIPS_QUERY = `
  query Tips($groupId: ID!) {
    tips(groupId: $groupId) {
      playerId nick gameId homeScore awayScore visible
    }
  }
`

export const PERFECTS_QUERY = `
  query Perfects {
    perfects { playerId nick gameId }
  }
`

export const SUBMIT_GROUP_MUTATION = `
  mutation SubmitGroup(
    $groupId: ID!
    $predictions: [PredictionInput!]!
    $standings: StandingsInput
    $lock: Boolean!
  ) {
    submitGroup(
      groupId: $groupId
      predictions: $predictions
      standings: $standings
      lock: $lock
    ) {
      id
      matchPredictions { gameId homeScore awayScore locked }
      standingsPredictions { groupId ordering drawOrder locked }
    }
  }
`

export const UPDATE_PROFILE_MUTATION = `
  mutation UpdateProfile($input: ProfileInput!) {
    updateProfile(input: $input) {
      id nick fullName email
    }
  }
`

export const INVITE_MUTATION = `
  mutation Invite($input: InviteInput!) {
    invite(input: $input) {
      id nick fullName email
    }
  }
`

export const CREATE_POOL_MUTATION = `
  mutation CreatePool($input: PoolInput!) {
    createPool(input: $input) { id name ownerId memberIds }
  }
`

export const UPDATE_POOL_MUTATION = `
  mutation UpdatePool($id: ID!, $input: PoolInput!) {
    updatePool(id: $id, input: $input) { id name ownerId memberIds }
  }
`

export const ENTER_RESULT_MUTATION = `
  mutation EnterResult($gameId: ID!, $homeScore: Int!, $awayScore: Int!, $lock: Boolean!) {
    enterResult(gameId: $gameId, homeScore: $homeScore, awayScore: $awayScore, lock: $lock) {
      id result { homeScore awayScore locked }
    }
  }
`

export const SET_MOTD_MUTATION = `
  mutation SetMotd($text: String!) {
    setMotd(text: $text) { text }
  }
`
