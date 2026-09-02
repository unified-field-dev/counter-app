//! CounterRoutes Playwright host.
#![allow(missing_docs)]

mod app;
#[cfg(feature = "ssr")]
mod app_state;
mod counter_routes_eager;
#[cfg(feature = "ssr")]
mod e2e_valence;
mod gate_demos;
mod harness_auth_menu;
#[cfg(feature = "ssr")]
mod photon_auth;
#[cfg(feature = "ssr")]
mod photon_boot;
#[cfg(feature = "ssr")]
pub mod seed;

pub use app::{shell, wire_gauge_permissions_bridge, App};
#[cfg(feature = "ssr")]
pub use app_state::AppState;
#[cfg(feature = "ssr")]
pub use e2e_valence::{e2e_higgs_config, e2e_router, init_e2e_valence};
#[cfg(feature = "ssr")]
pub use gate_demos::inject_e2e_session_snapshot;
#[cfg(feature = "ssr")]
pub use photon_auth::E2ePhotonAuth;
#[cfg(feature = "ssr")]
pub use photon_boot::build_photon;
