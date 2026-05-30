# Auth0 setup — xpool

This is a one-time manual checklist. Auth0 configuration is not Terraform-
managed (overkill for one tenant at hobby scale).

## Tenant
- Create one tenant: `xpool`.
- Region: closest to `ca-central-1` (the deployment region) — `EU` or `US`,
  whichever your account allows.

## Application
- Type: **Single-Page Application**.
- Name: `xpool`.
- Allowed Callback URLs:
  - `https://pool.xczimi.com`
  - `https://pool-dev.xczimi.com`
  - `http://localhost:5173`
- Allowed Logout URLs: same three.
- Allowed Web Origins: same three.

## API
- Identifier (audience): `xpool-api`.
- Signing algorithm: RS256.

## Connections
1. **Passwordless email**
   - Custom email provider: SES (`xczimi.com`).
   - Template: link (not code).
2. **Passwordless SMS**
   - Provider: Twilio (your account SID + auth token).
   - Code length: 6.
3. **Google social**
   - Use Auth0's dev keys for first wiring; replace with your own Google
     OAuth credentials before going live.

## Environment variables (per deployment)
- API Lambda:
  - `AUTH0_DOMAIN=<tenant>.auth0.com`
  - `AUTH0_AUDIENCE=xpool-api`
  - `INVITE_CODE_SECRET=<32-byte secret>` (generate per env, store via SSM
    SecureString, plumb to the Lambda env)
- SPA build:
  - `VITE_AUTH0_DOMAIN`
  - `VITE_AUTH0_CLIENT_ID`
  - `VITE_AUTH0_AUDIENCE=xpool-api`

## What is NOT in Auth0
- Sessions: stateless Bearer JWT, validated per request by the API.
- Invite codes: HS256-signed, owned entirely by the app.
- Pending players: lazy — they don't exist until claim.

## Related
- Design: [`docs/superpowers/specs/2026-05-30-auth-design.md`](../superpowers/specs/2026-05-30-auth-design.md)
- Implementation plan: [`docs/superpowers/plans/2026-05-30-xpool-auth.md`](../superpowers/plans/2026-05-30-xpool-auth.md)
- Scenarios: [`.specs/SCENARIOS.md`](../../.specs/SCENARIOS.md) AUTH-* + POOL-*.
