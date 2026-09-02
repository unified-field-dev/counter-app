#![allow(clippy::too_many_arguments)]

// Spectra UC1 counter for server-fn failures: `spectra_metric!` → `CounterServerErrors`.
//
// `level: Error` marks this metric as failure volume in Spectra. The primary
// emit path is `counter-app`'s `into_server_error`, which records `operation`
// and `error_kind` from `CounterServerError` variants. Optional direct emits
// use `record_server_error` in that crate's logging module.
//
// When adding kinds, extend both the enum match in `into_server_error` and any
// dashboards that filter on `error_kind`.

use spectra::spectra_metric;

spectra_metric! {
    CounterServerErrors {
        store: "counter",
        name: "counter_server_errors",
        version: "0.1.0",
        description: "Counter server-fn failures. Labels: operation, error_kind.",
        level: Error,
    }
}
