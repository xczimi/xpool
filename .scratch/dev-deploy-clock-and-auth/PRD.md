# Dev clock + dev/demo auth on the dev deployment

Status: needs-triage
Area: api / deployment (+ web)

## Idea

Make the "dev clock" and "dev/demo auth (no Auth0)" affordances — which work
locally today — available on the deployed **dev** environment too.

## Motivation

Time-dependent behaviour (deadlines, locking, today-window) and auth flows are
hard to demo/test on the dev deployment without these seams. Locally we have
them; the dev deployment currently doesn't, so exercising time-travel and
quick player switching requires running the full local stack.

## Sketch

- **Dev clock:** allow overriding `now` on the dev deployment via the existing
  server-authoritative seam (`X-Dev-Now` header / `XPOOL_NOW` env) so testers
  can time-travel. Keep it strictly off in production.
- **Dev/demo auth:** allow the dev-stub auth path (the `X-Dev-Player` header /
  dev-login route) on the dev deployment so testers can act as the seeded demo
  players without going through Auth0.
- Gate both behind an explicit dev-environment flag so they can never be enabled
  in prod.

## Open questions

- One env flag enabling both, or separate toggles for clock vs auth?
- Does the dev deployment expose a UI for clock/player switching, or is it
  header-only (driven by a tool / browser extension)?
- How does this interact with [[invite-only-hardening]] (prod tightens auth
  while dev deliberately keeps the bypass)?

## Related

- [[invite-only-hardening]]
