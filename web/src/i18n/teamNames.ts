import type { Team } from '../graphql/types'
import type { Locale } from './strings'

/**
 * Localised team display names, keyed by `Team.shortCode` (the stable,
 * language-neutral identity). A locale with no entry for a team — or an omitted
 * locale such as `en` — falls back to the English name from the tournament JSON,
 * so the roster can change without touching this file.
 *
 * `en` is intentionally omitted: English names already come from `fwc26.json`.
 * The `hu` set covers the current FWC26 roster; correct wording here before
 * go-live.
 */
export const teamNames: Partial<Record<Locale, Record<string, string>>> = {
  hu: {
    ALG: 'Algéria',
    ARG: 'Argentína',
    AUS: 'Ausztrália',
    AUT: 'Ausztria',
    BEL: 'Belgium',
    BIH: 'Bosznia-Hercegovina',
    BRA: 'Brazília',
    CAN: 'Kanada',
    CIV: 'Elefántcsontpart',
    COD: 'Kongói DK',
    COL: 'Kolumbia',
    CPV: 'Zöld-foki Köztársaság',
    CRO: 'Horvátország',
    CUW: 'Curaçao',
    CZE: 'Csehország',
    ECU: 'Ecuador',
    EGY: 'Egyiptom',
    ENG: 'Anglia',
    ESP: 'Spanyolország',
    FRA: 'Franciaország',
    GER: 'Németország',
    GHA: 'Ghána',
    HAI: 'Haiti',
    IRN: 'Irán',
    IRQ: 'Irak',
    JOR: 'Jordánia',
    JPN: 'Japán',
    KOR: 'Dél-Korea',
    KSA: 'Szaúd-Arábia',
    MAR: 'Marokkó',
    MEX: 'Mexikó',
    NED: 'Hollandia',
    NOR: 'Norvégia',
    NZL: 'Új-Zéland',
    PAN: 'Panama',
    PAR: 'Paraguay',
    POR: 'Portugália',
    QAT: 'Katar',
    RSA: 'Dél-Afrika',
    SCO: 'Skócia',
    SEN: 'Szenegál',
    SUI: 'Svájc',
    SWE: 'Svédország',
    TUN: 'Tunézia',
    TUR: 'Törökország',
    URU: 'Uruguay',
    USA: 'USA',
    UZB: 'Üzbegisztán',
  },
}

/**
 * Resolve a team's display name for a locale: the localised name if present,
 * otherwise the English `team.name`. Never blank.
 */
export function teamDisplayName(team: Team, locale: Locale): string {
  return teamNames[locale]?.[team.shortCode] ?? team.name
}
