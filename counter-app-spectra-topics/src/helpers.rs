//! Typed Spectra record helpers re-exported from Counter schema modules.
//!
//! Each `*Recorder` / logger wraps Spectra `record` (or structured log emit) with
//! the labels declared in the sibling `schemas` sources. UI server functions call
//! these after mapping domain errors — see crate-root **Record server errors**.

pub use crate::schemas::counter_get_requests::CounterGetRequestsRecorder;
pub use crate::schemas::counter_increment_requests::CounterIncrementRequestsRecorder;
pub use crate::schemas::counter_request_log::{CounterRequestLog, CounterRequestLogLogger};
pub use crate::schemas::counter_server_errors::CounterServerErrorsRecorder;
