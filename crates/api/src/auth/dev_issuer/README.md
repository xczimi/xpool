# Dev-issuer keypair

Test-only RS256 keypair for the `LOCAL_AUTH_ISSUER` JWT path
(see `docs/superpowers/specs/2026-05-30-auth-design.md` §2).

The private key has **no power** unless `LOCAL_AUTH_ISSUER` is set in the
environment, which never happens in production (the env var is unset in
`infrastructure/lambda.tf`, and the `dev_login` module that uses the key
is gated off the `dev_auth` Cargo feature, which is excluded from
`--features lambda` builds).

Regenerate: see the commands in `docs/superpowers/plans/2026-05-30-xpool-auth.md`
Task 2.
