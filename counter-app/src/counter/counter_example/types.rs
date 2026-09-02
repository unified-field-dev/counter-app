//! Wire payloads for counter server functions and the live UI.
//!
//! These DTOs cross the Higgs `#[server]` boundary (serde) and drive Orbital
//! page state. Domain persistence stays in Valence via `counter-app-worker`;
//! this module only shapes what the browser receives.
//!
//! Use [`CounterData`] when a page branches on anonymous vs authenticated
//! session. Use [`CounterResponse`] / [`UserCounterResponse`] when the call site
//! already knows which shape it needs (global get/set vs user get/increment).

use serde::{Deserialize, Serialize};

/// Maximum amount accepted by a single increment request (abuse guard).
///
/// Server functions and the worker service reject larger `amount` values with a
/// validation [`crate::CounterServerError`]. Keep UI
/// batching under this ceiling so one flush cannot exceed the guard.
pub const MAX_INCREMENT_AMOUNT: usize = 10_000;

/// Counter payload shown to the live page after get or increment.
///
/// - [`Global`](Self::Global) — anonymous sessions (shared public count only)
/// - [`User`](Self::User) — authenticated sessions (personal + global counts)
///
/// Build this in the client resource after `counter_get` / `user_counter_get`,
/// or receive it from `increment_counter` / `user_counter_increment`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CounterData {
    /// Anonymous-session view: global counter only.
    Global(CounterResponse),
    /// Authenticated-session view: personal and global counters.
    User(UserCounterResponse),
}

/// Response payload for the global (anonymous) counter.
///
/// Returned by `counter_get`, `counter_set`, and anonymous `increment_counter`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterResponse {
    /// Current global counter value.
    pub value: usize,
}

/// Response payload for per-user + global counters on authenticated sessions.
///
/// Returned by `user_counter_get` and `user_counter_increment` after Valence
/// reads both the caller's `UserCounter` and the shared global counter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCounterResponse {
    /// Current value of the caller's personal counter.
    pub user_count: usize,
    /// Current value of the shared global counter.
    pub global_count: usize,
}
