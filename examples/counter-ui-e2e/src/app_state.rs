//! Combined Axum state for Leptos + Photon WS.

use std::sync::Arc;

use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use photon::Photon;
use photon_axum::HasPhoton;

/// App state for the counter UI e2e host (Leptos + Photon).
#[derive(Clone)]
pub struct AppState {
    /// Leptos configuration.
    pub leptos_options: LeptosOptions,
    /// Process-wide Photon handle.
    pub photon: Arc<Photon>,
}

impl HasPhoton for AppState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }

    fn allow_ws_origin(&self, _origin: Option<&str>) -> bool {
        // Lab host only — production hosts use an Origin allowlist.
        true
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> LeptosOptions {
        state.leptos_options.clone()
    }
}
