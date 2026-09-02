//! Product-local counter service (Valence-backed, no Higgs / Leptos).
//!
//! `counter-app` server functions are thin wrappers: they resolve session +
//! Valence via Higgs, then call these APIs. Integration tests exercise the same
//! surface without spinning a host or compiling the Leptos UI graph.
//!
//! Each fallible entry takes `&Valence` and calls generated [`crate::generated`]
//! models (`get` / `get_mutable` / `commit` / `upsert`). Prefer this module when
//! teaching the Valence write path without Chronon or Boson in the frame.
//!
//! ## Errors
//!
//! Fallible APIs return [`CounterServiceError`]:
//! - [`CounterServiceError::Validation`] — permanent client input rejection
//! - [`CounterServiceError::Forbidden`] — cross-user personal-counter mutation
//! - [`CounterServiceError::RateLimited`] — transient; callers may retry shortly
//! - [`CounterServiceError::Valence`] — data-plane / authz failure from Valence
//!
//! The UI crate maps these into `CounterServerError` → `ServerFnError` and
//! records Spectra `counter_server_errors` by `error_kind`.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use valence::actor::Actor;
use valence::{Model, RecordId, Valence};

use crate::anon_rate_limit;
use crate::generated::{Counter, UserCounter};

/// Maximum amount accepted by a single increment request (abuse guard).
pub const MAX_INCREMENT_AMOUNT: usize = 10_000;

/// Lower per-request cap for anonymous increments (CA-05).
pub const MAX_ANON_INCREMENT_AMOUNT: usize = 100;

/// Counter service errors (mapped to `ServerFnError` by the UI crate).
#[derive(Error, Debug)]
pub enum CounterServiceError {
    /// A request payload failed validation, with a human-readable reason.
    ///
    /// Permanent for this request; fix the amount (or other input) before retrying.
    #[error("validation failed: {0}")]
    Validation(String),

    /// Caller is not allowed to mutate the requested personal counter.
    ///
    /// Permanent for this actor / `user_id` pair (IDOR / cross-user write).
    #[error("not authorized to mutate this user counter")]
    Forbidden,

    /// Anonymous increment budget exhausted (CA-05 in-process token bucket).
    ///
    /// Transient — safe to retry after a short delay.
    #[error("anonymous increment rate limit exceeded; try again shortly")]
    RateLimited,

    /// A Valence data-access error occurred.
    #[error(transparent)]
    Valence(#[from] valence::Error),
}

/// Response payload for the global (anonymous) counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterResponse {
    /// Current global counter value.
    pub value: usize,
}

/// Response payload for the per-user + global counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserCounterResponse {
    /// Current value of the caller's personal counter.
    pub user_count: usize,
    /// Current value of the shared global counter.
    pub global_count: usize,
}

fn count_to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or(0)
}

/// Reject zero and oversize increment amounts (abuse guard).
///
/// # Errors
///
/// Returns [`CounterServiceError::Validation`] when `amount` is zero or above
/// [`MAX_INCREMENT_AMOUNT`].
pub fn validate_increment_amount(amount: usize) -> Result<(), CounterServiceError> {
    if amount == 0 {
        return Err(CounterServiceError::Validation(
            "amount must be greater than 0".into(),
        ));
    }
    if amount > MAX_INCREMENT_AMOUNT {
        return Err(CounterServiceError::Validation(format!(
            "amount must be at most {MAX_INCREMENT_AMOUNT}"
        )));
    }
    Ok(())
}

/// Anonymous increment guard: tighter per-request cap plus a simple in-process rate limit.
///
/// # Errors
///
/// - [`CounterServiceError::Validation`] — zero / oversize amount
/// - [`CounterServiceError::RateLimited`] — per-minute anonymous budget exhausted
pub fn validate_anon_increment(amount: usize) -> Result<(), CounterServiceError> {
    validate_increment_amount(amount)?;
    if amount > MAX_ANON_INCREMENT_AMOUNT {
        return Err(CounterServiceError::Validation(format!(
            "anonymous amount must be at most {MAX_ANON_INCREMENT_AMOUNT}"
        )));
    }
    if !anon_rate_limit::allow_request() {
        return Err(CounterServiceError::RateLimited);
    }
    Ok(())
}

fn amount_as_i64(amount: usize) -> Result<i64, CounterServiceError> {
    i64::try_from(amount)
        .map_err(|e| CounterServiceError::Validation(format!("amount out of range: {e}")))
}

/// Strip a leading `table:` prefix so `user:alice` and `alice` compare equal.
fn bare_record_id(raw: &str) -> &str {
    raw.split_once(':').map_or(raw, |(_, id)| id)
}

/// Defense in depth: only the matching user actor (or System) may mutate `user_id`.
///
/// Server functions already bind `user_id` from the session; this guard closes the
/// create-path IDOR when a caller passes a foreign id under their own Valence.
fn ensure_may_mutate_user_counter(v: &Valence, user_id: &str) -> Result<(), CounterServiceError> {
    let target = bare_record_id(user_id);
    match v.actor() {
        Actor::System { .. } => Ok(()),
        Actor::User { user_id: actor_id } if bare_record_id(actor_id) == target => Ok(()),
        _ => Err(CounterServiceError::Forbidden),
    }
}

/// Read the global singleton counter (missing row → 0).
///
/// Calls [`Counter::get`] with the fixed id `"singleton"`. When no row exists yet
/// (before the first increment or set), the response value is `0` rather than an
/// error — the demo treats absence as a fresh counter.
///
/// Pass a [`Valence`] whose actor satisfies the Counter read policy (`PUBLIC_READ`).
/// UI server functions obtain that handle from Higgs; Chronon / Boson scripts use
/// `valence_from_context`.
///
/// # Errors
///
/// Returns [`CounterServiceError::Valence`] when the Valence read fails.
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app_worker::get_global;
///
/// let response = get_global(&valence).await?;
/// assert_eq!(response.value, 0); // or the persisted singleton value
/// ```
pub async fn get_global(v: &Valence) -> Result<CounterResponse, CounterServiceError> {
    // Generated model API: `Counter::get(id, &valence)` — missing row is fine (→ 0).
    let counter = Counter::get("singleton", v).await?;
    let value = counter.map_or(0, |c| count_to_usize(*c.value()));
    Ok(CounterResponse { value })
}

/// Increment the global singleton by `amount` (create on first write).
///
/// Valence call sequence:
/// 1. [`validate_increment_amount`] rejects zero and oversized amounts.
/// 2. [`Counter::get`] with id `"singleton"` loads the existing row, if any.
/// 3. Row present: `Model::get_mutable` → `set_value(next)` → `commit()`.
/// 4. Row missing: [`Counter::new`] then [`Counter::upsert`] with id `"singleton"`.
///
/// The mutable path is the normal update; upsert is only the first-write create.
/// Schema update policy allows `PUBLIC_READ` so anon demos can bump the shared
/// counter (intentional for this teaching app).
///
/// # Errors
///
/// Returns [`CounterServiceError::Validation`] for bad amounts, or
/// [`CounterServiceError::Valence`] on data-plane failure.
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app_worker::{get_global, increment_global};
///
/// let before = get_global(&valence).await?;
/// let after = increment_global(1, &valence).await?;
/// assert_eq!(after.value, before.value + 1);
/// ```
pub async fn increment_global(
    amount: usize,
    v: &Valence,
) -> Result<CounterResponse, CounterServiceError> {
    validate_increment_amount(amount)?;
    let amount_i64 = amount_as_i64(amount)?;

    let counter = Counter::get("singleton", v).await?;
    let updated = if let Some(counter) = counter {
        // Existing row: get_mutable → field setter → commit (Valence unit of work).
        let next = counter.value() + amount_i64;
        counter
            .get_mutable(v)
            .set_value(next)
            .map_err(|e| CounterServiceError::Validation(e.to_string()))?
            .commit()
            .await?
    } else {
        // First write: build a new model and upsert under the fixed singleton id.
        let new_counter =
            Counter::new(amount_i64).map_err(|e| CounterServiceError::Validation(e.to_string()))?;
        Counter::upsert("singleton", new_counter, v).await?
    };

    Ok(CounterResponse {
        value: count_to_usize(*updated.value()),
    })
}

/// Set the global singleton to an explicit value.
///
/// Builds a [`Counter`] via [`Counter::new`] and persists it with
/// [`Counter::upsert`] under id `"singleton"`. Upsert replaces any prior row, so
/// this is the teaching path for "write an absolute value" (tests, daily reset
/// follow-up, admin-style demos) rather than a read-modify-write increment.
///
/// # Errors
///
/// Returns [`CounterServiceError::Validation`] when `value` cannot be stored as
/// `i64`, or [`CounterServiceError::Valence`] on data-plane failure.
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app_worker::{get_global, set_global};
///
/// let set = set_global(10, &valence).await?;
/// assert_eq!(set.value, 10);
/// let again = get_global(&valence).await?;
/// assert_eq!(again.value, 10);
/// ```
pub async fn set_global(value: usize, v: &Valence) -> Result<CounterResponse, CounterServiceError> {
    let value_i64 =
        i64::try_from(value).map_err(|e| CounterServiceError::Validation(e.to_string()))?;
    let new_counter =
        Counter::new(value_i64).map_err(|e| CounterServiceError::Validation(e.to_string()))?;
    // Upsert = replace-or-create under a stable id (admin/demo absolute write).
    let updated = Counter::upsert("singleton", new_counter, v).await?;
    Ok(CounterResponse {
        value: count_to_usize(*updated.value()),
    })
}

/// Read personal + global counters for `user_id` (bare record id, no `user:` prefix).
///
/// Two Valence reads:
/// 1. [`UserCounter::get`] for the caller's personal score (missing → 0).
/// 2. [`Counter::get`] with id `"singleton"` for the shared global value (missing → 0).
///
/// `user_id` is the bare Surreal id segment (`alice`), not `user:alice`. UI server
/// functions strip the table prefix when binding from the session.
///
/// # Errors
///
/// Returns [`CounterServiceError::Valence`] when either Valence read fails.
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app_worker::get_user;
///
/// let pair = get_user("alice", &valence).await?;
/// let _user = pair.user_count;
/// let _global = pair.global_count;
/// ```
pub async fn get_user(
    user_id: &str,
    v: &Valence,
) -> Result<UserCounterResponse, CounterServiceError> {
    let user_counter = UserCounter::get(user_id, v).await?;
    let user_count = user_counter.map_or(0, |c| count_to_usize(*c.value()));

    let global_counter = Counter::get("singleton", v).await?;
    let global_count = global_counter.map_or(0, |c| count_to_usize(*c.value()));

    Ok(UserCounterResponse {
        user_count,
        global_count,
    })
}

/// Increment personal and global counters for `user_id` by `amount`.
///
/// Authz and Valence sequence:
/// 1. Actor check — only the matching [`Actor::User`] or [`Actor::System`] may
///    write; closes create-path IDOR if a foreign id is passed under the caller's
///    Valence.
/// 2. Personal row: [`UserCounter::get`] → `get_mutable` / `commit`, or
///    [`UserCounter::new`] + [`UserCounter::upsert`] on first write (FK is a
///    `user:` [`RecordId`]).
/// 3. Global row: same get → mutate-or-upsert pattern as [`increment_global`].
///
/// Schema create/update for `UserCounter` is `OWNER_BY_USER_FIELD` / `SYSTEM_ONLY`.
/// A successful personal write can fire [`crate::side_effects::LeaderboardNotifier`],
/// which enqueues the Boson leaderboard task off the request path.
///
/// # Errors
///
/// Returns [`CounterServiceError::Forbidden`] when the Valence actor is not the
/// matching user (or System), [`CounterServiceError::Validation`] for bad amounts,
/// or [`CounterServiceError::Valence`] on data-plane failure.
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app_worker::{get_user, increment_user};
///
/// // `valence` must be Actor::User { user_id: "alice" } (or System).
/// let before = get_user("alice", &valence).await?;
/// let after = increment_user("alice", 1, &valence).await?;
/// assert_eq!(after.user_count, before.user_count + 1);
/// assert_eq!(after.global_count, before.global_count + 1);
/// ```
pub async fn increment_user(
    user_id: &str,
    amount: usize,
    v: &Valence,
) -> Result<UserCounterResponse, CounterServiceError> {
    // Defense in depth: schema OWNER policy + this actor check (IDOR guard).
    ensure_may_mutate_user_counter(v, user_id)?;
    validate_increment_amount(amount)?;
    let amount_i64 = amount_as_i64(amount)?;
    // FK to lepton User table — Valence RecordId, not a bare string.
    let user_thing = RecordId::new("user", bare_record_id(user_id));

    // --- personal UserCounter ---
    let user_counter = UserCounter::get(user_id, v).await?;
    let new_user_val = if let Some(counter) = user_counter {
        let next = counter.value() + amount_i64;
        let updated = counter
            .get_mutable(v)
            .set_value(next)
            .map_err(|e| CounterServiceError::Validation(e.to_string()))?
            .commit()
            .await?;
        // Successful commit can run schema `side_effects: [LeaderboardNotifier]`.
        count_to_usize(*updated.value())
    } else {
        let new_counter = UserCounter::new(user_thing, amount_i64)
            .map_err(|e| CounterServiceError::Validation(e.to_string()))?;
        let updated = UserCounter::upsert(user_id, new_counter, v).await?;
        count_to_usize(*updated.value())
    };

    // --- shared global Counter (same pattern as increment_global) ---
    let global_counter = Counter::get("singleton", v).await?;
    let new_global_val = if let Some(counter) = global_counter {
        let next = counter.value() + amount_i64;
        let updated = counter
            .get_mutable(v)
            .set_value(next)
            .map_err(|e| CounterServiceError::Validation(e.to_string()))?
            .commit()
            .await?;
        count_to_usize(*updated.value())
    } else {
        let new_counter =
            Counter::new(amount_i64).map_err(|e| CounterServiceError::Validation(e.to_string()))?;
        let updated = Counter::upsert("singleton", new_counter, v).await?;
        count_to_usize(*updated.value())
    };

    Ok(UserCounterResponse {
        user_count: new_user_val,
        global_count: new_global_val,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limited_display_is_stable() {
        let msg = CounterServiceError::RateLimited.to_string();
        assert!(msg.contains("rate limit"), "got {msg}");
        assert!(!msg.contains("validation failed"), "got {msg}");
    }

    #[test]
    fn forbidden_display_is_stable() {
        let msg = CounterServiceError::Forbidden.to_string();
        assert!(msg.contains("not authorized"), "got {msg}");
        assert!(!msg.starts_with("validation failed"), "got {msg}");
    }

    #[test]
    fn validation_display_keeps_prefix() {
        let msg =
            CounterServiceError::Validation("amount must be greater than 0".into()).to_string();
        assert!(msg.starts_with("validation failed:"), "got {msg}");
    }
}
