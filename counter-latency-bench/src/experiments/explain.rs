//! Experiment E: EXPLAIN query-plan capture.

use anyhow::Result;

use crate::engine;
use crate::experiments::ExperimentOptions;
use crate::seed;

pub async fn run(opts: &ExperimentOptions) -> Result<()> {
    let rows = opts.volume_sweep.first().copied().unwrap_or(10_000);
    let run_dir = opts.data_dir.join("explain");
    std::fs::create_dir_all(&run_dir)?;

    let (db, _) = engine::fresh_db_and_router(opts.engine, &run_dir, opts.store_isolation).await?;
    seed::seed_background_tables(&db, rows).await?;

    println!(
        "[counter-latency-bench] experiment=explain rows={rows} define_index={}",
        opts.define_index
    );

    let plan_before = seed::explain_ownership_pending_query(&db).await?;
    println!("[counter-latency-bench]   explain_before_index={plan_before}");

    if opts.define_index {
        seed::define_ownership_lookup_index(&db).await?;
        let plan_after = seed::explain_ownership_pending_query(&db).await?;
        println!("[counter-latency-bench]   explain_after_index={plan_after}");
    }

    Ok(())
}
