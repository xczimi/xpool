# FIFA World Cup 26 — Competition Rules (Implementation Reference)

Extracted from *Regulations for the FIFA World Cup 26™* (May 2026 edition). Scope: only the rules a prediction tool needs — tournament structure, group stage, tiebreakers, knockout bracket logic, extra time, and the third-placed-team lookup table.

Article numbers refer to the source document.

---

## 1. Tournament structure

- **Dates:** 11 June – 19 July 2026 (Art. 1.5).
- **Hosts:** Canada, Mexico, USA — all qualify automatically (Art. 11.1, 11.4).
- **Total teams:** 48.
- **Format:** group stage → Round of 32 → Round of 16 → Quarter-finals → Semi-finals → Third-place play-off + Final (Art. 12.1).
- **Total matches:** 104 (M1–M72 group stage, M73–M88 R32, M89–M96 R16, M97–M100 QF, M101–M102 SF, M103 third-place, M104 final).

### Group structure (Art. 12.2–12.4)

- 12 groups (A–L) of 4 teams each.
- Each team plays the other 3 once.
- **Points:** Win = 3, Draw = 1, Loss = 0.
- Host positions are fixed:
  - **A1** = Mexico
  - **B1** = Canada
  - **D1** = USA
- Last two group matches kick off simultaneously (Art. 12.4, 16.3).

### Group match schedule (Art. 12.4)

For each group, three matchdays:

| Matchday | Match 1 | Match 2 |
|---|---|---|
| MD1 | X1 v X2 | X3 v X4 |
| MD2 | X1 v X3 | X4 v X2 |
| MD3 | X4 v X1 | X2 v X3 |

(Chronological order may differ; what's fixed is the pairings.)

### Advancing to knockout (Art. 12.5)

32 teams advance to the Round of 32:
- 12 group winners (1st place in each group)
- 12 group runners-up (2nd place in each group)
- 8 best third-placed teams (out of 12)

---

## 2. Group stage tiebreakers (Art. 13)

### Within a single group — when two or more teams are level on points

Apply the criteria **in order**, stopping as soon as the tie is broken.

#### Step 1 — Head-to-head between tied teams

a. Greatest **points** in matches between the tied teams
b. Superior **goal difference** in matches between the tied teams
c. Greatest **goals scored** in matches between the tied teams

#### Step 2 — If still tied after Step 1

If applying (a)–(c) to head-to-head matches doesn't resolve, re-apply (a)–(c) **to matches between the still-tied teams only** (i.e., drop any team already separated). If still tied:

d. Superior **goal difference** in all group matches
e. Greatest **goals scored** in all group matches
f. **Highest team conduct score** (see below)

> **Important:** During Step 2, ranking does **not** restart when one team is separated. Continue applying the remaining criteria to the remaining tied teams.

#### Step 3 — If still tied

g. Most recent FIFA/Coca-Cola Men's World Ranking
h. Preceding editions of the same ranking, continuing back until resolved

#### Team conduct score (criterion f)

Each card incurs negative points; **only one deduction per player per match** (use the most severe).

| Event | Points |
|---|---|
| Yellow card | −1 |
| Indirect red (two yellows in same match) | −3 |
| Direct red | −4 |
| Yellow + direct red (same match) | −5 |

Higher (less negative) team total ranks higher.

---

## 3. Ranking the 8 best third-placed teams (Art. 13)

After all 12 groups finish, the 12 teams in 3rd place are ranked against each other. The top 8 advance.

Apply in order:

a. Most **points** in all group matches
b. Superior **goal difference** in all group matches
c. Greatest **goals scored** in all group matches
d. Highest **team conduct score** (calculated as in §2)
e. Most recent FIFA/Coca-Cola Men's World Ranking
f. Preceding editions of the ranking, continuing back until resolved

The 8 qualifying third-placed teams are labeled **3rd-1 through 3rd-8** by this ranking, but for bracketing they are referenced by **group letter** (e.g., "3A" = the third-placed team from Group A).

---

## 4. Knockout bracket

### Round of 32 (Art. 12.6)

The 16 R32 matches (M73–M88). 4 group winners (C, F, H, J) face runners-up. 8 group winners (A, B, D, E, G, I, K, L) face a third-placed team — **which** third-placed team depends on which 8 groups produced the qualifying 3rd-placed teams (see §5, Annexe C lookup).

| # | Match | Team A | Team B |
|---|---|---|---|
| 1 | M73 | Runner-up A (2A) | Runner-up B (2B) |
| 2 | M74 | Winner E (1E) | Best 3rd from {A,B,C,D,F} |
| 3 | M75 | Winner F (1F) | Runner-up C (2C) |
| 4 | M76 | Winner C (1C) | Runner-up F (2F) |
| 5 | M77 | Winner I (1I) | Best 3rd from {C,D,F,G,H} |
| 6 | M78 | Runner-up E (2E) | Runner-up I (2I) |
| 7 | M79 | Winner A (1A) | Best 3rd from {C,E,F,H,I} |
| 8 | M80 | Winner L (1L) | Best 3rd from {E,H,I,J,K} |
| 9 | M81 | Winner D (1D) | Best 3rd from {B,E,F,I,J} |
| 10 | M82 | Winner G (1G) | Best 3rd from {A,E,H,I,J} |
| 11 | M83 | Runner-up K (2K) | Runner-up L (2L) |
| 12 | M84 | Winner H (1H) | Runner-up J (2J) |
| 13 | M85 | Winner B (1B) | Best 3rd from {E,F,G,I,J} |
| 14 | M86 | Winner J (1J) | Runner-up H (2H) |
| 15 | M87 | Winner K (1K) | Best 3rd from {D,E,I,J,L} |
| 16 | M88 | Runner-up D (2D) | Runner-up G (2G) |

**Rule:** Teams from the same group cannot meet in R32. The Annexe C lookup table (§5) enforces this for the 8 third-placed teams.

### Round of 16 (Art. 12.7)

| # | Match | Team A | Team B |
|---|---|---|---|
| 17 | M89 | Winner M74 | Winner M77 |
| 18 | M90 | Winner M73 | Winner M75 |
| 19 | M91 | Winner M76 | Winner M78 |
| 20 | M92 | Winner M79 | Winner M80 |
| 21 | M93 | Winner M83 | Winner M84 |
| 22 | M94 | Winner M81 | Winner M82 |
| 23 | M95 | Winner M86 | Winner M88 |
| 24 | M96 | Winner M85 | Winner M87 |

### Quarter-finals (Art. 12.8)

| # | Match | Team A | Team B |
|---|---|---|---|
| A | M97 | Winner M89 | Winner M90 |
| B | M98 | Winner M93 | Winner M94 |
| C | M99 | Winner M91 | Winner M92 |
| D | M100 | Winner M95 | Winner M96 |

### Semi-finals (Art. 12.9)

| # | Match | Team A | Team B |
|---|---|---|---|
| SF1 | M101 | Winner M97 | Winner M98 |
| SF2 | M102 | Winner M99 | Winner M100 |

### Third-place play-off (Art. 12.10)

| Match | Team A | Team B |
|---|---|---|
| M103 | Loser M101 | Loser M102 |

### Final (Art. 12.11)

| Match | Team A | Team B |
|---|---|---|
| M104 | Winner M101 | Winner M102 |

---

## 5. Annexe C — Third-placed teams lookup (495 combinations)

This table maps **which 8 group winners** receive **which third-placed team** in the Round of 32, based on which 8 of the 12 groups produced the qualifying third-placed teams.

### How to use it

1. After the group stage, determine the 8 groups whose 3rd-placed team qualified (see §3).
2. Sort the qualifying group letters alphabetically — this is your "input combination."
3. Find the row in the table whose set of 8 group letters (read across columns 3X) matches your combination.
4. The row's columns 1A, 1B, 1D, 1E, 1G, 1I, 1K, 1L tell you which 3X team each of those 8 group winners plays.

The 8 group winners that always face a third-placed team are: **1A, 1B, 1D, 1E, 1G, 1I, 1K, 1L** (the column headers). The other 4 group winners (1C, 1F, 1H, 1J) always face a runner-up and are not in this table.

| Option | 1A | 1B | 1D | 1E | 1G | 1I | 1K | 1L |
|---|---|---|---|---|---|---|---|---|
| 1 | 3E | 3J | 3I | 3F | 3H | 3G | 3L | 3K |
| 2 | 3H | 3G | 3I | 3D | 3J | 3F | 3L | 3K |
| 3 | 3E | 3J | 3I | 3D | 3H | 3G | 3L | 3K |
| 4 | 3E | 3J | 3I | 3D | 3H | 3F | 3L | 3K |
| 5 | 3E | 3G | 3I | 3D | 3J | 3F | 3L | 3K |
| 6 | 3E | 3G | 3J | 3D | 3H | 3F | 3L | 3K |
| 7 | 3E | 3G | 3I | 3D | 3H | 3F | 3L | 3K |
| 8 | 3E | 3G | 3J | 3D | 3H | 3F | 3L | 3I |
| 9 | 3E | 3G | 3J | 3D | 3H | 3F | 3I | 3K |
| 10 | 3H | 3G | 3I | 3C | 3J | 3F | 3L | 3K |
| 11 | 3E | 3J | 3I | 3C | 3H | 3G | 3L | 3K |
| 12 | 3E | 3J | 3I | 3C | 3H | 3F | 3L | 3K |
| 13 | 3E | 3G | 3I | 3C | 3J | 3F | 3L | 3K |
| 14 | 3E | 3G | 3J | 3C | 3H | 3F | 3L | 3K |
| 15 | 3E | 3G | 3I | 3C | 3H | 3F | 3L | 3K |
| 16 | 3E | 3G | 3J | 3C | 3H | 3F | 3L | 3I |
| 17 | 3E | 3G | 3J | 3C | 3H | 3F | 3I | 3K |
| 18 | 3H | 3G | 3I | 3C | 3J | 3D | 3L | 3K |
| 19 | 3C | 3J | 3I | 3D | 3H | 3F | 3L | 3K |
| 20 | 3C | 3G | 3I | 3D | 3J | 3F | 3L | 3K |
| 21 | 3C | 3G | 3J | 3D | 3H | 3F | 3L | 3K |
| 22 | 3C | 3G | 3I | 3D | 3H | 3F | 3L | 3K |
| 23 | 3C | 3G | 3J | 3D | 3H | 3F | 3L | 3I |
| 24 | 3C | 3G | 3J | 3D | 3H | 3F | 3I | 3K |
| 25 | 3E | 3J | 3I | 3C | 3H | 3D | 3L | 3K |
| 26 | 3E | 3G | 3I | 3C | 3J | 3D | 3L | 3K |
| 27 | 3E | 3G | 3J | 3C | 3H | 3D | 3L | 3K |
| 28 | 3E | 3G | 3I | 3C | 3H | 3D | 3L | 3K |
| 29 | 3E | 3G | 3J | 3C | 3H | 3D | 3L | 3I |
| 30 | 3E | 3G | 3J | 3C | 3H | 3D | 3I | 3K |
| 31 | 3C | 3J | 3E | 3D | 3I | 3F | 3L | 3K |
| 32 | 3C | 3J | 3E | 3D | 3H | 3F | 3L | 3K |
| 33 | 3C | 3E | 3I | 3D | 3H | 3F | 3L | 3K |
| 34 | 3C | 3J | 3E | 3D | 3H | 3F | 3L | 3I |
| 35 | 3C | 3J | 3E | 3D | 3H | 3F | 3I | 3K |
| 36 | 3C | 3G | 3E | 3D | 3J | 3F | 3L | 3K |
| 37 | 3C | 3G | 3E | 3D | 3I | 3F | 3L | 3K |
| 38 | 3C | 3G | 3E | 3D | 3J | 3F | 3L | 3I |
| 39 | 3C | 3G | 3E | 3D | 3J | 3F | 3I | 3K |
| 40 | 3C | 3G | 3E | 3D | 3H | 3F | 3L | 3K |
| 41 | 3C | 3G | 3J | 3D | 3H | 3F | 3L | 3E |
| 42 | 3C | 3G | 3J | 3D | 3H | 3F | 3E | 3K |
| 43 | 3C | 3G | 3E | 3D | 3H | 3F | 3L | 3I |
| 44 | 3C | 3G | 3E | 3D | 3H | 3F | 3I | 3K |
| 45 | 3C | 3G | 3J | 3D | 3H | 3F | 3E | 3I |
| 46 | 3H | 3J | 3B | 3F | 3I | 3G | 3L | 3K |
| 47 | 3E | 3J | 3I | 3B | 3H | 3G | 3L | 3K |
| 48 | 3E | 3J | 3B | 3F | 3I | 3H | 3L | 3K |
| 49 | 3E | 3J | 3B | 3F | 3I | 3G | 3L | 3K |
| 50 | 3E | 3J | 3B | 3F | 3H | 3G | 3L | 3K |
| 51 | 3E | 3G | 3B | 3F | 3I | 3H | 3L | 3K |
| 52 | 3E | 3J | 3B | 3F | 3H | 3G | 3L | 3I |
| 53 | 3E | 3J | 3B | 3F | 3H | 3G | 3I | 3K |
| 54 | 3H | 3J | 3B | 3D | 3I | 3G | 3L | 3K |
| 55 | 3H | 3J | 3B | 3D | 3I | 3F | 3L | 3K |
| 56 | 3I | 3G | 3B | 3D | 3J | 3F | 3L | 3K |
| 57 | 3H | 3G | 3B | 3D | 3J | 3F | 3L | 3K |
| 58 | 3H | 3G | 3B | 3D | 3I | 3F | 3L | 3K |
| 59 | 3H | 3G | 3B | 3D | 3J | 3F | 3L | 3I |
| 60 | 3H | 3G | 3B | 3D | 3J | 3F | 3I | 3K |
| 61 | 3E | 3J | 3B | 3D | 3I | 3H | 3L | 3K |
| 62 | 3E | 3J | 3B | 3D | 3I | 3G | 3L | 3K |
| 63 | 3E | 3J | 3B | 3D | 3H | 3G | 3L | 3K |
| 64 | 3E | 3G | 3B | 3D | 3I | 3H | 3L | 3K |
| 65 | 3E | 3J | 3B | 3D | 3H | 3G | 3L | 3I |
| 66 | 3E | 3J | 3B | 3D | 3H | 3G | 3I | 3K |
| 67 | 3E | 3J | 3B | 3D | 3I | 3F | 3L | 3K |
| 68 | 3E | 3J | 3B | 3D | 3H | 3F | 3L | 3K |
| 69 | 3E | 3I | 3B | 3D | 3H | 3F | 3L | 3K |
| 70 | 3E | 3J | 3B | 3D | 3H | 3F | 3L | 3I |
| 71 | 3E | 3J | 3B | 3D | 3H | 3F | 3I | 3K |
| 72 | 3E | 3G | 3B | 3D | 3J | 3F | 3L | 3K |
| 73 | 3E | 3G | 3B | 3D | 3I | 3F | 3L | 3K |
| 74 | 3E | 3G | 3B | 3D | 3J | 3F | 3L | 3I |
| 75 | 3E | 3G | 3B | 3D | 3J | 3F | 3I | 3K |
| 76 | 3E | 3G | 3B | 3D | 3H | 3F | 3L | 3K |
| 77 | 3H | 3G | 3B | 3D | 3J | 3F | 3L | 3E |
| 78 | 3H | 3G | 3B | 3D | 3J | 3F | 3E | 3K |
| 79 | 3E | 3G | 3B | 3D | 3H | 3F | 3L | 3I |
| 80 | 3E | 3G | 3B | 3D | 3H | 3F | 3I | 3K |
| 81 | 3H | 3G | 3B | 3D | 3J | 3F | 3E | 3I |
| 82 | 3H | 3J | 3B | 3C | 3I | 3G | 3L | 3K |
| 83 | 3H | 3J | 3B | 3C | 3I | 3F | 3L | 3K |
| 84 | 3I | 3G | 3B | 3C | 3J | 3F | 3L | 3K |
| 85 | 3H | 3G | 3B | 3C | 3J | 3F | 3L | 3K |
| 86 | 3H | 3G | 3B | 3C | 3I | 3F | 3L | 3K |
| 87 | 3H | 3G | 3B | 3C | 3J | 3F | 3L | 3I |
| 88 | 3H | 3G | 3B | 3C | 3J | 3F | 3I | 3K |
| 89 | 3E | 3J | 3B | 3C | 3I | 3H | 3L | 3K |
| 90 | 3E | 3J | 3B | 3C | 3I | 3G | 3L | 3K |
| 91 | 3E | 3J | 3B | 3C | 3H | 3G | 3L | 3K |
| 92 | 3E | 3G | 3B | 3C | 3I | 3H | 3L | 3K |
| 93 | 3E | 3J | 3B | 3C | 3H | 3G | 3L | 3I |
| 94 | 3E | 3J | 3B | 3C | 3H | 3G | 3I | 3K |
| 95 | 3E | 3J | 3B | 3C | 3I | 3F | 3L | 3K |
| 96 | 3E | 3J | 3B | 3C | 3H | 3F | 3L | 3K |
| 97 | 3E | 3I | 3B | 3C | 3H | 3F | 3L | 3K |
| 98 | 3E | 3J | 3B | 3C | 3H | 3F | 3L | 3I |
| 99 | 3E | 3J | 3B | 3C | 3H | 3F | 3I | 3K |
| 100 | 3E | 3G | 3B | 3C | 3J | 3F | 3L | 3K |
| 101 | 3E | 3G | 3B | 3C | 3I | 3F | 3L | 3K |
| 102 | 3E | 3G | 3B | 3C | 3J | 3F | 3L | 3I |
| 103 | 3E | 3G | 3B | 3C | 3J | 3F | 3I | 3K |
| 104 | 3E | 3G | 3B | 3C | 3H | 3F | 3L | 3K |
| 105 | 3H | 3G | 3B | 3C | 3J | 3F | 3L | 3E |
| 106 | 3H | 3G | 3B | 3C | 3J | 3F | 3E | 3K |
| 107 | 3E | 3G | 3B | 3C | 3H | 3F | 3L | 3I |
| 108 | 3E | 3G | 3B | 3C | 3H | 3F | 3I | 3K |
| 109 | 3H | 3G | 3B | 3C | 3J | 3F | 3E | 3I |
| 110 | 3H | 3J | 3B | 3C | 3I | 3D | 3L | 3K |
| 111 | 3I | 3G | 3B | 3C | 3J | 3D | 3L | 3K |
| 112 | 3H | 3G | 3B | 3C | 3J | 3D | 3L | 3K |
| 113 | 3H | 3G | 3B | 3C | 3I | 3D | 3L | 3K |
| 114 | 3H | 3G | 3B | 3C | 3J | 3D | 3L | 3I |
| 115 | 3H | 3G | 3B | 3C | 3J | 3D | 3I | 3K |
| 116 | 3C | 3J | 3B | 3D | 3I | 3F | 3L | 3K |
| 117 | 3C | 3J | 3B | 3D | 3H | 3F | 3L | 3K |
| 118 | 3C | 3I | 3B | 3D | 3H | 3F | 3L | 3K |
| 119 | 3C | 3J | 3B | 3D | 3H | 3F | 3L | 3I |
| 120 | 3C | 3J | 3B | 3D | 3H | 3F | 3I | 3K |
| 121 | 3C | 3G | 3B | 3D | 3J | 3F | 3L | 3K |
| 122 | 3C | 3G | 3B | 3D | 3I | 3F | 3L | 3K |
| 123 | 3C | 3G | 3B | 3D | 3J | 3F | 3L | 3I |
| 124 | 3C | 3G | 3B | 3D | 3J | 3F | 3I | 3K |
| 125 | 3C | 3G | 3B | 3D | 3H | 3F | 3L | 3K |
| 126 | 3C | 3G | 3B | 3D | 3H | 3F | 3L | 3J |
| 127 | 3H | 3G | 3B | 3C | 3J | 3F | 3D | 3K |
| 128 | 3C | 3G | 3B | 3D | 3H | 3F | 3L | 3I |
| 129 | 3C | 3G | 3B | 3D | 3H | 3F | 3I | 3K |
| 130 | 3H | 3G | 3B | 3C | 3J | 3F | 3D | 3I |
| 131 | 3E | 3J | 3B | 3C | 3I | 3D | 3L | 3K |
| 132 | 3E | 3J | 3B | 3C | 3H | 3D | 3L | 3K |
| 133 | 3E | 3I | 3B | 3C | 3H | 3D | 3L | 3K |
| 134 | 3E | 3J | 3B | 3C | 3H | 3D | 3L | 3I |
| 135 | 3E | 3J | 3B | 3C | 3H | 3D | 3I | 3K |
| 136 | 3E | 3G | 3B | 3C | 3J | 3D | 3L | 3K |
| 137 | 3E | 3G | 3B | 3C | 3I | 3D | 3L | 3K |
| 138 | 3E | 3G | 3B | 3C | 3J | 3D | 3L | 3I |
| 139 | 3E | 3G | 3B | 3C | 3J | 3D | 3I | 3K |
| 140 | 3E | 3G | 3B | 3C | 3H | 3D | 3L | 3K |
| 141 | 3H | 3G | 3B | 3C | 3J | 3D | 3L | 3E |
| 142 | 3H | 3G | 3B | 3C | 3J | 3D | 3E | 3K |
| 143 | 3E | 3G | 3B | 3C | 3H | 3D | 3L | 3I |
| 144 | 3E | 3G | 3B | 3C | 3H | 3D | 3I | 3K |
| 145 | 3H | 3G | 3B | 3C | 3J | 3D | 3E | 3I |
| 146 | 3C | 3J | 3B | 3D | 3E | 3F | 3L | 3K |
| 147 | 3C | 3E | 3B | 3D | 3I | 3F | 3L | 3K |
| 148 | 3C | 3J | 3B | 3D | 3E | 3F | 3L | 3I |
| 149 | 3C | 3J | 3B | 3D | 3E | 3F | 3I | 3K |
| 150 | 3C | 3E | 3B | 3D | 3H | 3F | 3L | 3K |
| 151 | 3C | 3J | 3B | 3D | 3H | 3F | 3L | 3E |
| 152 | 3C | 3J | 3B | 3D | 3H | 3F | 3E | 3K |
| 153 | 3C | 3E | 3B | 3D | 3H | 3F | 3L | 3I |
| 154 | 3C | 3E | 3B | 3D | 3H | 3F | 3I | 3K |
| 155 | 3C | 3J | 3B | 3D | 3H | 3F | 3E | 3I |
| 156 | 3C | 3G | 3B | 3D | 3E | 3F | 3L | 3K |
| 157 | 3C | 3G | 3B | 3D | 3J | 3F | 3L | 3E |
| 158 | 3C | 3G | 3B | 3D | 3J | 3F | 3E | 3K |
| 159 | 3C | 3G | 3B | 3D | 3E | 3F | 3L | 3I |
| 160 | 3C | 3G | 3B | 3D | 3E | 3F | 3I | 3K |
| 161 | 3C | 3G | 3B | 3D | 3J | 3F | 3E | 3I |
| 162 | 3C | 3G | 3B | 3D | 3H | 3F | 3L | 3E |
| 163 | 3C | 3G | 3B | 3D | 3H | 3F | 3E | 3K |
| 164 | 3H | 3G | 3B | 3C | 3J | 3F | 3D | 3E |
| 165 | 3C | 3G | 3B | 3D | 3H | 3F | 3E | 3I |
| 166 | 3H | 3J | 3I | 3F | 3A | 3G | 3L | 3K |
| 167 | 3E | 3J | 3I | 3A | 3H | 3G | 3L | 3K |
| 168 | 3E | 3J | 3I | 3F | 3A | 3H | 3L | 3K |
| 169 | 3E | 3J | 3I | 3F | 3A | 3G | 3L | 3K |
| 170 | 3E | 3G | 3J | 3F | 3A | 3H | 3L | 3K |
| 171 | 3E | 3G | 3I | 3F | 3A | 3H | 3L | 3K |
| 172 | 3E | 3G | 3J | 3F | 3A | 3H | 3L | 3I |
| 173 | 3E | 3G | 3J | 3F | 3A | 3H | 3I | 3K |
| 174 | 3H | 3J | 3I | 3D | 3A | 3G | 3L | 3K |
| 175 | 3H | 3J | 3I | 3D | 3A | 3F | 3L | 3K |
| 176 | 3I | 3G | 3J | 3D | 3A | 3F | 3L | 3K |
| 177 | 3H | 3G | 3J | 3D | 3A | 3F | 3L | 3K |
| 178 | 3H | 3G | 3I | 3D | 3A | 3F | 3L | 3K |
| 179 | 3H | 3G | 3J | 3D | 3A | 3F | 3L | 3I |
| 180 | 3H | 3G | 3J | 3D | 3A | 3F | 3I | 3K |
| 181 | 3E | 3J | 3I | 3D | 3A | 3H | 3L | 3K |
| 182 | 3E | 3J | 3I | 3D | 3A | 3G | 3L | 3K |
| 183 | 3E | 3G | 3J | 3D | 3A | 3H | 3L | 3K |
| 184 | 3E | 3G | 3I | 3D | 3A | 3H | 3L | 3K |
| 185 | 3E | 3G | 3J | 3D | 3A | 3H | 3L | 3I |
| 186 | 3E | 3G | 3J | 3D | 3A | 3H | 3I | 3K |
| 187 | 3E | 3J | 3I | 3D | 3A | 3F | 3L | 3K |
| 188 | 3H | 3J | 3E | 3D | 3A | 3F | 3L | 3K |
| 189 | 3H | 3E | 3I | 3D | 3A | 3F | 3L | 3K |
| 190 | 3H | 3J | 3E | 3D | 3A | 3F | 3L | 3I |
| 191 | 3H | 3J | 3E | 3D | 3A | 3F | 3I | 3K |
| 192 | 3E | 3G | 3J | 3D | 3A | 3F | 3L | 3K |
| 193 | 3E | 3G | 3I | 3D | 3A | 3F | 3L | 3K |
| 194 | 3E | 3G | 3J | 3D | 3A | 3F | 3L | 3I |
| 195 | 3E | 3G | 3J | 3D | 3A | 3F | 3I | 3K |
| 196 | 3H | 3G | 3E | 3D | 3A | 3F | 3L | 3K |
| 197 | 3H | 3G | 3J | 3D | 3A | 3F | 3L | 3E |
| 198 | 3H | 3G | 3J | 3D | 3A | 3F | 3E | 3K |
| 199 | 3H | 3G | 3E | 3D | 3A | 3F | 3L | 3I |
| 200 | 3H | 3G | 3E | 3D | 3A | 3F | 3I | 3K |
| 201 | 3H | 3G | 3J | 3D | 3A | 3F | 3E | 3I |
| 202 | 3H | 3J | 3I | 3C | 3A | 3G | 3L | 3K |
| 203 | 3H | 3J | 3I | 3C | 3A | 3F | 3L | 3K |
| 204 | 3I | 3G | 3J | 3C | 3A | 3F | 3L | 3K |
| 205 | 3H | 3G | 3J | 3C | 3A | 3F | 3L | 3K |
| 206 | 3H | 3G | 3I | 3C | 3A | 3F | 3L | 3K |
| 207 | 3H | 3G | 3J | 3C | 3A | 3F | 3L | 3I |
| 208 | 3H | 3G | 3J | 3C | 3A | 3F | 3I | 3K |
| 209 | 3E | 3J | 3I | 3C | 3A | 3H | 3L | 3K |
| 210 | 3E | 3J | 3I | 3C | 3A | 3G | 3L | 3K |
| 211 | 3E | 3G | 3J | 3C | 3A | 3H | 3L | 3K |
| 212 | 3E | 3G | 3I | 3C | 3A | 3H | 3L | 3K |
| 213 | 3E | 3G | 3J | 3C | 3A | 3H | 3L | 3I |
| 214 | 3E | 3G | 3J | 3C | 3A | 3H | 3I | 3K |
| 215 | 3E | 3J | 3I | 3C | 3A | 3F | 3L | 3K |
| 216 | 3H | 3J | 3E | 3C | 3A | 3F | 3L | 3K |
| 217 | 3H | 3E | 3I | 3C | 3A | 3F | 3L | 3K |
| 218 | 3H | 3J | 3E | 3C | 3A | 3F | 3L | 3I |
| 219 | 3H | 3J | 3E | 3C | 3A | 3F | 3I | 3K |
| 220 | 3E | 3G | 3J | 3C | 3A | 3F | 3L | 3K |
| 221 | 3E | 3G | 3I | 3C | 3A | 3F | 3L | 3K |
| 222 | 3E | 3G | 3J | 3C | 3A | 3F | 3L | 3I |
| 223 | 3E | 3G | 3J | 3C | 3A | 3F | 3I | 3K |
| 224 | 3H | 3G | 3E | 3C | 3A | 3F | 3L | 3K |
| 225 | 3H | 3G | 3J | 3C | 3A | 3F | 3L | 3E |
| 226 | 3H | 3G | 3J | 3C | 3A | 3F | 3E | 3K |
| 227 | 3H | 3G | 3E | 3C | 3A | 3F | 3L | 3I |
| 228 | 3H | 3G | 3E | 3C | 3A | 3F | 3I | 3K |
| 229 | 3H | 3G | 3J | 3C | 3A | 3F | 3E | 3I |
| 230 | 3H | 3J | 3I | 3C | 3A | 3D | 3L | 3K |
| 231 | 3I | 3G | 3J | 3C | 3A | 3D | 3L | 3K |
| 232 | 3H | 3G | 3J | 3C | 3A | 3D | 3L | 3K |
| 233 | 3H | 3G | 3I | 3C | 3A | 3D | 3L | 3K |
| 234 | 3H | 3G | 3J | 3C | 3A | 3D | 3L | 3I |
| 235 | 3H | 3G | 3J | 3C | 3A | 3D | 3I | 3K |
| 236 | 3C | 3J | 3I | 3D | 3A | 3F | 3L | 3K |
| 237 | 3H | 3J | 3F | 3C | 3A | 3D | 3L | 3K |
| 238 | 3H | 3F | 3I | 3C | 3A | 3D | 3L | 3K |
| 239 | 3H | 3J | 3F | 3C | 3A | 3D | 3L | 3I |
| 240 | 3H | 3J | 3F | 3C | 3A | 3D | 3I | 3K |
| 241 | 3C | 3G | 3J | 3D | 3A | 3F | 3L | 3K |
| 242 | 3C | 3G | 3I | 3D | 3A | 3F | 3L | 3K |
| 243 | 3C | 3G | 3J | 3D | 3A | 3F | 3L | 3I |
| 244 | 3C | 3G | 3J | 3D | 3A | 3F | 3I | 3K |
| 245 | 3H | 3G | 3F | 3C | 3A | 3D | 3L | 3K |
| 246 | 3C | 3G | 3J | 3D | 3A | 3F | 3L | 3H |
| 247 | 3H | 3G | 3J | 3C | 3A | 3F | 3D | 3K |
| 248 | 3H | 3G | 3F | 3C | 3A | 3D | 3L | 3I |
| 249 | 3H | 3G | 3F | 3C | 3A | 3D | 3I | 3K |
| 250 | 3H | 3G | 3J | 3C | 3A | 3F | 3D | 3I |
| 251 | 3E | 3J | 3I | 3C | 3A | 3D | 3L | 3K |
| 252 | 3H | 3J | 3E | 3C | 3A | 3D | 3L | 3K |
| 253 | 3H | 3E | 3I | 3C | 3A | 3D | 3L | 3K |
| 254 | 3H | 3J | 3E | 3C | 3A | 3D | 3L | 3I |
| 255 | 3H | 3J | 3E | 3C | 3A | 3D | 3I | 3K |
| 256 | 3E | 3G | 3J | 3C | 3A | 3D | 3L | 3K |
| 257 | 3E | 3G | 3I | 3C | 3A | 3D | 3L | 3K |
| 258 | 3E | 3G | 3J | 3C | 3A | 3D | 3L | 3I |
| 259 | 3E | 3G | 3J | 3C | 3A | 3D | 3I | 3K |
| 260 | 3H | 3G | 3E | 3C | 3A | 3D | 3L | 3K |
| 261 | 3H | 3G | 3J | 3C | 3A | 3D | 3L | 3E |
| 262 | 3H | 3G | 3J | 3C | 3A | 3D | 3E | 3K |
| 263 | 3H | 3G | 3E | 3C | 3A | 3D | 3L | 3I |
| 264 | 3H | 3G | 3E | 3C | 3A | 3D | 3I | 3K |
| 265 | 3H | 3G | 3J | 3C | 3A | 3D | 3E | 3I |
| 266 | 3C | 3J | 3E | 3D | 3A | 3F | 3L | 3K |
| 267 | 3C | 3E | 3I | 3D | 3A | 3F | 3L | 3K |
| 268 | 3C | 3J | 3E | 3D | 3A | 3F | 3L | 3I |
| 269 | 3C | 3J | 3E | 3D | 3A | 3F | 3I | 3K |
| 270 | 3H | 3E | 3F | 3C | 3A | 3D | 3L | 3K |
| 271 | 3H | 3J | 3F | 3C | 3A | 3D | 3L | 3E |
| 272 | 3H | 3J | 3E | 3C | 3A | 3F | 3D | 3K |
| 273 | 3H | 3E | 3F | 3C | 3A | 3D | 3L | 3I |
| 274 | 3H | 3E | 3F | 3C | 3A | 3D | 3I | 3K |
| 275 | 3H | 3J | 3E | 3C | 3A | 3F | 3D | 3I |
| 276 | 3C | 3G | 3E | 3D | 3A | 3F | 3L | 3K |
| 277 | 3C | 3G | 3J | 3D | 3A | 3F | 3L | 3E |
| 278 | 3C | 3G | 3J | 3D | 3A | 3F | 3E | 3K |
| 279 | 3C | 3G | 3E | 3D | 3A | 3F | 3L | 3I |
| 280 | 3C | 3G | 3E | 3D | 3A | 3F | 3I | 3K |
| 281 | 3C | 3G | 3J | 3D | 3A | 3F | 3E | 3I |
| 282 | 3H | 3G | 3F | 3C | 3A | 3D | 3L | 3E |
| 283 | 3H | 3G | 3E | 3C | 3A | 3F | 3D | 3K |
| 284 | 3H | 3G | 3J | 3C | 3A | 3F | 3D | 3E |
| 285 | 3H | 3G | 3E | 3C | 3A | 3F | 3D | 3I |
| 286 | 3H | 3J | 3B | 3A | 3I | 3G | 3L | 3K |
| 287 | 3H | 3J | 3B | 3A | 3I | 3F | 3L | 3K |
| 288 | 3I | 3J | 3B | 3F | 3A | 3G | 3L | 3K |
| 289 | 3H | 3J | 3B | 3F | 3A | 3G | 3L | 3K |
| 290 | 3H | 3G | 3B | 3A | 3I | 3F | 3L | 3K |
| 291 | 3H | 3J | 3B | 3F | 3A | 3G | 3L | 3I |
| 292 | 3H | 3J | 3B | 3F | 3A | 3G | 3I | 3K |
| 293 | 3E | 3J | 3B | 3A | 3I | 3H | 3L | 3K |
| 294 | 3E | 3J | 3B | 3A | 3I | 3G | 3L | 3K |
| 295 | 3E | 3J | 3B | 3A | 3H | 3G | 3L | 3K |
| 296 | 3E | 3G | 3B | 3A | 3I | 3H | 3L | 3K |
| 297 | 3E | 3J | 3B | 3A | 3H | 3G | 3L | 3I |
| 298 | 3E | 3J | 3B | 3A | 3H | 3G | 3I | 3K |
| 299 | 3E | 3J | 3B | 3A | 3I | 3F | 3L | 3K |
| 300 | 3E | 3J | 3B | 3F | 3A | 3H | 3L | 3K |
| 301 | 3E | 3I | 3B | 3F | 3A | 3H | 3L | 3K |
| 302 | 3E | 3J | 3B | 3F | 3A | 3H | 3L | 3I |
| 303 | 3E | 3J | 3B | 3F | 3A | 3H | 3I | 3K |
| 304 | 3E | 3J | 3B | 3F | 3A | 3G | 3L | 3K |
| 305 | 3E | 3G | 3B | 3A | 3I | 3F | 3L | 3K |
| 306 | 3E | 3J | 3B | 3F | 3A | 3G | 3L | 3I |
| 307 | 3E | 3J | 3B | 3F | 3A | 3G | 3I | 3K |
| 308 | 3E | 3G | 3B | 3F | 3A | 3H | 3L | 3K |
| 309 | 3H | 3J | 3B | 3F | 3A | 3G | 3L | 3E |
| 310 | 3H | 3J | 3B | 3F | 3A | 3G | 3E | 3K |
| 311 | 3E | 3G | 3B | 3F | 3A | 3H | 3L | 3I |
| 312 | 3E | 3G | 3B | 3F | 3A | 3H | 3I | 3K |
| 313 | 3H | 3J | 3B | 3F | 3A | 3G | 3E | 3I |
| 314 | 3I | 3J | 3B | 3D | 3A | 3H | 3L | 3K |
| 315 | 3I | 3J | 3B | 3D | 3A | 3G | 3L | 3K |
| 316 | 3H | 3J | 3B | 3D | 3A | 3G | 3L | 3K |
| 317 | 3I | 3G | 3B | 3D | 3A | 3H | 3L | 3K |
| 318 | 3H | 3J | 3B | 3D | 3A | 3G | 3L | 3I |
| 319 | 3H | 3J | 3B | 3D | 3A | 3G | 3I | 3K |
| 320 | 3I | 3J | 3B | 3D | 3A | 3F | 3L | 3K |
| 321 | 3H | 3J | 3B | 3D | 3A | 3F | 3L | 3K |
| 322 | 3H | 3I | 3B | 3D | 3A | 3F | 3L | 3K |
| 323 | 3H | 3J | 3B | 3D | 3A | 3F | 3L | 3I |
| 324 | 3H | 3J | 3B | 3D | 3A | 3F | 3I | 3K |
| 325 | 3F | 3J | 3B | 3D | 3A | 3G | 3L | 3K |
| 326 | 3I | 3G | 3B | 3D | 3A | 3F | 3L | 3K |
| 327 | 3F | 3J | 3B | 3D | 3A | 3G | 3L | 3I |
| 328 | 3F | 3J | 3B | 3D | 3A | 3G | 3I | 3K |
| 329 | 3H | 3G | 3B | 3D | 3A | 3F | 3L | 3K |
| 330 | 3H | 3G | 3B | 3D | 3A | 3F | 3L | 3J |
| 331 | 3H | 3G | 3B | 3D | 3A | 3F | 3J | 3K |
| 332 | 3H | 3G | 3B | 3D | 3A | 3F | 3L | 3I |
| 333 | 3H | 3G | 3B | 3D | 3A | 3F | 3I | 3K |
| 334 | 3H | 3G | 3B | 3D | 3A | 3F | 3I | 3J |
| 335 | 3E | 3J | 3B | 3A | 3I | 3D | 3L | 3K |
| 336 | 3E | 3J | 3B | 3D | 3A | 3H | 3L | 3K |
| 337 | 3E | 3I | 3B | 3D | 3A | 3H | 3L | 3K |
| 338 | 3E | 3J | 3B | 3D | 3A | 3H | 3L | 3I |
| 339 | 3E | 3J | 3B | 3D | 3A | 3H | 3I | 3K |
| 340 | 3E | 3J | 3B | 3D | 3A | 3G | 3L | 3K |
| 341 | 3E | 3G | 3B | 3A | 3I | 3D | 3L | 3K |
| 342 | 3E | 3J | 3B | 3D | 3A | 3G | 3L | 3I |
| 343 | 3E | 3J | 3B | 3D | 3A | 3G | 3I | 3K |
| 344 | 3E | 3G | 3B | 3D | 3A | 3H | 3L | 3K |
| 345 | 3H | 3J | 3B | 3D | 3A | 3G | 3L | 3E |
| 346 | 3H | 3J | 3B | 3D | 3A | 3G | 3E | 3K |
| 347 | 3E | 3G | 3B | 3D | 3A | 3H | 3L | 3I |
| 348 | 3E | 3G | 3B | 3D | 3A | 3H | 3I | 3K |
| 349 | 3H | 3J | 3B | 3D | 3A | 3G | 3E | 3I |
| 350 | 3E | 3J | 3B | 3D | 3A | 3F | 3L | 3K |
| 351 | 3E | 3I | 3B | 3D | 3A | 3F | 3L | 3K |
| 352 | 3E | 3J | 3B | 3D | 3A | 3F | 3L | 3I |
| 353 | 3E | 3J | 3B | 3D | 3A | 3F | 3I | 3K |
| 354 | 3H | 3E | 3B | 3D | 3A | 3F | 3L | 3K |
| 355 | 3H | 3J | 3B | 3D | 3A | 3F | 3L | 3E |
| 356 | 3H | 3J | 3B | 3D | 3A | 3F | 3E | 3K |
| 357 | 3H | 3E | 3B | 3D | 3A | 3F | 3L | 3I |
| 358 | 3H | 3E | 3B | 3D | 3A | 3F | 3I | 3K |
| 359 | 3H | 3J | 3B | 3D | 3A | 3F | 3E | 3I |
| 360 | 3E | 3G | 3B | 3D | 3A | 3F | 3L | 3K |
| 361 | 3E | 3G | 3B | 3D | 3A | 3F | 3L | 3J |
| 362 | 3E | 3G | 3B | 3D | 3A | 3F | 3J | 3K |
| 363 | 3E | 3G | 3B | 3D | 3A | 3F | 3L | 3I |
| 364 | 3E | 3G | 3B | 3D | 3A | 3F | 3I | 3K |
| 365 | 3E | 3G | 3B | 3D | 3A | 3F | 3I | 3J |
| 366 | 3H | 3G | 3B | 3D | 3A | 3F | 3L | 3E |
| 367 | 3H | 3G | 3B | 3D | 3A | 3F | 3E | 3K |
| 368 | 3H | 3G | 3B | 3D | 3A | 3F | 3E | 3J |
| 369 | 3H | 3G | 3B | 3D | 3A | 3F | 3E | 3I |
| 370 | 3I | 3J | 3B | 3C | 3A | 3H | 3L | 3K |
| 371 | 3I | 3J | 3B | 3C | 3A | 3G | 3L | 3K |
| 372 | 3H | 3J | 3B | 3C | 3A | 3G | 3L | 3K |
| 373 | 3I | 3G | 3B | 3C | 3A | 3H | 3L | 3K |
| 374 | 3H | 3J | 3B | 3C | 3A | 3G | 3L | 3I |
| 375 | 3H | 3J | 3B | 3C | 3A | 3G | 3I | 3K |
| 376 | 3I | 3J | 3B | 3C | 3A | 3F | 3L | 3K |
| 377 | 3H | 3J | 3B | 3C | 3A | 3F | 3L | 3K |
| 378 | 3H | 3I | 3B | 3C | 3A | 3F | 3L | 3K |
| 379 | 3H | 3J | 3B | 3C | 3A | 3F | 3L | 3I |
| 380 | 3H | 3J | 3B | 3C | 3A | 3F | 3I | 3K |
| 381 | 3C | 3J | 3B | 3F | 3A | 3G | 3L | 3K |
| 382 | 3I | 3G | 3B | 3C | 3A | 3F | 3L | 3K |
| 383 | 3C | 3J | 3B | 3F | 3A | 3G | 3L | 3I |
| 384 | 3C | 3J | 3B | 3F | 3A | 3G | 3I | 3K |
| 385 | 3H | 3G | 3B | 3C | 3A | 3F | 3L | 3K |
| 386 | 3H | 3G | 3B | 3C | 3A | 3F | 3L | 3J |
| 387 | 3H | 3G | 3B | 3C | 3A | 3F | 3J | 3K |
| 388 | 3H | 3G | 3B | 3C | 3A | 3F | 3L | 3I |
| 389 | 3H | 3G | 3B | 3C | 3A | 3F | 3I | 3K |
| 390 | 3H | 3G | 3B | 3C | 3A | 3F | 3I | 3J |
| 391 | 3E | 3J | 3B | 3A | 3I | 3C | 3L | 3K |
| 392 | 3E | 3J | 3B | 3C | 3A | 3H | 3L | 3K |
| 393 | 3E | 3I | 3B | 3C | 3A | 3H | 3L | 3K |
| 394 | 3E | 3J | 3B | 3C | 3A | 3H | 3L | 3I |
| 395 | 3E | 3J | 3B | 3C | 3A | 3H | 3I | 3K |
| 396 | 3E | 3J | 3B | 3C | 3A | 3G | 3L | 3K |
| 397 | 3E | 3G | 3B | 3A | 3I | 3C | 3L | 3K |
| 398 | 3E | 3J | 3B | 3C | 3A | 3G | 3L | 3I |
| 399 | 3E | 3J | 3B | 3C | 3A | 3G | 3I | 3K |
| 400 | 3E | 3G | 3B | 3C | 3A | 3H | 3L | 3K |
| 401 | 3H | 3J | 3B | 3C | 3A | 3G | 3L | 3E |
| 402 | 3H | 3J | 3B | 3C | 3A | 3G | 3E | 3K |
| 403 | 3E | 3G | 3B | 3C | 3A | 3H | 3L | 3I |
| 404 | 3E | 3G | 3B | 3C | 3A | 3H | 3I | 3K |
| 405 | 3H | 3J | 3B | 3C | 3A | 3G | 3E | 3I |
| 406 | 3E | 3J | 3B | 3C | 3A | 3F | 3L | 3K |
| 407 | 3E | 3I | 3B | 3C | 3A | 3F | 3L | 3K |
| 408 | 3E | 3J | 3B | 3C | 3A | 3F | 3L | 3I |
| 409 | 3E | 3J | 3B | 3C | 3A | 3F | 3I | 3K |
| 410 | 3H | 3E | 3B | 3C | 3A | 3F | 3L | 3K |
| 411 | 3H | 3J | 3B | 3C | 3A | 3F | 3L | 3E |
| 412 | 3H | 3J | 3B | 3C | 3A | 3F | 3E | 3K |
| 413 | 3H | 3E | 3B | 3C | 3A | 3F | 3L | 3I |
| 414 | 3H | 3E | 3B | 3C | 3A | 3F | 3I | 3K |
| 415 | 3H | 3J | 3B | 3C | 3A | 3F | 3E | 3I |
| 416 | 3E | 3G | 3B | 3C | 3A | 3F | 3L | 3K |
| 417 | 3E | 3G | 3B | 3C | 3A | 3F | 3L | 3J |
| 418 | 3E | 3G | 3B | 3C | 3A | 3F | 3J | 3K |
| 419 | 3E | 3G | 3B | 3C | 3A | 3F | 3L | 3I |
| 420 | 3E | 3G | 3B | 3C | 3A | 3F | 3I | 3K |
| 421 | 3E | 3G | 3B | 3C | 3A | 3F | 3I | 3J |
| 422 | 3H | 3G | 3B | 3C | 3A | 3F | 3L | 3E |
| 423 | 3H | 3G | 3B | 3C | 3A | 3F | 3E | 3K |
| 424 | 3H | 3G | 3B | 3C | 3A | 3F | 3E | 3J |
| 425 | 3H | 3G | 3B | 3C | 3A | 3F | 3E | 3I |
| 426 | 3I | 3J | 3B | 3C | 3A | 3D | 3L | 3K |
| 427 | 3H | 3J | 3B | 3C | 3A | 3D | 3L | 3K |
| 428 | 3H | 3I | 3B | 3C | 3A | 3D | 3L | 3K |
| 429 | 3H | 3J | 3B | 3C | 3A | 3D | 3L | 3I |
| 430 | 3H | 3J | 3B | 3C | 3A | 3D | 3I | 3K |
| 431 | 3C | 3J | 3B | 3D | 3A | 3G | 3L | 3K |
| 432 | 3I | 3G | 3B | 3C | 3A | 3D | 3L | 3K |
| 433 | 3C | 3J | 3B | 3D | 3A | 3G | 3L | 3I |
| 434 | 3C | 3J | 3B | 3D | 3A | 3G | 3I | 3K |
| 435 | 3H | 3G | 3B | 3C | 3A | 3D | 3L | 3K |
| 436 | 3H | 3G | 3B | 3C | 3A | 3D | 3L | 3J |
| 437 | 3H | 3G | 3B | 3C | 3A | 3D | 3J | 3K |
| 438 | 3H | 3G | 3B | 3C | 3A | 3D | 3L | 3I |
| 439 | 3H | 3G | 3B | 3C | 3A | 3D | 3I | 3K |
| 440 | 3H | 3G | 3B | 3C | 3A | 3D | 3I | 3J |
| 441 | 3C | 3J | 3B | 3D | 3A | 3F | 3L | 3K |
| 442 | 3C | 3I | 3B | 3D | 3A | 3F | 3L | 3K |
| 443 | 3C | 3J | 3B | 3D | 3A | 3F | 3L | 3I |
| 444 | 3C | 3J | 3B | 3D | 3A | 3F | 3I | 3K |
| 445 | 3H | 3F | 3B | 3C | 3A | 3D | 3L | 3K |
| 446 | 3C | 3J | 3B | 3D | 3A | 3F | 3L | 3H |
| 447 | 3H | 3J | 3B | 3C | 3A | 3F | 3D | 3K |
| 448 | 3H | 3F | 3B | 3C | 3A | 3D | 3L | 3I |
| 449 | 3H | 3F | 3B | 3C | 3A | 3D | 3I | 3K |
| 450 | 3H | 3J | 3B | 3C | 3A | 3F | 3D | 3I |
| 451 | 3C | 3G | 3B | 3D | 3A | 3F | 3L | 3K |
| 452 | 3C | 3G | 3B | 3D | 3A | 3F | 3L | 3J |
| 453 | 3C | 3G | 3B | 3D | 3A | 3F | 3J | 3K |
| 454 | 3C | 3G | 3B | 3D | 3A | 3F | 3L | 3I |
| 455 | 3C | 3G | 3B | 3D | 3A | 3F | 3I | 3K |
| 456 | 3C | 3G | 3B | 3D | 3A | 3F | 3I | 3J |
| 457 | 3C | 3G | 3B | 3D | 3A | 3F | 3L | 3H |
| 458 | 3H | 3G | 3B | 3C | 3A | 3F | 3D | 3K |
| 459 | 3H | 3G | 3B | 3C | 3A | 3F | 3D | 3J |
| 460 | 3H | 3G | 3B | 3C | 3A | 3F | 3D | 3I |
| 461 | 3E | 3J | 3B | 3C | 3A | 3D | 3L | 3K |
| 462 | 3E | 3I | 3B | 3C | 3A | 3D | 3L | 3K |
| 463 | 3E | 3J | 3B | 3C | 3A | 3D | 3L | 3I |
| 464 | 3E | 3J | 3B | 3C | 3A | 3D | 3I | 3K |
| 465 | 3H | 3E | 3B | 3C | 3A | 3D | 3L | 3K |
| 466 | 3H | 3J | 3B | 3C | 3A | 3D | 3L | 3E |
| 467 | 3H | 3J | 3B | 3C | 3A | 3D | 3E | 3K |
| 468 | 3H | 3E | 3B | 3C | 3A | 3D | 3L | 3I |
| 469 | 3H | 3E | 3B | 3C | 3A | 3D | 3I | 3K |
| 470 | 3H | 3J | 3B | 3C | 3A | 3D | 3E | 3I |
| 471 | 3E | 3G | 3B | 3C | 3A | 3D | 3L | 3K |
| 472 | 3E | 3G | 3B | 3C | 3A | 3D | 3L | 3J |
| 473 | 3E | 3G | 3B | 3C | 3A | 3D | 3J | 3K |
| 474 | 3E | 3G | 3B | 3C | 3A | 3D | 3L | 3I |
| 475 | 3E | 3G | 3B | 3C | 3A | 3D | 3I | 3K |
| 476 | 3E | 3G | 3B | 3C | 3A | 3D | 3I | 3J |
| 477 | 3H | 3G | 3B | 3C | 3A | 3D | 3L | 3E |
| 478 | 3H | 3G | 3B | 3C | 3A | 3D | 3E | 3K |
| 479 | 3H | 3G | 3B | 3C | 3A | 3D | 3E | 3J |
| 480 | 3H | 3G | 3B | 3C | 3A | 3D | 3E | 3I |
| 481 | 3C | 3E | 3B | 3D | 3A | 3F | 3L | 3K |
| 482 | 3C | 3J | 3B | 3D | 3A | 3F | 3L | 3E |
| 483 | 3C | 3J | 3B | 3D | 3A | 3F | 3E | 3K |
| 484 | 3C | 3E | 3B | 3D | 3A | 3F | 3L | 3I |
| 485 | 3C | 3E | 3B | 3D | 3A | 3F | 3I | 3K |
| 486 | 3C | 3J | 3B | 3D | 3A | 3F | 3E | 3I |
| 487 | 3H | 3F | 3B | 3C | 3A | 3D | 3L | 3E |
| 488 | 3H | 3E | 3B | 3C | 3A | 3F | 3D | 3K |
| 489 | 3H | 3J | 3B | 3C | 3A | 3F | 3D | 3E |
| 490 | 3H | 3E | 3B | 3C | 3A | 3F | 3D | 3I |
| 491 | 3C | 3G | 3B | 3D | 3A | 3F | 3L | 3E |
| 492 | 3C | 3G | 3B | 3D | 3A | 3F | 3E | 3K |
| 493 | 3C | 3G | 3B | 3D | 3A | 3F | 3E | 3J |
| 494 | 3C | 3G | 3B | 3D | 3A | 3F | 3E | 3I |
| 495 | 3H | 3G | 3B | 3C | 3A | 3F | 3D | 3E |

---

## 6. Extra time and penalties (Art. 14)

Applies only in **knockout stages** (R32 onwards).

- If level after 90 minutes:
  - 2 × 15-minute periods of extra time.
  - 5-minute interval before extra time begins; no interval between the two periods of ET (a short ≤1-minute drinks break is permitted).
  - Players remain on the pitch during these intervals.
- If still level after extra time: **penalty shoot-out** per the Laws of the Game.
- Coin toss procedure before shoot-out (Art. 14.3):
  - First coin toss: decides which goal is used.
  - Second coin toss: winner chooses to shoot first or second.

---

## 7. Match-day rules relevant to predictions

### Substitutions (Art. 36.2–36.4)

- Per regulation time: **max 5 substitutes**, **max 3 substitution opportunities** (half-time doesn't count).
- Both teams making a substitution simultaneously counts as one used opportunity for each.
- **Concussion substitution:** 1 additional permanent concussion sub per match (does not count against the 5). The opposing team gets 1 additional regular sub in exchange.
- **In extra time:**
  - +1 additional substitute (regardless of how many regular subs already used)
  - +1 additional substitution opportunity
  - Subs at the start of ET and at ET half-time don't count as opportunities.

### Match length (Art. 36.9)

- 90 minutes (two 45-minute halves).
- 15-minute half-time interval (Art. 36.5).

### Card carryover (Art. 10.2–10.6)

Important for predicting which players are available:

- **From preliminary to final competition:**
  - Single yellows: NOT carried over.
  - Pending 1- or 2-match suspensions from accumulated cautions: NOT carried over.
  - Indirect red: NOT carried over.
  - Direct red for denial of goal/goalscoring opportunity OR serious foul play: NOT carried over.
  - Any **other** pending match suspensions from a red card: **carried over**.
- **Within the final competition:**
  - Single yellows are **cancelled after the group stage** AND **cancelled again after the quarter-finals**.
  - 2 yellows in different matches → automatic 1-match suspension for next match.
  - Direct or indirect red → automatic suspension for next match (additional sanctions possible).
- Suspensions that can't be served during the tournament carry over to the team's next official match.

---

## 8. Withdrawal / abandonment edge cases (Art. 6)

For completeness — these affect prediction validity:

- If a team **withdraws or is disqualified before the end of the group stage**: results of all its matches are **declared null and void** (Art. 6.8).
- If a match is abandoned due to force majeure after kick-off: it **resumes from the minute and score at which play was interrupted**, with the same players/substitutes available and remaining substitution opportunities (Art. 6.6).
- FIFA has unilateral right to cancel, reschedule, or relocate matches (Art. 6.9).

---

## 9. Implementation notes

Suggested data model:

```
Team:
  id, group_letter, fifa_ranking, name

GroupMatch:
  group_letter, matchday (1-3), home_team, away_team,
  home_goals, away_goals,
  home_cards: list of (player, type), away_cards: list of (player, type)

GroupStanding (computed):
  team, played, won, drawn, lost,
  goals_for, goals_against, goal_diff, points,
  conduct_score, position_in_group

ThirdPlacedRanking (computed):
  ranked list of 12 third-placed teams
  top 8 advance, labelled by group letter
```

**Bracket resolution algorithm:**

1. Compute group standings, applying §2 tiebreakers.
2. Extract 12 third-placed teams, rank them per §3, take top 8.
3. Collect the group letters of the 8 qualifying third-placed teams → sorted set.
4. Look up that set in the Annexe C table (§5) to get the third-placed team → group winner mapping.
5. Build all 16 R32 fixtures per §4 (Art. 12.6).
6. Subsequent rounds are deterministic given R32 outcomes — see Art. 12.7–12.11 tables in §4.

**Annexe C as a dictionary:** Each row's set `{3X column values}` is a unique 8-element subset of `{A..L}`. Keyed lookup: `combinations[frozenset(group_letters_of_qualifying_3rds)] → {1A: '3X', 1B: '3X', ..., 1L: '3X'}`. 12-choose-8 = 495, matching the table.

---

*Source: FIFA, "Regulations for the FIFA World Cup 26™", May 2026 edition. Articles 1–14, 36, 45 and Annexe C. Approved by the Bureau of the FIFA Council on 8 May 2026.*
