# xPool — FWC26 Rebuild · Project Recap

> A 15-year-old Python / Google App Engine soccer pool, torn down and rebuilt in **Rust + React** for the **FIFA World Cup 2026**.
>
> **Season 2026 · May 16 → Jun 10 · 11 active days over a 26-day window**

Reconstructed from the `xpool` git history and `~/.claude` session logs. `soccer-pool` excluded.

---

## 🏆 High Score

| Active Hours | Commits | Sessions | Net Lines | Worktrees | Tool Calls |
|:---:|:---:|:---:|:---:|:---:|:---:|
| **~55h** | **330** | **521** | **+66.9k** | **30** | **12.2k** |

---

## 🥅 The Lineage — 4th Incarnation

This is the **4th incarnation** of the soccer pool. The git history and `archive/` tell the tale:

| Tournament | Stack | Evidence in `archive/` |
|---|---|---|
| 2010 World Cup | Python / GAE | `fifa2010.py`, `fifa.py.2010` |
| 2012 Euro | Python / GAE | `uefa2012.py` |
| 2014 World Cup | Python / GAE | `fifa2014.py`, `index2014.html` |
| **2026 World Cup** | **Rust + React / TS** | 5-crate workspace, Auth0 ◀ **THIS BUILD** |

The rebuild officially kicked off **2026-05-16 at 11:48 AM** with:

> `chore: archive legacy GAE app, add rewrite specs`

The old Python app was buried in `archive/`; the new app is a 5-crate Rust workspace
(`api`, `domain`, `fwc26`, `storage`, `xtask`) with a React + Auth0 frontend.

---

## ⏱️ Time On The Pitch — Active Hours / Day

```
2026-05-16   5.0 h   ██████████
2026-05-17   5.7 h   ███████████
2026-05-18   3.0 h   ██████
2026-05-19   1.1 h   ██
2026-05-30   4.9 h   ██████████        ← back after a 10-day gap
2026-05-31   2.0 h   ████
2026-06-06   5.0 h   ██████████
2026-06-07  10.1 h   ████████████████████  ← the marathon (83 commits!)
2026-06-08   4.8 h   ██████████
2026-06-09   5.2 h   ██████████
2026-06-10   3.2 h   ██████
            ─────
            50.1 h   (5-min idle cap; 58.9 h at 15-min, 68.8 h at 30-min)
```

Two distinct sprints: a **setup sprint** (May 16–19, scaffolding + specs) and the
**build sprint** (May 30 → Jun 10, where the app came to life). June 7 was a marathon —
~10 active hours and 83 commits in a single day.

---

## 🎲 Fun Stats

| Stat | Count |
|---|---:|
| 🤖 Claude sessions logged | 521 |
| 🌳 Working directories | 31 (1 main + 30 parallel worktrees) |
| 💬 Prompts you actually typed | 1,145 |
| 🔧 Tool calls Claude ran | 12,248 |
| 📝 Commits | 330 |
| ➕ Lines added | 73,659 |
| ➖ Lines deleted | 6,740 (net **+66,919**) |
| 🦀 Rust source lines (current) | 17,275 |
| 📦 Tracked lines, all languages | ~63,000 |
| 📄 Markdown / spec files in tree | 110 |

---

## 🤖 Claude's Busywork — Tool Calls

```
Bash    5,705   ████████████████████
Read    3,114   ███████████
Edit    1,604   ██████
Write     496   ██
Agent     299   █   ← sub-agents dispatched
Grep      180
Skill      95
```

## 🧬 Commit DNA

`feat 134` · `docs 91` · `fix 37` · `test 19` · `chore 14` · `refactor 9` · `style 4`

91 docs commits is notable — a *very* spec-driven, documentation-heavy rebuild
(110 markdown files in the tree).

---

## 📣 Final Whistle

> Rebuilt a 15-year-old Python pool into a **Rust + React** app for World Cup 2026 in
> roughly **50–60 hours** across **11 days** — **330 commits**, **~67k net new lines**,
> **17k lines of Rust** — orchestrating Claude across **30 parallel worktrees** and
> **12,000+ tool calls**, all from about **1,145 prompts**.

---

### Methodology

Hours derive from gaps between Claude session event timestamps with idle stretches capped
(5-min cap = **50.1h** conservative · 15-min = **58.9h** · 30-min = **68.8h**), merged across
all parallel sessions so simultaneous worktrees never double-count — a realistic floor, not
wall-clock elapsed. Commits, lines, tool calls and prompts are counted directly from the
`xpool` git repo and `~/.claude` session logs. `soccer-pool` excluded.
Scope: **2026-05-16 → 2026-06-10**.
