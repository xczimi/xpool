# xpool

A soccer score-prediction pool for major tournaments (FIFA World Cup, UEFA
Euro). Players predict match scores and compete on a points scoreboard.

> **Status: rewrite in progress.** The original app (Google App Engine,
> Python 2.7) has been moved to [`archive/`](./archive/) for reference. The new
> implementation will be built at the repo root.

## Documentation

| Document | What it covers |
|----------|----------------|
| [`REWRITE_USE_CASES.md`](./REWRITE_USE_CASES.md) | User journeys, scenarios, and use cases — *what the app does*, technology-independent |
| [`REWRITE_IMPLEMENTATION.md`](./REWRITE_IMPLEMENTATION.md) | Domain model, scoring engine, data ingestion, legacy anti-patterns — *how to build it* |
| [`GAME_RULES.md`](./GAME_RULES.md) | The prediction/scoring rules in detail, including known bugs |

## `archive/`

The complete legacy Google App Engine application, kept as the behavioral
ground truth to consult while rewriting. Not deployed, not maintained.
