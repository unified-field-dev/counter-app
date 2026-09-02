#[cfg(feature = "tier-soliton")]
use std::sync::Arc;

#[cfg(feature = "tier-soliton")]
use anyhow::{Context, Result};
#[cfg(feature = "tier-soliton")]
use surrealdb::engine::local::RocksDb;
#[cfg(feature = "tier-soliton")]
use valence::{DatabaseRouter, SDb, SurrealMemBackend};

#[cfg(feature = "tier-soliton")]
use crate::stack::tier0;

#[cfg(feature = "tier-soliton")]
const BENCH_NS: &str = "counter_bench";

#[cfg(feature = "tier-soliton")]
pub async fn rocksdb_db_and_router(
    data_dir: &std::path::Path,
) -> Result<(SDb, Arc<DatabaseRouter>)> {
    let db_path = data_dir.join("surreal/bench");
    std::fs::create_dir_all(&db_path).context("create bench RocksDB dir")?;

    let lock_path = db_path.join("LOCK");
    if lock_path.exists() {
        let _ = std::fs::remove_file(&lock_path);
    }

    let db = SDb::init();
    db.connect::<RocksDb>(db_path.to_string_lossy().as_ref())
        .await
        .context("connect bench RocksDB")?;
    db.use_ns(BENCH_NS)
        .use_db(BENCH_NS)
        .await
        .context("use ns/db")?;

    let mut router = DatabaseRouter::new();
    SurrealMemBackend::register_embedded_logical_names_slices(
        &mut router,
        db.clone(),
        tier0::router_groups(),
    );
    DatabaseRouter::set_global(router);
    Ok((db, DatabaseRouter::global()))
}
