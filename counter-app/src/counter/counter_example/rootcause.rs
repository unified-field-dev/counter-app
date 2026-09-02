//! `COUNTER_ROOTCAUSE`-gated wall-clock timers for counter increment forensics.
//!
//! These helpers wrap `spectra_core` rootcause APIs so increment server fns can
//! time Valence/worker spans and diff Spectra snapshots around a mutate. Enable
//! with the `COUNTER_ROOTCAUSE` environment variable (see `spectra_core::rootcause_enabled`).
//! Call only under `ssr` when diagnosing slow or unexpected increment paths;
//! leave disabled in normal demos.

#[cfg(feature = "ssr")]
use std::future::Future;
#[cfg(feature = "ssr")]
use std::time::Instant;

/// Whether rootcause forensic timing/logging is enabled (via `COUNTER_ROOTCAUSE`).
///
/// Gate expensive timers and snapshot diffs behind this so production-like hosts
/// pay nothing when the env var is unset.
#[cfg(feature = "ssr")]
pub fn enabled() -> bool {
    spectra_core::rootcause_enabled()
}

/// Milliseconds elapsed since `start`.
///
/// Prefer after an [`Instant::now`] taken around a Valence or worker call when
/// [`enabled`] is true.
#[cfg(feature = "ssr")]
pub fn elapsed_ms(start: Instant) -> f64 {
    spectra_core::elapsed_ms(start)
}

/// Log a named timing span with an optional detail string, if rootcause logging is enabled.
///
/// No-op when [`enabled`] is false. Detail may be empty; non-empty strings append
/// after `wall_ms` on the debug line.
#[cfg(feature = "ssr")]
pub fn log_span(name: &str, wall_ms: f64, detail: &str) {
    if !enabled() {
        return;
    }
    if detail.is_empty() {
        log::debug!("[rootcause] span={name} wall_ms={wall_ms:.3}");
    } else {
        log::debug!("[rootcause] span={name} wall_ms={wall_ms:.3} {detail}");
    }
}

/// Run `f`, logging its wall-clock duration under `name` if rootcause logging is enabled.
///
/// Wrap async Valence or service steps in increment server fns:
/// `rootcause::timed("load_counter", || async { … }).await`. When disabled,
/// runs `f` with no timer overhead beyond the branch.
#[cfg(feature = "ssr")]
pub async fn timed<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    if !enabled() {
        return f().await;
    }
    let start = Instant::now();
    let out = f().await;
    log_span(name, elapsed_ms(start), "");
    out
}

/// Capture a Spectra snapshot to diff against after an increment operation.
///
/// Call once before the mutate, then pass the snapshot to
/// [`log_spectra_per_increment`] after success or failure.
#[cfg(feature = "ssr")]
pub fn spectra_snapshot_before() -> spectra_core::RootcauseSnapshot {
    spectra_core::RootcauseSnapshot::capture()
}

/// Capture a fresh snapshot and log the delta against `before` under `label`.
///
/// Use after increment (or on the error path) to see which Spectra counters
/// moved for a single request when `COUNTER_ROOTCAUSE` is on.
#[cfg(feature = "ssr")]
pub fn log_spectra_per_increment(label: &str, before: spectra_core::RootcauseSnapshot) {
    let after = spectra_core::RootcauseSnapshot::capture();
    spectra_core::RootcauseSnapshot::log_per_increment_delta(label, before, after);
}
