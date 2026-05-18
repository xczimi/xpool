# Domain Docs

How the engineering skills should consume this repo's domain documentation.

xpool keeps its authoritative design in **`.specs/`** — there is **no** root
`CONTEXT.md` and no `docs/adr/`. The skills read `.specs/` instead, so the
project has one source of truth.

## Before exploring, read these

- **`.specs/DATA_MODEL.md`** — the domain glossary: entities, the recursive
  tournament tree, identity (`Identity`/`Person`/`Player`), pools, storage.
  This is the vocabulary authority.
- **`.specs/SCORING.md`** — the scoring engine's terms and rules.
- **`.specs/DESIGN_REVIEW.md`** — the decision record: Peter's positions,
  rationale, and the calls that overrode an initial recommendation. This is
  xpool's equivalent of an ADR log.
- The other `.specs/*.md` (`API.md`, `FWC26_RULES.md`, `DEPLOYMENT.md`,
  `DATA_SOURCES.md`, `REWRITE_USE_CASES.md`, `LEGACY_I18N.md`) as relevant.

`.specs/` is authoritative — where code and an older spec conflict,
`DATA_MODEL.md` and `SCORING.md` win (see their "corrections" sections).

If a doc doesn't exist, **proceed silently** — don't flag its absence.

## Use the glossary's vocabulary

When your output names a domain concept (issue title, refactor proposal,
hypothesis, test name), use the term as defined in `.specs/DATA_MODEL.md` /
`SCORING.md`. Don't drift to synonyms the specs avoid — e.g. it is a
`Player`'s `MatchPrediction`, not "a user's bet"; the official results are the
"result user"; nodes form a recursive `GroupGame` tree.

If a concept you need isn't in `.specs/` yet, that's a signal — either you're
inventing language the project doesn't use (reconsider), or there's a real gap
(note it).

## Flag decision conflicts

If your output contradicts a decision recorded in `.specs/DESIGN_REVIEW.md`
(or a spec's "corrections" section), surface it explicitly rather than
silently overriding:

> _Contradicts DESIGN_REVIEW.md (recursive GroupGame tree kept, `[override]`)
> — but worth reopening because…_

## Layout

Single-context — one design corpus in `.specs/`, no `CONTEXT-MAP.md`. The
producer skill `/grill-with-docs` would by default create a root `CONTEXT.md`
and `docs/adr/`; if you run it, decide whether to fold its output into
`.specs/` to keep the single source of truth.
