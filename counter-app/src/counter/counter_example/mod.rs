//! Live counter product surface for Unified Field newcomers.
//!
//! This module is the teaching walkthrough behind `/counter`: Orbital layout and
//! pages on the client, Higgs `#[server]` functions on the host, Valence reads and
//! writes via `counter-app-worker`, Photon refetch on `CounterUpdated`, and Spectra
//! counters/logs for get/increment/error volume.
//!
//! Call the server functions from Leptos resources and actions after
//! [`crate::CounterRoutes`] is mounted. Prefer the typed
//! [`crate::CounterServerError`] path and
//! [`crate::into_server_error`] at the UI boundary so failures
//! become `ServerFnError` with Spectra `error_kind` labels.
//!
//! ## Features
//!
//! - **Pages and layout** — [`CounterExamplePage`], [`HighScoresPage`],
//!   [`CounterAdminPage`], and [`CounterLayout`] compose Orbital chrome and auth.
//! - **Server functions** — Re-exported from the `server` module (`counter_get`,
//!   `increment_counter`, `counter_set`, and user-counter variants).
//! - **Typed errors** — [`crate::CounterServerError`] and
//!   [`crate::into_server_error`] for Spectra-aware mapping.
//! - **DTOs** — [`crate::CounterData`], [`crate::CounterResponse`],
//!   [`crate::UserCounterResponse`].
//! - **SSR helpers** — `logging` (Spectra UC1/UC3) and `rootcause` (gated
//!   wall-clock forensics) when the `ssr` feature is on.

/// Error types, Spectra recording, and `ServerFnError` mapping.
pub mod error;
/// Photon topic re-export for live counter updates (`CounterUpdated`).
pub mod events;
/// Orbital shell layout and left-nav links into Valence / Chronon / Boson / Photon.
pub mod layout;
#[cfg(feature = "ssr")]
/// Spectra UC1/UC3 emit helpers used by server functions.
pub mod logging;
/// Live, high-scores, and admin page components.
pub mod pages;
#[cfg(feature = "ssr")]
/// `COUNTER_ROOTCAUSE`-gated timing around increment paths.
pub mod rootcause;
/// Higgs server functions and Photon-leptos `#[synced]` get path.
pub mod server;
/// Wire payloads shared by server functions and the UI.
pub mod types;

pub use error::{
    ctx_err, into_server_error, to_srv_result, CResult, CounterErrorExt, CounterServerError,
    SrvResult,
};

#[cfg(feature = "ssr")]
pub use error::ctx_valence_err;
pub use layout::CounterLayout;
pub use pages::{CounterAdminPage, CounterExamplePage, HighScoresPage};
pub use server::*;
pub use types::{CounterData, CounterResponse, UserCounterResponse, MAX_INCREMENT_AMOUNT};
