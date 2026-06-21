import { useI18n } from '../i18n/useI18n'
import type { Pool } from '../graphql/types'
import { effectiveSelectedPool } from '../lib/selectedPool'
import { useSelectedPool } from './useSelectedPool'

/**
 * The shared pool picker (`<label className="pool-selector">`) used by the
 * Scoreboard, All Tips and Perfect pages. Empty option = "everyone" (global);
 * a pool option scopes the listing. Selection is sticky across pages via
 * `useSelectedPool`. `pools` is the viewer's pool list (empty for a visitor).
 */
export function PoolSelector({ pools }: { pools: Pool[] }) {
  const { t } = useI18n()
  const { selected, setSelected } = useSelectedPool()
  const effective = effectiveSelectedPool(
    selected,
    pools.map((p) => p.id),
  )

  return (
    <label className="pool-selector">
      {t('pool')}:{' '}
      <select
        value={effective ?? ''}
        onChange={(e) => setSelected(e.target.value || null)}
      >
        <option value="">{t('everyone')}</option>
        {pools.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
    </label>
  )
}
