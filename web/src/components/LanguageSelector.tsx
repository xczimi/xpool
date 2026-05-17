import { useI18n } from '../i18n/useI18n'
import { localeNames, type Locale } from '../i18n/strings'

export function LanguageSelector() {
  const { locale, setLocale, t } = useI18n()
  return (
    <label className="lang-selector">
      {t('language')}:{' '}
      <select
        value={locale}
        onChange={(e) => setLocale(e.target.value as Locale)}
      >
        {(Object.keys(localeNames) as Locale[]).map((l) => (
          <option key={l} value={l}>
            {localeNames[l]}
          </option>
        ))}
      </select>
    </label>
  )
}
