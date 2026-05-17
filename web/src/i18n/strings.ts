/**
 * UI string catalogue. English + Hungarian (REWRITE_USE_CASES §4 i18n).
 * Hungarian seeded from `archive/locale/hu/LC_MESSAGES/django.po` where a
 * legacy equivalent existed; the rest translated directly.
 *
 * To add a language: add a `Locale` value and a matching key block.
 */

export type Locale = 'en' | 'hu'

export type StringKey = keyof typeof en

const en = {
  // chrome
  tagline: 'Predict every match. Beat your friends.',
  language: 'Language',
  loggedInAs: 'Logged in as',
  logOut: 'get me outside!',
  logIn: 'Log in',
  visitor: 'You are outside.',
  footer: 'xpool — a friendly soccer prediction pool',

  // nav
  navHome: 'Home',
  navToday: 'Today',
  navGames: 'Schedule',
  navMyTips: 'My Tips',
  navAllTips: 'All Tips',
  navScoreboard: 'Scoreboard',
  navPerfect: 'Perfect',
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
  homeWelcome: 'Welcome to xpool',
  homeIntro:
    'A private, invite-only pool where friends predict every match of the tournament. Earn points for accurate scores; climb the live scoreboard.',

  // today
  todayTitle: "Today / Fresh",
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

  // profile
  profileTitle: 'Profile',
  nick: 'Nick',
  fullName: 'Full name',
  email: 'Email',
  password: 'New password',
  passwordConfirm: 'Confirm password',
  profileSaved: 'Profile updated.',
  passwordMismatch: 'Passwords do not match.',

  // invite
  inviteTitle: 'Invite a friend',
  inviteIntro: 'Send a referral. They get a magic-link email to join.',
  sendInvite: 'Send invitation',
  inviteSent: 'Invitation sent.',
  inviteExists: 'This user is already in the system (based on email).',

  // rules
  rulesTitle: 'Rules & Scoring',

  // admin
  adminTitle: 'Admin',
  adminResults: 'Results entry',
  adminBanner: 'Banner',
  adminTeams: 'Teams',
  adminPlayers: 'Players',
  enterResult: 'Enter result',
  bannerText: 'Banner message',
  setBanner: 'Set banner',
  bannerSet: 'Banner updated.',
} as const

const hu: Record<StringKey, string> = {
  tagline: 'Tippeld meg minden meccset. Győzd le a barátaidat.',
  language: 'Nyelv',
  loggedInAs: 'Belépve mint',
  logOut: 'Engedj ki!',
  logIn: 'Belépés',
  visitor: 'Kint vagy.',
  footer: 'xpool — baráti focitippjáték',

  navHome: 'Kezdőlap',
  navToday: 'Ma',
  navGames: 'Menetrend',
  navMyTips: 'Tippjeim',
  navAllTips: 'Minden tipp',
  navScoreboard: 'Eredménytábla',
  navPerfect: 'Telitalálat',
  navProfile: 'Profil',
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

  homeWelcome: 'Üdvözlünk a xpoolban',
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

  allTipsTitle: 'Minden tipp',
  hiddenTip: 'rejtett',

  scoreboardTitle: 'Eredménytábla',
  pool: 'Liga',
  everyone: 'Mindenki',
  overall: 'Összesített',
  multiplier: 'Szorzó',

  perfectTitle: 'Telitalálatok',
  perfectIntro: 'Játékosok, akik maximum 4 pontot értek el egy meccsen.',
  perfectEmpty: 'Még nincs telitalálat.',

  profileTitle: 'Profil',
  nick: 'Becenév',
  fullName: 'Teljes név',
  email: 'Email',
  password: 'Új jelszó',
  passwordConfirm: 'Jelszó megerősítése',
  profileSaved: 'Profil frissítve.',
  passwordMismatch: 'A jelszavak nem egyeznek.',

  inviteTitle: 'Hívj meg egy barátot',
  inviteIntro: 'Küldj meghívót. A barátod magic-link emailt kap a belépéshez.',
  sendInvite: 'Meghívó küldése',
  inviteSent: 'Meghívó elküldve.',
  inviteExists: 'Ez a felhasználó már benne van a tutiban (email alapján).',

  rulesTitle: 'Szabályok és pontozás',

  adminTitle: 'Admin',
  adminResults: 'Eredmény rögzítés',
  adminBanner: 'Hirdetmény',
  adminTeams: 'Csapatok',
  adminPlayers: 'Játékosok',
  enterResult: 'Eredmény rögzítése',
  bannerText: 'Hirdetmény szövege',
  setBanner: 'Hirdetmény beállítása',
  bannerSet: 'Hirdetmény frissítve.',
}

export const catalogues: Record<Locale, Record<StringKey, string>> = {
  en,
  hu,
}

export const localeNames: Record<Locale, string> = {
  en: 'English',
  hu: 'Magyar',
}
