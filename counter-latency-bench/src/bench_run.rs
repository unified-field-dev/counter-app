use anyhow::Result;
use serde::Serialize;

use crate::ops::{run_increment_once, run_read_once, run_write_once};
use crate::snapshot::{capture_iteration_delta, retry_baseline, spectra_baselines, StackSnapshot};
use crate::stack::BenchRuntime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchOp {
    Read,
    Write,
    Increment,
}

impl BenchOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Increment => "increment",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct IncrementIteration {
    pub user_counter_get_ms: f64,
    pub counter_get_ms: f64,
    pub user_counter_commit_ms: f64,
    pub counter_commit_ms: f64,
    pub increment_total_ms: f64,
    pub spectra_events_per_iter: u32,
    pub spectra_gauges_per_iter: u32,
    pub boson_queued: u32,
    pub db_retry_count: u32,
    pub db_retry_total_sleep_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadIteration {
    pub user_counter_get_ms: f64,
    pub counter_get_ms: f64,
    pub read_total_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteIteration {
    pub user_counter_commit_ms: f64,
    pub counter_commit_ms: f64,
    pub write_total_ms: f64,
}

#[derive(Debug, Clone, Default)]
pub struct IncrementSamples {
    pub user_counter_get_ms: Vec<f64>,
    pub counter_get_ms: Vec<f64>,
    pub user_counter_commit_ms: Vec<f64>,
    pub counter_commit_ms: Vec<f64>,
    pub increment_total_ms: Vec<f64>,
    pub spectra_events_per_iter: Vec<f64>,
    pub spectra_gauges_per_iter: Vec<f64>,
    pub boson_queued: Vec<f64>,
    pub db_retry_count: Vec<f64>,
    pub db_retry_total_sleep_ms: Vec<f64>,
    pub rootcause_emits_counter: Vec<f64>,
    pub rootcause_emits_gauge: Vec<f64>,
    pub rootcause_emits_event: Vec<f64>,
    pub rootcause_surreal_writes_metrics: Vec<f64>,
    pub rootcause_surreal_writes_events: Vec<f64>,
    pub rootcause_surreal_batch_flushes_metrics: Vec<f64>,
    pub rootcause_surreal_batch_flushes_events: Vec<f64>,
    pub rootcause_surreal_wall_ms: Vec<f64>,
    pub rootcause_ndjson_appends: Vec<f64>,
    pub rootcause_ndjson_wall_ms: Vec<f64>,
    pub rootcause_inline_wall_ms: Vec<f64>,
    pub rootcause_buffer_pushes: Vec<f64>,
    pub rootcause_buffer_drains: Vec<f64>,
    pub rootcause_drain_wall_ms: Vec<f64>,
    pub rootcause_aggregate_coalesced: Vec<f64>,
}

pub async fn run_increment_timed(
    runtime: &BenchRuntime,
    iterations: usize,
    json: bool,
) -> Result<IncrementSamples> {
    let mut samples = IncrementSamples::default();
    for _ in 0..iterations {
        let (ev_b, ga_b, _) = spectra_baselines(runtime);
        let retry_b = retry_baseline(runtime);
        #[cfg(feature = "tier-spectra")]
        let rc_before = spectra_core::RootcauseSnapshot::capture();

        // Lever B: buffer the foreground increment's emits (no-op unless
        // SPECTRA_REQUEST_BUFFER). The foreground rootcause delta is captured before the
        // drain, so inline/ndjson wall reflects only the cheap buffer pushes; the drain
        // then replays the batch off the measured path.
        let (step, buffered) =
            spectra_core::request_scope(run_increment_once(&runtime.valence, &runtime.user_pk))
                .await;
        let step = step?;

        #[cfg(feature = "tier-spectra")]
        let rc_foreground = spectra_core::RootcauseSnapshot::capture();
        spectra_core::drain(buffered);
        let snap = capture_iteration_delta(runtime, ev_b, ga_b, retry_b).await;

        #[cfg(feature = "tier-spectra")]
        let (rc_delta, rc_drain) = {
            let after_drain = spectra_core::RootcauseSnapshot::capture();
            (
                spectra_core::RootcauseSnapshot::delta(rc_before, rc_foreground),
                spectra_core::RootcauseSnapshot::delta(rc_foreground, after_drain),
            )
        };
        #[cfg(not(feature = "tier-spectra"))]
        let (rc_delta, rc_drain) = (
            spectra_core::RootcauseSnapshot::default(),
            spectra_core::RootcauseSnapshot::default(),
        );

        let row = IncrementIteration {
            user_counter_get_ms: step.user_counter_get_ms,
            counter_get_ms: step.counter_get_ms,
            user_counter_commit_ms: step.user_counter_commit_ms,
            counter_commit_ms: step.counter_commit_ms,
            increment_total_ms: step.increment_total_ms,
            spectra_events_per_iter: snap.spectra_events,
            spectra_gauges_per_iter: snap.spectra_gauges,
            boson_queued: snap.boson_queued,
            db_retry_count: snap.db_retry_count,
            db_retry_total_sleep_ms: snap.db_retry_total_sleep_ms,
        };

        if json {
            println!("{}", serde_json::to_string(&row)?);
        }

        if snap.db_retry_count > 0 {
            eprintln!(
                "[counter-latency-bench] db_retry operation=increment count={} sleep_ms={:.1}",
                snap.db_retry_count, snap.db_retry_total_sleep_ms
            );
        }

        samples.user_counter_get_ms.push(row.user_counter_get_ms);
        samples.counter_get_ms.push(row.counter_get_ms);
        samples
            .user_counter_commit_ms
            .push(row.user_counter_commit_ms);
        samples.counter_commit_ms.push(row.counter_commit_ms);
        samples.increment_total_ms.push(row.increment_total_ms);
        samples
            .spectra_events_per_iter
            .push(row.spectra_events_per_iter as f64);
        samples
            .spectra_gauges_per_iter
            .push(row.spectra_gauges_per_iter as f64);
        samples.boson_queued.push(row.boson_queued as f64);
        samples.db_retry_count.push(row.db_retry_count as f64);
        samples
            .db_retry_total_sleep_ms
            .push(row.db_retry_total_sleep_ms);
        samples
            .rootcause_emits_counter
            .push(rc_delta.emits_counter as f64);
        samples
            .rootcause_emits_gauge
            .push(rc_delta.emits_gauge as f64);
        samples
            .rootcause_emits_event
            .push(rc_delta.emits_event as f64);
        samples
            .rootcause_surreal_writes_metrics
            .push(rc_delta.storage_writes_metrics as f64);
        samples
            .rootcause_surreal_writes_events
            .push(rc_delta.storage_writes_events as f64);
        samples
            .rootcause_surreal_batch_flushes_metrics
            .push(rc_delta.storage_batch_flushes_metrics as f64);
        samples
            .rootcause_surreal_batch_flushes_events
            .push(rc_delta.storage_batch_flushes_events as f64);
        samples
            .rootcause_surreal_wall_ms
            .push(rc_delta.storage_wall_ms);
        samples
            .rootcause_ndjson_appends
            .push(rc_delta.ndjson_appends as f64);
        samples
            .rootcause_ndjson_wall_ms
            .push(rc_delta.ndjson_wall_ms);
        samples
            .rootcause_inline_wall_ms
            .push(rc_delta.inline_wall_ms);
        samples
            .rootcause_buffer_pushes
            .push(rc_delta.buffer_pushes as f64);
        samples
            .rootcause_buffer_drains
            .push(rc_drain.buffer_drains as f64);
        samples.rootcause_drain_wall_ms.push(rc_drain.drain_wall_ms);
        samples
            .rootcause_aggregate_coalesced
            .push(rc_drain.aggregate_coalesced as f64);
    }
    Ok(samples)
}

pub async fn warmup(runtime: &BenchRuntime, op: BenchOp, n: usize) -> Result<()> {
    for _ in 0..n {
        dispatch_op(op, runtime).await?;
    }
    Ok(())
}

async fn dispatch_op(op: BenchOp, runtime: &BenchRuntime) -> Result<()> {
    match op {
        BenchOp::Read => {
            let _ = run_read_once(&runtime.valence, &runtime.user_pk).await?;
        }
        BenchOp::Write => {
            let _ = run_write_once(&runtime.valence, &runtime.user_pk).await?;
        }
        BenchOp::Increment => {
            let _ = run_increment_once(&runtime.valence, &runtime.user_pk).await?;
        }
    }
    Ok(())
}

pub fn print_increment_results(
    runtime: &BenchRuntime,
    op: &str,
    iterations: usize,
    warmup: usize,
    run: Option<(usize, usize)>,
    samples: &IncrementSamples,
    prev_tier_p95: Option<f64>,
) {
    use crate::gates::{amplification_correlates, classify_delta, delta_ms, overall_verdict_label};
    use crate::stats::{MetricReport, Stats};

    if run.is_some() {
        runtime.print_header(op, iterations, warmup, run);
    }

    let mut report = MetricReport::new();
    report.push("user_counter_get_ms", samples.user_counter_get_ms.clone());
    report.push("counter_get_ms", samples.counter_get_ms.clone());
    report.push(
        "user_counter_commit_ms",
        samples.user_counter_commit_ms.clone(),
    );
    report.push("counter_commit_ms", samples.counter_commit_ms.clone());
    report.push("increment_total_ms", samples.increment_total_ms.clone());
    if runtime.tier >= 2 {
        report.push(
            "spectra_events_per_iter",
            samples.spectra_events_per_iter.clone(),
        );
        report.push(
            "spectra_gauges_per_iter",
            samples.spectra_gauges_per_iter.clone(),
        );
    }
    if runtime.tier >= 1 {
        report.push("boson_queued", samples.boson_queued.clone());
    }
    report.push("db_retry_count", samples.db_retry_count.clone());
    report.push(
        "db_retry_total_sleep_ms",
        samples.db_retry_total_sleep_ms.clone(),
    );
    report.print_summary(&format!("results op={op}"));

    let curr_p95 = Stats::summarize(samples.increment_total_ms.clone()).p95;
    if let Some(prev) = prev_tier_p95 {
        let delta = delta_ms(Some(prev), curr_p95).unwrap_or(0.0);
        let delta_v = classify_delta(Some(prev), curr_p95);
        let verdict = overall_verdict_label(
            delta_v,
            amplification_correlates(
                delta_v,
                None,
                Stats::summarize(samples.spectra_events_per_iter.clone()).p95,
                None,
                Stats::summarize(samples.spectra_gauges_per_iter.clone()).p95,
                None,
                Stats::summarize(samples.boson_queued.clone()).p95,
                None,
                Stats::summarize(samples.db_retry_count.clone()).p95,
                Stats::summarize(samples.db_retry_total_sleep_ms.clone()).p95,
            ),
        );
        println!(
            "[counter-latency-bench] delta_vs_prev increment_total_p95={delta:+.1}ms verdict={verdict}"
        );
    }
}
