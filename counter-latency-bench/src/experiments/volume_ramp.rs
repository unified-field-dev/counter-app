//! Experiment A: volume ramp + soak.

use std::time::Instant;

use anyhow::Result;

use crate::bench_run::{run_increment_timed, warmup, BenchOp};
use crate::engine::{self, BenchEngine};
use crate::experiments::{self, raw_surreal, ExperimentOptions};
use crate::ops::run_increment_once;
use crate::seed;
use crate::stack::{BenchRuntime, StackOptions};

pub async fn run(opts: &ExperimentOptions) -> Result<()> {
    println!(
        "[counter-latency-bench] experiment=volume-ramp engine={} sweep={:?}",
        opts.engine.label(),
        opts.volume_sweep
    );

    for &rows in &opts.volume_sweep {
        let run_dir = opts.data_dir.join(format!("volume-{}", rows));
        std::fs::create_dir_all(&run_dir)?;

        let (db, router) =
            engine::fresh_db_and_router(opts.engine, &run_dir, opts.store_isolation).await?;
        seed::seed_background_tables(&db, rows).await?;
        if opts.define_index {
            seed::define_ownership_lookup_index(&db).await?;
        }

        let (valence, user_pk) = crate::stack::tier0::seed_counters(
            router.clone(),
            &opts.user_id,
            opts.seed_value,
            opts.permission_cache,
        )
        .await?;

        let runtime = BenchRuntime {
            tier: 0,
            valence,
            user_pk,
            router,
            db: db.clone(),
            permission_cache: opts.permission_cache,
            data_dir: run_dir.clone(),
            boson_worker: false,
            chronon_disable_worker: true,
            chronon_no_jobs: true,
            #[cfg(feature = "tier-boson")]
            boson: None,
            #[cfg(feature = "tier-photon")]
            photon_executor: None,
            #[cfg(feature = "tier-chronon")]
            chronon: None,
            #[cfg(feature = "tier-spectra")]
            spectra_recording: None,
        };

        println!("[counter-latency-bench] volume rows={rows} mode=valence_increment");
        warmup(&runtime, BenchOp::Increment, opts.warmup).await?;
        let samples = run_increment_timed(&runtime, opts.iterations, false).await?;
        experiments::print_stats_line(
            &format!("valence_increment rows={rows}"),
            &samples.increment_total_ms,
        );

        if opts.raw_surreal {
            println!("[counter-latency-bench] volume rows={rows} mode=raw_surreal_rmw");
            let raw = raw_surreal::raw_point_read_write(&db, opts.iterations).await?;
            experiments::print_stats_line(&format!("raw_surreal_rmw rows={rows}"), &raw);

            let scan = raw_surreal::raw_scan_filter(&db, opts.iterations.min(20)).await?;
            experiments::print_stats_line(&format!("raw_surreal_scan rows={rows}"), &scan);
        }

        if let Some(soak) = opts.soak_seconds {
            run_soak(&runtime, &db, rows, soak).await?;
        }
    }

    Ok(())
}

pub async fn run_index_ab(opts: &ExperimentOptions) -> Result<()> {
    println!(
        "[counter-latency-bench] experiment=index-ab sweep={:?}",
        opts.volume_sweep
    );
    for define_index in [false, true] {
        let mut sub = opts.clone();
        sub.define_index = define_index;
        sub.raw_surreal = true;
        sub.soak_seconds = None;
        sub.data_dir = opts.data_dir.join(if define_index {
            "with-index"
        } else {
            "no-index"
        });
        println!("[counter-latency-bench] index-ab define_index={define_index}");
        run(&sub).await?;
    }
    Ok(())
}

pub async fn run_contention(opts: &ExperimentOptions) -> Result<()> {
    println!(
        "[counter-latency-bench] experiment=contention rows={:?} engine={} (requires --features tier-chronon)",
        opts.volume_sweep.first(),
        opts.engine.label(),
    );
    let rows = opts.volume_sweep.first().copied().unwrap_or(10_000);
    let isolation_modes = [
        (
            crate::engine::BenchStoreIsolation::Shared,
            "isolation=shared",
        ),
        (
            crate::engine::BenchStoreIsolation::PerLogical,
            "isolation=per-logical",
        ),
    ];
    for (isolation, iso_label) in isolation_modes {
        for (tier, label) in [(0u8, "jobs_off"), (5u8, "jobs_on")] {
            let run_dir = opts
                .data_dir
                .join(format!("contention-{iso_label}-{label}-{rows}"));
            std::fs::create_dir_all(&run_dir)?;
            engine::cleanup_data_dir(&run_dir);
            let stack_opts = StackOptions {
                tier,
                engine: opts.engine,
                store_isolation: isolation,
                user_id: opts.user_id.clone(),
                seed_value: opts.seed_value,
                permission_cache: opts.permission_cache,
                data_dir: run_dir,
                boson_worker: false,
                chronon_disable_worker: tier < 5,
                chronon_no_jobs: tier < 5,
            };
            if tier >= 5 {
                #[cfg(not(feature = "tier-chronon"))]
                {
                    anyhow::bail!("contention experiment tier 5 requires --features tier-chronon");
                }
            }
            stack_opts.ensure_tier_available()?;
            let runtime = BenchRuntime::boot(&stack_opts).await?;
            seed::seed_background_tables(&runtime.db, rows).await?;
            warmup(&runtime, BenchOp::Increment, opts.warmup).await?;
            let samples = run_increment_timed(&runtime, opts.iterations, false).await?;
            experiments::print_stats_line(
                &format!("contention {iso_label} {label} rows={rows}"),
                &samples.increment_total_ms,
            );
        }
    }
    Ok(())
}

async fn run_soak(
    runtime: &BenchRuntime,
    db: &valence::SDb,
    rows: usize,
    seconds: u64,
) -> Result<()> {
    println!("[counter-latency-bench] soak seconds={seconds} rows={rows}");
    let start = Instant::now();
    let mut bucket = 0u64;
    let mut samples: Vec<f64> = Vec::new();
    while start.elapsed().as_secs() < seconds {
        let t0 = Instant::now();
        let _ = run_increment_once(&runtime.valence, &runtime.user_pk).await?;
        samples.push(t0.elapsed().as_secs_f64() * 1000.0);
        let elapsed_min = start.elapsed().as_secs() / 60;
        if elapsed_min > bucket {
            bucket = elapsed_min;
            experiments::print_stats_line(&format!("soak minute={bucket} rows={rows}"), &samples);
            samples.clear();
        }
    }
    Ok(())
}
