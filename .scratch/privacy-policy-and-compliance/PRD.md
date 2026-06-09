# Privacy policy + third-party data/compliance basics

Status: MVP shipped (2026-06-09) — minimal bilingual `/privacy` page
Area: web / legal / api

## Resolution (MVP)

Shipped a minimal, self-authored `/privacy` page (`web/src/pages/PrivacyPage.tsx`),
footer-linked, public route, EN/HU body switched by locale. Covers: data
collected (email, nick, full name, predictions, referrer), storage (DynamoDB on
AWS ca-central-1), processors (Auth0/Okta, Google sign-in, AWS, Amazon SES),
functional-cookies-only (no banner), retention, and request-based access/
deletion at `pool@xczimi.com`. Verified against infra: SES send permission is
wired (`infrastructure/lambda.tf`).

Decisions: minimal scope — no Terms of Service, no self-serve deletion UI, no
cookie consent banner, no DPAs. Contact address: `pool@xczimi.com`.

**Follow-ups (not done):**
- **Hungarian copy needs a native-speaker review pass** (Peter) — the HU body
  was machine-authored.
- Self-serve account deletion/export UI (today deletion is request-by-email).
- Terms of Service, if the pool ever opens beyond friends-and-family.
- DPAs with Auth0 / Google if scope grows.

## Idea

Add a privacy policy and the basic legal/compliance plumbing needed because the
app uses third parties (Google sign-in, Auth0) and will likely integrate more —
so the project is properly covered before/while it goes public.

## Motivation

The app authenticates real people via Auth0 (and Google as an identity
provider), stores personal data (email, nickname, predictions), and may add more
third-party integrations later. With Hungarian + English users, GDPR-style
obligations are very likely in scope. "Want to make sure I'm covered" — a clear
privacy policy and a known list of data processors is the baseline.

## Sketch

- **Privacy policy page** (e.g. a new `PrivacyPage`, linked from the footer /
  chrome): what data is collected, why, where it's stored (DynamoDB), retention,
  and user rights (access / deletion).
- **Third-party / data-processor list:** Auth0 (auth), Google (sign-in
  provider), email (MailHog locally / a real sender in prod), hosting — kept up
  to date as integrations are added.
- **Consent / cookie notice** if cookies or tracking are used (check what Auth0
  sets).
- **Account deletion / data export** path so a user can exercise their rights
  (ties into the player/Auth0 model).
- All copy i18n'd (EN + HU) in `web/src/i18n/strings.ts` — there are currently
  **no** privacy strings.

## Open questions

- Which jurisdiction(s) drive the requirements (Hungary/EU GDPR is the likely
  baseline)? Is a Terms of Service needed alongside the privacy policy?
- Self-authored policy vs. a generator/template reviewed by a human?
- Where does account deletion actually delete from (Auth0 identity + the
  DynamoDB player record + predictions)?
- Do we need a Data Processing Agreement with Auth0 / Google?

## Related

- [[invite-only-hardening]] — tightening signup interacts with what data we
  collect and from whom.
- [[dev-deploy-clock-and-auth]] — the no-Auth0 dev path means dev/test data
  isn't real users; worth noting in the policy's scope.
