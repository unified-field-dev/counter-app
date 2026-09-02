//! Typed Spectra emit helpers for counter server functions.
//!
//! Each function wraps a generated recorder/logger from
//! `counter-app-spectra-topics`. Call at the points in a Higgs server fn where
//! the metric or log row should fire (start of get, successful increment,
//! classified failure). Labels must match the Spectra schema/metric
//! descriptions in `schemas/` so operator explore and dashboards stay aligned.

use counter_app_spectra_topics::{
    CounterGetRequestsRecorder, CounterIncrementRequestsRecorder, CounterRequestLogLogger,
    CounterServerErrorsRecorder,
};
use serde_json::json;

/// Emit a structured UC3 trace row for a server-fn step.
///
/// Use around meaningful stages (`starting`, `loaded counter`, `incremented`)
/// so Spectra explore can reconstruct a request. Fields map to
/// `counter_request_log` (`operation`, `message`, `value_before`, `value_after`).
pub fn log_request_step(operation: &str, message: &str, value_before: &str, value_after: &str) {
    let () = CounterRequestLogLogger::log(
        operation.to_string(),
        message.to_string(),
        value_before.to_string(),
        value_after.to_string(),
    );
}

/// Record a counter get request (UC1 metric `counter_get_requests`).
///
/// Pass `auth` as `"anon"` or `"user"` (or the label set your schema documents).
/// Call once per successful path entry after Higgs context is known.
pub fn record_get_request(auth: &str) {
    let labels = json!({ "auth": auth });
    let () = CounterGetRequestsRecorder::record(1, labels);
}

/// Record a counter increment request (UC1 metric `counter_increment_requests`).
///
/// Labels: `auth` and `outcome` (for example `"ok"`). Call after the worker
/// increment succeeds, or with a failure outcome when you track denied attempts
/// separately from `record_server_error`.
pub fn record_increment_request(auth: &str, outcome: &str) {
    let labels = json!({ "auth": auth, "outcome": outcome });
    let () = CounterIncrementRequestsRecorder::record(1, labels);
}

/// Record a server-fn failure (UC1 metric `counter_server_errors`).
///
/// Labels: `operation` and `error_kind`. Prefer [`crate::into_server_error`] when
/// the failure is already a [`crate::CounterServerError`]; use this helper for
/// explicit emits that mirror the same label vocabulary.
pub fn record_server_error(operation: &str, error_kind: &str) {
    let labels = json!({ "operation": operation, "error_kind": error_kind });
    let () = CounterServerErrorsRecorder::record(1, labels);
}
