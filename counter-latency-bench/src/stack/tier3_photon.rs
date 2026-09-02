#[cfg(feature = "tier-photon")]
use anyhow::{bail, Result};

#[cfg(feature = "tier-photon")]
use crate::stack::BenchRuntime;

/// Photon tier previously used cancelled Surreal embedded bootstrap adapters.
/// Hosts should inject a Photon runtime via family adapters; this example tier
/// is intentionally unavailable until a durable mem/Fluvio boot path is pinned.
#[cfg(feature = "tier-photon")]
pub async fn boot(_runtime: &mut BenchRuntime) -> Result<()> {
    bail!(
        "tier-photon: Surreal embedded Photon bootstrap is retired in this upstream example; \
         run tiers 0–2 or provide host Photon wiring"
    )
}
