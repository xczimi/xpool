/** Inputs to `submitGroup` — shared by the desktop form and the mobile flow. */
export interface PredictionInput {
  gameId: string
  homeScore: number
  awayScore: number
}

export interface StandingsInput {
  ordering: string[]
  drawOrder: string[]
}
