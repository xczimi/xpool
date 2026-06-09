import { useI18n } from '../i18n/useI18n'
import { localeFlags, localeNames, type Locale } from '../i18n/strings'
import { SegToggle } from './SegToggle'
import { Flag } from './TeamLabel'

/**
 * Language picker as a labelled segmented toggle. Each segment shows the
 * country flag (English → Canada, Hungarian → Hungary); the full language name
 * is the accessible label / tooltip.
 */
export function LanguageSelector() {
  const { locale, setLocale, t } = useI18n()
  const locales = Object.keys(localeNames) as Locale[]
  return (
    <SegToggle
      label={t('language')}
      options={locales}
      value={locale}
      onChange={setLocale}
      renderOption={(l) => (
        <Flag iso={localeFlags[l]} name={localeNames[l]} className="lang-flag" />
      )}
      optionAriaLabel={(l) => localeNames[l]}
    />
  )
}
