# Reminder opt-out (DND) flag

Status: needs-triage
Area: api + web

## Idea

A first-class **opt-out / do-not-disturb flag** so a recipient can be excluded
from deadline reminders **without** manually editing their identity data.

## Motivation

Today there is no opt-out flag. Removing someone from the reminder sweep means
running `xtask clear-verified-email <addr> --apply`, which blanks
`verified_email` on every matching `Identity`. That works but it's a blunt
manual hack with downsides:

- **Coarse:** it nukes the email entirely, not just reminders. It also disables
  invite-*re*-linking by that email (`find_identities_by_verified_email`).
- **Not self-service:** the admin (me) has to run a CLI command against prod for
  every opt-out request. The email body only offers a "reply to be removed" line.
- **Fragile:** signing in via a brand-new provider whose verified email matches
  re-creates an identity carrying the address, silently re-subscribing them.

A proper flag is strictly better: reminders off, login + email + linking
untouched, and (ideally) the user flips it themselves.

## Sketch

- **Flag on `Person`** — reminders are per-person (the sweep dedups per
  `person|group`), so a `reminders_opt_out: bool` (default false) on `Person` is
  the natural home, not on `Identity`.
- **Sweep respects it** — `mail::sweep` skips opted-out persons, ideally counting
  them in a distinct `skipped_opt_out` (separate from `skipped_no_email`) so the
  summary stays honest.
- **Self-service:**
  - One-click **unsubscribe link** in the email body (token/HMAC over person id,
    no login needed) — replaces the current "reply to be removed" line.
  - A **toggle** on the player's own settings / `/me` page for signed-in users.
- **Keep `verified_email` intact** — login keys on `(provider, provider_id)` and
  invite-linking uses the email, so leaving it set is correct; only the new flag
  gates reminders.
- **Retire the manual hack** — keep `xtask clear-verified-email` as a break-glass
  tool, but the flag becomes the normal path.

## Open questions

- Single global opt-out, or per-channel (last-call vs digest) granularity?
- Token scheme for the unsubscribe link (stateless HMAC vs stored token)?
- Where exactly does the toggle live — `/me`, a dedicated settings page, or the
  invite/profile flow?

## Migration note

One recipient was opted out the manual way on 2026-06-29 (their `verified_email`
was blanked on both of their identities). When this ships, restore that
`verified_email` and set the opt-out flag instead, so login/linking is normal.
See the SES-reminders opt-out decision and `xtask clear-verified-email`.
