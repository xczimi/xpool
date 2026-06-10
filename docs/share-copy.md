# xPool — invite share copy

Ready-to-send messages for inviting friends and colleagues to an xPool pool.
Drop your reusable pool invite link (the `…/invite/<pool>-<code>` one from the
Pools page) in place of `{LINK}`. No money, just for fun — keep that tone.

> **In the app:** the four ready-to-send messages below (A short, B one-liner,
> C email, D Hungarian) are also surfaced on the **Pools page** "Ready-to-send
> invite messages" panel, copyable straight from the browser — their canonical
> wording lives in `web/src/content/shareTemplates.ts`. The objection-handling
> and scoring blurbs are folded into the Home page how-it-works FAQ
> (`web/src/i18n/strings.ts`, `homeFaq*`). Keep this doc and those in sync.

## A — WhatsApp / text, friends (short)

> ⚽ The World Cup kicks off this week and I'm running a score-prediction pool.
> Predict the score of every match, earn points for getting it right, climb the
> leaderboard. No money — just bragging rights and a reason to watch every game.
> Takes 5 min to fill in the group stage. Get in before the first kickoff 👉 {LINK}

## B — One-liner (paste anywhere)

> Running a World Cup tipping pool — predict scores, earn points, win bragging
> rights. Free and for fun. Join before kickoff: {LINK}

## C — Colleagues / email (a little more, with rules)

> **Subject: World Cup prediction pool — get your picks in before Thursday**
>
> Hi all — the 2026 World Cup starts this week and I've set up a friendly
> score-prediction pool.
>
> **How it works:** predict the exact score of each match. You get **+1** for the
> right home score, **+1** for the right away score, and **+2** for the correct
> result — up to **4 points a game**, and the knockout rounds are worth more.
> There's also a bonus for predicting the group standings order. Your picks lock
> at kickoff, and nobody can see them until then.
>
> No stakes, no money — just office bragging rights and something to follow every
> match for. It takes a few minutes to enter the group stage.
>
> Join here 👉 {LINK} — first match is **Wed June 11**, so get your picks in
> before then.

## D — Hungarian (short, for the magyar crowd)

> ⚽ A héten kezdődik a vébé, csináltam rá egy tippjátékot. Tippeld meg minden
> meccs eredményét, pontot kapsz a találatokért, és gyűjtöd a dicsőséget. *Ha
> kicsi a tét, kedvem sötét* — de most a tét csak a dicsőség, pénz nincs a
> játékban, viszont lesz okod minden meccset végignézni. Pár perc a csoportkört
> kitölteni. Csatlakozz az első sípszó előtt 👉 {LINK}

## If someone balks at "that's a lot of matches"

> You don't predict everything at once — you fill in **one group at a time**, and
> each pick only has to be in before that match kicks off. By the knockouts you've
> watched everyone play, so the picks get quicker and the calls get sharper. It's a
> score-prediction pool, **not fantasy football** — no transfers, no line-ups, no
> daily admin. The rules have barely changed since the first pool back in **2002**,
> so they're well broken-in by now.

## Scoring at a glance (for any custom message)

- Predict the exact score of every match.
- **+1** right home score · **+1** right away score · **+2** correct result
  (win/draw/loss) → up to **4 points per match**.
- Knockout rounds carry higher stage multipliers (R32 ×2 … Final ×6).
- Bonus point for each correctly-ordered pair of teams in a group's standings.
- Picks lock at kickoff; others can't see yours until they lock theirs.

The full, authoritative rules live on the in-app **Rules** page.
