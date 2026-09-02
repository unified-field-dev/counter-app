//! Pre-seed background-shaped tables for volume-ramp experiments.

use std::time::Instant;

use anyhow::{Context, Result};
use valence::SDb;

const BATCH: usize = 500;

/// Seed synthetic rows into tables that grow during live server runs.
pub async fn seed_background_tables(db: &SDb, row_count: usize) -> Result<()> {
    if row_count == 0 {
        return Ok(());
    }

    ensure_tables(db).await?;

    let t0 = Instant::now();
    seed_table(db, "chronon_run", row_count, |i| {
        format!(
            r#"{{ "job_name": "bench-seed", "status": "completed", "duration_ms": {}, "seq": {} }}"#,
            i % 100,
            i
        )
    })
    .await?;
    seed_table(db, "valence_data_ownership", row_count, |i| {
        format!(
            r#"{{ "valence_model": "bench_model", "record_id": "bench:{}", "owner_id": "system", "owner_type": "system", "status": "active" }}"#,
            i
        )
    })
    .await?;
    seed_table(db, "database_health_snapshot", row_count.min(10_000), |i| {
        format!(
            r#"{{ "instance_id": "default", "healthy": true, "latency_ms": {}, "seq": {} }}"#,
            i % 50,
            i
        )
    })
    .await?;

    let elapsed = t0.elapsed().as_secs_f64();
    println!(
        "[counter-latency-bench] seeded background tables rows~={row_count} elapsed_s={elapsed:.2}"
    );
    Ok(())
}

async fn ensure_tables(db: &SDb) -> Result<()> {
    for table in [
        "chronon_run",
        "valence_data_ownership",
        "database_health_snapshot",
        "bench_kv",
    ] {
        db.query(format!("DEFINE TABLE IF NOT EXISTS {table} SCHEMALESS"))
            .await
            .with_context(|| format!("define table {table}"))?;
    }
    Ok(())
}

async fn seed_table<F>(db: &SDb, table: &str, row_count: usize, row_json: F) -> Result<()>
where
    F: Fn(usize) -> String,
{
    let mut i = 0usize;
    while i < row_count {
        let end = (i + BATCH).min(row_count);
        let mut stmt = String::from("BEGIN TRANSACTION;\n");
        for j in i..end {
            stmt.push_str(&format!(
                "CREATE {table}:seed_{j} CONTENT {};\n",
                row_json(j)
            ));
        }
        stmt.push_str("COMMIT TRANSACTION;");
        db.query(&stmt)
            .await
            .with_context(|| format!("seed batch {table} {i}..{end}"))?;
        i = end;
    }
    Ok(())
}

/// Optional composite index for ownership scan experiments (Experiment D).
pub async fn define_ownership_lookup_index(db: &SDb) -> Result<()> {
    db.query(
        "DEFINE INDEX IF NOT EXISTS idx_valence_data_ownership_model_status \
         ON TABLE valence_data_ownership COLUMNS valence_model, status",
    )
    .await
    .context("define ownership model+status index")?;
    Ok(())
}

/// Run EXPLAIN on the hot ownership pending-deletion query (Experiment E).
pub async fn explain_ownership_pending_query(db: &SDb) -> Result<String> {
    let mut resp = db
        .query(
            "EXPLAIN SELECT VALUE record_id FROM valence_data_ownership \
             WHERE valence_model = $model AND status = 'pending_deletion' AND record_id IN $ids",
        )
        .bind(("model", "bench_model"))
        .bind(("ids", vec!["bench:1".to_string(), "bench:2".to_string()]))
        .await
        .context("EXPLAIN ownership pending query")?;
    let plan: surrealdb::types::Value = resp.take(0).context("take explain plan")?;
    Ok(serde_json::to_string(&plan.into_json_value()).context("serialize explain plan")?)
}
