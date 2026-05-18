import { useMemo, useState, type ReactNode } from 'react'
import { catalogues, type Locale, type StringKey } from './strings'
import { I18nContext, type I18nState } from './i18nContextValue'

const STORAGE_KEY = 'xpool.locale'

function initialLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'en' || stored === 'hu') {
      return stored
    }
  } catch {
    /* ignore */
  }
  return 'en'
}

/** i18n provider — English + Hungarian (REWRITE_USE_CASES §4). */
export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(initialLocale())

  const value = useMemo<I18nState>(
    () => ({
      locale,
      setLocale: (next: Locale) => {
        try {
          localStorage.setItem(STORAGE_KEY, next)
        } catch {
          /* ignore */
        }
        setLocaleState(next)
      },
      t: (key: StringKey) => catalogues[locale][key] ?? key,
    }),
    [locale],
  )

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}
