# 30 — AdminResults follow-up refetches are unguarded

Status: done
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

## Comments

urql v5's `useQuery` re-execute returns `void`, so the refetch promise cannot
be awaited. Instead surfaced `resultsQuery.error` as a `refreshFailed` error
notice (new i18n key, en+hu); a failed `RESULTS_QUERY` refetch after a mutation
now shows an error rather than silently leaving stale data. Tournament-refetch
failures were already covered by the existing `result.error` ErrorView.
