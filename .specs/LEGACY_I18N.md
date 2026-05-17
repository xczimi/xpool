# xpool — Legacy UI Strings (English / Hungarian)

Stage 1 of the i18n reconciliation: every translatable string from the legacy
app, extracted **verbatim**, so the rewrite's `web/src/i18n/strings.ts` can be
made consistent with the original wording (Stage 2).

## Provenance

- **Primary:** the gettext catalogs `archive/locale/en/LC_MESSAGES/django.po`
  and `archive/locale/hu/LC_MESSAGES/django.po` — `PO-Revision-Date 2012-06-06`,
  translator Peter Czimmermann. 49 message entries, fully aligned.
- **Secondary:** hardcoded English text scanned out of `archive/view/*.html`
  that was never added to the catalog (see [§13](#13-untranslated--hardcoded-template-strings)).

## Notes on the original

- **Branding drift.** The app was rebranded across tournaments — `xHomePool`,
  `xEuroPool`, `xHomePool, eh!`, `EB-Tipp`. The catalog is inconsistent: e.g.
  the English of *"xHomePool invitation"* is *"xEuroPool invitation"*; the
  error-page strings are crossed (see §11). Verbatim values are kept below;
  the rewrite should settle on **xpool**.
- The English column is the gettext **msgid** (the source string). Where the
  English `.po`'s `msgstr` actually displayed something *different* from the
  msgid, it is flagged **[en shown: …]**.
- Tone: the Hungarian is deliberately casual/slangy (*"már benne van a
  tutiban"*, *"Ne csalj, Boborján!"*, *"akkor ők kapják is be"*). Preserve it.

---

## 1. Flash messages (`control.py`)

| English | Hungarian |
|---|---|
| Sucessful google login *(sic — typo in original)* | Sikeres google belépés |
| Successful google logout | Sikeres google kilépés |
| Successful logout | Sikeres kilépés |
| Failed to log you in, try it again! | Belépés nem sikerült, próbáld újra! |
| Successful login | Sikeres belépés |
| Invalid auth code | Helytelen kód |
| Successful referral, set your password, or link your google account! | Sikeres belépés, állíts be jelszót! |
| Login required | Belépés szükséges |
| This user is already in the system (based on email)! | Ez a felhasználó már benne van a tutiban! |
| xHomePool invitation **[en shown: xEuroPool invitation]** | xHomePool meghívó |
| Invitation sent | Meghívó elküldve. |

## 2. Auth / user line (`userline.html`)

| English | Hungarian |
|---|---|
| Hi %(user)s! You are inside.&nbsp; | Helló %(user)s! benn vagy! |
| get me outside! | Engedj ki! |
| Google linked to | Google link:&nbsp; |
| You are outside. | Kint vagy. |
| let me in! | engedj be! |

## 3. Navigation menu (`menuline.html`)

| English | Hungarian |
|---|---|
| Fresh | Aktuális |
| Schedule | Menetrend |
| Bets | Tippjeim |
| All bets | Összes tipp |
| Scoreboard | Tippverseny |
| Profile | Adatok |
| Invite | Meghívó |
| Rules | Szabályok |
| Admin | Admin |

## 4. Profile (`profile.html`)

| English | Hungarian |
|---|---|
| Profile | Adatok |
| E-mail | E-mail |
| Nick | Becenév |
| Full name | Teljes név |
| New password | Új jelszó |
| Password again | Jelszó újra |
| do it! | csináld! |

## 5. Invite (`invite.html`)

| English | Hungarian |
|---|---|
| Invitation | Meghívó |
| E-mail | E-mail |
| Nick | Becenév |
| Full name | Teljes név |
| send the invite which may end up in the spam folder. | küldd el a meghívót, ami lehet hogy spamként végzi... |

## 6. Schedule & My Tips table headers (`games.html`, `mytips.html`)

| English | Hungarian |
|---|---|
| home team | hazai csapat |
| away team | vendég csapat |
| kick off | kezdés |
| Stadium **[en shown: stadium]** | stadion |
| result | eredmény |
| bet (pt) | tipp (pont) |
| save | mentés |

## 7. Layout / chrome (`layout.html`, `footer.html`, `index.html`)

| English | Hungarian |
|---|---|
| xHomePool, eh! | xHomePool, jól! |
| Hi there! | Szevasztok! |
| This is our little friendly pool. | Ez a mi kis baráti tippelő környezetünk |
| language | nyelv |
| Server time | Szerveridő |

## 8. Admin (`admin/menuline.html`, `admin/layout.html`, …)

| English | Hungarian |
|---|---|
| Main | Főoldal |
| Teams | Csapatok |
| Games | Meccsek |
| Users | Felhasználók |
| Fifa | Fifa |
| This is the admin interface. | Ez az admin felület |

## 9. Error page (`error.html`)

The catalog entry is crossed — the **msgid is Hungarian** and the translations
are swapped. Verbatim:

| msgid | English `msgstr` | Hungarian `msgstr` |
|---|---|---|
| `EB-Tipp, rosszul!` | EB-Tipp, rosszul! | xHomePool, fail! |

## 10. Referral invitation email (`referral.email.html`)

**English** (msgid; the English `msgstr` shows `xEuroPool` for `xHomePool`):

```
Hi %(referral.nick)s,

%(user.nick)s ( %(user.full_name)s &lt; %(user.email)s &gt; ) has invited you to xHomePool.

This link will let you in: %(auth_url)s

Up until setting a password, you can use this link to get in.

Good luck!

regards,
Czimi <xczimi@gmail.com>
```

**Hungarian:**

```
Szia %(referral.nick)s!

%(user.nick)s ( %(user.full_name)s &lt; %(user.email)s &gt; ) meghívott az xEuroPool rendszerbe.

Ezzel a linkkel tudod aktiválni magad az oldalon:
%(auth_url)s

A link addig használható, amíg nem állítasz be jelszót magadnak.

Sok sikert!

üdv,
Czimi <xczimi@gmail.com>
```

## 11. Rules page (`rules.html`)

One large HTML blob. The English `msgstr` equals the msgid verbatim.

**English:**

```html
<h1>EC predict the score game</h1>
<h2>1. What the hell?</h2>
This page is doing the administration for a predict-the-score game of some friends (and friends' friends, of course). Players try to guess the scores of the matches of the UEFA Euro 2008, and collect points based on how good they are at it.
<br>
<h2>2. How?</h2>
The exact score of every match is to be predicted, by choosing the appropriate numbers from the drop-down lists. The bet is evaluated based on the score of the game AFTER THE END OF THE SECOND HALF (90 mins). In certain scenarios when a non-draw score is required to decide which team qualifies, then, if the player bets on a tie, the team to qualify (after extra time, penalty shootout etc.) has to be predicted separately. See also the <a href="http://www.uefa.com/newsfiles/19079.pdf">official regulations of the UEFA Euro 2008 (PDF)</a>.

<h2>3. Who wins?</h2>
Well, hopefully all participants. A bit more precisely, who gathers the most points. By every bet, points can be won the following way (home team is the team listed first in a match listing):
<ul>
<li>Predicting Win-Draw-Loss correctly: 2 points.</li>
<li>Guessing the score of the home team correctly: 1 point. (In case the home team scores at least 4 goals in a game, then 1 point is awarded to players predicting at least 4 home goals.)</li>
<li>Guessing the score of the visitor team correctly: 1 point. (In case the visitor team scores at least 4 goals in a game, then 1 point is awarded to players predicting at least 4 visitor goals.)</li>
<li>Bonus:<br>
After every round (group stage, 1/4 finals, 1/2 finals, final) there is on average 1 more point per game to be won based on the following principles:
<br>
For every pair of teams which directly competed aganst each other (were in the same group, or played a knock-out game, respectively) 1 point is awarded to the player if in the rankings based on his/her predictions (in the knock-out stage, there are two possible rank orderings, in te group stage it is 4!=24), the two teams finish in the order as they do based on the results of the UEFA Euro 2008. To see how orders are determined, please consult the official regulations (PDF).
</li></ul>
<h2>4. Disclaimer</h2>
Please don't cheat! We would only like to play a good game, and we prefer to do this against people with similar preferences.
<br>
To avoid strategic betting made in order to preserve one's advantage at later stages of the game, a player cannot watch other players' bets until they finalize it. Except, of course, for the case when more people collude in order to get around this but then they should not have signed up in the first place. Finalizing bets can be done by checking the box adjacent to the drop-downs where one selects the score guessed.
<br>
<h2>5. Participation</h2>
...is based on invitation. The system sends the invitationin email, so an email address is in fact needed to play.

<h2>6. Deadlines</h2>
There are several "betting units" of the tournament:
<ul>
</li><li> 8 group-stage games
</li><li>second 8 group-stage games
</li><li>third 8 group-stage games
</li><li>quarterfinals
</li><li>semifinals
</li><li>final</li>
</ul>
Predictions concerning games of a betting unit can be made prior to 12pm on the day of the first game in the unit. This deadline may change, though - the current deadlines are always posted on the "My Bets" page.
<h2>7. Help,FAQ,whatever</h2>
Please email the admin team (see below).
```

**Hungarian** (the original `msgstr` is a single run-on line — reflowed here
for readability; wording is verbatim):

```html
<h1>EB-Tipp góltotó játék szabályai</h1>
<h2>1. Miaszösz?</h2>
"Ez itt a mi kis tippelő környezetünk." Avagy, ez az oldal egy haveri alapon működő góltotó játékot üzemeltet. A résztvevők a 2008-as foci EB mérkőzéseinek végeredményére tippelnek, és a tipp minőségétől függően pontokat gyűjtenek. Az nyer, aki többet.
<h2>2. De hogy?</h2>
Minden mérkőzés számszerű végeredményét kell megtippelni, a legördülő menükből a megfelelő számokat kiválasztva. A tipp a mérkőzés rendes játékidejének végeredményére szól (90. perc). Speciális esetben, amikor a két résztvevő csapat továbbjutása múlik az eredményen, úgy amennyiben a mérkőzésre döntetlent tippel a játékos, úgy külön kell tippelni a továbbjutóra, azaz a hosszabbítás, ill. büntetőrúgások győztesére. Lásd még az <a href="http://www.uefa.com/newsfiles/19079.pdf">EB hivatalos szabálykönyvét (PDF)</a>
<h2>3. Ki nyer ma?</h2>
Az értékelés a végeredmény függvényében a következő tételek szuperpozíciójából áll (hazai csapatként az elöl listázott csapat értendő):
<ul>
<li>1-x-2 eltalálása (a csapatok által elért pont): 2 pont.</li>
<li>Hazai lőtt gól eltalálása: 1 pont. (Amennyiben a hazai csapat legalább 4 gólt ér el a mérkőzésen, úgy az 1 pont akkor jár, ha a játékos is legalább 4 hazai gólt tippelt.)</li>
<li>Vendég lőtt gól eltalálása: 1 pont. (Amennyiben a hazai csapat legalább 4 gólt ér el a mérkőzésen, úgy az 1 pont akkor jár, ha a játékos is legalább 4 hazai gólt tippelt.)</li>
<li>Bónusz:<br>
A megfelelő fordulók (csoportmérkőzések, negyeddöntők, elődöntők, döntő) végén mérkőzésenként átlagban 1 pont szerezhető a következő elv alapján: Minden olyan csapat-párra, melyek egymás ellen versenyeztek (egy csoportban voltak, ill. egymás ellen játszottak a legjobb 8 között) akkor jár pont egy játékosnak, ha a tippjei alapján kialakuló sorrendben (a 8 között ez kétféle lehet, a csoportmeccseken csoportonként 4!=24) a két csapat olyan sorrendben végez, amilyenben az EB tényleges eredményei alapján. A sorrend eldöntéséhez lásd az EB hivatalos szabálykönyvét (PDF).</ul>
<h2>4. Disclaimer</h2>
Ne csalj, Boborján! Mi elsősorban játszani szeretnénk, olyanok ellen, akik hasonló céllal vannak az oldalon.<br>
(Hogy ne lehessen az előny megőrzésére játszani, a saját tipp véglegesítéséig az ellenfeleké sem tekinthető meg. Kivétel, ha többen összedolgoznak, de akkor ők kapják is be.)
<h2>5. Részvétel a játékban</h2>
A játékban részt venni meghívásos alapon lehet. A meghívót e-mailben küldi a rendszer, ezért kell hozzá egy ilyen cím.
<h2>6. Határidők</h2>
A torna a következő tippelési egységekre bomlik:
<ul><li>első 8 csoportmeccs</li><li>második 8 csoportmeccs</li><li>harmadik 8 csoportmeccs</li><li>negyeddöntők</li><li>elődöntők</li><li>döntő</li></ul>
Egy egység mérkőzéseire az első találkozó napján déli 12 óráig lehet tippelni (szerveridő szerint). Ez azonban módosulhat, az aktuális határidők megtalálhatóak a tippjeim oldalon.
<h2>7. Műsorváltozás, kezelőfelület, GyIK</h2>
Tessék írni az adminisztrátornak (cím alant)
```

> ⚠️ The rules text is **stale** — it describes *UEFA Euro 2008*, the wrong
> scoring multipliers, and "betting units" rather than the rewrite's model.
> For Stage 2 it is a *tone/wording* reference only; the actual rules copy must
> follow [`SCORING.md`](./SCORING.md) and [`GAME_RULES.md`](./GAME_RULES.md).

## 12. Stale-content warnings

The legacy strings reference tournaments and rules that no longer apply:
*UEFA Euro 2008*, *foci EB*, the 2008 regulations PDF, "1/4 / 1/2 / final"
multipliers. These are **wording references**, not content to copy — the
rewrite targets FWC26.

## 13. Untranslated / hardcoded template strings

English-only text found in templates, **never added to the catalog** — no
Hungarian exists. Stage 2 must translate these fresh.

| Source | English |
|---|---|
| `alltips.html` | Ranking |
| `mytips.html` | LOCK your predictions before the deadline! |
| `mytips.html` | Use the checkboxes to lock your predictions. |
| `mytips.html` | The list of teams in the Bet column shows the group standings based on your predictions. |
| `mytips.html` | The list of teams in the Result column shows the group standings based on the results. |
| `mytips.html` | For completeness (and to confuse everybody) : in case your predictions result in a tiebreak which would be resolved by a draw, you need to predict the result of the draw. - for details of the tiebreaking procedure refer to the PDF linked from the rules page |

---

*Stage 2: reconcile `web/src/i18n/strings.ts` against this file — reuse the
original Hungarian wording verbatim where a matching string exists; keep the
casual register; do not carry over the stale Euro-2008 rules content.*
