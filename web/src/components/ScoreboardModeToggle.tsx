import { NavLink } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'

/**
 * Overall ⇄ Knockout-only switch for the scoreboard. Each option is a route, so
 * the toggle state IS the URL — `/scoreboard/knockout` is directly linkable and
 * shareable. The knockout board re-sums points from knockout matches only
 * (a re-engagement view — `.scratch/knockout-only-scoreboard/PRD.md`).
 */
export function ScoreboardModeToggle() {
  const { t } = useI18n()
  const cls = ({ isActive }: { isActive: boolean }) =>
    isActive ? 'active' : undefined
  return (
    <nav className="scoreboard-toggle" aria-label={t('scoreboardTitle')}>
      <NavLink to="/scoreboard" end className={cls}>
        {t('overall')}
      </NavLink>
      <NavLink to="/scoreboard/knockout" className={cls}>
        {t('knockoutOnly')}
      </NavLink>
    </nav>
  )
}
