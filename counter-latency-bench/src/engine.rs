//! Embedded Surreal engine boot (Mem, RocksDB, or SurrealKV) for bench runs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
#[cfg(feature = "kv-surrealkv")]
use surrealdb::engine::local::SurrealKv;
use surrealdb::engine::local::{Mem, RocksDb};
use valence::{DatabaseRouter, SDb, SurrealMemBackend};

use crate::stack::tier0;

pub const BENCH_NS: &str = "counter_bench";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum BenchEngine {
    #[default]
    Rocksdb,
    Mem,
    #[cfg(feature = "kv-surrealkv")]
    Surrealkv,
}

impl BenchEngine {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rocksdb => "rocksdb",
            Self::Mem => "mem",
            #[cfg(feature = "kv-surrealkv")]
            Self::Surrealkv => "surrealkv",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum BenchStoreIsolation {
    #[default]
    Shared,
    PerLogical,
}

impl BenchStoreIsolation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::PerLogical => "per-logical",
        }
    }
}

pub async fn db_and_router(
    engine: BenchEngine,
    data_dir: &Path,
    isolation: BenchStoreIsolation,
) -> Result<(SDb, Arc<DatabaseRouter>)> {
    match (engine, isolation) {
        (BenchEngine::Mem, BenchStoreIsolation::Shared) => tier0::mem_db_and_router().await,
        (BenchEngine::Mem, BenchStoreIsolation::PerLogical) => {
            per_logical_db_and_router(engine, data_dir).await
        }
        (BenchEngine::Rocksdb, _) => persistent_db_and_router(engine, data_dir, isolation).await,
        #[cfg(feature = "kv-surrealkv")]
        (BenchEngine::Surrealkv, _) => persistent_db_and_router(engine, data_dir, isolation).await,
    }
}

async fn connect_store(engine: BenchEngine, path: &Path) -> Result<SDb> {
    let db = SDb::init();
    match engine {
        BenchEngine::Mem => {
            db.connect::<Mem>(()).await.context("connect bench Mem")?;
        }
        BenchEngine::Rocksdb => {
            if path.exists() {
                let lock = path.join("LOCK");
                if lock.exists() {
                    let _ = std::fs::remove_file(&lock);
                }
            } else {
                std::fs::create_dir_all(path).context("create bench store dir")?;
            }
            db.connect::<RocksDb>(path.to_string_lossy().as_ref())
                .await
                .context("connect bench RocksDB")?;
        }
        #[cfg(feature = "kv-surrealkv")]
        BenchEngine::Surrealkv => {
            if path.exists() {
                let lock = path.join("LOCK");
                if lock.exists() {
                    let _ = std::fs::remove_file(&lock);
                }
            } else {
                std::fs::create_dir_all(path).context("create bench store dir")?;
            }
            db.connect::<SurrealKv>(path.to_string_lossy().as_ref())
                .await
                .context("connect bench SurrealKV")?;
        }
    }
    db.use_ns(BENCH_NS)
        .use_db(BENCH_NS)
        .await
        .context("use ns/db")?;
    Ok(db)
}

async fn persistent_db_and_router(
    engine: BenchEngine,
    data_dir: &Path,
    isolation: BenchStoreIsolation,
) -> Result<(SDb, Arc<DatabaseRouter>)> {
    let mut router = DatabaseRouter::new();
    let primary_path = data_dir.join("surreal/bench");

    match isolation {
        BenchStoreIsolation::Shared => {
            let db = connect_store(engine, &primary_path).await?;
            SurrealMemBackend::register_embedded_logical_names_slices(
                &mut router,
                db.clone(),
                tier0::router_groups(),
            );
            DatabaseRouter::set_global(router);
            Ok((db, DatabaseRouter::global()))
        }
        BenchStoreIsolation::PerLogical => {
            let mut handles: Vec<(&str, SDb)> = Vec::new();
            let mut primary: Option<SDb> = None;
            for group in tier0::router_groups() {
                for &logical in *group {
                    let path = data_dir.join(format!("surreal/{logical}"));
                    let db = connect_store(engine, &path).await?;
                    if logical == "default" {
                        primary = Some(db.clone());
                    }
                    handles.push((logical, db));
                }
            }
            SurrealMemBackend::register_embedded_logical_handles(
                &mut router,
                &handles,
                valence::RegisterEmbeddedLogicalNamesOptions::default(),
            );
            DatabaseRouter::set_global(router);
            Ok((
                primary.context("default logical store missing")?,
                DatabaseRouter::global(),
            ))
        }
    }
}

async fn per_logical_db_and_router(
    engine: BenchEngine,
    data_dir: &Path,
) -> Result<(SDb, Arc<DatabaseRouter>)> {
    persistent_db_and_router(engine, data_dir, BenchStoreIsolation::PerLogical).await
}

pub async fn rocksdb_db_and_router(data_dir: &Path) -> Result<(SDb, Arc<DatabaseRouter>)> {
    persistent_db_and_router(BenchEngine::Rocksdb, data_dir, BenchStoreIsolation::Shared).await
}

pub async fn fresh_db_and_router(
    engine: BenchEngine,
    data_dir: &Path,
    isolation: BenchStoreIsolation,
) -> Result<(SDb, Arc<DatabaseRouter>)> {
    if engine != BenchEngine::Mem {
        let root = data_dir.join("surreal");
        if root.exists() {
            std::fs::remove_dir_all(&root).ok();
        }
    }
    db_and_router(engine, data_dir, isolation).await
}

/// Remove stale bench store dirs for isolation A/B reruns.
pub fn cleanup_data_dir(data_dir: &Path) {
    let root = data_dir.join("surreal");
    if root.exists() {
        let _ = std::fs::remove_dir_all(&root);
    }
}

pub fn logical_paths(data_dir: &Path) -> BTreeMap<String, PathBuf> {
    tier0::router_groups()
        .iter()
        .flat_map(|g| g.iter())
        .map(|&logical| {
            (
                logical.to_string(),
                data_dir.join(format!("surreal/{logical}")),
            )
        })
        .collect()
}
