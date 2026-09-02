#[cfg(feature = "tier-spectra-composite")]
use std::sync::Arc;

#[cfg(feature = "tier-spectra-composite")]
use anyhow::{Context, Result};
#[cfg(feature = "tier-spectra-composite")]
use spectra_core::{NdjsonFileSink, SpectraSink};

#[cfg(feature = "tier-spectra-composite")]
use crate::stack::counting_sink::CountingSink;
#[cfg(feature = "tier-spectra-composite")]
use crate::stack::BenchRuntime;

#[cfg(feature = "tier-spectra-composite")]
pub async fn boot(runtime: &mut BenchRuntime) -> Result<()> {
    spectra_core::install_config(spectra_core::SpectraConfig::from_env());

    if spectra::spectra_persist_enabled() {
        let prod_db = Arc::new(runtime.db.clone());
        let isolation = soliton::valence_bootstrap::spectra_store_isolation_from_env();
        if matches!(
            isolation,
            soliton::valence_bootstrap::SpectraStoreIsolationMode::PerStore
        ) {
            let spectra_base = runtime.data_dir.join("surreal/spectra");
            std::env::set_var(
                "SPECTRA_STORE_BASE_PATH",
                spectra_base.to_string_lossy().to_string(),
            );
        }
        let spectra_stores = soliton::valence_bootstrap::bootstrap_spectra_stores_from_inventory(
            prod_db.clone(),
            soliton::valence_bootstrap::spectra_store_isolation_from_env(),
            soliton::valence_bootstrap::embedded_valence_engine_from_env(),
        )
        .await;
        let pairs: Vec<(String, std::sync::Arc<_>)> = spectra_stores
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect();
        spectra::install_spectra_stores(&pairs);

        let router =
            spectra::build_spectra_storage(&spectra_stores).context("build spectra storage")?;
        spectra::configure_storage(router);
    }

    let recording = runtime
        .spectra_recording
        .clone()
        .unwrap_or_else(spectra_core::RecordingSink::new);

    if spectra::spectra_sink_noop() {
        spectra_core::set_sink(Arc::new(spectra_core::NoOpSink));
        runtime.spectra_recording = Some(recording);
        return Ok(());
    }

    let spectra_dir = runtime.data_dir.join("spectra");
    std::fs::create_dir_all(&spectra_dir).context("create spectra data dir")?;
    let ndjson = NdjsonFileSink::new(
        spectra_dir.join("metrics.ndjson"),
        spectra_dir.join("events.ndjson"),
    )
    .context("NdjsonFileSink")?;

    let composite: Arc<dyn SpectraSink> = if spectra::off_thread_emit_enabled() {
        let off_thread = spectra::OffThreadSpectraSink::new(ndjson);
        Arc::new(spectra::host_sink::CompositeSpectraSink::with_inner(
            off_thread,
        ))
    } else {
        Arc::new(spectra::host_sink::CompositeSpectraSink::new(ndjson))
    };

    let inner: Arc<dyn SpectraSink> = if spectra_core::rootcause_enabled() {
        Arc::new(spectra_core::CountingSink::new(composite))
    } else {
        composite
    };
    let counting = CountingSink::new(recording.clone(), inner);
    counting.install();
    runtime.spectra_recording = Some(recording);

    Ok(())
}
