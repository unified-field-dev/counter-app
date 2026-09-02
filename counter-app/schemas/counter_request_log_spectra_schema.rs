#![allow(clippy::too_many_arguments)]

// Spectra UC3 structured log: `spectra_schema!` → `CounterRequestLog`.
//
// Schemas differ from metrics: they declare a `table` and typed `fields` with
// PII / console-safety classifications. Each `log_request_step` call in
// `counter-app` writes one row operators can explore by `operation` and
// `message`. This file is `#[path]`-included by `counter-app-spectra-topics`.
//
// Teaching checklist for a new schema:
// 1. Pick `store` + `table` names (stable once deployed).
// 2. List fields with `r#type` and `classification` (`pii`, `safe_for_console`).
// 3. Generate topics/recorders and call the logger from Higgs server fns.
//
// Keep field names aligned with the logger argument order in
// `logging::log_request_step`.

use spectra::spectra_schema;

spectra_schema! {
    CounterRequestLog {
        store: "counter",
        table: "counter_request_log",
        version: "0.1.0",
        description: "Structured counter server-fn trace steps for operator explore.",
        fields: [
            operation: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            message: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            value_before: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            value_after: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
        ],
    }
}
