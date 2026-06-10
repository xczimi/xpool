import { useI18n } from '../i18n/useI18n'
import { roundLabel, ROUND_ORDER, STAGE_MULTIPLIERS } from '../lib/rounds'

/**
 * Player-facing rules of play (REWRITE_USE_CASES §5, SCORING.md). All copy is
 * i18n'd (EN + HU) in `strings.ts` — see `.scratch/rules-content/PRD.md`. The
 * round labels and stage multipliers are sourced from `lib/rounds` so the table
 * can't drift from the scoring engine's `STAGE_MULTIPLIERS`.
 */
export function RulesPage() {
  const { t } = useI18n()
  return (
    <section className="page">
      <h2>{t('rulesTitle')}</h2>
      <p>{t('rulesIntro')}</p>

      <h3>{t('rulesPerMatchTitle')}</h3>
      <ul>
        <li>{t('rulesPerMatchExactHome')}</li>
        <li>{t('rulesPerMatchExactAway')}</li>
        <li>{t('rulesPerMatchOutcome')}</li>
        <li>{t('rulesPerMatchMax')}</li>
        <li>{t('rulesPerMatchFourGoal')}</li>
        <li>{t('rulesPerMatchFullTime')}</li>
      </ul>

      <h3>{t('rulesPerGroupTitle')}</h3>
      <ul>
        <li>{t('rulesPerGroupPairs')}</li>
        <li>{t('rulesPerGroupStandings')}</li>
      </ul>

      <h3>{t('rulesMultipliersTitle')}</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>{t('rulesRoundColumn')}</th>
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

      <h3>{t('rulesFairPlayTitle')}</h3>
      <ul>
        <li>{t('rulesFairPlayLock')}</li>
        <li>{t('rulesFairPlayPerfect')}</li>
        <li>{t('rulesFairPlayDeadline')}</li>
        <li>{t('rulesFairPlayHidden')}</li>
      </ul>
    </section>
  )
}
