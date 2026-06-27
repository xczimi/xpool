import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import type { ScoreEntry } from '../graphql/types'

/** "Pick two" head-to-head entry rendered above the scoreboard. */
export function H2HPicker({ entries }: { entries: ScoreEntry[] }) {
  const { t } = useI18n()
  const navigate = useNavigate()
  const [a, setA] = useState('')
  const [b, setB] = useState('')
  const ready = a !== '' && b !== '' && a !== b

  const options = entries.map((e) => (
    <option key={e.playerId} value={e.playerId}>
      {e.nick}
    </option>
  ))

  return (
    <div className="h2h-picker">
      <span className="h2h-picker-label">{t('h2hPickTwo')}</span>
      <select
        value={a}
        aria-label={t('h2hPickPrompt')}
        onChange={(e) => setA(e.target.value)}
      >
        <option value="">—</option>
        {options}
      </select>
      <span className="h2h-picker-vs">×</span>
      <select
        value={b}
        aria-label={t('h2hPickPrompt')}
        onChange={(e) => setB(e.target.value)}
      >
        <option value="">—</option>
        {options}
      </select>
      <button
        type="button"
        className="h2h-picker-go"
        disabled={!ready}
        onClick={() => navigate(`/h2h/${a}/${b}`)}
      >
        {t('h2hCompare')}
      </button>
    </div>
  )
}
