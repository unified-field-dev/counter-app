//! Counter Spectra schema modules (inventory + typed helpers + topics).
//!
//! Each submodule `#[path]`-includes a schema source from the `counter-app`
//! product `schemas/` tree. Compiling this crate registers Spectra metrics /
//! log schemas via inventory and exposes the generated recorder, payload, and
//! topic symbols that [`crate::helpers`] and [`crate::topics`] re-export.
//!
//! ## Features
//!
//! - **Get / increment request counters** — traffic metrics for demo dashboards.
//! - **Request log rows** — structured step events for forensics.
//! - **Server-error counter** — low-cardinality `operation` / `error_kind` labels
//!   used by `counter-app`'s `into_server_error` path.

#[path = "../../counter-app/schemas/counter_get_requests_spectra_metric.rs"]
pub mod counter_get_requests;

#[path = "../../counter-app/schemas/counter_increment_requests_spectra_metric.rs"]
pub mod counter_increment_requests;

#[path = "../../counter-app/schemas/counter_request_log_spectra_schema.rs"]
pub mod counter_request_log;

#[path = "../../counter-app/schemas/counter_server_errors_spectra_metric.rs"]
pub mod counter_server_errors;
