//! Experiment H: hot-key overwrite sweep.

use std::time::Instant;

use anyhow::Result;

use crate::engine;
use crate::experiments::{self, ExperimentOptions};
use crate::stack::tier0;

pub async fn run(opts: &ExperimentOptions) -> Result<()> {
    println!(
        "[counter-latency-bench] experiment=hot-key overwrite_sweep={:?}",
        opts.overwrite_sweep
    );

    for &overwrites in &opts.overwrite_sweep {
        let run_dir = opts.data_dir.join(format!("hot-key-{overwrites}"));
        std::fs::create_dir_all(&run_dir)?;

        let (_db, router) =
            engine::fresh_db_and_router(opts.engine, &run_dir, opts.store_isolation).await?;
        let (valence, user_pk) = tier0::seed_counters(
            router,
            &opts.user_id,
            opts.seed_value,
            opts.permission_cache,
        )
        .await?;

        for _ in 0..overwrites {
            let _ = crate::ops::run_increment_once(&valence, &user_pk).await?;
        }

        let mut read_samples = Vec::with_capacity(opts.iterations);
        let mut write_samples = Vec::with_capacity(opts.iterations);

        for _ in 0..opts.iterations {
            let t0 = Instant::now();
            let _ = crate::ops::run_read_once(&valence, &user_pk).await?;
            read_samples.push(t0.elapsed().as_secs_f64() * 1000.0);

            let t1 = Instant::now();
            let _ = crate::ops::run_increment_once(&valence, &user_pk).await?;
            write_samples.push(t1.elapsed().as_secs_f64() * 1000.0);
        }

        experiments::print_stats_line(
            &format!("hot_key_read overwrites={overwrites}"),
            &read_samples,
        );
        experiments::print_stats_line(
            &format!("hot_key_increment overwrites={overwrites}"),
            &write_samples,
        );
    }

    Ok(())
}
