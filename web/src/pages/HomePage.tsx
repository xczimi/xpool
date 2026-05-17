import { Link } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'

export function HomePage() {
  const { t } = useI18n()
  return (
    <section className="page">
      <h2>{t('homeWelcome')}</h2>
      <p>{t('homeIntro')}</p>
      <div className="home-links">
        <Link to="/today">{t('navToday')}</Link>
        <Link to="/scoreboard">{t('navScoreboard')}</Link>
        <Link to="/games">{t('navGames')}</Link>
        <Link to="/rules">{t('navRules')}</Link>
      </div>
    </section>
  )
}
