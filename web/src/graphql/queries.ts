/** GraphQL query & mutation documents — the agreed reconciled schema. */

export const TOURNAMENT_QUERY = `
  query Tournament {
    tournament {
      root
      groups {
        id name parent round lockMode carriesStandings
        childGroupIds childGameIds deadline
      }
      games {
        id kickoff venue groupId
        home { teamId description }
        away { teamId description }
      }
      teams { id name shortCode flag externalId }
    }
    motd
  }
`

export const ME_QUERY = `
  query Me {
    me {
      id nick fullName isResultUser version
      matchPredictions { gameId homeScore awayScore locked }
      standingsPredictions { groupId ordering drawOrder locked }
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
    pools { id name owner members }
  }
`

export const TIPS_QUERY = `
  query Tips($groupId: ID!) {
    tips(groupId: $groupId) {
      playerId nick gameId
      prediction { gameId homeScore awayScore locked }
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

export const CREATE_POOL_MUTATION = `
  mutation CreatePool($id: ID!, $name: String!) {
    createPool(id: $id, name: $name) { id name owner members }
  }
`

export const UPDATE_POOL_MUTATION = `
  mutation UpdatePool($id: ID!, $name: String, $members: [ID!]) {
    updatePool(id: $id, name: $name, members: $members) { id name owner members }
  }
`

export const ENTER_RESULT_MUTATION = `
  mutation EnterResult(
    $gameId: ID!
    $homeScore: Int!
    $awayScore: Int!
    $advancer: ID
    $lock: Boolean!
  ) {
    enterResult(
      gameId: $gameId
      homeScore: $homeScore
      awayScore: $awayScore
      advancer: $advancer
      lock: $lock
    )
  }
`

export const SET_MOTD_MUTATION = `
  mutation SetMotd($text: String!) {
    setMotd(text: $text)
  }
`
