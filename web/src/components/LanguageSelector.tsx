import { useI18n } from '../i18n/useI18n'
import { localeNames, type Locale } from '../i18n/strings'
import { SegToggle } from './SegToggle'

/**
 * Language picker as a labelled segmented toggle (`EN | HU`). Segments show the
 * uppercase locale code; the full language name is the accessible label.
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
      renderOption={(l) => l.toUpperCase()}
      optionAriaLabel={(l) => localeNames[l]}
    />
  )
}
