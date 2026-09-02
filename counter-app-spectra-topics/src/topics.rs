//! Transport `*Payload` DTOs and `*_TOPIC` constants from Counter Spectra schemas.
//!
//! Prefer these when an emit site needs the topic name or a serializable payload
//! shape without going through the recorder helpers. Topic strings match what
//! Spectra registers from the schema inventory when this crate is linked.

pub use crate::schemas::counter_get_requests::{
    CounterGetRequestsPayload, COUNTER_GET_REQUESTS_TOPIC,
};
pub use crate::schemas::counter_increment_requests::{
    CounterIncrementRequestsPayload, COUNTER_INCREMENT_REQUESTS_TOPIC,
};
pub use crate::schemas::counter_request_log::{
    CounterRequestLogPayload, COUNTER_REQUEST_LOG_TOPIC,
};
pub use crate::schemas::counter_server_errors::{
    CounterServerErrorsPayload, COUNTER_SERVER_ERRORS_TOPIC,
};
