//! Counter Spectra telemetry: typed recorders, payloads, and topic constants.
//!
//! Declares Spectra metrics used by the counter example (get/increment request
//! counters, request-log rows, and server-error counters) and re-exports the
//! generated recorder helpers so UI server functions can emit without hand-rolling
//! labels. Pair this crate with `counter-app`'s `into_server_error` path when you
//! want ops-visible failure kinds on demo hosts.
//!
//! ## Features
//!
//! - **Spectra telemetry recorders** — [`CounterServerErrorsRecorder`] and sibling
//!   recorders for get/increment traffic and structured request logs.
//!   [Get started](#record-server-errors)
//! - **Topic payloads** — `*Payload` DTOs and `*_TOPIC` constants re-exported from
//!   the `topics` module for transport-shaped emits.
//!
//! ## Record server errors
//!
//! Server functions map domain failures through `into_server_error`, which records
//! a Spectra counter via [`CounterServerErrorsRecorder::record`] with low-cardinality
//! labels (`operation`, `error_kind`). Call `record` at the UI error boundary when a
//! request fails so dashboards can split validation vs rate-limit vs auth vs Valence
//! failures without storing user payloads.
//!
//! **Prerequisites:** Spectra runtime registered in the host; this crate linked from
//! the UI (or any emitter); choose an `error_kind` string from the demo vocabulary
//! (`validation`, `rate_limited`, `not_authenticated`, `forbidden`, `valence`, …).
//!
//! ```rust,ignore
//! use counter_app_spectra_topics::CounterServerErrorsRecorder;
//! use serde_json::json;
//!
//! let error_kind = "validation";
//! let labels = json!({ "operation": "increment_counter", "error_kind": error_kind });
//! CounterServerErrorsRecorder::record(1, labels);
//! ```
//!
//! On success the counter increments by the recorded amount for those labels. Do not
//! put user ids or emails in `labels`. Next: `counter-app` `into_server_error` and
//! the worker service error variants that map into each `error_kind`.
//!
//! ## Examples
//!
//! Start with [Record server errors](#record-server-errors). UI emit sites live in
//! `counter-app` (`logging` / `into_server_error`). Schema sources are compiled into
//! this crate's `schemas` module.
//!
//! ## Where to look next
//!
//! - [`CounterServerErrorsRecorder`] — primary failure counter.
//! - [`CounterGetRequestsRecorder`] / [`CounterIncrementRequestsRecorder`] — traffic.
//! - [`CounterRequestLogLogger`] — structured step rows for forensics.

#![allow(missing_docs)]

mod helpers;
mod schemas;
mod topics;

pub use helpers::*;
pub use topics::*;
