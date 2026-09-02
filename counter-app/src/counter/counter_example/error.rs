//! Typed failures for counter server functions, mapped into Leptos `ServerFnError`.
//!
//! Keep domain failures as [`CounterServerError`] while you still own the request
//! (Higgs context, Valence handles, worker service results). At the last step before
//! returning from a `#[server]` fn, call [`into_server_error`] so the client sees a
//! safe string and Spectra records `counter_server_errors` with an `error_kind` label.
//!
//! Prefer this path over wrapping everything in `anyhow` early: permanent auth and
//! validation variants stay distinguishable until the Spectra emit, and
//! [`RateLimited`](CounterServerError::RateLimited) can still tell clients to retry.

use leptos::prelude::ServerFnError;
use thiserror::Error;

/// Counter-specific server function errors.
///
/// Classification:
/// - [`NotAuthenticated`](Self::NotAuthenticated) / [`Forbidden`](Self::Forbidden) /
///   [`Validation`](Self::Validation) — permanent for this request
/// - [`RateLimited`](Self::RateLimited) — transient; client may retry shortly
/// - [`Valence`](Self::Valence) / [`Unexpected`](Self::Unexpected) — infrastructure / unexpected
#[derive(Error, Debug)]
pub enum CounterServerError {
    /// The caller has no authenticated session but one is required.
    #[error("not authenticated")]
    NotAuthenticated,

    /// The caller is authenticated but not allowed to mutate this personal counter.
    #[error("not authorized to mutate this user counter")]
    Forbidden,

    /// A request payload failed validation, with a human-readable reason.
    #[error("validation failed: {0}")]
    Validation(String),

    /// Anonymous increment budget exhausted (maps from worker `CounterServiceError::RateLimited`).
    #[error("anonymous increment rate limit exceeded; try again shortly")]
    RateLimited,

    /// A Valence data-access error occurred.
    #[cfg(feature = "ssr")]
    #[error(transparent)]
    Valence(#[from] valence::Error),

    /// Any other unexpected error, carried with context via [`anyhow::Error`].
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

/// Map [`CounterServerError`] to Leptos [`ServerFnError`] and record Spectra.
///
/// Call this once at the UI/server-fn boundary (typically
/// `result.map_err(into_server_error)` or [`CounterErrorExt::into_srv`]). Under
/// `ssr`, it emits Spectra `counter_server_errors` with labels
/// `operation=counter` and `error_kind` drawn from the variant
/// (`not_authenticated`, `forbidden`, `validation`, `rate_limited`, `valence`,
/// `unexpected`). The returned [`ServerFnError::ServerError`] message is the
/// `Display` of `e` — safe for the client, no stack traces.
///
/// Log at this boundary only; do not re-log while propagating the same failure.
///
/// # Errors
///
/// Always returns a [`ServerFnError`]; the input error is the source of the
/// user-facing message and the Spectra `error_kind`.
// By-value so it can be passed directly to `Result::map_err`.
#[allow(clippy::needless_pass_by_value)]
pub fn into_server_error(e: CounterServerError) -> ServerFnError {
    #[cfg(feature = "ssr")]
    {
        // Map typed variants → low-cardinality Spectra labels (never put user ids here).
        let (operation, error_kind) = match &e {
            CounterServerError::NotAuthenticated => ("counter", "not_authenticated"),
            CounterServerError::Forbidden => ("counter", "forbidden"),
            CounterServerError::Validation(_) => ("counter", "validation"),
            CounterServerError::RateLimited => ("counter", "rate_limited"),
            CounterServerError::Valence(_) => ("counter", "valence"),
            CounterServerError::Unexpected(_) => ("counter", "unexpected"),
        };
        // Spectra UC1 counter; typed recorder also exists on counter-app-spectra-topics.
        spectra_core::try_record_counter(
            "counter_server_errors",
            &[("operation", operation), ("error_kind", error_kind)],
            1,
        );
    }

    // Leptos wire type: client sees Display text only.
    ServerFnError::ServerError(e.to_string())
}

/// Clamp an `i64` counter value into `usize` for UI payloads (negatives → 0).
///
/// Use when a Valence `i64` field crosses into a Leptos DTO that stores counts as
/// `usize`. Negative stored values become `0` rather than panicking.
pub fn count_to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or(0)
}

/// Result of a counter domain step before the Leptos wire boundary.
///
/// Prefer this inside helpers that still speak [`CounterServerError`]. Convert to
/// [`SrvResult`] with [`to_srv_result`] or [`CounterErrorExt::into_srv`] before
/// returning from a `#[server]` function.
pub type CResult<T> = Result<T, CounterServerError>;

/// Result type returned by counter `#[server]` functions to Leptos.
///
/// Errors are already mapped through [`into_server_error`] (or equivalent).
pub type SrvResult<T> = Result<T, ServerFnError>;

/// Convert a [`CResult`] into a [`SrvResult`] via [`into_server_error`].
///
/// Use at the end of a server fn when you have a domain `Result` and need the
/// Spectra-aware Leptos error type.
///
/// # Errors
///
/// Propagates the mapped [`ServerFnError`] when `result` is `Err`.
pub fn to_srv_result<T>(result: CResult<T>) -> SrvResult<T> {
    result.map_err(into_server_error)
}

/// Extension so `?`-style conversion from [`CResult`] reaches [`ServerFnError`].
///
/// Prefer [`into_srv`](CounterErrorExt::into_srv) on the final `Result` of a
/// helper that still uses [`CounterServerError`], instead of spelling
/// `map_err(into_server_error)` at every call site.
pub trait CounterErrorExt<T> {
    /// Convert `Self` (a [`CResult<T>`]) into a [`SrvResult<T>`].
    ///
    /// # Errors
    ///
    /// Returns [`ServerFnError`] when the inner [`CounterServerError`] is present;
    /// Spectra recording runs inside [`into_server_error`].
    fn into_srv(self) -> Result<T, ServerFnError>;
}

impl<T> CounterErrorExt<T> for CResult<T> {
    fn into_srv(self) -> Result<T, ServerFnError> {
        self.map_err(into_server_error)
    }
}

/// Wrap an unexpected error with an operation label as [`CounterServerError::Unexpected`].
///
/// Call when a non-Valence failure needs context before [`into_server_error`]
/// (Higgs, I/O, parse). The `op` string becomes the anyhow context and usually
/// shows up in operator logs; the client still sees a generic unexpected message
/// once mapped.
pub fn ctx_err(op: &str, e: impl std::error::Error + Send + Sync + 'static) -> CounterServerError {
    CounterServerError::Unexpected(anyhow::Error::new(e).context(op.to_string()))
}

/// Wrap a Valence error with an operation label as [`CounterServerError::Unexpected`].
///
/// Prefer mapping known Valence failures through [`CounterServerError::Valence`]
/// when you want the Spectra `error_kind=valence` path. Use this helper when the
/// failure is "context around a Valence call failed" and should count as
/// `unexpected` after [`into_server_error`].
#[cfg(feature = "ssr")]
pub fn ctx_valence_err(op: &str, e: valence::Error) -> CounterServerError {
    CounterServerError::Unexpected(anyhow::Error::from(e).context(op.to_string()))
}
