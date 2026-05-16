# xpool — User Journeys, Scenarios & Use Cases

The **functional behavior** of the xpool / xEuroPool / xHomePool soccer
score-prediction pool, described independently of any technology. This is what
the rewrite must *do*. For *how* — entities, algorithms, anti-patterns to avoid
— see [`REWRITE_IMPLEMENTATION.md`](./REWRITE_IMPLEMENTATION.md). For the
scoring math, see [`GAME_RULES.md`](./GAME_RULES.md).

---

## 1. What the product is

A private, invite-only web app where a group of friends predict the scores of
every match in a soccer tournament (FIFA World Cup, UEFA Euro). Players earn
points for accurate predictions; a live scoreboard ranks them. It has been
reused across multiple tournaments since ~2010 (2010 World Cup, Euro 2012,
2014 World Cup).

**Core value:** friendly competition. It is small-scale (dozens of players,
hundreds of matches), not a commercial betting platform — no money, no odds.

### Actors

| Actor | Description |
|-------|-------------|
| **Visitor** | Not logged in. Can see public pages only. |
| **Player** | Logged-in participant. Predicts scores, has a scoreboard rank. |
| **Invited player** | A player created via referral who logs in by emailed magic link until they set a password. |
| **Admin** | Tournament organizer. Sets up the tournament, enters official results. |

---

## 2. End-to-end user journeys

Narrative walkthroughs of how each actor moves through the product. Use cases
in §3 break these into testable units.

### Journey A — A friend joins and plays a tournament

1. **Invitation.** Existing player Anna opens **Invite** and enters her friend
   Ben's email and name. Ben receives an email with a magic link.
2. **First login.** Ben clicks the link, lands logged-in on his **Profile**
   page, and is prompted to set a password and/or link his Google account.
3. **First predictions.** Ben opens **My Tips**, sees the group-stage matches,
   picks a score for each, and **locks** them before the group's deadline.
   Until he locks, his picks are drafts — worth nothing and hidden from others.
4. **Group standings.** Below the matches, Ben sees the standings his predicted
   scores imply. Where his scores create a qualification-affecting tie, he
   manually orders the tied teams.
5. **Watching the tournament.** As matches are played and the admin enters
   results, Ben checks **Today** to see his points, the **Scoreboard** to see
   his rank, **All Tips** to compare with everyone, and **Perfect** to see who
   nailed exact scores.
6. **Knockout stage.** Each knockout match has its own deadline; Ben predicts
   each one as the bracket fills in. Later rounds are worth more points.
7. **Tournament ends.** The final scoreboard decides the winner.

### Journey B — The organizer runs a tournament

1. **Setup.** Before the tournament, the admin initializes the tournament tree:
   teams, groups, the fixture schedule with kickoff times and venues.
2. **Opening.** Players are invited and start locking predictions ahead of
   each group's deadline.
3. **During play.** After each match the admin enters the official final score
   and locks it. This updates scoreboards and the perfect-predictions list.
4. **Knockout maintenance.** As group results determine knockout participants,
   the admin assigns the now-known teams to the bracket's matches.
5. **Corrections.** If a player reports a problem, the admin can edit teams,
   fixtures, a player's prediction, or post a banner message to everyone.

### Journey C — A casual visitor

A non-logged-in visitor can browse the **Schedule**, **Scoreboard**,
**Perfect** list, **Today** page, and **Rules** to follow the pool, but cannot
predict or see the betting screens until they log in.

---

## 3. Use cases

### 3.1 Authentication & onboarding

**UC-1 — Visitor browses public pages.** Anyone can view the schedule,
scoreboard, perfect-predictions list, today's games, and the rules without
logging in. Betting and all-tips pages require login.

**UC-2 — Player logs in.** Three interchangeable methods, all resolving to the
same player account (matched/linked by email):
- **Email + password.**
- **Google account.** First Google login auto-creates or links an account.
- **Facebook.** (Modern rewrite: OAuth, or drop it.)

A player may have several linked identities (password + Google + Facebook) on
one account. Linking different identities should require explicit confirmation
(the legacy app linked silently by email).

**UC-3 — Player invites a friend (referral).**
1. A logged-in player opens **Invite**, enters a friend's email, nick, and full
   name.
2. The system rejects the invite if the email already belongs to a player.
3. Otherwise it creates a new player account with a random magic-link token,
   recording who the referrer was.
4. An invitation email is sent containing the magic link.
5. The invitee clicks the link → logged in automatically → landed on their
   profile page → prompted to set a password and/or link a social account.
6. The magic link keeps working until the invitee sets a password, then
   expires.

**UC-4 — Player edits their profile.** Change email, nick, full name, and
password (with a confirmation field). Changing the password invalidates the
magic-link token.

### 3.2 Making predictions (the core loop)

**UC-5 — Player predicts match scores ("My Tips").**
- The **My Tips** screen shows the matches for a selected stage/group, each
  with a home-score input, an away-score input, and a **lock** control.
- The player picks a predicted score for each match.
- A prediction is a **draft** until the player **locks** it. Drafts score zero
  and are hidden from other players (anti-cheating — see UC-9).
- Locking is irreversible by the player. A prediction can only be locked when
  both scores are filled in.
- Saving submits all matches in the group together.

**UC-6 — Player predicts group standings.**
- For each group, the screen also shows two standings tables: the **actual**
  standings and the standings **implied by the player's predicted scores**.
- When the player's predicted scores produce a tie that affects qualification,
  the player must manually order the tied teams — modeling extra time /
  penalties / drawing of lots.
- The whole group can be locked at once with a group-lock control.

**UC-7 — Deadlines are enforced.**
- Group-stage predictions must be locked **before the first match of the
  group** kicks off.
- Knockout predictions have a **per-match** deadline (each match is its own
  group).
- After the deadline a player can no longer edit; inputs become read-only.
- Admins may still edit a player's prediction after the deadline.

### 3.3 Viewing results & competition

**UC-8 — Player views the scoreboard.** A ranked leaderboard of total points,
for the whole tournament and per stage/group. Each stage shows its point
multiplier. Public.

**UC-9 — Player views everyone's tips ("All Tips").** A grid of every active
player's predictions for a group, side by side. A given player's prediction is
revealed to others **only after that player locked it or the match kicked
off** — preventing strategic copying.

**UC-10 — Player sees "perfect" predictions.** A list of players who scored a
maximum (4-point) prediction on a match. Public, celebratory.

**UC-11 — Player checks "Today / Fresh".** A flat list of matches within a
~±2-day window of now, showing each match's result, the player's prediction,
and points earned.

**UC-12 — Player views the schedule.** The full fixture list with kickoff
times and venues, navigable by tournament stage. Read-only, public.

### 3.4 Administration

**UC-13 — Admin sets up a tournament.** Initializes the tournament tree
(groups, matches, teams, kickoff times, venues) from a tournament definition.

**UC-14 — Admin enters official match results.** For each match, the admin
enters the final score and locks it. Locking a result triggers recomputation
of scoreboards and perfect-prediction lists.

**UC-15 — Admin manages teams and fixtures.** Edit team metadata; reassign the
home/away teams of a match (needed for knockout matches once the participants
are known).

**UC-16 — Admin manages players and the banner.** List players; set the
message-of-the-day banner shown site-wide.

---

## 4. Screens / UI inventory

Persistent chrome: header (tagline + language selector), auth bar, horizontal
nav menu, content area (with an optional flash-message bar and a sub-navigation
for tournament groups), footer.

| Screen | Route | Purpose | Access |
|--------|-------|---------|--------|
| Home | `/` | Landing page | Public |
| Today / Fresh | `/today` | Matches near now + your tips | Public (richer if logged in) |
| Schedule | `/games` | Full fixture list & venues | Public |
| My Tips | `/mytips` | Enter & lock predictions | Player |
| All Tips | `/alltips` | Compare everyone's predictions | Player |
| Scoreboard | `/scoreboard` | Leaderboard, overall + per stage | Public |
| Perfect | `/perfect` | Players with perfect predictions | Public |
| Profile | `/profile` | Account settings | Player |
| Invite | `/invite` | Send referral invitations | Player |
| Rules | `/rules` | Rules & scoring explanation | Public |
| Admin | `/admin/*` | Setup, results, teams, users | Admin |

### Betting interaction detail

Score entry = two dropdowns per match (legacy allowed 0–9). Lock = a control
per match, plus a group-level lock. The legacy app used full-page POST + reload
for every save; a modern rewrite can make this interactive (inline save,
optimistic UI) but **must preserve two rules**:

1. The **draft → locked** state machine — a prediction scores nothing and is
   editable until locked; locking is final for the player.
2. **Hidden-until-locked visibility** — other players cannot see a prediction
   until it is locked or the match has kicked off.

### Internationalization

Two languages: **English** and **Hungarian**, switchable from a header
dropdown. All UI strings, button labels, column headers, and the invitation
email are translated. The rewrite should keep i18n as a first-class concern
and be structured to add more languages.

---

## 5. User-visible rules of play

Player-facing summary; the precise math is in [`GAME_RULES.md`](./GAME_RULES.md).

- **Per match:** +1 for the exact home score, +1 for the exact away score,
  +2 for the correct outcome (win / draw / loss). Maximum 4 points per match.
- **Per group:** +1 for every pair of teams the player ordered correctly in
  their predicted standings.
- **Predicted standings** are derived from the player's own predicted scores,
  then ranked by points (3/1/0), head-to-head among tied teams, goal
  difference, goals scored, and finally the player's manual tie order.
- **Stage multiplier:** group-stage points count ×1; each later knockout round
  is worth progressively more.
- **Scoreboard** = the sum of every match and group's points, multiplied by the
  stage factor, across the whole tournament.
- **Predictions only count once locked.** Unlocked predictions score 0 and stay
  hidden.
- **A "perfect"** is a maximum-point (4) match prediction.
- **Deadlines:** lock before the group's first match (group stage) or before
  the match (knockout).
- **Fair play:** you cannot see other players' predictions until they lock
  them — by design, to prevent copying.
