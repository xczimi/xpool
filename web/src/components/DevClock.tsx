import { useMemo, useState, type ChangeEvent } from 'react'
import { useMutation, useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import type { StringKey } from '../i18n/strings'
import { DEV_CLOCK_GAMES_QUERY, REMATERIALIZE_MUTATION } from '../graphql/queries'
import { getDevNow, setDevNow, clearDevNow } from '../auth/devAuth'
import { formatKickoff } from '../lib/format'
import { devClockInstant, type DevClockPhase } from './devClockTimes'

/**
 * Dev-only clock control. Two `<select>`s — a game and a phase relative to its
 * kickoff — set the server clock (`X-Dev-Now`) to a time-dependent state
 * (predictions open / match in progress / match over) without hand-typing a
 * timestamp. The instant still flows through `setDevNow(iso)` → `X-Dev-Now`
 * header → server `now`; only the picking changes.
 *
 * The selects are write-only controls, not state mirrors: changing either,
 * once both are chosen, applies the instant and reloads. After reload they
 * return to their placeholders and the active dev time is shown as text only.
 */

interface SlimSlot {
  teamId: string | null
  description: string
}
interface SlimGame {
  id: string
  kickoff: string
  home: SlimSlot
  away: SlimSlot
}
interface SlimTeam {
  id: string
  shortCode: string
}
interface DevClockData {
  tournament: { games: SlimGame[]; teams: SlimTeam[] }
}

const PHASES: DevClockPhase[] = ['before', 'during', 'after']
const PHASE_LABEL: Record<DevClockPhase, StringKey> = {
  before: 'devClockBefore',
  during: 'devClockDuring',
  after: 'devClockAfter',
}

/** Short kickoff in the browser's local time, e.g. `Jun 11 18:00`. */
function shortKickoff(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString(locale, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  })
}

/** Resolved short code for a slot — team `shortCode`, else the slot description. */
function slotCode(slot: SlimSlot, codes: Map<string, string>): string {
  if (slot.teamId) return codes.get(slot.teamId) ?? slot.teamId
  return slot.description || 'TBD'
}

export function DevClock() {
  const { t, locale } = useI18n()
  const current = getDevNow()

  const [gameId, setGameId] = useState('')
  const [phase, setPhase] = useState<'' | DevClockPhase>('')

  const [{ data }] = useQuery<DevClockData>({ query: DEV_CLOCK_GAMES_QUERY })
  const [, rematerialize] = useMutation(REMATERIALIZE_MUTATION)

  const games = useMemo(
    () =>
      [...(data?.tournament.games ?? [])].sort(
        (a, b) => Date.parse(a.kickoff) - Date.parse(b.kickoff),
      ),
    [data],
  )
  const codes = useMemo(
    () => new Map((data?.tournament.teams ?? []).map((tm) => [tm.id, tm.shortCode])),
    [data],
  )

  // Apply only when BOTH a game and a phase are chosen, then re-materialise the
  // board as-of the new clock and reload. The mutation carries the freshly-set
  // X-Dev-Now header (client.ts reads it per request), so the server rebuilds
  // for the just-picked instant. Failures are swallowed — a prod build without
  // the dev mutation still reloads cleanly.
  const apply = async (g: string, p: '' | DevClockPhase) => {
    if (!g || !p) return
    const game = games.find((x) => x.id === g)
    if (!game) return
    setDevNow(devClockInstant(game.kickoff, p))
    try {
      await rematerialize({})
    } catch {
      /* ignore — reload regardless */
    }
    location.reload()
  }

  const onGame = (e: ChangeEvent<HTMLSelectElement>) => {
    setGameId(e.target.value)
    void apply(e.target.value, phase)
  }
  const onPhase = (e: ChangeEvent<HTMLSelectElement>) => {
    const p = e.target.value as '' | DevClockPhase
    setPhase(p)
    void apply(gameId, p)
  }

  return (
    <span className="dev-clock">
      <span className="dev-clock-label">{t('devClock')}</span>
      <label>
        {t('devClockGame')}
        <select value={gameId} onChange={onGame}>
          <option value="" disabled>
            {t('devClockGamePlaceholder')}
          </option>
          {games.map((g) => (
            <option key={g.id} value={g.id}>
              {shortKickoff(g.kickoff, locale)} · {g.id} · {slotCode(g.home, codes)} v{' '}
              {slotCode(g.away, codes)}
            </option>
          ))}
        </select>
      </label>
      <label>
        {t('devClockWhen')}
        <select value={phase} onChange={onPhase} disabled={!gameId}>
          <option value="" disabled>
            {t('devClockWhenPlaceholder')}
          </option>
          {PHASES.map((p) => (
            <option key={p} value={p}>
              {t(PHASE_LABEL[p])}
            </option>
          ))}
        </select>
      </label>
      {current && (
        <span className="dev-clock-now">
          {formatKickoff(current, locale)}
          <button
            type="button"
            onClick={async () => {
              clearDevNow()
              try {
                await rematerialize({})
              } catch {
                /* ignore */
              }
              location.reload()
            }}
          >
            {t('devClockReset')}
          </button>
        </span>
      )}
    </span>
  )
}
