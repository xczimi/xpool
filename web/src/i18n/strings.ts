/**
 * UI string catalogue. English + Hungarian (REWRITE_USE_CASES §4 i18n).
 *
 * Stage 2 of the i18n reconciliation (`.specs/LEGACY_I18N.md`): where a string
 * maps to one in the legacy gettext catalog, the **original Hungarian wording
 * is reused verbatim** — keeping the casual register (`Tippverseny`, `Adatok`,
 * `tutiban`, `engedj be!`, `Szevasztok!`). The two signature lowercase phrases
 * (`get me outside!` / `let me in!`) are kept verbatim in English too. English
 * is otherwise conventional Title Case for a consistent modern UI. The stale
 * Euro-2008 rules copy is deliberately NOT carried over.
 *
 * To add a language: add a `Locale` value and a matching key block.
 */

export type Locale = 'en' | 'hu'

export type StringKey = keyof typeof en

const en = {
  // chrome
  tagline: 'This is our little friendly pool.',
  language: 'Language',
  loggedInAs: 'Logged in as',
  logOut: 'get me outside!',
  logIn: 'let me in!',
  visitor: 'You are outside.',
  footer: 'xPool — a friendly soccer prediction pool',

  // nav
  navHome: 'Home',
  navToday: 'Today',
  navGames: 'Schedule',
  navMyTips: 'My Tips',
  navAllTips: 'All Tips',
  navScoreboard: 'Scoreboard',
  navPerfect: 'Perfect',
  navPools: 'Pools',
  navProfile: 'Profile',
  navInvite: 'Invite',
  navRules: 'Rules',
  navAdmin: 'Admin',

  // generic
  loading: 'Loading…',
  errorPrefix: 'Something went wrong',
  retry: 'Retry',
  save: 'Save',
  cancel: 'Cancel',
  group: 'Group',
  team: 'Team',
  teams: 'Teams',
  match: 'Match',
  kickoff: 'Kickoff',
  venue: 'Venue',
  result: 'Result',
  prediction: 'Prediction',
  points: 'Points',
  rank: 'Rank',
  player: 'Player',
  total: 'Total',
  notLoggedInTitle: 'Login required',
  notLoggedInBody: 'This screen is for players only. Pick a dev player above.',
  notAdminBody: 'This screen is for admins only.',

  // home
  homeWelcome: 'Hi there!',
  homeIntro:
    'A private, invite-only pool where friends predict every match of the tournament. Earn points for accurate scores; climb the live scoreboard.',

  // today
  todayTitle: 'Today / Fresh',
  todayEmpty: 'No matches near now.',
  yourTip: 'Your tip',
  yourPoints: 'Your points',

  // schedule
  scheduleTitle: 'Schedule',

  // my tips
  myTipsTitle: 'My Tips',
  selectGroup: 'Select a group',
  saveDraft: 'Save draft',
  lockGroup: 'Lock group',
  locked: 'Locked',
  draft: 'Draft',
  predictedStandings: 'Predicted standings',
  actualStandings: 'Actual standings',
  drawOrderHint: 'Drag tied teams to set your tiebreak order.',
  saved: 'Saved.',
  lockedNotice: 'This group is locked — predictions are read-only.',
  moveUp: 'Move up',
  moveDown: 'Move down',

  // all tips
  allTipsTitle: 'All Tips',
  hiddenTip: 'hidden',

  // scoreboard
  scoreboardTitle: 'Scoreboard',
  pool: 'Pool',
  everyone: 'Everyone',
  overall: 'Overall',
  multiplier: 'Multiplier',

  // perfect
  perfectTitle: 'Perfect Predictions',
  perfectIntro: 'Players who scored the maximum 4 points on a match.',
  perfectEmpty: 'No perfect predictions yet.',

  // pools
  poolsTitle: 'Pools',
  poolsIntro:
    'Create a pool and share its join code, or join one with a code. A pool is a private scoreboard among its members.',
  poolName: 'Pool name',
  poolNamePlaceholder: 'e.g. Office League',
  createPool: 'Create pool',
  joinCodeLabel: 'Join code',
  joinCodePlaceholder: 'paste a join code',
  joinPool: 'Join pool',
  noPools: 'You are not in any pool yet.',
  ownerTag: 'owner',
  removeMember: 'remove',
  renamePool: 'Rename',
  renamePrompt: 'New pool name:',
  rotateCode: 'New join code',
  deletePool: 'Delete',
  deleteConfirm: 'Delete this pool? This cannot be undone.',
  leavePool: 'Leave',
  poolCreated: 'Pool created.',
  poolJoined: 'Joined the pool.',
  poolLeft: 'Left the pool.',
  poolDeleted: 'Pool deleted.',
  codeRotated: 'Join code rotated.',
  memberRemoved: 'Member removed.',
  poolRenamed: 'Pool renamed.',

  // profile
  profileTitle: 'Profile',
  nick: 'Nick',
  fullName: 'Full name',
  email: 'E-mail',
  password: 'New password',
  passwordConfirm: 'Password again',
  profileSaved: 'Profile updated.',
  passwordMismatch: 'Passwords do not match.',

  // invite
  inviteTitle: 'Invite a friend',
  inviteIntro: 'Send a referral. They get a magic-link email to join.',
  sendInvite: 'send the invite which may end up in the spam folder.',
  inviteSent: 'Invitation sent.',
  inviteExists: 'This user is already in the system (based on email)!',

  // rules
  rulesTitle: 'Rules & Scoring',

  // admin
  adminTitle: 'Admin',
  adminResults: 'Results entry',
  adminTeams: 'Teams',
  adminPlayers: 'Players',
  enterResult: 'Enter result',
} as const

const hu: Record<StringKey, string> = {
  tagline: 'Ez a mi kis baráti tippelő környezetünk',
  language: 'Nyelv',
  loggedInAs: 'Belépve mint',
  logOut: 'Engedj ki!',
  logIn: 'engedj be!',
  visitor: 'Kint vagy.',
  footer: 'xPool — baráti focitippjáték',

  navHome: 'Kezdőlap',
  navToday: 'Aktuális',
  navGames: 'Menetrend',
  navMyTips: 'Tippjeim',
  navAllTips: 'Összes tipp',
  navScoreboard: 'Tippverseny',
  navPerfect: 'Telitalálat',
  navPools: 'Ligák',
  navProfile: 'Adatok',
  navInvite: 'Meghívó',
  navRules: 'Szabályok',
  navAdmin: 'Admin',

  loading: 'Betöltés…',
  errorPrefix: 'Valami hiba történt',
  retry: 'Újra',
  save: 'Mentés',
  cancel: 'Mégse',
  group: 'Csoport',
  team: 'Csapat',
  teams: 'Csapatok',
  match: 'Meccs',
  kickoff: 'Kezdés',
  venue: 'Helyszín',
  result: 'Eredmény',
  prediction: 'Tipp',
  points: 'Pont',
  rank: 'Helyezés',
  player: 'Játékos',
  total: 'Összesen',
  notLoggedInTitle: 'Belépés szükséges',
  notLoggedInBody: 'Ez az oldal csak játékosoknak. Válassz fent egy dev játékost.',
  notAdminBody: 'Ez az oldal csak adminoknak.',

  homeWelcome: 'Szevasztok!',
  homeIntro:
    'Zárt, meghívásos tippjáték, ahol a barátok megtippelik a torna minden meccsét. Pontot kapsz a pontos tippekért; mászhatsz az élő eredménytáblán.',

  todayTitle: 'Ma / Friss',
  todayEmpty: 'Nincs meccs a közelben.',
  yourTip: 'A tipped',
  yourPoints: 'Pontjaid',

  scheduleTitle: 'Menetrend',

  myTipsTitle: 'Tippjeim',
  selectGroup: 'Válassz csoportot',
  saveDraft: 'Piszkozat mentése',
  lockGroup: 'Csoport zárása',
  locked: 'Zárva',
  draft: 'Piszkozat',
  predictedStandings: 'Tippelt sorrend',
  actualStandings: 'Valós sorrend',
  drawOrderHint: 'Rendezd a holtversenyes csapatokat a sorrend beállításához.',
  saved: 'Mentve.',
  lockedNotice: 'Ez a csoport zárva — a tippek csak olvashatók.',
  moveUp: 'Fel',
  moveDown: 'Le',

  allTipsTitle: 'Összes tipp',
  hiddenTip: 'rejtett',

  scoreboardTitle: 'Tippverseny',
  pool: 'Liga',
  everyone: 'Mindenki',
  overall: 'Összesített',
  multiplier: 'Szorzó',

  perfectTitle: 'Telitalálatok',
  perfectIntro: 'Játékosok, akik maximum 4 pontot értek el egy meccsen.',
  perfectEmpty: 'Még nincs telitalálat.',

  poolsTitle: 'Ligák',
  poolsIntro:
    'Hozz létre egy ligát és oszd meg a belépőkódját, vagy lépj be egybe kóddal. A liga a tagjai közötti privát eredménytábla.',
  poolName: 'Liga neve',
  poolNamePlaceholder: 'pl. Irodai Liga',
  createPool: 'Liga létrehozása',
  joinCodeLabel: 'Belépőkód',
  joinCodePlaceholder: 'illeszd be a belépőkódot',
  joinPool: 'Belépés ligába',
  noPools: 'Még egyetlen ligában sem vagy.',
  ownerTag: 'tulajdonos',
  removeMember: 'eltávolítás',
  renamePool: 'Átnevezés',
  renamePrompt: 'A liga új neve:',
  rotateCode: 'Új belépőkód',
  deletePool: 'Törlés',
  deleteConfirm: 'Törlöd ezt a ligát? Ez nem vonható vissza.',
  leavePool: 'Kilépés',
  poolCreated: 'Liga létrehozva.',
  poolJoined: 'Beléptél a ligába.',
  poolLeft: 'Kiléptél a ligából.',
  poolDeleted: 'Liga törölve.',
  codeRotated: 'Belépőkód lecserélve.',
  memberRemoved: 'Tag eltávolítva.',
  poolRenamed: 'Liga átnevezve.',

  profileTitle: 'Adatok',
  nick: 'Becenév',
  fullName: 'Teljes név',
  email: 'E-mail',
  password: 'Új jelszó',
  passwordConfirm: 'Jelszó újra',
  profileSaved: 'Adatok frissítve.',
  passwordMismatch: 'A jelszavak nem egyeznek.',

  inviteTitle: 'Hívj meg egy barátot',
  inviteIntro: 'Küldj meghívót. A barátod magic-link emailt kap a belépéshez.',
  sendInvite: 'küldd el a meghívót, ami lehet hogy spamként végzi...',
  inviteSent: 'Meghívó elküldve.',
  inviteExists: 'Ez a felhasználó már benne van a tutiban!',

  rulesTitle: 'Szabályok és pontozás',

  adminTitle: 'Admin',
  adminResults: 'Eredmény rögzítés',
  adminTeams: 'Csapatok',
  adminPlayers: 'Játékosok',
  enterResult: 'Eredmény rögzítése',
}

export const catalogues: Record<Locale, Record<StringKey, string>> = {
  en,
  hu,
}

export const localeNames: Record<Locale, string> = {
  en: 'English',
  hu: 'Magyar',
}
