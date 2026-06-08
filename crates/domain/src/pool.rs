//! Pure pool-membership operations (`SCENARIOS.md` §5).
//!
//! A `Pool` is scoreboard-scoping only. These functions are pure, I/O-free
//! transformations: each takes a `Pool` and returns a new one (or a
//! `PoolError`), never mutating in place. Join-code *generation* and
//! persistence belong to the application layer; this module only enforces the
//! membership rules.

use crate::{Player, Pool};

/// Why a pool operation was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolError {
    /// The result user can never own or join a pool (POOL-12).
    ResultUserExcluded,
    /// The owner is a permanent member — they must delete the pool, not leave
    /// it (POOL-10).
    OwnerCannotLeave,
    /// The action is owner-only and the requester is not the owner.
    NotOwner,
    /// The target player is not a member of the pool.
    NotAMember,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            PoolError::ResultUserExcluded => "the result user cannot join a pool",
            PoolError::OwnerCannotLeave => {
                "the owner cannot leave their own pool — delete it instead"
            }
            PoolError::NotOwner => "only the pool owner may perform this action",
            PoolError::NotAMember => "that player is not a member of the pool",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for PoolError {}

/// Add a player to a pool (POOL-02). Idempotent — re-joining is a no-op.
/// The result user is rejected (POOL-12).
pub fn join(pool: &Pool, player: &Player) -> Result<Pool, PoolError> {
    if player.is_result_user {
        return Err(PoolError::ResultUserExcluded);
    }
    if pool.members.contains(&player.id) {
        return Ok(pool.clone());
    }
    let mut members = pool.members.clone();
    members.push(player.id.clone());
    Ok(Pool {
        members,
        ..pool.clone()
    })
}

/// Remove a player from a pool at their own request (POOL-05). The owner
/// cannot leave (POOL-10); a non-member cannot leave.
pub fn leave(pool: &Pool, player_id: &str) -> Result<Pool, PoolError> {
    if pool.owner == player_id {
        return Err(PoolError::OwnerCannotLeave);
    }
    if !pool.members.iter().any(|m| m == player_id) {
        return Err(PoolError::NotAMember);
    }
    Ok(without_member(pool, player_id))
}

/// Remove a member at the owner's request (POOL-04). Owner-only; the owner
/// cannot be removed.
pub fn remove_member(pool: &Pool, requester_id: &str, member_id: &str) -> Result<Pool, PoolError> {
    if pool.owner != requester_id {
        return Err(PoolError::NotOwner);
    }
    if pool.owner == member_id {
        return Err(PoolError::OwnerCannotLeave);
    }
    Ok(without_member(pool, member_id))
}

/// Rename a pool (POOL-08). Owner-only.
pub fn rename(pool: &Pool, requester_id: &str, name: String) -> Result<Pool, PoolError> {
    if pool.owner != requester_id {
        return Err(PoolError::NotOwner);
    }
    Ok(Pool {
        name,
        ..pool.clone()
    })
}

/// Replace the join code (POOL-03). Owner-only. The caller supplies the new
/// (randomly generated) code.
pub fn set_join_code(
    pool: &Pool,
    requester_id: &str,
    join_code: String,
) -> Result<Pool, PoolError> {
    if pool.owner != requester_id {
        return Err(PoolError::NotOwner);
    }
    Ok(Pool {
        join_code,
        ..pool.clone()
    })
}

/// Whether `player` is permitted to create pools (restricted creation).
///
/// Pool-creation is gated on the referral graph: the result user is its root,
/// and only players it referred (directly) are "admins" who may create pools.
/// The result user itself can never create a pool (POOL-12). Everyone joining a
/// pool via a normal member's invite has a normal-player referrer, so they
/// cannot create pools — "restricted creation, open inviting" as a data rule.
pub fn may_create_pool(player: &Player, result_user_id: &str) -> bool {
    !player.is_result_user && player.referrer.as_deref() == Some(result_user_id)
}

fn without_member(pool: &Pool, player_id: &str) -> Pool {
    Pool {
        members: pool
            .members
            .iter()
            .filter(|m| *m != player_id)
            .cloned()
            .collect(),
        ..pool.clone()
    }
}
