//! Spectra UC1/UC3 emit helpers for counter-app server functions.
//!
//! Call these from Higgs `#[server]` bodies (see `super::server`) after you
//! have enough context for labels (`auth`, `outcome`, `operation`,
//! `error_kind`). Typed recorders come from `counter-app-spectra-topics`, which
//! is generated from the `schemas/*.rs` Spectra DSL files in this package.
//!
//! Prefer [`crate::into_server_error`] for the failure counter path when mapping
//! [`crate::CounterServerError`]; use `record_server_error` when a call site needs
//! an explicit UC1 emit without going through that mapper.

mod request_trace;

pub use request_trace::{
    log_request_step, record_get_request, record_increment_request, record_server_error,
};
