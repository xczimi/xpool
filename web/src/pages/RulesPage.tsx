import { useI18n } from '../i18n/useI18n'
import { roundLabel, ROUND_ORDER, STAGE_MULTIPLIERS } from '../lib/rounds'

/**
 * Player-facing rules of play (REWRITE_USE_CASES §5, SCORING.md). Static copy.
 */
export function RulesPage() {
  const { t } = useI18n()
  return (
    <section className="page">
      <h2>{t('rulesTitle')}</h2>

      <h3>Per match</h3>
      <ul>
        <li>+1 for the exact home score.</li>
        <li>+1 for the exact away score.</li>
        <li>+2 for the correct outcome (win / draw / loss).</li>
        <li>Maximum 4 points per match.</li>
        <li>
          4-goal rule: a side scoring 4 or more is matched by any prediction of
          4 or more for that side.
        </li>
        <li>Scores are judged at full time (90 minutes) — extra time and
          penalties do not count toward the per-match score.</li>
      </ul>

      <h3>Per group</h3>
      <ul>
        <li>
          +1 for every pair of teams ordered correctly in your predicted
          standings.
        </li>
        <li>
          Predicted standings are derived from your own predicted scores, then
          ranked by points (3/1/0), head-to-head, goal difference, goals
          scored, and finally your manual tie order.
        </li>
      </ul>

      <h3>Stage multipliers</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Round</th>
            <th>{t('multiplier')}</th>
          </tr>
        </thead>
        <tbody>
          {ROUND_ORDER.map((r) => (
            <tr key={r}>
              <td>{roundLabel(r, t)}</td>
              <td>×{STAGE_MULTIPLIERS[r]}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>Fair play</h3>
      <ul>
        <li>Predictions only count once locked; unlocked predictions score 0
          and stay hidden from others.</li>
        <li>A "perfect" is a maximum-point (4) match prediction.</li>
        <li>Lock before the group's first match (group stage) or before the
          match (knockout).</li>
        <li>You cannot see other players' predictions until they lock them — by
          design, to prevent copying.</li>
      </ul>
    </section>
  )
}
