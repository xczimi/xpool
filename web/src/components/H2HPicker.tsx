import { useNavigate } from 'react-router-dom'
import type { ScoreEntry } from '../graphql/types'

/**
 * Anchored head-to-head entry point: pick an opponent to compare against a
 * fixed `anchorId` (the player whose page this is). Selecting an opponent
 * navigates to `/h2h/<anchor>/<opponent>` with the anchor first. The anchor is
 * excluded from the candidate list. `label` is supplied by the caller so the
 * same component reads "Compare me with…" on the viewer's own page and
 * "Compare <nick> with…" on another player's page.
 */
export function H2HPicker({
  anchorId,
  label,
  candidates,
}: {
  anchorId: string
  label: string
  candidates: ScoreEntry[]
}) {
  const navigate = useNavigate()
  const opponents = candidates.filter((e) => e.playerId !== anchorId)

  // Nobody to compare against (solo pool, or own page before any results) — the
  // label would invite an action that can't be taken, so render nothing.
  if (opponents.length === 0) return null

  return (
    <div className="h2h-picker">
      <label className="h2h-picker-label">
        {label}{' '}
        <select
          defaultValue=""
          onChange={(e) => {
            if (e.target.value) navigate(`/h2h/${anchorId}/${e.target.value}`)
          }}
        >
          <option value="">—</option>
          {opponents.map((e) => (
            <option key={e.playerId} value={e.playerId}>
              {e.nick}
            </option>
          ))}
        </select>
      </label>
    </div>
  )
}
