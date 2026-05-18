# 30 — AdminResults follow-up refetches are unguarded

Status: ready-for-agent
Severity: MEDIUM
Area: web

## Problem

`AdminResults.tsx` `refresh()` fires `refetch` / `refetchResults` and discards
the returned promises. A failed refetch after a successful `enterResult` /
`unlockResult` / `recompute` is swallowed — the screen shows stale data with no
error surfaced. The mutation calls themselves correctly `throw res.error`; only
the follow-up refresh is unguarded.

## Expected

Await the refetches and surface a refresh failure (an error notice, or reuse
the existing error-view path), consistent with how the mutation errors are
handled.

## Acceptance

- A failed post-mutation refetch shows an error rather than stale data.
- `web` build + lint + test green.
