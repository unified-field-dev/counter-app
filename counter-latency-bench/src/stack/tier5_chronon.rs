#[cfg(feature = "tier-chronon")]
use std::sync::Arc;

#[cfg(feature = "tier-chronon")]
use anyhow::{Context, Result};

#[cfg(feature = "tier-chronon")]
use crate::stack::{BenchRuntime, StackOptions};

/// Platform job names linked via other crates' inventory — skip when only counter-app-worker is linked.
#[cfg(feature = "tier-chronon")]
const PLATFORM_JOB_SKIP: &[&str] = &[
    "gluon-process-bootstraps",
    "gluon-sync-registry-images",
    "gluon-app-reconcile",
    "gluon-cell-ingress-renew",
    "ensure-gluon-images",
    "gluon-sync-quotes",
    "gluon-sync-reservations",
    "gluon-app-health-check",
    "gluon-reconcile-cells",
    "gluon-expire-handoff-directives",
    "gluon-reconcile-split-build-timeouts",
    "migrate-gluon-cp-tables-to-pion",
    "migrate-permission-principals",
    "shard-health-check",
    "nucleus-db-health-probe",
    "database-health-check",
    "cleanup-expired-sessions",
    "rebalance-evaluate",
    "nucleus-db-reconcile",
    "boson-worker-autoscale",
    "database-reconcile",
    "valence-deletion-orchestrator",
    "valence-iter-orchestrator",
    "ensure-super-user-group",
    "sync-super-user-membership-roles",
];

#[cfg(feature = "tier-chronon")]
pub async fn boot(runtime: &mut BenchRuntime, opts: &StackOptions) -> Result<()> {
    let rt = chronon::runtime::build_chronon_runtime(runtime.db.clone())
        .context("build chronon runtime")?;

    if let Err(e) = rt.backend.load_jobs_from_db().await {
        eprintln!("[counter-latency-bench] chronon load_jobs_from_db: {e:#}");
    }

    if !opts.chronon_no_jobs {
        chronon::runtime::register_default_jobs_embedded_with_skip(
            rt.backend.clone(),
            PLATFORM_JOB_SKIP,
        )
        .await;
    }

    runtime.chronon = Some(Arc::new(rt));
    Ok(())
}
