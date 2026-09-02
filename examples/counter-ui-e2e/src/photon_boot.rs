//! In-process mem Photon for counter-ui-e2e (`/ws/counter`).

use std::sync::Arc;

use anyhow::Result;
use photon::{configure, Photon};

/// Lab-only transport key (same as photon-leptos examples). Not for production.
const DEV_TRANSPORT_KEY: &str = "cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=";

/// Build and configure the process-wide [`Photon`] instance (required for publish + WS).
pub fn build_photon() -> Result<Arc<Photon>> {
    if std::env::var_os("PHOTON_TRANSPORT_KEY").is_none() {
        // SAFETY: e2e host boot only; sets lab transport before Photon builder.
        unsafe {
            std::env::set_var("PHOTON_TRANSPORT_KEY", DEV_TRANSPORT_KEY);
        }
    }
    let photon = Photon::builder().auto_registry().build()?;
    configure(photon.clone());
    Ok(Arc::new(photon))
}
