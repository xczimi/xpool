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
  settings: 'Settings',
  language: 'Language',
  displayFlag: 'Flag',
  displayText: 'Text',
  flagOn: 'On',
  flagOff: 'Off',
  textAuto: 'Auto',
  textName: 'Name',
  textCode: 'Code',
  textOff: 'Off',
  theme: 'Theme',
  mode: 'Mode',
  accentAmber: 'Amber',
  accentGreen: 'Green',
  accentCyan: 'Cyan',
  accentMagenta: 'Magenta',
  accentViolet: 'Violet',
  accentMono: 'Mono',
  modeSystem: 'System',
  modeDark: 'Dark',
  modeLight: 'Light',
  loggedInAs: 'Logged in as',
  logOut: 'get me outside!',
  logIn: 'let me in!',
  visitor: 'You are outside.',
  devClock: 'Dev clock',
  devClockReset: 'real time',
  devClockGame: 'Game',
  devClockGamePlaceholder: 'pick a game…',
  devClockWhen: 'When',
  devClockWhenPlaceholder: 'pick a time…',
  devClockBefore: '10 min before kickoff',
  devClockDuring: 'during (60 min in)',
  devClockAfter: '15 min after full time',
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
  navRules: 'Rules',
  navAdmin: 'Admin',

  // generic
  loading: 'Loading…',
  errorPrefix: 'Something went wrong',
  retry: 'Retry',
  save: 'Save',
  cancel: 'Cancel',
  confirmAction: 'Confirm',
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
  unknownPlayer: '(unknown)',
  total: 'Total',
  // points breakdown / standings transparency
  exactHome: 'Exact home score',
  exactAway: 'Exact away score',
  outcomeResult: 'Correct outcome',
  base: 'base',
  standingsCol: 'Standings',
  standingsBonus: 'Standings bonus',
  pairsCorrect: 'pairs correct',
  noPointsYet: 'awaiting result',
  notLoggedInTitle: 'Login required',
  notLoggedInBody: 'This screen is for players only. Pick a dev player above.',
  notAdminBody: 'This screen is for admins only.',

  // invite-only funnel + dead-end (invite-only-hardening)
  frontDoorLead: 'Got an invite? Open the link a friend sent you to join.',
  frontDoorMembers: 'Already playing? Log in',
  inviteOnlyTitle: 'You need an invite',
  inviteOnlyBody:
    'xPool is a small, friendly, invite-only pool. To play, open the invite link a friend who already plays sent you — that link signs you in and adds you to their pool.',
  inviteOnlyHaveLink: 'Have an invite link?',
  inviteOnlyPastePlaceholder: 'Paste your invite link or code',
  inviteOnlyOpen: 'Open',
  inviteOnlyBadLink: "That doesn't look like an invite link or code.",

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
  lockConfirm: 'Lock these predictions? Once locked, they cannot be changed.',
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

  // round labels
  roundGroupStage: 'Group Stage',
  roundR32: 'Round of 32',
  roundR16: 'Round of 16',
  roundQF: 'Quarter-final',
  roundSF: 'Semi-final',
  roundThirdPlace: 'Third place',
  roundFinal: 'Final',

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
    'Create a pool and share your invite, or join one with a code or link. A pool is a private scoreboard among its members.',
  poolName: 'Pool name',
  poolNamePlaceholder: 'e.g. Office League',
  createPool: 'Create pool',
  inviteCodeLabel: 'Invite code or link',
  inviteCodePlaceholder: 'paste a link or type a code',
  joinAction: 'Join',
  noPools: 'You are not in any pool yet.',
  ownerTag: 'owner',
  removeMember: 'remove',
  renamePool: 'Rename',
  renamePrompt: 'New pool name:',
  shareInvite: 'Share invite',
  inviteLinkLabel: 'Your invite link',
  copyLink: 'Copy',
  linkCopied: 'Link copied.',
  revokeInvite: 'Revoke invite',
  deletePool: 'Delete',
  deleteConfirm: 'Delete this pool? This cannot be undone.',
  leavePool: 'Leave',
  poolCreated: 'Pool created.',
  poolJoined: 'Joined the pool.',
  poolLeft: 'Left the pool.',
  poolDeleted: 'Pool deleted.',
  inviteShared: 'Invite ready to share.',
  inviteRevoked: 'Invite revoked.',
  memberRemoved: 'Member removed.',
  poolRenamed: 'Pool renamed.',
  transferOwnership: 'Hand over',
  handOverTo: 'Hand over ownership to…',
  handOverConfirm: 'Hand over ownership to',
  ownershipTransferred: 'Ownership handed over.',

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
  // onboarding / invite claim
  inviteWelcomeTitle: "You've been invited to xPool!",
  inviteWelcomeBody:
    "We'll set up a quick, secure sign-in (email or Google) so only you can enter your picks.",
  inviteContinue: 'Continue to join',
  inviteClaimTitle: 'Accept your invite',
  inviteClaimBody: 'Set your display name.',
  join: 'Join',
  inviteJoinTitle: 'Join this pool',
  inviteJoinBody: 'Accept this invite to join the pool.',
  inviteJoinedPrefix: "You're in",
  inviteGoScoreboard: 'Go to the scoreboard',
  inviteLinkTitle: 'Link this login?',
  inviteLinkBody: 'An account already exists for this email. Link this login to it?',
  inviteLinkConfirm: 'Yes, link',
  inviteLinkCancel: 'No, cancel',
  inviteMissingCode: 'Missing invite code.',

  // privacy
  privacy: 'Privacy',
  privacyTitle: 'Privacy Policy',

  // admin
  adminTitle: 'Admin',
  adminTeams: 'Teams',
  adminPlayers: 'Players',
  refreshFailed: 'Could not refresh — the screen may be out of date.',
} as const

const hu: Record<StringKey, string> = {
  tagline: 'Ez a mi kis baráti tippelő környezetünk',
  settings: 'Beállítások',
  language: 'Nyelv',
  displayFlag: 'Zászló',
  displayText: 'Szöveg',
  flagOn: 'Be',
  flagOff: 'Ki',
  textAuto: 'Auto',
  textName: 'Név',
  textCode: 'Kód',
  textOff: 'Nincs',
  theme: 'Téma',
  mode: 'Mód',
  accentAmber: 'Borostyán',
  accentGreen: 'Zöld',
  accentCyan: 'Cián',
  accentMagenta: 'Magenta',
  accentViolet: 'Lila',
  accentMono: 'Mono',
  modeSystem: 'Rendszer',
  modeDark: 'Sötét',
  modeLight: 'Világos',
  loggedInAs: 'Belépve mint',
  logOut: 'Engedj ki!',
  logIn: 'engedj be!',
  visitor: 'Kint vagy.',
  devClock: 'Dev óra',
  devClockReset: 'valós idő',
  devClockGame: 'Meccs',
  devClockGamePlaceholder: 'válassz meccset…',
  devClockWhen: 'Mikor',
  devClockWhenPlaceholder: 'válassz időt…',
  devClockBefore: '10 perccel kezdés előtt',
  devClockDuring: 'közben (60. perc)',
  devClockAfter: '15 perccel a vége után',
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
  navRules: 'Szabályok',
  navAdmin: 'Admin',

  loading: 'Betöltés…',
  errorPrefix: 'Valami hiba történt',
  retry: 'Újra',
  save: 'Mentés',
  cancel: 'Mégse',
  confirmAction: 'Megerősítés',
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
  unknownPlayer: '(ismeretlen)',
  total: 'Összesen',
  // pontozás részletei / tabella átláthatóság
  exactHome: 'Pontos hazai eredmény',
  exactAway: 'Pontos vendég eredmény',
  outcomeResult: 'Helyes végkimenetel',
  base: 'alap',
  standingsCol: 'Tabella',
  standingsBonus: 'Tabella bónusz',
  pairsCorrect: 'helyes pár',
  noPointsYet: 'eredményre vár',
  notLoggedInTitle: 'Belépés szükséges',
  notLoggedInBody: 'Ez az oldal csak játékosoknak. Válassz fent egy dev játékost.',
  notAdminBody: 'Ez az oldal csak adminoknak.',

  // meghívásos tölcsér + zsákutca (invite-only-hardening)
  frontDoorLead: 'Van meghívód? Nyisd meg a linket, amit egy barátod küldött, hogy csatlakozz.',
  frontDoorMembers: 'Már játszol? Belépés',
  inviteOnlyTitle: 'Meghívó szükséges',
  inviteOnlyBody:
    'A xPool egy kicsi, baráti, csak meghívással elérhető tippverseny. A játékhoz nyisd meg a meghívó linket, amit egy már játszó barátod küldött — a link beléptet és hozzáad a tippcsapatához.',
  inviteOnlyHaveLink: 'Van meghívó linked?',
  inviteOnlyPastePlaceholder: 'Illeszd be a meghívó linket vagy kódot',
  inviteOnlyOpen: 'Megnyitás',
  inviteOnlyBadLink: 'Ez nem tűnik meghívó linknek vagy kódnak.',

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
  lockConfirm: 'Lezárod ezeket a tippeket? Lezárás után nem módosíthatók.',
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

  roundGroupStage: 'Csoportkör',
  roundR32: 'Legjobb 32',
  roundR16: 'Nyolcaddöntő',
  roundQF: 'Negyeddöntő',
  roundSF: 'Elődöntő',
  roundThirdPlace: 'Bronzmérkőzés',
  roundFinal: 'Döntő',

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
    'Hozz létre egy ligát és oszd meg a meghívódat, vagy lépj be egybe kóddal vagy linkkel. A liga a tagjai közötti privát eredménytábla.',
  poolName: 'Liga neve',
  poolNamePlaceholder: 'pl. Irodai Liga',
  createPool: 'Liga létrehozása',
  inviteCodeLabel: 'Meghívókód vagy link',
  inviteCodePlaceholder: 'illeszd be a linket vagy írd be a kódot',
  joinAction: 'Belépés',
  noPools: 'Még egyetlen ligában sem vagy.',
  ownerTag: 'tulajdonos',
  removeMember: 'eltávolítás',
  renamePool: 'Átnevezés',
  renamePrompt: 'A liga új neve:',
  shareInvite: 'Meghívó megosztása',
  inviteLinkLabel: 'A meghívó linked',
  copyLink: 'Másolás',
  linkCopied: 'Link másolva.',
  revokeInvite: 'Meghívó visszavonása',
  deletePool: 'Törlés',
  deleteConfirm: 'Törlöd ezt a ligát? Ez nem vonható vissza.',
  leavePool: 'Kilépés',
  poolCreated: 'Liga létrehozva.',
  poolJoined: 'Beléptél a ligába.',
  poolLeft: 'Kiléptél a ligából.',
  poolDeleted: 'Liga törölve.',
  inviteShared: 'A meghívó megosztható.',
  inviteRevoked: 'Meghívó visszavonva.',
  memberRemoved: 'Tag eltávolítva.',
  poolRenamed: 'Liga átnevezve.',
  transferOwnership: 'Átadás',
  handOverTo: 'Tulajdon átadása…',
  handOverConfirm: 'Tulajdon átadása neki:',
  ownershipTransferred: 'Tulajdon átadva.',

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
  inviteWelcomeTitle: 'Meghívtak az xPoolba!',
  inviteWelcomeBody:
    'Beállítunk egy gyors, biztonságos belépést (e-mail vagy Google), hogy csak te adhasd meg a tippjeidet.',
  inviteContinue: 'Tovább a csatlakozáshoz',
  inviteClaimTitle: 'Fogadd el a meghívót',
  inviteClaimBody: 'Add meg a megjelenítendő neved.',
  join: 'Csatlakozás',
  inviteJoinTitle: 'Csatlakozz ehhez a tutihoz',
  inviteJoinBody: 'Fogadd el a meghívót, hogy csatlakozz a tutihoz.',
  inviteJoinedPrefix: 'Bent vagy itt:',
  inviteGoScoreboard: 'Irány az eredménytábla',
  inviteLinkTitle: 'Összekapcsolod ezt a belépést?',
  inviteLinkBody: 'Már létezik fiók ehhez az e-mailhez. Összekapcsolod vele ezt a belépést?',
  inviteLinkConfirm: 'Igen, kapcsold össze',
  inviteLinkCancel: 'Nem, mégse',
  inviteMissingCode: 'Hiányzó meghívókód.',

  privacy: 'Adatvédelem',
  privacyTitle: 'Adatvédelmi tájékoztató',

  adminTitle: 'Admin',
  adminTeams: 'Csapatok',
  adminPlayers: 'Játékosok',
  refreshFailed: 'A frissítés nem sikerült — a képernyő elavult lehet.',
}

export const catalogues: Record<Locale, Record<StringKey, string>> = {
  en,
  hu,
}

export const localeNames: Record<Locale, string> = {
  en: 'English',
  hu: 'Magyar',
}

/**
 * Flag (ISO code → `/flags/{iso}.png`) shown on each language segment. English
 * flies the Canadian flag (this pool's home), Hungarian the Hungarian flag.
 */
export const localeFlags: Record<Locale, string> = {
  en: 'ca',
  hu: 'hu',
}
