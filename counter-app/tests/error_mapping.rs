//! Counter server error → `ServerFnError` mapping (non-UI).
//!
//! Run with: `cargo test -p counter-app --test error_mapping --features ssr`

#![cfg(feature = "ssr")]
#![allow(missing_docs)]

use counter_app::{into_server_error, CounterServerError};
use leptos::prelude::ServerFnError;

#[test]
fn not_authenticated_maps_to_server_error_message_sad() {
    let err = into_server_error(CounterServerError::NotAuthenticated);
    match err {
        ServerFnError::ServerError(msg) => assert_eq!(msg, "not authenticated"),
        other => panic!("expected ServerError, got {other:?}"),
    }
}

#[test]
fn validation_maps_to_server_error_message_sad() {
    let err = into_server_error(CounterServerError::Validation("bad value".into()));
    match err {
        ServerFnError::ServerError(msg) => {
            assert!(msg.contains("validation failed"), "got {msg}");
            assert!(msg.contains("bad value"), "got {msg}");
        }
        other => panic!("expected ServerError, got {other:?}"),
    }
}

#[test]
fn rate_limited_maps_to_server_error_message_sad() {
    let err = into_server_error(CounterServerError::RateLimited);
    match err {
        ServerFnError::ServerError(msg) => {
            assert!(msg.contains("rate limit"), "got {msg}");
            assert!(!msg.contains("validation failed"), "got {msg}");
        }
        other => panic!("expected ServerError, got {other:?}"),
    }
}
