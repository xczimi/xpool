import type { WhatIfOutcome } from '../lib/whatIf'

/**
 * One what-if outcome: the new absolute total, with the delta-vs-current
 * emphasised (the delta is what people watch mid-match). A minus sign uses the
 * unicode minus (−) for typographic parity with the rest of the UI.
 */
export function WhatIfCell({ outcome }: { outcome: WhatIfOutcome }) {
  const direction =
    outcome.delta > 0 ? 'up' : outcome.delta < 0 ? 'down' : 'flat'
  const sign = outcome.delta > 0 ? '+' : outcome.delta < 0 ? '−' : '±'
  return (
    <span className="what-if">
      <span className="what-if-total">{outcome.total}</span>
      <span className={`what-if-delta ${direction}`}>
        {sign}
        {Math.abs(outcome.delta)}
      </span>
    </span>
  )
}
