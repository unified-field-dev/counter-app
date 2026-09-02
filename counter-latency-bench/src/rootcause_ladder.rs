use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::bench_run::{run_increment_timed, IncrementSamples};
use crate::stack::{BenchRuntime, StackOptions};
use crate::stats::Stats;

#[derive(Debug, Clone, Serialize)]
pub struct RootcauseVariant {
    pub name: String,
    pub env: Vec<(String, String)>,
    pub increment_p50_ms: f64,
    pub increment_p95_ms: f64,
    pub emits_counter_p50: f64,
    pub emits_gauge_p50: f64,
    pub emits_event_p50: f64,
    pub surreal_writes_metrics_p50: f64,
    pub surreal_writes_events_p50: f64,
    pub surreal_batch_flushes_metrics_p50: f64,
    pub surreal_batch_flushes_events_p50: f64,
    pub surreal_wall_ms_p50: f64,
    pub ndjson_appends_p50: f64,
    pub ndjson_wall_ms_p50: f64,
    pub inline_wall_ms_p50: f64,
    pub buffer_pushes_p50: f64,
    pub buffer_drains_p50: f64,
    pub drain_wall_ms_p50: f64,
    pub aggregate_coalesced_p50: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RootcauseLadderReport {
    pub variants: Vec<RootcauseVariant>,
    pub surreal_off_reduction_pct: Option<f64>,
    pub noop_reduction_pct: Option<f64>,
    pub dispatch_ndjson_cost_ms: Option<f64>,
}

pub async fn run_rootcause_ladder(
    iterations: usize,
    warmup: usize,
    data_dir: PathBuf,
    user_id: String,
) -> Result<RootcauseLadderReport> {
    std::env::set_var("COUNTER_ROOTCAUSE", "1");

    let variants_spec = [
        ("A_baseline", vec![("SPECTRA_PERSIST".into(), "1".into())]),
        (
            "B_surreal_off",
            vec![("SPECTRA_PERSIST".into(), "0".into())],
        ),
        (
            "C_noop",
            vec![
                ("SPECTRA_PERSIST".into(), "0".into()),
                ("SPECTRA_SINK".into(), "noop".into()),
            ],
        ),
        (
            "D_request_buffer",
            vec![
                ("SPECTRA_PERSIST".into(), "1".into()),
                ("SPECTRA_REQUEST_BUFFER".into(), "1".into()),
            ],
        ),
        (
            "E_counter_aggregate",
            vec![
                ("SPECTRA_PERSIST".into(), "1".into()),
                ("SPECTRA_REQUEST_BUFFER".into(), "1".into()),
                ("SPECTRA_COUNTER_AGGREGATE".into(), "1".into()),
            ],
        ),
        (
            "F_batch_txn",
            vec![
                ("SPECTRA_PERSIST".into(), "1".into()),
                ("SPECTRA_PERSIST_BATCH_TXN".into(), "1".into()),
            ],
        ),
        (
            "G_dedicated_store",
            vec![
                ("SPECTRA_PERSIST".into(), "1".into()),
                ("SPECTRA_DEDICATED_STORE".into(), "rocksdb".into()),
            ],
        ),
        (
            "G_per_store",
            vec![
                ("SPECTRA_PERSIST".into(), "1".into()),
                ("SPECTRA_STORE_ISOLATION".into(), "per-store".into()),
            ],
        ),
        (
            "H_all_levers",
            vec![
                ("SPECTRA_PERSIST".into(), "1".into()),
                ("SPECTRA_REQUEST_BUFFER".into(), "1".into()),
                ("SPECTRA_COUNTER_AGGREGATE".into(), "1".into()),
                ("SPECTRA_PERSIST_BATCH_TXN".into(), "1".into()),
                ("SPECTRA_DEDICATED_STORE".into(), "rocksdb".into()),
            ],
        ),
        (
            "H_all_levers_per_store",
            vec![
                ("SPECTRA_PERSIST".into(), "1".into()),
                ("SPECTRA_REQUEST_BUFFER".into(), "1".into()),
                ("SPECTRA_COUNTER_AGGREGATE".into(), "1".into()),
                ("SPECTRA_PERSIST_BATCH_TXN".into(), "1".into()),
                ("SPECTRA_STORE_ISOLATION".into(), "per-store".into()),
            ],
        ),
    ];

    let mut variants = Vec::new();
    for (name, env_pairs) in variants_spec {
        // Each variant re-declares its toggles; clear the optional ones a prior variant
        // may have set (env is read fresh per call, so this is the in-process control).
        std::env::remove_var("SPECTRA_SINK");
        std::env::remove_var("SPECTRA_REQUEST_BUFFER");
        std::env::remove_var("SPECTRA_COUNTER_AGGREGATE");
        std::env::remove_var("SPECTRA_PERSIST_BATCH_TXN");
        std::env::remove_var("SPECTRA_DEDICATED_STORE");
        std::env::remove_var("SPECTRA_DEDICATED_STORE_PATH");
        std::env::remove_var("SPECTRA_STORE_ISOLATION");
        std::env::remove_var("SPECTRA_STORE_BASE_PATH");
        for (k, v) in &env_pairs {
            std::env::set_var(k, v);
        }
        if env_pairs
            .iter()
            .any(|(k, v)| k == "SPECTRA_DEDICATED_STORE" && (v == "rocksdb" || v == "1"))
        {
            let spectra_path = data_dir.join(name).join("surreal/spectra");
            std::env::set_var(
                "SPECTRA_DEDICATED_STORE_PATH",
                spectra_path.to_string_lossy().to_string(),
            );
        }
        if env_pairs
            .iter()
            .any(|(k, v)| k == "SPECTRA_STORE_ISOLATION" && v == "per-store")
        {
            let spectra_base = data_dir.join(name).join("surreal/spectra");
            std::env::set_var(
                "SPECTRA_STORE_BASE_PATH",
                spectra_base.to_string_lossy().to_string(),
            );
        }

        let stack_opts = StackOptions {
            tier: StackOptions::max_tier_available().max(4),
            engine: crate::engine::BenchEngine::Rocksdb,
            store_isolation: crate::engine::BenchStoreIsolation::Shared,
            user_id: user_id.clone(),
            seed_value: 0,
            permission_cache: true,
            data_dir: data_dir.join(name),
            boson_worker: false,
            chronon_disable_worker: false,
            chronon_no_jobs: true,
        };
        stack_opts.ensure_tier_available()?;

        let runtime = BenchRuntime::boot(&stack_opts).await?;
        crate::bench_run::warmup(&runtime, crate::bench_run::BenchOp::Increment, warmup).await?;
        let samples = run_increment_timed(&runtime, iterations, false).await?;
        variants.push(summarize_variant(name, env_pairs, samples));
    }

    let a_p95 = variants
        .iter()
        .find(|v| v.name == "A_baseline")
        .map(|v| v.increment_p95_ms);
    let b_p95 = variants
        .iter()
        .find(|v| v.name == "B_surreal_off")
        .map(|v| v.increment_p95_ms);
    let c_p95 = variants
        .iter()
        .find(|v| v.name == "C_noop")
        .map(|v| v.increment_p95_ms);

    let surreal_off_reduction_pct = match (a_p95, b_p95) {
        (Some(a), Some(b)) if a > 0.0 => Some(((a - b) / a) * 100.0),
        _ => None,
    };
    let noop_reduction_pct = match (a_p95, c_p95) {
        (Some(a), Some(c)) if a > 0.0 => Some(((a - c) / a) * 100.0),
        _ => None,
    };
    let dispatch_ndjson_cost_ms = match (b_p95, c_p95) {
        (Some(b), Some(c)) => Some((b - c).max(0.0)),
        _ => None,
    };

    Ok(RootcauseLadderReport {
        variants,
        surreal_off_reduction_pct,
        noop_reduction_pct,
        dispatch_ndjson_cost_ms,
    })
}

fn summarize_variant(
    name: &str,
    env: Vec<(String, String)>,
    samples: IncrementSamples,
) -> RootcauseVariant {
    RootcauseVariant {
        name: name.to_string(),
        env,
        increment_p50_ms: Stats::summarize(samples.increment_total_ms.clone()).p50,
        increment_p95_ms: Stats::summarize(samples.increment_total_ms.clone()).p95,
        emits_counter_p50: Stats::summarize(samples.rootcause_emits_counter.clone()).p50,
        emits_gauge_p50: Stats::summarize(samples.rootcause_emits_gauge.clone()).p50,
        emits_event_p50: Stats::summarize(samples.rootcause_emits_event.clone()).p50,
        surreal_writes_metrics_p50: Stats::summarize(
            samples.rootcause_surreal_writes_metrics.clone(),
        )
        .p50,
        surreal_writes_events_p50: Stats::summarize(
            samples.rootcause_surreal_writes_events.clone(),
        )
        .p50,
        surreal_batch_flushes_metrics_p50: Stats::summarize(
            samples.rootcause_surreal_batch_flushes_metrics.clone(),
        )
        .p50,
        surreal_batch_flushes_events_p50: Stats::summarize(
            samples.rootcause_surreal_batch_flushes_events.clone(),
        )
        .p50,
        surreal_wall_ms_p50: Stats::summarize(samples.rootcause_surreal_wall_ms.clone()).p50,
        ndjson_appends_p50: Stats::summarize(samples.rootcause_ndjson_appends.clone()).p50,
        ndjson_wall_ms_p50: Stats::summarize(samples.rootcause_ndjson_wall_ms.clone()).p50,
        inline_wall_ms_p50: Stats::summarize(samples.rootcause_inline_wall_ms.clone()).p50,
        buffer_pushes_p50: Stats::summarize(samples.rootcause_buffer_pushes.clone()).p50,
        buffer_drains_p50: Stats::summarize(samples.rootcause_buffer_drains.clone()).p50,
        drain_wall_ms_p50: Stats::summarize(samples.rootcause_drain_wall_ms.clone()).p50,
        aggregate_coalesced_p50: Stats::summarize(samples.rootcause_aggregate_coalesced.clone())
            .p50,
    }
}

pub fn print_report(report: &RootcauseLadderReport) {
    println!("=== Arena B rootcause ladder (tier-full) ===");
    for v in &report.variants {
        println!(
            "{name}: increment p50={:.2}ms p95={:.2}ms | emits c/g/e={:.0}/{:.0}/{:.0} | \
             surreal m/e={:.0}/{:.0} batch_flushes m/e={:.0}/{:.0} wall={:.2}ms | ndjson={:.0}/{:.2}ms | inline={:.2}ms | \
             buffer push/drain={:.0}/{:.0} drain_wall={:.2}ms aggregate_coalesced={:.0}",
            v.increment_p50_ms,
            v.increment_p95_ms,
            v.emits_counter_p50,
            v.emits_gauge_p50,
            v.emits_event_p50,
            v.surreal_writes_metrics_p50,
            v.surreal_writes_events_p50,
            v.surreal_batch_flushes_metrics_p50,
            v.surreal_batch_flushes_events_p50,
            v.surreal_wall_ms_p50,
            v.ndjson_appends_p50,
            v.ndjson_wall_ms_p50,
            v.inline_wall_ms_p50,
            v.buffer_pushes_p50,
            v.buffer_drains_p50,
            v.drain_wall_ms_p50,
            v.aggregate_coalesced_p50,
            name = v.name,
        );
    }
    if let Some(pct) = report.surreal_off_reduction_pct {
        println!("A-B Surreal-off reduction: {pct:.1}% of baseline p95");
    }
    if let Some(ms) = report.dispatch_ndjson_cost_ms {
        println!("B-C dispatch+NDJSON cost (p95 delta): {ms:.2}ms");
    }
    if let Some(pct) = report.noop_reduction_pct {
        println!("A-C noop reduction: {pct:.1}% of baseline p95");
    }
}

pub fn write_report(path: &std::path::Path, report: &RootcauseLadderReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create report dir")?;
    }
    std::fs::write(path, serde_json::to_string_pretty(report)?).context("write report")?;
    Ok(())
}
