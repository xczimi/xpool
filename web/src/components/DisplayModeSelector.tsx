import { useI18n } from '../i18n/useI18n'
import { useDisplayMode } from '../display/useDisplayMode'
import {
  FLAG_MODES,
  TEXT_MODES,
  type FlagMode,
  type TextMode,
} from '../lib/displayMode'
import type { StringKey } from '../i18n/strings'
import { SegToggle } from './SegToggle'

const FLAG_LABEL: Record<FlagMode, StringKey> = {
  on: 'flagOn',
  off: 'flagOff',
}

const TEXT_LABEL: Record<TextMode, StringKey> = {
  auto: 'textAuto',
  name: 'textName',
  code: 'textCode',
  off: 'textOff',
}

/**
 * The "show" picker, split into two labelled segmented toggles: whether to show
 * a flag, and what text accompanies it. Flag off + Text off would render an
 * empty label, so the Text `off` segment is disabled while Flag is off.
 */
export function DisplayModeSelector() {
  const { t } = useI18n()
  const { flag, text, setFlag, setText } = useDisplayMode()

  return (
    <>
      <SegToggle
        label={t('displayFlag')}
        options={FLAG_MODES}
        value={flag}
        onChange={setFlag}
        renderOption={(f) => t(FLAG_LABEL[f])}
      />
      <SegToggle
        label={t('displayText')}
        options={TEXT_MODES}
        value={text}
        onChange={setText}
        renderOption={(m) => t(TEXT_LABEL[m])}
        isDisabled={(m) => m === 'off' && flag === 'off'}
      />
    </>
  )
}
