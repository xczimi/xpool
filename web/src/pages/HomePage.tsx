import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'
import { ME_QUERY } from '../graphql/queries'
import type { Me } from '../graphql/types'
import { InviteCodeEntry } from '../components/InviteCodeEntry'

/**
 * Identity-aware welcome (design:
 * docs/superpowers/specs/2026-06-09-home-identity-aware-welcome-design.md).
 * A non-player (logged-out or authenticated-unclaimed) gets the invite-code
 * entry — the front door to a pool. A Player gets quick-action links. While a
 * session's `me` is still resolving we show only the neutral welcome, to avoid
 * a flash of the wrong branch. `me` is paused without a session (mirrors
 * `Layout`/`PoolsPage`), so a logged-out viewer fires no auth query.
 */
export function HomePage() {
  const { t } = useI18n()
  const { label } = useAuth()
  const [meResult] = useQuery<{ me: Me }>({ query: ME_QUERY, pause: !label })

  const isPlayer = meResult.data?.me?.__typename === 'Player'
  const loading = Boolean(label) && meResult.fetching && !meResult.data

  return (
    <section className="page">
      <h2>{t('homeWelcome')}</h2>
      <p>{t('homeIntro')}</p>

      <div className="home-howto">
        <h3>{t('homeHowTitle')}</h3>
        <ol>
          <li>{t('homeHowStep1')}</li>
          <li>{t('homeHowStep2')}</li>
          <li>{t('homeHowStep3')}</li>
        </ol>
        <Link to="/rules">{t('homeRulesLink')}</Link>
      </div>

      <div className="home-faq">
        <h3>{t('homeFaqTitle')}</h3>
        <dl>
          <dt>{t('homeFaqQ1')}</dt>
          <dd>{t('homeFaqA1')}</dd>
          <dt>{t('homeFaqQ2')}</dt>
          <dd>{t('homeFaqA2')}</dd>
        </dl>
      </div>

      {!loading &&
        (isPlayer ? (
          <div className="home-links">
            <Link to="/mytips">{t('navMyTips')}</Link>
            <Link to="/today">{t('navToday')}</Link>
            <Link to="/scoreboard">{t('navScoreboard')}</Link>
            <Link to="/pools">{t('navPools')}</Link>
          </div>
        ) : (
          <InviteCodeEntry />
        ))}
    </section>
  )
}
