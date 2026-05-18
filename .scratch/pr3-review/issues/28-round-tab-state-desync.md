# 28 — round-tab state can desync from the selected group

Status: done
Severity: MEDIUM
Area: web

## Problem

`MyTipsPage.tsx` / `AllTipsPage.tsx` keep `selectedRound` as state starting
`null`, with `activeRound` falling back to `currentRoundNode(rounds)?.round`.
When the user picks a Group-Stage group pill, `selectedGroupId` is set but
`selectedRound` stays `null`.

If `tournament` later refetches (polling, or an `X-Dev-Now` change) and
`currentRoundNode` now returns a different round — e.g. a deadline passed —
`activeRound` silently flips while `selectedGroupId` still points at a group
from the old round. `roundLeaves.filter(g => g.id === activeGroupId)` then
yields `[]` → the bare "select a group" empty state, with no indication why.

## Expected

Keep the round/group selection coherent: clear `selectedGroupId` when the
derived `activeRound` changes, or store the resolved round once the user
interacts so it no longer floats with `currentRoundNode`.

## Acceptance

- Switching the current round (e.g. via the dev clock) does not strand the
  page in an unexplained empty state.
- `web` build + lint + test green.

## Comments

Added a `useEffect` (placed with the other hooks, before the early returns) in
both `MyTipsPage.tsx` and `AllTipsPage.tsx` that clears `selectedGroupId` when
the derived `activeRound` changes, tracked via a `useRef`. A round flip from a
`tournament` refetch no longer strands the page in the empty group state.
