//! Raw SurrealDB operations (no Valence) for volume-ramp baseline.

use std::time::Instant;

use anyhow::Result;
use valence::SDb;

pub async fn raw_point_read_write(db: &SDb, iterations: usize) -> Result<Vec<f64>> {
    db.query("DEFINE TABLE IF NOT EXISTS bench_kv SCHEMALESS")
        .await?;
    db.query("UPSERT bench_kv:singleton CONTENT { value: 0 }")
        .await?;

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        db.query("UPDATE bench_kv:singleton SET value += 1 RETURN NONE")
            .await?;
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    Ok(samples)
}

/// Mimic unindexed WHERE scan cost on growing table.
pub async fn raw_scan_filter(db: &SDb, iterations: usize) -> Result<Vec<f64>> {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let mut resp = db
            .query(
                "SELECT VALUE record_id FROM valence_data_ownership \
                 WHERE valence_model = $model AND status = 'active' LIMIT 10",
            )
            .bind(("model", "bench_model"))
            .await?;
        let _: surrealdb::types::Value = resp.take(0)?;
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    Ok(samples)
}
