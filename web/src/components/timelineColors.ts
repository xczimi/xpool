/**
 * Legible series colours on the dark scoreboard background. The first two
 * (amber, cyan) are the head-to-head pair; the rest extend the palette for the
 * scoreboard's all-pool overlay. `seriesColor` cycles for N players.
 */
export const TIMELINE_COLORS = [
  '#ffd76a', // amber
  '#21d4fd', // cyan
  '#9b87f5', // violet
  '#4ade80', // green
  '#f472b6', // pink
  '#fb923c', // orange
  '#38bdf8', // sky
  '#facc15', // yellow
] as const

/** The stroke colour for the i-th overlaid series, cycling the palette. */
export function seriesColor(i: number): string {
  return TIMELINE_COLORS[i % TIMELINE_COLORS.length]
}
