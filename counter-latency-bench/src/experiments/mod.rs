//! Phase 0 isolation experiments (A–H).

pub mod explain;
pub mod hot_key;
pub mod load_bell;
pub mod load_ramp;
pub mod profile_compare;
pub mod raw_surreal;
pub mod rocksdb_floor;
pub mod volume_ramp;

use std::path::PathBuf;

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExperimentKind {
    /// A: volume ramp — raw Surreal vs Valence at seeded row counts.
    VolumeRamp,
    /// B: concurrency / capacity sweep (in-process + optional HTTP).
    LoadRamp,
    /// M1: open-loop bell-curve HTTP load (1 req/s floor -> peak -> floor).
    LoadBell,
    /// M2: single-connection raw Surreal->RocksDB RMW on bell schedule.
    RawFloorBell,
    /// C: direct RocksDB floor vs Surreal vs Valence.
    RocksdbFloor,
    /// D: index on/off A/B within volume ramp.
    IndexAb,
    /// E: EXPLAIN capture on hot queries.
    Explain,
    /// F: debug vs release profile comparison (document multiplier from env).
    DebugRelease,
    /// G: contention — fixed volume, background job load on/off (via tier 5 chronon).
    Contention,
    /// H: hot-key overwrite sweep.
    HotKey,
}

#[derive(Debug, Clone)]
pub struct ExperimentOptions {
    pub kind: ExperimentKind,
    pub engine: crate::engine::BenchEngine,
    pub store_isolation: crate::engine::BenchStoreIsolation,
    pub data_dir: PathBuf,
    pub user_id: String,
    pub seed_value: i64,
    pub iterations: usize,
    pub warmup: usize,
    pub permission_cache: bool,
    pub volume_sweep: Vec<usize>,
    pub concurrency_sweep: Vec<usize>,
    pub overwrite_sweep: Vec<usize>,
    pub define_index: bool,
    pub raw_surreal: bool,
    pub soak_seconds: Option<u64>,
    pub http_url: Option<String>,
    pub http_path: String,
    pub http_duration_secs: u64,
    pub bell_peak_rps: f64,
    pub bell_floor_rps: f64,
    pub bell_duration_secs: u64,
    pub bell_max_inflight: usize,
    pub bell_bucket_secs: u64,
    pub budget_ms: f64,
    pub report: Option<PathBuf>,
}

impl Default for ExperimentOptions {
    fn default() -> Self {
        Self {
            kind: ExperimentKind::VolumeRamp,
            engine: crate::engine::BenchEngine::Rocksdb,
            store_isolation: crate::engine::BenchStoreIsolation::Shared,
            data_dir: PathBuf::from("profiling/counter-latency-bench/experiments"),
            user_id: "bench-user".to_string(),
            seed_value: 0,
            iterations: 50,
            warmup: 5,
            permission_cache: true,
            volume_sweep: vec![0, 1_000, 10_000, 100_000],
            concurrency_sweep: vec![1, 2, 4, 8, 16, 32],
            overwrite_sweep: vec![1_000, 10_000, 100_000],
            define_index: false,
            raw_surreal: false,
            soak_seconds: None,
            http_url: None,
            http_path: "/api/increment_counter".to_string(),
            http_duration_secs: 10,
            bell_peak_rps: 1000.0,
            bell_floor_rps: 1.0,
            bell_duration_secs: 3600,
            bell_max_inflight: 2000,
            bell_bucket_secs: 60,
            budget_ms: 200.0,
            report: None,
        }
    }
}

pub async fn run(opts: &ExperimentOptions) -> anyhow::Result<()> {
    match opts.kind {
        ExperimentKind::VolumeRamp => volume_ramp::run(opts).await,
        ExperimentKind::LoadRamp => load_ramp::run(opts).await,
        ExperimentKind::LoadBell => load_bell::run_http(opts).await,
        ExperimentKind::RawFloorBell => load_bell::run_raw_floor(opts).await,
        ExperimentKind::RocksdbFloor => rocksdb_floor::run(opts).await,
        ExperimentKind::IndexAb => volume_ramp::run_index_ab(opts).await,
        ExperimentKind::Explain => explain::run(opts).await,
        ExperimentKind::DebugRelease => profile_compare::run(opts).await,
        ExperimentKind::Contention => volume_ramp::run_contention(opts).await,
        ExperimentKind::HotKey => hot_key::run(opts).await,
    }
}

pub fn parse_usize_list(s: &str) -> Vec<usize> {
    s.split(',').filter_map(|p| p.trim().parse().ok()).collect()
}

pub fn print_stats_line(label: &str, samples: &[f64]) {
    let s = crate::stats::Stats::summarize(samples.to_vec());
    println!(
        "[counter-latency-bench]   {label}: min={:.1} p50={:.1} p90={:.1} p95={:.1} p99={:.1} max={:.1} n={}",
        s.min,
        s.p50,
        percentile(samples, 0.90),
        s.p95,
        percentile(samples, 0.99),
        s.max,
        s.count
    );
}

pub fn percentile(sorted_input: &[f64], p: f64) -> f64 {
    if sorted_input.is_empty() {
        return 0.0;
    }
    let mut sorted = sorted_input.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f64 * p).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[idx]
}
