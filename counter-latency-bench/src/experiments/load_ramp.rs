//! Experiment B: concurrency / capacity load ramp.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tokio::sync::Barrier;
use tokio::task::JoinSet;

use crate::bench_run::{warmup, BenchOp};
use crate::experiments::{self, ExperimentOptions};
use crate::ops::run_increment_once;
use crate::seed;
use crate::stack::{BenchRuntime, StackOptions};

pub async fn run(opts: &ExperimentOptions) -> Result<()> {
    let rows = opts.volume_sweep.first().copied().unwrap_or(0);
    let run_dir = opts.data_dir.join("load-ramp");
    std::fs::create_dir_all(&run_dir)?;

    let stack_opts = StackOptions {
        tier: 0,
        engine: opts.engine,
        store_isolation: opts.store_isolation,
        user_id: opts.user_id.clone(),
        seed_value: opts.seed_value,
        permission_cache: opts.permission_cache,
        data_dir: run_dir.clone(),
        boson_worker: false,
        chronon_disable_worker: true,
        chronon_no_jobs: true,
    };

    println!(
        "[counter-latency-bench] experiment=load-ramp engine={} concurrency_sweep={:?}",
        opts.engine.label(),
        opts.concurrency_sweep
    );

    let runtime = BenchRuntime::boot(&stack_opts).await?;
    seed::seed_background_tables(&runtime.db, rows).await?;
    for &k in &opts.concurrency_sweep {
        warmup(&runtime, BenchOp::Increment, opts.warmup).await?;
        let samples = in_process_concurrency(&runtime, k, opts.http_duration_secs).await?;
        let throughput = samples.len() as f64 / opts.http_duration_secs as f64;
        experiments::print_stats_line(
            &format!("in_process concurrency={k} throughput={throughput:.1} incr/s"),
            &samples,
        );
    }

    if let Some(url) = &opts.http_url {
        run_http_load(
            url,
            &opts.http_path,
            &opts.concurrency_sweep,
            opts.http_duration_secs,
        )
        .await?;
    }

    Ok(())
}

async fn in_process_concurrency(
    runtime: &BenchRuntime,
    concurrency: usize,
    duration_secs: u64,
) -> Result<Vec<f64>> {
    let runtime = Arc::new(runtime.clone());
    let barrier = Arc::new(Barrier::new(concurrency));
    let deadline = Instant::now() + Duration::from_secs(duration_secs);
    let mut set = JoinSet::new();

    for _ in 0..concurrency {
        let rt = Arc::clone(&runtime);
        let bar = Arc::clone(&barrier);
        set.spawn(async move {
            bar.wait().await;
            let mut local = Vec::new();
            while Instant::now() < deadline {
                let t0 = Instant::now();
                if run_increment_once(&rt.valence, &rt.user_pk).await.is_ok() {
                    local.push(t0.elapsed().as_secs_f64() * 1000.0);
                }
            }
            local
        });
    }

    let mut all = Vec::new();
    while let Some(res) = set.join_next().await {
        all.extend(res.context("join concurrency task")?);
    }
    Ok(all)
}

async fn run_http_load(
    base_url: &str,
    path: &str,
    sweep: &[usize],
    duration_secs: u64,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build reqwest client")?;
    let path = path.trim();
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);

    for &k in sweep {
        let deadline = Instant::now() + Duration::from_secs(duration_secs);
        let barrier = Arc::new(Barrier::new(k));
        let mut set = JoinSet::new();
        for _ in 0..k {
            let client = client.clone();
            let url = url.clone();
            let bar = Arc::clone(&barrier);
            set.spawn(async move {
                bar.wait().await;
                let mut samples = Vec::new();
                while Instant::now() < deadline {
                    let t0 = Instant::now();
                    match client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .body(r#"{"amount":1}"#)
                        .send()
                        .await
                    {
                        Ok(r) if r.status().is_success() => {
                            samples.push(t0.elapsed().as_secs_f64() * 1000.0);
                        }
                        _ => {}
                    }
                }
                samples
            });
        }
        let mut all = Vec::new();
        while let Some(res) = set.join_next().await {
            all.extend(res.context("http load task")?);
        }
        let throughput = all.len() as f64 / duration_secs as f64;
        experiments::print_stats_line(
            &format!("http concurrency={k} throughput={throughput:.1} req/s"),
            &all,
        );
    }
    Ok(())
}
