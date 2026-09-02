//! Experiment F: debug vs release profile note + single run stats.

use anyhow::Result;

use crate::bench_run::{run_increment_timed, warmup, BenchOp};
use crate::experiments::{self, ExperimentOptions};
use crate::seed;
use crate::stack::{BenchRuntime, StackOptions};

pub async fn run(opts: &ExperimentOptions) -> Result<()> {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let rows = opts.volume_sweep.first().copied().unwrap_or(10_000);
    let run_dir = opts.data_dir.join(format!("profile-{profile}"));
    std::fs::create_dir_all(&run_dir)?;

    let stack_opts = StackOptions {
        tier: 0,
        engine: opts.engine,
        store_isolation: opts.store_isolation,
        user_id: opts.user_id.clone(),
        seed_value: opts.seed_value,
        permission_cache: opts.permission_cache,
        data_dir: run_dir,
        boson_worker: false,
        chronon_disable_worker: true,
        chronon_no_jobs: true,
    };

    println!(
        "[counter-latency-bench] experiment=debug-release profile={profile} rows={rows} \
         (compare by re-running with `cargo run --release -p counter-latency-bench`)"
    );

    let runtime = BenchRuntime::boot(&stack_opts).await?;
    seed::seed_background_tables(&runtime.db, rows).await?;
    warmup(&runtime, BenchOp::Increment, opts.warmup).await?;
    let samples = run_increment_timed(&runtime, opts.iterations, false).await?;
    experiments::print_stats_line(
        &format!("increment profile={profile} rows={rows}"),
        &samples.increment_total_ms,
    );

    Ok(())
}
