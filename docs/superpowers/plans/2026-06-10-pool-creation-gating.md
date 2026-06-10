# Pool-creation gating + solo-pool handover hint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hide the create-pool form from users who aren't allowed to create pools, and replace the silently-absent handover control on solo pools with a clear "invite someone first" hint.

**Architecture:** The API gains one computed boolean on the `me` viewer — `mayCreatePool` — derived from the existing `domain::pool::may_create_pool` (the same rule `createPool` enforces). The frontend reads that flag to gate the create form, and changes the owner-only handover block from an `&&`-hide into a ternary that shows the dropdown when there's a member to hand to and a hint otherwise. No change to `transfer_ownership` or the member-only handover rule.

**Tech Stack:** Rust (axum + async-graphql, `crates/api`), React + TS + urql (`web/`), Playwright (`web/e2e`).

**Branch:** All work goes on a branch/worktree `pool-creation-gating` (crate + web source — never straight to `master`). Spec: `docs/superpowers/specs/2026-06-10-pool-creation-gating-handover-hint-design.md`.

---

## File structure

- `crates/api/src/gql/types.rs` — add `may_create_pool: bool` to the viewer `Player` SimpleObject + default it in the `From` impl.
- `crates/api/src/gql/mutation.rs` — widen `result_user_id` visibility to `pub(crate)` so the query side can reuse it.
- `crates/api/src/gql/query.rs` — in the `me` resolver's `Player` branch, compute `may_create_pool` and override the default.
- `crates/api/tests/me_permissions.rs` — new test: `me { mayCreatePool }` is `true` for an admin (referred by result-user) and `false` for a plain joiner.
- `web/src/graphql/types.ts` — add `mayCreatePool` to the `Player` interface.
- `web/src/graphql/queries.ts` — select `mayCreatePool` in `ME_QUERY`.
- `web/src/pages/PoolsPage.tsx` — gate the create form on `mayCreatePool`; turn the handover block into dropdown-or-hint.
- `web/src/i18n/strings.ts` — new `handOverNeedsMember` key (en + hu).
- `web/e2e/pools.spec.ts` — add: authorized user sees the create form; a solo owned pool shows the hint (not the dropdown); a multi-member owned pool shows the dropdown.

---

## Task 1: Expose `mayCreatePool` on the `me` viewer (backend)

**Files:**
- Create: `crates/api/tests/me_permissions.rs`
- Modify: `crates/api/src/gql/types.rs:250-282`
- Modify: `crates/api/src/gql/mutation.rs:51`
- Modify: `crates/api/src/gql/query.rs:148-153`

- [ ] **Step 1: Write the failing API test**

Create `crates/api/tests/me_permissions.rs`. `common::ALICE` is seeded with `referrer = result-user` (an admin → may create); `common::BOB` has no referrer (a plain joiner → may not). This mirrors the harness in `crates/api/tests/invite_mutations.rs`.

```rust
//! The `me` viewer exposes `mayCreatePool`, computed from the same referral
//! rule (`domain::pool::may_create_pool`) that gates the `createPool` mutation:
//! the result user and its direct referrals may create pools; everyone else
//! cannot ("restricted creation, open inviting").

mod common;

use common::{ALICE, BOB};

const ME_MAY_CREATE: &str =
    r#"{"query":"query { me { ... on Player { mayCreatePool } } }"}"#;

#[tokio::test]
async fn me_reports_may_create_pool_for_a_result_user_referral() {
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, ALICE, "alice@dev.invalid").await;

    let res = common::query_as(&app, ALICE, ME_MAY_CREATE).await;
    assert!(res.get("errors").is_none(), "query errored: {res:?}");
    assert_eq!(
        res["data"]["me"]["mayCreatePool"],
        serde_json::json!(true),
        "ALICE is referred by the result-user → may create pools"
    );
}

#[tokio::test]
async fn me_denies_may_create_pool_for_a_plain_joiner() {
    let (app, repo) = common::test_app_with_local_auth().await;
    common::seed_identity_for(&repo, BOB, "bob@dev.invalid").await;

    let res = common::query_as(&app, BOB, ME_MAY_CREATE).await;
    assert!(res.get("errors").is_none(), "query errored: {res:?}");
    assert_eq!(
        res["data"]["me"]["mayCreatePool"],
        serde_json::json!(false),
        "BOB has no result-user referrer → may not create pools"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p api --test me_permissions`
Expected: FAIL — the GraphQL response carries an `errors` entry like `Unknown field "mayCreatePool" on type "Player"`, so the `assert!(res.get("errors").is_none())` (or the value assertion) fails.

- [ ] **Step 3: Add the field to the viewer `Player` type**

In `crates/api/src/gql/types.rs`, add the field to the struct (after `is_result_user`) and a default in the `From` impl. The default is `false` (least privilege); the `me` resolver overrides it with the real value in Step 5. The two mutation call-sites of `Player::from` (`mutation.rs:455,631`) return the acting player, whose `mayCreatePool` is never read off a mutation result — urql uses the document `cacheExchange`, so the `me` query's value is not overwritten.

Struct (around `types.rs:251`):

```rust
/// The current player plus their predictions (`me`).
#[derive(SimpleObject, Clone, Debug)]
pub struct Player {
    pub id: String,
    pub nick: String,
    pub full_name: String,
    pub is_result_user: bool,
    /// Whether this viewer may create pools — the result user or one of its
    /// direct referrals (`domain::pool::may_create_pool`). Computed in the `me`
    /// resolver; defaults to `false` here (least privilege) for the
    /// mutation-return call-sites that never expose it.
    pub may_create_pool: bool,
    pub version: u64,
    pub match_predictions: Vec<MatchPrediction>,
    pub standings_predictions: Vec<StandingsPrediction>,
}
```

`From` impl (around `types.rs:262`), add the field:

```rust
impl From<&domain::Player> for Player {
    fn from(p: &domain::Player) -> Self {
        Player {
            id: p.id.clone(),
            nick: p.nick.clone(),
            full_name: p.full_name.clone(),
            is_result_user: p.is_result_user,
            may_create_pool: false,
            version: p.version,
            match_predictions: p
                .match_predictions
                .iter()
                .map(MatchPrediction::from)
                .collect(),
            standings_predictions: p
                .standings_predictions
                .iter()
                .map(StandingsPrediction::from)
                .collect(),
        }
    }
}
```

- [ ] **Step 4: Make `result_user_id` reusable from the query side**

In `crates/api/src/gql/mutation.rs:51`, change the helper's visibility from private to `pub(crate)`:

```rust
pub(crate) async fn result_user_id(repo: &dyn Repository) -> async_graphql::Result<String> {
```

(Leave the body unchanged.)

- [ ] **Step 5: Compute `may_create_pool` in the `me` resolver**

In `crates/api/src/gql/query.rs`, replace the `CurrentPlayer::Player(p)` arm (currently lines 151-153) so it loads the result-user id and overrides the default via struct-update syntax (immutable — builds a new `Player`):

```rust
            CurrentPlayer::Player(p) => {
                let repo = ctx.data_unchecked::<Arc<dyn Repository>>();
                let ruid = crate::gql::mutation::result_user_id(repo.as_ref()).await?;
                let player = Player {
                    may_create_pool: domain::pool::may_create_pool(p.as_ref(), &ruid),
                    ..Player::from(p.as_ref())
                };
                Ok(Some(Viewer::Player(Box::new(player))))
            }
```

`Arc` and `Repository` are already imported in `query.rs` (used by the `AuthenticatedUnclaimed` arm and the `pools` resolver). `domain::pool::may_create_pool` is reachable — `domain` is already a dependency; no new `use` needed (call it fully-qualified as written).

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p api --test me_permissions`
Expected: PASS (both tests).

- [ ] **Step 7: Verify the workspace still builds and is warning-clean**

Run: `cargo build -p api && cargo clippy -p api -- -D warnings`
Expected: builds, no warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/api/src/gql/types.rs crates/api/src/gql/mutation.rs crates/api/src/gql/query.rs crates/api/tests/me_permissions.rs
git commit -m "feat(api): expose mayCreatePool on the me viewer"
```

---

## Task 2: Gate the create-pool form on `mayCreatePool` (frontend)

**Files:**
- Modify: `web/src/graphql/types.ts:82-92`
- Modify: `web/src/graphql/queries.ts:41-57`
- Modify: `web/src/pages/PoolsPage.tsx:41-42` and `:152-167`
- Test: `web/e2e/pools.spec.ts`

- [ ] **Step 1: Write the failing e2e test**

Add to `web/e2e/pools.spec.ts` (an authorized seeded player — every demo player is referred by the result-user, so `demo-grace` may create):

```ts
test('an authorized player sees the create-pool form', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-grace')
  await page.goto('/pools')

  await expect(
    page.getByRole('button', { name: 'Create pool' }),
  ).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run it to confirm the suite runs (it should already PASS — guard test)**

Run: `cd web && npm run e2e -- pools.spec.ts -g "authorized player sees the create-pool form"`
Expected: PASS — the form is currently always rendered. This test guards against the gating in Step 5 accidentally hiding the form from users who *should* see it. (It is not a red test; it pins existing behaviour before the change.)

- [ ] **Step 3: Add `mayCreatePool` to the TS `Player` interface**

In `web/src/graphql/types.ts`, add the field to the `Player` interface (after `isResultUser`):

```ts
export interface Player {
  __typename: 'Player'
  id: string
  nick: string
  fullName: string
  /** The result user IS the admin — gate admin features on this flag. */
  isResultUser: boolean
  /** Whether this viewer may create pools (result user or a direct referral). */
  mayCreatePool: boolean
  version: number
  matchPredictions: MatchPrediction[]
  standingsPredictions: StandingsPrediction[]
}
```

- [ ] **Step 4: Select the field in `ME_QUERY`**

In `web/src/graphql/queries.ts`, add `mayCreatePool` to the `Player` inline fragment:

```ts
      ... on Player {
        id nick fullName isResultUser mayCreatePool version
        matchPredictions { gameId homeScore awayScore locked }
        standingsPredictions { groupId ordering drawOrder locked }
      }
```

- [ ] **Step 5: Gate the create form in `PoolsPage`**

In `web/src/pages/PoolsPage.tsx`, after the `myId` derivation (around line 42), add:

```tsx
  const canCreatePool = me?.__typename === 'Player' && me.mayCreatePool
```

Then wrap **only** the create form (the `<form onSubmit={onCreate}>`, currently lines 153-166) — leave the adjacent join form untouched so unauthorized users can still join via an invite code:

```tsx
      <div className="pool-forms">
        {canCreatePool && (
          <form className="form" onSubmit={onCreate}>
            <label>
              {t('poolName')}
              <input
                required
                placeholder={t('poolNamePlaceholder')}
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
              />
            </label>
            <button type="submit" className="primary" disabled={!newName.trim()}>
              {t('createPool')}
            </button>
          </form>
        )}

        <form className="form" onSubmit={onJoin}>
```

- [ ] **Step 6: Typecheck + lint**

Run: `cd web && npm run build && npm run lint`
Expected: `tsc -b` passes (the `me.mayCreatePool` access typechecks against the updated interface), eslint clean.

- [ ] **Step 7: Re-run the guard e2e**

Run: `cd web && npm run e2e -- pools.spec.ts -g "authorized player sees the create-pool form"`
Expected: PASS — authorized `demo-grace` still sees the form after gating.

- [ ] **Step 8: Commit**

```bash
git add web/src/graphql/types.ts web/src/graphql/queries.ts web/src/pages/PoolsPage.tsx web/e2e/pools.spec.ts
git commit -m "feat(web): hide create-pool form for users who can't create pools"
```

---

## Task 3: Solo-pool handover hint (frontend)

**Files:**
- Modify: `web/src/i18n/strings.ts:222` (en) and `:498` (hu)
- Modify: `web/src/pages/PoolsPage.tsx:298-326`
- Test: `web/e2e/pools.spec.ts`

- [ ] **Step 1: Write the failing e2e tests**

Add to `web/e2e/pools.spec.ts`. First, a solo pool (just the creator) shows the hint and no dropdown:

```ts
test('a solo owned pool shows the handover hint, not the dropdown', async ({
  page,
}) => {
  const net = watchNetwork(page)
  const name = `Solo Cup ${Date.now()}`
  await page.goto('/')
  await devLogin(page, 'demo-grace')
  await page.goto('/pools')

  await page.getByPlaceholder('e.g. Office League').fill(name)
  await page.getByRole('button', { name: 'Create pool' }).click()

  const card = page.locator('.pool-card', { hasText: name })
  await expect(card).toBeVisible()
  await expect(
    card.getByText('Invite someone to this pool before you can hand it over.'),
  ).toBeVisible()
  await expect(card.locator('.handover-select')).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

Second, a multi-member owned pool still shows the dropdown — the seeded "Demo Pool" is owned by `demo-ada` and has all six demo players:

```ts
test('a multi-member owned pool shows the handover dropdown', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/pools')

  const card = page.locator('.pool-card', { hasText: 'Demo Pool' })
  await expect(card).toBeVisible()
  await expect(card.locator('.handover-select')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run them to verify the hint test fails**

Run: `cd web && npm run e2e -- pools.spec.ts -g "handover"`
Expected: the "solo … shows the handover hint" test FAILS (no hint text rendered today; the control is simply absent). The "multi-member … dropdown" test PASSES (existing behaviour — it pins it before the refactor).

- [ ] **Step 3: Add the i18n strings**

In `web/src/i18n/strings.ts`, add the key to the `en` object (after `handOverConfirm`, line 222):

```ts
  handOverNeedsMember: 'Invite someone to this pool before you can hand it over.',
```

and the same key to the `hu` object (after `handOverConfirm`, line 498):

```ts
  handOverNeedsMember: 'Hívj meg valakit a ligába, mielőtt átadhatnád.',
```

- [ ] **Step 4: Render dropdown-or-hint in `PoolsPage`**

In `web/src/pages/PoolsPage.tsx`, the owner-only handover block currently hides the control with `&&` (lines 298-326). Replace that `&&` expression with a ternary so a pool with no other member shows the hint instead of nothing. The `<select>` body is unchanged — only its wrapping condition and the new `:` branch:

```tsx
                      ) : pool.members.some((m) => m !== pool.owner) ? (
                        <select
                          className="handover-select"
                          aria-label={t('handOverTo')}
                          value=""
                          onChange={(e) => {
                            if (e.target.value) {
                              setPendingOwner({
                                poolId: pool.id,
                                memberId: e.target.value,
                              })
                            }
                            e.target.value = ''
                          }}
                        >
                          <option value="" disabled>
                            {t('transferOwnership')}…
                          </option>
                          {pool.members
                            .filter((m) => m !== pool.owner)
                            .map((m) => (
                              <option key={m} value={m}>
                                {displayName(m)}
                              </option>
                            ))}
                        </select>
                      ) : (
                        <span className="hint">{t('handOverNeedsMember')}</span>
                      )}
```

(The `.hint` class already exists in `web/src/index.css:786` — muted mono text — so no new CSS is needed.)

- [ ] **Step 5: Typecheck + lint**

Run: `cd web && npm run build && npm run lint`
Expected: passes. (`StringKey` now includes `handOverNeedsMember`; `hu` is a `Record<StringKey, string>`, so omitting it from either object is a type error — both were added in Step 3.)

- [ ] **Step 6: Run the handover e2e tests**

Run: `cd web && npm run e2e -- pools.spec.ts -g "handover"`
Expected: both PASS — solo pool shows the hint with no `.handover-select`; Demo Pool shows the `.handover-select`.

- [ ] **Step 7: Commit**

```bash
git add web/src/i18n/strings.ts web/src/pages/PoolsPage.tsx web/e2e/pools.spec.ts
git commit -m "feat(web): hint to invite a member before handing over a solo pool"
```

---

## Task 4: Full verification

- [ ] **Step 1: Rust workspace green**

Run: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`
Expected: all pass.

- [ ] **Step 2: Frontend green**

Run: `cd web && npm run build && npm run lint`
Expected: pass.

- [ ] **Step 3: Full e2e suite (catches any seed/regression interaction)**

Run: `cd web && npm run e2e -- pools.spec.ts`
Expected: all pools specs pass (the new three plus the existing list/create/rename/join specs).

- [ ] **Step 4: Merge the branch into `master` locally**

Per the working agreement (solo project, routine change), merge locally:

```bash
git checkout master && git merge --no-ff pool-creation-gating
```

(Open a PR instead only if you want CI/self-review as a record.)

---

## Notes / deliberate scope

- **No backend handover change.** `transfer_ownership` keeps its member-only rule (`PoolError::NotAMember`); the hint just makes the existing constraint legible. Verified design decision (user chose "existing members only").
- **Negative form-visibility is covered at the API layer, not e2e.** Every dev-login-able seeded player is referred by the result-user (all admins), so there is no logged-in-able "cannot create" player without perturbing the shared demo seed. Task 1's `me_denies_may_create_pool_for_a_plain_joiner` test authoritatively covers the `false` branch; the frontend trivially renders on that boolean. Adding a non-admin seeded fixture purely for a negative e2e was judged not worth the seed blast-radius.
