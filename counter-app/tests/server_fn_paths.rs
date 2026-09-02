//! `ServerFn` PATH smoke tests for counter endpoints (non-UI).
//!
//! Run with: `cargo test -p counter-app --test server_fn_paths --features ssr`

#![cfg(feature = "ssr")]
#![allow(missing_docs)]

use counter_app::counter::components::high_scores::GetHighScoresPage;
use counter_app::counter::counter_example::server::{
    CounterGet, CounterIncrement, IncrementCounter, SetCounter, UserCounterGet,
    UserCounterIncrement,
};
use leptos::server_fn::ServerFn;

#[test]
fn counter_server_fn_paths_are_stable_and_nonempty_happy_path() {
    for (label, path) in [
        ("IncrementCounter", IncrementCounter::PATH),
        ("UserCounterIncrement", UserCounterIncrement::PATH),
        ("CounterIncrement", CounterIncrement::PATH),
        ("CounterGet", CounterGet::PATH),
        ("SetCounter", SetCounter::PATH),
        ("UserCounterGet", UserCounterGet::PATH),
        ("GetHighScoresPage", GetHighScoresPage::PATH),
    ] {
        assert!(!path.is_empty(), "{label} PATH should be non-empty");
        assert!(
            path.starts_with('/'),
            "{label} PATH should be absolute, got {path}"
        );
    }
}
