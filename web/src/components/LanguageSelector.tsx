import { useI18n } from '../i18n/useI18n'
import { localeNames, type Locale } from '../i18n/strings'

/**
 * Language picker as a segmented toggle (`EN | HU`), styled like the theme
 * picker. Segments show the uppercase locale code; the full language name is
 * the accessible label / tooltip.
 */
export function LanguageSelector() {
  const { locale, setLocale, t } = useI18n()
  const locales = Object.keys(localeNames) as Locale[]
  return (
    <div className="seg-toggle" role="radiogroup" aria-label={t('language')}>
      {locales.map((l) => (
        <button
          key={l}
          type="button"
          role="radio"
          aria-checked={l === locale}
          aria-label={localeNames[l]}
          title={localeNames[l]}
          className={`seg-option${l === locale ? ' is-active' : ''}`}
          onClick={() => setLocale(l)}
        >
          {l.toUpperCase()}
        </button>
      ))}
    </div>
  )
}
