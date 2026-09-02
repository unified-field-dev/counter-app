//! Experiment C: direct RocksDB floor vs Surreal vs Valence.

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::bench_run::{run_increment_timed, warmup, BenchOp};
use crate::engine;
use crate::experiments::{self, raw_surreal, ExperimentOptions};
use crate::seed;
use crate::stack::{BenchRuntime, StackOptions};

pub async fn run(opts: &ExperimentOptions) -> Result<()> {
    let rows = opts.volume_sweep.first().copied().unwrap_or(10_000);
    let run_dir = opts.data_dir.join("rocksdb-floor");
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
    let runtime = BenchRuntime::boot(&stack_opts).await?;
    seed::seed_background_tables(&runtime.db, rows).await?;

    println!(
        "[counter-latency-bench] experiment=rocksdb-floor rows={rows} iterations={}",
        opts.iterations
    );

    #[cfg(feature = "bench-rocksdb-direct")]
    {
        let rb_path = run_dir.join("rocksdb/direct");
        let direct = direct_rocksdb_ops(&rb_path, rows, opts.iterations).await?;
        experiments::print_stats_line("direct_rocksdb_put_get", &direct.put_get_ms);
        experiments::print_stats_line("direct_rocksdb_scan", &direct.scan_ms);
    }
    #[cfg(not(feature = "bench-rocksdb-direct"))]
    {
        println!(
            "[counter-latency-bench]   direct_rocksdb: skipped (enable --features bench-rocksdb-direct)"
        );
    }

    let raw = raw_surreal::raw_point_read_write(&runtime.db, opts.iterations).await?;
    experiments::print_stats_line("raw_surreal_rmw", &raw);

    warmup(&runtime, BenchOp::Increment, opts.warmup).await?;
    let valence = run_increment_timed(&runtime, opts.iterations, false).await?;
    experiments::print_stats_line("valence_increment", &valence.increment_total_ms);

    Ok(())
}

#[cfg(feature = "bench-rocksdb-direct")]
struct DirectRocksDbSamples {
    put_get_ms: Vec<f64>,
    scan_ms: Vec<f64>,
}

#[cfg(feature = "bench-rocksdb-direct")]
async fn direct_rocksdb_ops(
    path: &Path,
    seed_rows: usize,
    iterations: usize,
) -> Result<DirectRocksDbSamples> {
    use rocksdb::{Options, DB};

    if path.exists() {
        std::fs::remove_dir_all(path).ok();
    }
    std::fs::create_dir_all(path)?;

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, path).context("open direct rocksdb")?;

    for i in 0..seed_rows {
        db.put(format!("ownership:bench:{i}"), b"{\"status\":\"active\"}")
            .context("rocksdb seed put")?;
    }

    let mut put_get_ms = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let v = db.get(b"bench_kv:singleton").context("rocksdb get")?;
        let next = v.as_ref().map(|b| b.len()).unwrap_or(0) as u64 + 1;
        db.put(b"bench_kv:singleton", next.to_string())
            .context("rocksdb put")?;
        put_get_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let mut scan_ms = Vec::with_capacity(iterations.min(20));
    for _ in 0..iterations.min(20) {
        let start = Instant::now();
        let iter = db.prefix_iterator(b"ownership:bench:");
        let mut count = 0usize;
        for item in iter {
            let _ = item.context("rocksdb iter")?;
            count += 1;
            if count >= 10 {
                break;
            }
        }
        scan_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    Ok(DirectRocksDbSamples {
        put_get_ms,
        scan_ms,
    })
}
