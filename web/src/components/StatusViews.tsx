import { useI18n } from '../i18n/useI18n'

export function Loading() {
  const { t } = useI18n()
  return <p className="status">{t('loading')}</p>
}

export function ErrorView({
  message,
  onRetry,
}: {
  message?: string
  onRetry?: () => void
}) {
  const { t } = useI18n()
  return (
    <div className="status status-error">
      <p>
        {t('errorPrefix')}
        {message ? `: ${message}` : '.'}
      </p>
      {onRetry && (
        <button type="button" onClick={onRetry}>
          {t('retry')}
        </button>
      )}
    </div>
  )
}

export function NeedsLogin() {
  const { t } = useI18n()
  return (
    <div className="status">
      <h2>{t('notLoggedInTitle')}</h2>
      <p>{t('notLoggedInBody')}</p>
    </div>
  )
}

export function NeedsAdmin() {
  const { t } = useI18n()
  return (
    <div className="status">
      <h2>{t('notLoggedInTitle')}</h2>
      <p>{t('notAdminBody')}</p>
    </div>
  )
}
