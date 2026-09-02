//! Counter product UI: shared components plus the full `/counter` example.
//!
//! Open this module when you want the Leptos surface of the demo app without the
//! crate-root `uf_app!` inventory. Hosts still mount [`crate::CounterRoutes`]; the
//! pieces here are what those routes render and call.
//!
//! ## Features
//!
//! - **Shared components** — Auth warning banner and high-scores table reused by
//!   pages under `counter_example`.
//! - **Counter example** — Layout, pages, Higgs server functions, typed errors,
//!   Photon live wiring, and Spectra emit helpers in `counter_example`.

/// Small shared UI pieces (auth warnings, high-scores leaderboard) reused across pages.
pub mod components;
/// The counter example itself: layout, pages, server functions, and error/logging helpers.
pub mod counter_example;

pub use counter_example::{CounterAdminPage, CounterExamplePage, CounterLayout, HighScoresPage};
