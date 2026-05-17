# TheSportsDB API — Reference (for xpool data ingestion)

Working notes on [TheSportsDB](https://www.thesportsdb.com) — the planned
fixture/result data source for the rewrite. See
[`DATA_SOURCES.md`](./DATA_SOURCES.md) (tournament import & sources).

Primary docs: <https://www.thesportsdb.com/documentation>
Extended docs / OpenAPI specs / Postman collections / MCP spec are linked from
that page. **There is no official SDK** — use the OpenAPI spec to generate a
client, or just call the JSON endpoints over plain HTTP.

---

## 1. Accounts, keys, tiers

| Tier | Price | Rate limit | Unlocks |
|---|---|---|---|
| Free | $0 — test key `123` | 30 req/min (`429` over limit) | V1 only, limited result counts |
| Premium | ~$9/mo (Patreon) | 100 req/min | **V2 API, livescores, video highlights**, higher result caps |
| Business | higher | 120 req/min | as above |

The premium key appears in the user's profile after upgrade.
**Never commit the key** — load it from an env var (`THESPORTSDB_API_KEY`),
per the project security rules.

## 2. Two API versions

| | V1 | V2 |
|---|---|---|
| Base URL | `https://www.thesportsdb.com/api/v1/json/{KEY}/` | `https://www.thesportsdb.com/api/v2/json/` |
| Auth | key in the **URL path** | `X-API-KEY: {KEY}` **header** |
| Access | free + premium | **premium only** |
| Style | older PHP-style `*.php?param=` endpoints | modern REST-style `/segment/{id}` paths |
| Format | JSON | JSON (more verbose) |

V1 example: `…/api/v1/json/123/lookupleague.php?id=4429`
V2 example: `GET …/api/v2/json/lookup/league/4429` with header `X-API-KEY: …`

Recommendation for xpool: use **V2** (header auth is cleaner, livescores
included) with V1 as fallback for anything missing from V2 (e.g. league table).

---

## 3. xpool-relevant subset — ingesting FIFA World Cup 26

**Verified:** FIFA World Cup → `idLeague = 4429`, `strSport = Soccer`.
The season parameter for the 2026 edition is **`2026`**. The season *list* also
contains noise entries (`2021-2022`, `2025-2026`, `2026-2027`); ignore the
hyphenated ones and the `strCurrentSeason` field — use plain **`2026`**.
`/schedule/league/4429/2026` returns **72 fixtures, all in rounds 1–3** (the
group stage); the 32 knockout matches (M73+) are not yet published.

| Need | V1 | V2 |
|---|---|---|
| League metadata | `lookupleague.php?id=4429` | `/lookup/league/4429` |
| Available seasons | `search_all_seasons.php?id=4429` | `/list/seasons/4429` |
| All teams in tournament | `search_all_teams.php?l=FIFA_World_Cup` | `/list/teams/4429` |
| **Full fixture list** (tournament import) | `eventsseason.php?id=4429&s=2026` | `/schedule/league/4429/2026` |
| Group standings table | `lookuptable.php?l=4429&s=2026` | *(no V2 equivalent — use V1)* |
| Single match + result | `lookupevent.php?id={id}` | `/lookup/event/{id}` |
| Matches on a date ("Today" screen) | `eventsday.php?d=2026-06-11&l=4429` | *(use V1, or per-league next/prev)* |
| Upcoming / recent league matches | `eventsnextleague.php?id=4429` / `eventspastleague.php?id=4429` | `/schedule/next/league/4429` / `/schedule/previous/league/4429` |
| Live scores (premium) | — | `/livescore/4429` (league) · `/livescore/Soccer` · `/livescore/all` |

The single highest-value call is **`eventsseason` / `/schedule/league`** — it
returns every fixture for the tournament and is the source for `xpool`'s
declarative `tournaments/fwc26.json`.

### Caveats for the tournament-tree import

- **No bracket dependencies, and knockout fixtures arrive late.** Confirmed:
  the 2026 schedule currently exposes only the 72 group-stage matches. Knockout
  fixtures appear later, and events never carry "winner of match 53"
  references — they get real `idHomeTeam`/`idAwayTeam` once known. `xpool`'s
  admin still resolves the bracket, or a job re-fetches as teams firm up.
- **Stage is encoded in `intRound`** (group rounds vs. knockout round codes)
  and `strSeason` — map these to the `GroupGame` tree explicitly.
- For the FWC26-specific structure (12 groups, R32, third-placed lookup), the
  authoritative source is [`FWC26_RULES.md`](./FWC26_RULES.md), not the API.

---

## 4. Full V1 endpoint list

Base: `https://www.thesportsdb.com/api/v1/json/{KEY}/` · counts shown as free/premium result caps.

**Search:** `searchteams.php?t=` · `searchplayers.php?p=` ·
`searchevents.php?e=&s=&d=&f=` · `searchfilename.php?e=&s=` · `searchvenues.php?v=`

**Lookup (single id):** `lookupleague.php?id=` · `lookupteam.php?id=` ·
`lookupplayer.php?id=` · `lookupevent.php?id=` · `lookupvenue.php?id=` ·
`lookuptable.php?l=&s=` · `lookuplineup.php?id=` · `lookuptimeline.php?id=` ·
`lookupeventstats.php?id=` · `lookuptv.php?id=` · `eventresults.php?id=` ·
`lookupequipment.php?id=` · `lookuphonours.php?id=` · `lookupformerteams.php?id=` ·
`lookupmilestones.php?id=` · `lookupcontracts.php?id=` · `playerresults.php?id=`

**Lists:** `all_sports.php` · `all_countries.php` · `all_leagues.php`
(curated subset only) · `search_all_leagues.php?c=&s=` ·
`search_all_seasons.php?id=&poster=1&badge=1&description=1` ·
`search_all_teams.php?l=&s=&c=` · `lookup_all_players.php?id=`

**Schedule:** `eventsnext.php?id=` · `eventslast.php?id=` ·
`eventsnextleague.php?id=` · `eventspastleague.php?id=` ·
`eventsseason.php?id=&s=` · `eventsday.php?d=&s=&l=` · `eventstv.php?d=&s=&a=&c=&id=`

**Video:** `eventshighlights.php?d=&l=&s=` (premium)

## 5. Full V2 endpoint list

Base: `https://www.thesportsdb.com/api/v2/json/` · header `X-API-KEY` · premium only.

**Search:** `/search/league/{name}` · `/search/team/{name}` ·
`/search/player/{name}` · `/search/event/{name}` · `/search/venue/{name}`

**Lookup:** `/lookup/league/{id}` · `/lookup/team/{id}` · `/lookup/player/{id}` ·
`/lookup/event/{id}` · `/lookup/venue/{id}` · `/lookup/team_equipment/{id}` ·
`/lookup/player_contracts/{id}` · `/lookup/player_results/{id}` ·
`/lookup/player_honours/{id}` · `/lookup/player_milestones/{id}` ·
`/lookup/player_teams/{id}` · `/lookup/event_lineup/{id}` ·
`/lookup/event_results/{id}` · `/lookup/event_stats/{id}` ·
`/lookup/event_timeline/{id}` · `/lookup/event_tv/{id}` · `/lookup/event_highlights/{id}`

**List:** `/list/teams/{leagueId}` · `/list/seasons/{leagueId}` · `/list/players/{teamId}`

**All:** `/all/countries` · `/all/sports` · `/all/leagues`

**Schedule:** `/schedule/next/league/{id}` · `/schedule/previous/league/{id}` ·
`/schedule/next/team/{id}` · `/schedule/previous/team/{id}` ·
`/schedule/next/venue/{id}` · `/schedule/previous/venue/{id}` ·
`/schedule/full/team/{id}` · `/schedule/league/{id}/{season}`

**Filter (TV):** `/filter/tv/day/{date}` · `/filter/tv/country/{country}` ·
`/filter/tv/sport/{sport}` · `/filter/tv/channel/{channel}` · `/filter/tv/channelid/{id}`

**Livescore (premium):** `/livescore/{sport}` · `/livescore/{leagueId}` · `/livescore/all`

---

## 6. Response shape

**The two versions wrap results differently — this matters when writing the client:**

- **V1** keys the top-level array by **entity type** — `leagues`, `teams`,
  `events`, `table`, `players`, `seasons`, `countries`. `null` (not `[]`) when
  nothing matches.
- **V2** keys it by **operation type** — `lookup`, `list`, `schedule`,
  `livescore` — regardless of entity. So `/lookup/league/4429` → `.lookup[]`,
  `/list/seasons/4429` → `.list[]`, `/schedule/league/...` → `.schedule[]`,
  `/livescore/Soccer` → `.livescore[]`. (Verified live.)

Useful **event** fields for xpool: `idEvent`, `strEvent`, `idLeague`,
`strSeason`, `intRound`, `dateEvent`, `strTime` / `strTimestamp`,
`idHomeTeam` / `idAwayTeam`, `strHomeTeam` / `strAwayTeam`,
`intHomeScore` / `intAwayScore`, `strStatus`, `strVenue` / `idVenue`.

Images come in several sizes via URL suffix: original (~720px), `/medium`
(500px), `/small` (250px), `/tiny` (50px) — relevant for team flags/badges.

---

## 7. References

- API documentation index — <https://www.thesportsdb.com/documentation>
- Free test key: `123` (V1 only; low result caps)
- Premium / Patreon (V2, livescores, highlights) — linked from the docs page
- OpenAPI specs, Postman collections, MCP server spec — linked from the docs page

*Verified live on 2026-05-16: V1 (free key `123`) and V2 (premium key, header
auth). Confirmed `idLeague 4429`, season `2026` returns 72 group-stage
fixtures, V2 envelope is operation-keyed (`.lookup`/`.list`/`.schedule`/
`.livescore`), and `/livescore/Soccer` is live.*
