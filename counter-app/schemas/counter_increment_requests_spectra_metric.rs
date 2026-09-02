#![allow(clippy::too_many_arguments)]

// Spectra UC1 counter for increment volume: `spectra_metric!` → `CounterIncrementRequests`.
//
// Same DSL shape as the get-requests sibling: `store`, `name`, `version`,
// `description`. Emit via `record_increment_request(auth, outcome)` in the
// increment server fn after the worker call succeeds (or with a documented
// failure `outcome`).
//
// Labels in the description (`auth`, `outcome`) must match the JSON keys passed
// to the generated recorder in `counter-app-spectra-topics`.

use spectra::spectra_metric;

spectra_metric! {
    CounterIncrementRequests {
        store: "counter",
        name: "counter_increment_requests",
        version: "0.1.0",
        description: "Counter increment server-fn volume. Labels: auth, outcome.",
    }
}
