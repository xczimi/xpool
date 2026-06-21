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
  homeHowTitle: 'How it works',
  homeHowStep1: 'Predict the exact score of every match.',
  homeHowStep2:
    'Earn points for accuracy — up to 4 a match, worth more in the knockouts.',
  homeHowStep3:
    "Climb the live scoreboard. It's free and just for bragging rights.",
  homeRulesLink: 'See full rules & scoring',
  homeFaqTitle: 'Common questions',
  homeFaqQ1: "Isn't that a lot of predictions?",
  homeFaqA1:
    "You fill in one group at a time, and each pick only has to be in before that match kicks off. It's a score-prediction pool, not fantasy football — no transfers, no line-ups, no daily admin.",
  homeFaqQ2: 'How does scoring work?',
  homeFaqA2:
    '+1 for the right home score, +1 for the right away score, and +2 for the correct result — up to 4 points a match. Knockout rounds carry higher multipliers, and there is a bonus for the group standings order. Picks lock at kickoff; nobody sees yours until then.',

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
  nextToFinalize: 'Next to finalize',
  finalizeBy: 'finalize by',
  enterAllGamesHint:
    'Make sure you enter predictions for all games of the group before the deadline.',
  finalizeClosed: 'Finalize closed',
  saveDraft: 'Save draft',
  lockGroup: 'Finalize predictions',
  lockConfirm: "Finalize these predictions? Once final, they can't be changed.",
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
  perfectByMatch: 'By match',
  perfectByPlayer: 'By player',

  // player-detail page
  playerNotInPool: 'This player is not in your pool.',
  playerPageOwnLink: 'My player page',
  playerPerfectsHeading: 'Perfect predictions',
  playerNowHeading: 'Around now',
  tipCol: 'Tip',

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
  handOverNeedsMember: 'Invite someone to this pool before you can hand it over.',
  ownershipTransferred: 'Ownership handed over.',
  shareTemplatesTitle: 'Ready-to-send invite messages',
  shareTemplatesHint:
    'Copy a message and paste your invite link where it says {LINK}.',
  shareTemplateShort: 'Short (WhatsApp / text)',
  shareTemplateOneLiner: 'One-liner',
  shareTemplateEmail: 'Email (colleagues)',
  shareTemplateHungarian: 'Hungarian',
  copied: 'Copied!',

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
  rulesIntro:
    'Predict the score of every match. The closer you are, the more you score — and the live scoreboard does the rest.',
  rulesPerMatchTitle: 'Per match',
  rulesPerMatchExactHome: '+1 for the exact home score.',
  rulesPerMatchExactAway: '+1 for the exact away score.',
  rulesPerMatchOutcome: '+2 for the correct outcome (win / draw / loss).',
  rulesPerMatchMax: 'Maximum 4 points per match.',
  rulesPerMatchFourGoal:
    'A side scoring 4 or more is matched by any prediction of 4 or more for that side.',
  rulesPerMatchFullTime:
    'Scores are judged at full time (90 minutes) — extra time and penalties do not count toward the per-match score.',
  rulesPerGroupTitle: 'Per group',
  rulesPerGroupPairs:
    '+1 for every pair of teams ordered correctly in your predicted standings.',
  rulesPerGroupStandings:
    'Predicted standings are derived from your own predicted scores, then ranked by points (3/1/0), head-to-head, goal difference, goals scored, and finally your manual tie order.',
  rulesMultipliersTitle: 'Stage multipliers',
  rulesRoundColumn: 'Round',
  rulesFairPlayTitle: 'Fair play',
  rulesFairPlayLock:
    'Predictions only count once locked; unlocked predictions score 0 and stay hidden from others.',
  rulesFairPlayPerfect: 'A "perfect" is a maximum-point (4) match prediction.',
  rulesFairPlayDeadline:
    "Lock before the group's first match (group stage) or before the match (knockout).",
  rulesFairPlayHidden:
    "You cannot see other players' predictions until they lock them — by design, to prevent copying.",
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

  // match page (#2 live preview)
  provisionalLabel: 'Provisional — if it ended now',
  liveLabel: 'Live',
  finalLabel: 'Final',
  awaitingResult: 'Awaiting official result',
  ninetyMinuteNote:
    'Knockout — provisional points use the 90-minute rule; extra time may change the official result.',

  // scoreboard live ceiling (live-scoring cluster)
  ceilingLabel: 'Max',
  ceilingTooltip: 'Best still-reachable total — provisional while matches are live',
  liveBoardNote: 'Live — “Max” shows each player’s best still-reachable total',

  // match page force-refresh (live-scoring cluster)
  refreshNow: 'Refresh now',
  refreshing: 'Refreshing…',
  lastUpdated: 'Updated',
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
  loggedInAs: 'Belépve:',
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
  navScoreboard: 'Eredményjelző',
  navPerfect: 'Telitalálatok',
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
  homeHowTitle: 'Hogyan működik',
  homeHowStep1: 'Tippeld meg minden meccs pontos eredményét.',
  homeHowStep2:
    'Pontot kapsz a pontosságért — meccsenként akár 4-et, az egyenes kieséses szakaszban többet.',
  homeHowStep3:
    'Mászfel az élő eredménytáblán. Ingyenes, és csak a dicsőségért megy.',
  homeRulesLink: 'Teljes szabályok és pontozás',
  homeFaqTitle: 'Gyakori kérdések',
  homeFaqQ1: 'Nem túl sok az a tipp?',
  homeFaqA1:
    'Egyszerre csak egy csoportot töltesz ki, és minden tippet csak az adott meccs kezdése előtt kell leadni. Ez eredménytippelő játék, nem fantasy foci — nincs igazolás, nincs kezdőcsapat, nincs napi pepecselés.',
  homeFaqQ2: 'Hogyan megy a pontozás?',
  homeFaqA2:
    '+1 a jó hazai eredményért, +1 a jó vendégeredményért, és +2 a helyes végkimenetelért — meccsenként akár 4 pont. Az egyenes kieséses körök nagyobb szorzót érnek, és a csoport végeredmény sorrendjéért is jár bónusz. A tippek a kezdő sípszóra zárolódnak; a tiédet addig más nem látja.',

  todayTitle: 'Ma / Friss',
  todayEmpty: 'Nincs meccs a közelben.',
  yourTip: 'A tipped',
  yourPoints: 'Pontjaid',

  scheduleTitle: 'Menetrend',

  myTipsTitle: 'Tippjeim',
  selectGroup: 'Válassz csoportot',
  nextToFinalize: 'Következő véglegesítés',
  finalizeBy: 'véglegesítés eddig',
  enterAllGamesHint:
    'Ügyelj rá, hogy a csoport összes meccsére adj tippet a határidő előtt.',
  finalizeClosed: 'Véglegesítés lezárva',
  saveDraft: 'Piszkozat mentése',
  lockGroup: 'Tippek véglegesítése',
  lockConfirm: 'Véglegesíted ezeket a tippeket? Ezután már nem módosíthatók.',
  locked: 'végleges',
  draft: 'Piszkozat',
  predictedStandings: 'Tippelt sorrend',
  actualStandings: 'Valós sorrend',
  drawOrderHint: 'Rendezd a holtversenyes csapatokat a sorrend beállításához.',
  saved: 'Mentve.',
  lockedNotice: 'Tippek véglegesek – csak nézni, ne nyúlj hozzá!',
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
  perfectIntro: 'Játékosok, akik maximum megszerezhető 4 pontot értek el egy meccsen.',
  perfectEmpty: 'Még nincs telitalálat.',
  perfectByMatch: 'Meccs szerint',
  perfectByPlayer: 'Játékos szerint',

  // player-detail page
  playerNotInPool: 'Ez a játékos nincs a ligádban.',
  playerPageOwnLink: 'Saját oldalam',
  playerPerfectsHeading: 'Telitalálatok',
  playerNowHeading: 'Épp most',
  tipCol: 'Tipp',

  poolsTitle: 'Ligák',
  poolsIntro:
    'Hozz létre egy ligát és oszd meg a meghívódat, vagy lépj be egybe kóddal vagy linkkel. A liga a tagjai közötti privát eredménytábla.',
  poolName: 'Liga neve',
  poolNamePlaceholder: 'pl. Irodai Liga',
  createPool: 'Liga létrehozása',
  inviteCodeLabel: 'Meghívókód vagy link',
  inviteCodePlaceholder: 'kopi-pészt a linket vagy írd be a kódot',
  joinAction: 'Csatlakozás',
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
  handOverNeedsMember: 'Hívj meg valakit a ligába, mielőtt átadhatnád.',
  ownershipTransferred: 'Tulajdon átadva.',
  shareTemplatesTitle: 'Kész meghívó üzenetek',
  shareTemplatesHint:
    'Másolj ki egy üzenetet, és illeszd be a meghívó linkedet oda, ahol {LINK} áll.',
  shareTemplateShort: 'Rövid (WhatsApp / SMS)',
  shareTemplateOneLiner: 'Egysoros',
  shareTemplateEmail: 'E-mail (kollégáknak)',
  shareTemplateHungarian: 'Magyar',
  copied: 'Másolva!',

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
  rulesIntro:
    'Tippeld meg minden meccs eredményét. Minél közelebb vagy, annál több pontod van — a többit az élő eredménytábla elintézi.',
  rulesPerMatchTitle: 'Meccsenként',
  rulesPerMatchExactHome: '+1 a pontos hazai eredményért.',
  rulesPerMatchExactAway: '+1 a pontos vendég eredményért.',
  rulesPerMatchOutcome:
    '+2 a helyes végkimenetelért (győzelem / döntetlen / vereség).',
  rulesPerMatchMax: 'Meccsenként legfeljebb 4 pont.',
  rulesPerMatchFourGoal:
    'Ha egy csapat 4 vagy több gólt rúg, bármely 4-re vagy többre tett tipp találat arra az oldalra.',
  rulesPerMatchFullTime:
    'Az eredményeket a rendes játékidő (90 perc) végén értékeljük — a hosszabbítás és a büntetők nem számítanak a meccs pontozásába.',
  rulesPerGroupTitle: 'Csoportonként',
  rulesPerGroupPairs:
    '+1 minden olyan csapatpárért, amelynek sorrendjét helyesen tippelted a tabelládon.',
  rulesPerGroupStandings:
    'A tippelt tabella a saját tippelt eredményeidből áll össze, majd pontszám (3/1/0), egymás elleni eredmény, gólkülönbség, lőtt gólok és végül a kézi sorrended szerint rendeződik.',
  rulesMultipliersTitle: 'Szakasz-szorzók',
  rulesRoundColumn: 'Forduló',
  rulesFairPlayTitle: 'Fair play',
  rulesFairPlayLock:
    'A tippek csak rögzítés után számítanak; a nem rögzített tippek 0 pontot érnek és rejtve maradnak a többiek elől.',
  rulesFairPlayPerfect:
    'A „telitalálat" egy maximális pontot (4) érő meccstipp.',
  rulesFairPlayDeadline:
    'Rögzíts a csoport első meccse előtt (csoportkör), illetve a meccs előtt (egyenes kiesés).',
  rulesFairPlayHidden:
    'A többiek tippjeit nem látod, amíg nem rögzítik őket — szándékosan, a másolás megelőzésére.',
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

  // match page (#2 live preview)
  provisionalLabel: 'Ideiglenes — ha most érne véget',
  liveLabel: 'Élő',
  finalLabel: 'Vége',
  awaitingResult: 'Hivatalos eredményre várunk',
  ninetyMinuteNote:
    'Kieséses szakasz — az ideiglenes pontok a 90 perces szabály szerint számolnak; a hosszabbítás módosíthatja a hivatalos eredményt.',

  // scoreboard live ceiling (live-scoring cluster)
  ceilingLabel: 'Max',
  ceilingTooltip: 'Elérhető legjobb összpontszám — ideiglenes, amíg meccsek élnek',
  liveBoardNote: 'Élő — a „Max” mutatja kinek mennyi a még elérhető pontja',

  // match page force-refresh (live-scoring cluster)
  refreshNow: 'Frissítés most',
  refreshing: 'Frissítés…',
  lastUpdated: 'Frissítve',
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
