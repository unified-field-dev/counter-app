#![allow(clippy::too_many_arguments)]

// Spectra UC1 counter for get volume: `spectra_metric!` → `CounterGetRequests`.
//
// Declare metrics next to the product that emits them. This file is
// `#[path]`-included by `counter-app-spectra-topics`, which expands the macro
// into typed recorders. UI server functions call `record_get_request` from
// `counter-app`'s logging helpers.
//
// Fields that matter for newcomers:
// - `store` — Spectra store key (`"counter"`) shared with sibling metrics/logs
// - `name` — wire/metric name operators query (`counter_get_requests`)
// - `description` — documents label keys (`auth`: anon/user)
//
// Add a new metric the same way when a server fn needs another volume counter;
// keep labels stable once dashboards depend on them.

use spectra::spectra_metric;

spectra_metric! {
    CounterGetRequests {
        store: "counter",
        name: "counter_get_requests",
        version: "0.1.0",
        description: "Counter get server-fn volume. Labels: auth (anon/user).",
    }
}
