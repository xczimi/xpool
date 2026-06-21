# Prediction stats on the match page — most common scoreline, etc.

Status: needs-triage
Area: web (+ possibly api)

## Idea

On the match page (`/match/:gameId`), show aggregate stats about what players
**predicted** for this match — e.g. the most common scoreline, distribution of
outcomes (home / draw / away), how many picked the eventual result.

## Motivation

A score-prediction pool is social. "What did everyone else tip?" is one of the
most fun questions, especially once a match opens. The match page is the natural
home for it, and it complements the [[sportsdb-live-preview]] tip grid already
there.

## Sketch

- After the visibility gate opens for a match (deadline passed / match started),
  aggregate all participants' predictions for it:
  - Most common scoreline(s) + count.
  - Home/draw/away split.
  - Optionally "N players nailed it" once the result is in.
- Respect tip-visibility rules — only aggregate predictions that are already
  revealable (don't leak still-locked tips).
- May need a small GraphQL aggregation resolver, or compute client-side from the
  already-loaded tip grid if that's complete enough.

## Open questions

- Pool-scoped (this pool only) or tournament-wide aggregate?
- Compute server-side (new resolver) or client-side from the existing grid?
- Show before kickoff (counts only, no scorelines) or strictly after the gate?
