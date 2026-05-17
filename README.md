# xpool

A soccer score-prediction pool for major tournaments (FIFA World Cup, UEFA
Euro). Players predict match scores and compete on a points scoreboard.

> **Status: rewrite in progress.** The original app (Google App Engine,
> Python 2.7) has been moved to [`archive/`](./archive/) for reference. The new
> implementation will be built at the repo root.

## Documentation

Specs and reference docs for agentic development live in [`.specs/`](./.specs/).

| Document | What it covers |
|----------|----------------|
| [`.specs/REWRITE_USE_CASES.md`](./.specs/REWRITE_USE_CASES.md) | User journeys, scenarios, and use cases — *what the app does*, technology-independent |
| [`.specs/REWRITE_IMPLEMENTATION.md`](./.specs/REWRITE_IMPLEMENTATION.md) | Domain model, scoring engine, data ingestion, legacy anti-patterns — *how to build it* |
| [`.specs/DATA_MODEL.md`](./.specs/DATA_MODEL.md) | The agreed domain & storage model — entities, tournament tree, pools, identity, DynamoDB layout |
| [`.specs/SCORING.md`](./.specs/SCORING.md) | The agreed scoring engine — per-match points, standings bonus, multipliers, materialized scoreboard |
| [`.specs/REWRITE_ARCHITECTURE.md`](./.specs/REWRITE_ARCHITECTURE.md) | Serverless AWS rewrite architecture — stack choices, project structure, build phases |
| [`.specs/GAME_RULES.md`](./.specs/GAME_RULES.md) | The prediction/scoring rules in detail, including known bugs |
| [`.specs/fwc26_rules.md`](./.specs/fwc26_rules.md) | FIFA World Cup 26 competition rules — tournament structure, tiebreakers, knockout bracket |
| [`.specs/data_sources.md`](./.specs/data_sources.md) | Tournament data sources — FotMob calendar feed, TheSportsDB, and the ingestion flow |
| [`.specs/thesportsdb_api.md`](./.specs/thesportsdb_api.md) | TheSportsDB API reference — endpoints and the World Cup 26 ingestion subset |

## `archive/`

The complete legacy Google App Engine application, kept as the behavioral
ground truth to consult while rewriting. Not deployed, not maintained.
