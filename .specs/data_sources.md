# xpool — Data Sources

How the rewrite gets tournament data: fixtures, kickoff times, the knockout
bracket, and official results. See [`REWRITE_IMPLEMENTATION.md` §3](./REWRITE_IMPLEMENTATION.md)
(data ingestion redesign) and [`thesportsdb_api.md`](./thesportsdb_api.md).

The legacy app scraped cached FIFA/UEFA HTML. The rewrite drops scraping in
favour of a **declarative tournament definition** (`tournaments/fwc26.json`)
generated from the sources below, with [`fwc26_rules.md`](./fwc26_rules.md) as
the structural authority.

---

## 1. Source comparison

| | FotMob ICS | TheSportsDB | `fwc26_rules.md` |
|---|---|---|---|
| Type | iCalendar feed | JSON API (V1/V2) | static spec doc |
| Fixtures | **all 104** (M1–M104) | 72 (group stage only, so far) | full pairings |
| Kickoff times | yes (UTC) | yes | no |
| Knockout bracket | placeholders, pre-encoded | not published yet | full (§4, Annexe C) |
| Group structure | positional only | `intRound` only | **authoritative** |
| Official results | **no** | yes (livescore / event lookup) | no |
| Venues / team IDs / images | no | yes | no |
| Auth | none (public) | premium key for V2 | n/a |
| Stability | undocumented endpoint | documented API | in-repo |

**Roles:**

| Concern | Source |
|---|---|
| Fixture list, kickoff times, bracket scaffold | **FotMob ICS** |
| Group/tree structure, tiebreakers, Annexe C lookup | **`fwc26_rules.md`** |
| Official results, livescores, venues, badges | **TheSportsDB V2** (or admin entry) |

---

## 2. FotMob ICS calendar feed

```
webcal://pub.fotmob.com/prod/pub/api/v2/calendar/league/77.ics
https://pub.fotmob.com/prod/pub/api/v2/calendar/league/77.ics
```

`league/77` = FIFA World Cup. Public, no key. Standard iCalendar
(`VCALENDAR` / `VEVENT`), ~37 KB, 6-hour refresh hint.

**Verified (2026-05-16):** 104 `VEVENT`s, in match order **M1 → M104**, so the
event's position in the file is its match number. Date range
2026-06-11 → 2026-07-19.

Per-event fields used:

| iCal field | Use |
|---|---|
| `DTSTART` / `DTEND` | kickoff / end, UTC |
| `SUMMARY` | the two teams (see below) |
| `UID` | stable id (`<fotmobMatchId>@fotmob.com`) |
| `URL` | FotMob match page |

`SUMMARY` parsing — split on `" - "`:

- Group matches → real names: `🇲🇽 Mexico - 🇿🇦 South Africa`
- Knockout matches → placeholders in **`fwc26_rules.md` notation**:
  `2A - 2B` (M73), `1E - 3ABCDF` (M74), `1L - 3EHIJK` (M80),
  `Winner SF 1 - Winner SF 2` (M104).
  `3ABCDF` = "best 3rd-placed team from groups {A,B,C,D,F}" — §4/§5.
- A leading emoji (flag, or `⚽️` for knockout) is **inconsistent** — strip
  it; rely on the `" - "` separator, not the emoji.

**Limitations:** no scores/results (fixtures only), no group labels for group
matches, no venues / team IDs / flag images. Undocumented endpoint — treat as
best-effort and validate on import.

## 3. TheSportsDB

Full reference in [`thesportsdb_api.md`](./thesportsdb_api.md). For xpool:
official results, livescores, venues, and team badges. FIFA World Cup is
`idLeague 4429`, season `2026`. V2 needs the premium key (`X-API-KEY` header).

## 4. `fwc26_rules.md`

The in-repo competition rules: 12-group structure, the MD1–MD3 group schedule,
group-stage tiebreakers, the knockout bracket (§4), and the 495-row Annexe C
third-placed-team lookup (§5). This is the **authority** when a live source is
ambiguous or incomplete — it tells the importer which group each of M1–M72
belongs to, and how to resolve knockout placeholders once group results land.

---

## 5. Ingestion flow

1. Fetch the FotMob ICS → 104 events, ordered M1–M104.
2. Map each event to its stage/group **positionally** against `fwc26_rules.md`
   (M1–M72 = group stage per the §1.3 schedule; M73–M104 = knockout per §4).
3. Parse `SUMMARY`: group matches → team names; knockout matches → keep the
   `fwc26_rules.md` placeholder string as an unresolved bracket reference.
4. Emit `tournaments/fwc26.json` — the declarative definition: the `GroupGame`
   tree, 104 `SingleGame`s with kickoff times, and explicit knockout references.
5. Cross-check the generated file against `fwc26_rules.md` (104 matches,
   12 groups × 6, bracket references valid) — fail the import on mismatch.
6. During the tournament: admin enters official results (or a TheSportsDB
   results job feeds them); a job resolves knockout placeholders to real teams
   as group standings finalise.

---

*Verified live 2026-05-16: FotMob ICS `league/77` — 104 events, M1–M104 order,
knockout placeholders in `fwc26_rules.md` notation.*
