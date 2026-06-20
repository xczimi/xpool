import { useMemo, useState } from 'react'
import { useI18n } from '../../i18n/useI18n'
import type {
  MatchPrediction,
  ScoreEntry,
  Tournament,
} from '../../graphql/types'
import type { Locale } from '../../i18n/strings'
import { roundPointsOf } from '../../lib/playerPage'
import { roundLabel, visibleRoundNodes } from '../../lib/rounds'
import { PlayerRoundDetail } from './PlayerRoundDetail'

/**
 * The per-round drill-down: one collapsed row per ready round showing its score,
 * expanding to a lazily-fetched `PlayerRoundDetail`. All rows start collapsed
 * (the page opens compact); each detail only mounts — and therefore only
 * fetches — when its row is opened.
 *
 * `isOwn` is part of the props contract for forward use: own pages render
 * identically today because the `tips` resolver already returns everything for
 * your own predictions. It is intentionally not consumed in the body yet.
 */
export function PlayerRounds({
  playerId,
  entry,
  tournament,
  resultByGame,
  locale,
}: {
  playerId: string
  isOwn: boolean
  entry: ScoreEntry
  tournament: Tournament
  resultByGame: Map<string, MatchPrediction>
  locale: Locale
}) {
  const { t } = useI18n()
  const rounds = useMemo(
    () => visibleRoundNodes(tournament.groups, tournament.games),
    [tournament.groups, tournament.games],
  )
  const byRound = roundPointsOf(entry)
  const [openId, setOpenId] = useState<string | null>(null)

  return (
    <ul className="player-rounds">
      {rounds.map((node) => {
        const isOpen = openId === node.id
        return (
          <li key={node.id} className="player-round">
            <button
              type="button"
              className="player-round-row"
              aria-expanded={isOpen}
              onClick={() => setOpenId(isOpen ? null : node.id)}
            >
              <span className="player-round-label">
                {roundLabel(node.round, t)}
              </span>
              <span className="player-round-points">
                {byRound.get(node.round) ?? 0}
              </span>
            </button>
            {isOpen && (
              <PlayerRoundDetail
                playerId={playerId}
                roundNode={node}
                tournament={tournament}
                resultByGame={resultByGame}
                locale={locale}
              />
            )}
          </li>
        )
      })}
    </ul>
  )
}
