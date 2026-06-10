import type { StringKey } from '../i18n/strings'

/**
 * Ready-to-send invite messages surfaced on the Pools page so a member can copy
 * one straight from the browser. The canonical wording lives here (the repo's
 * `docs/share-copy.md` keeps the full set — incl. the objection/scoring blurbs
 * that became Home content — for reference).
 *
 * These are *content*, not UI chrome: each body is fixed in its own language
 * (three English, one Hungarian) and shown regardless of the en/hu toggle —
 * the inviter picks by their recipient, not by their own UI language. Only the
 * surrounding label (`labelKey`) is translated. Markdown emphasis is stripped so
 * the text pastes clean into WhatsApp/email. `{LINK}` is a literal placeholder
 * the inviter swaps for their pool invite link.
 */
export type ShareTemplate = {
  id: string
  labelKey: StringKey
  body: string
}

const SHORT = `⚽ The World Cup kicks off this week and I'm running a score-prediction pool. Predict the score of every match, earn points for getting it right, climb the leaderboard. No money — just bragging rights and a reason to watch every game. Takes 5 min to fill in the group stage. Get in before the first kickoff 👉 {LINK}`

const ONE_LINER = `Running a World Cup tipping pool — predict scores, earn points, win bragging rights. Free and for fun. Join before kickoff: {LINK}`

const EMAIL = `Subject: World Cup prediction pool — get your picks in before Thursday

Hi all — the 2026 World Cup starts this week and I've set up a friendly score-prediction pool.

How it works: predict the exact score of each match. You get +1 for the right home score, +1 for the right away score, and +2 for the correct result — up to 4 points a game, and the knockout rounds are worth more. There's also a bonus for predicting the group standings order. Your picks lock at kickoff, and nobody can see them until then.

No stakes, no money — just office bragging rights and something to follow every match for. It takes a few minutes to enter the group stage.

Join here 👉 {LINK} — first match is Wed June 11, so get your picks in before then.`

const HUNGARIAN = `⚽ A héten kezdődik a vébé, csináltam rá egy tippjátékot. Tippeld meg minden meccs eredményét, pontot kapsz a találatokért, és gyűjtöd a dicsőséget. Ha kicsi a tét, kedvem sötét — de most a tét csak a dicsőség, pénz nincs a játékban, viszont lesz okod minden meccset végignézni. Pár perc a csoportkört kitölteni. Csatlakozz az első sípszó előtt 👉 {LINK}`

export const SHARE_TEMPLATES: ShareTemplate[] = [
  { id: 'short', labelKey: 'shareTemplateShort', body: SHORT },
  { id: 'oneLiner', labelKey: 'shareTemplateOneLiner', body: ONE_LINER },
  { id: 'email', labelKey: 'shareTemplateEmail', body: EMAIL },
  { id: 'hungarian', labelKey: 'shareTemplateHungarian', body: HUNGARIAN },
]
