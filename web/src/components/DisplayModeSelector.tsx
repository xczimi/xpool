import { useI18n } from '../i18n/useI18n'
import { useDisplayMode } from '../display/useDisplayMode'
import { DISPLAY_MODES, type DisplayMode } from '../lib/displayMode'
import type { StringKey } from '../i18n/strings'

const LABEL_KEY: Record<DisplayMode, StringKey> = {
  auto: 'displayAuto',
  flag: 'displayFlag',
  code: 'displayCode',
  name: 'displayName',
  'flag-name': 'displayFlagName',
  'flag-code': 'displayFlagCode',
}

export function DisplayModeSelector() {
  const { t } = useI18n()
  const { mode, setMode } = useDisplayMode()
  return (
    <label className="lang-selector">
      {t('display')}:{' '}
      <select
        value={mode}
        onChange={(e) => setMode(e.target.value as DisplayMode)}
      >
        {DISPLAY_MODES.map((m) => (
          <option key={m} value={m}>
            {t(LABEL_KEY[m])}
          </option>
        ))}
      </select>
    </label>
  )
}
