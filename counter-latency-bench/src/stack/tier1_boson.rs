#[cfg(feature = "tier-boson")]
use std::sync::Arc;

#[cfg(feature = "tier-boson")]
use anyhow::{Context, Result};
#[cfg(feature = "tier-boson")]
use boson_backend_mem::MemQueueBackend;
#[cfg(feature = "tier-boson")]
use boson_coordinator::{ensure_default_task_configs_embedded, CoordinatorAdapter};
#[cfg(feature = "tier-boson")]
use boson_core::{JobStatus, QueueBackend, QueueRouter};
#[cfg(feature = "tier-boson")]
use boson_runtime::{configure, Boson};
#[cfg(feature = "tier-boson")]
use boson_valence_identity::ValenceExecutionContextFactory;

#[cfg(feature = "tier-boson")]
use crate::bench_valence_factory::factory_arc;
#[cfg(feature = "tier-boson")]
use crate::stack::{BenchRuntime, StackOptions};

#[cfg(feature = "tier-boson")]
fn install_mem_backend() {
    let backend: Arc<dyn QueueBackend> = Arc::new(MemQueueBackend::new());
    QueueRouter::set_global(QueueRouter::with_default(backend));
}

#[cfg(feature = "tier-boson")]
pub async fn boot(runtime: &mut BenchRuntime, opts: &StackOptions) -> Result<()> {
    install_mem_backend();
    let vf = factory_arc();
    let identity = Arc::new(ValenceExecutionContextFactory::new(vf));

    let mut builder = Boson::builder()
        .queue_backend_from_global()
        .execution_context_factory_arc(identity)
        .auto_registry();

    if !opts.boson_worker {
        builder = builder.without_worker();
    }

    let boson = builder.build().map_err(|e| anyhow::anyhow!("{e}"))?;

    let coordinator = Arc::new(CoordinatorAdapter::new(Arc::new(boson.clone())));
    configure(boson.clone());
    ensure_default_task_configs_embedded(coordinator)
        .await
        .context("ensure default boson task configs")?;
    runtime.boson = Some(Arc::new(boson));

    Ok(())
}

#[cfg(feature = "tier-boson")]
pub async fn boson_queued_count(runtime: &BenchRuntime) -> u32 {
    let Some(boson) = &runtime.boson else {
        return 0;
    };
    let jobs = boson
        .list_jobs(Some(JobStatus::Queued), 0, 100)
        .await
        .unwrap_or_default();
    jobs.len() as u32
}
