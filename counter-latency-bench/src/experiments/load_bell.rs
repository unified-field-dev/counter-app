//! Open-loop bell-curve load: Gaussian RPS schedule (floor -> peak -> floor).

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use valence::SDb;

use crate::engine::BENCH_NS;
use crate::experiments::{self, ExperimentOptions};
use crate::stats::Stats;
use surrealdb::engine::local::RocksDb;

/// Bell-curve load parameters.
#[derive(Debug, Clone, Copy)]
pub struct BellConfig {
    pub peak_rps: f64,
    pub floor_rps: f64,
    pub duration_secs: u64,
    pub max_inflight: usize,
    pub bucket_secs: u64,
}

#[derive(Debug, Clone)]
struct Sample {
    latency_ms: f64,
    ok: bool,
    overflow: bool,
    bucket: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BellStats {
    pub count: usize,
    pub min: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MinuteBucket {
    pub minute: u64,
    pub offered: u64,
    pub completed: u64,
    pub overflow: u64,
    pub errors: u64,
    pub offered_rps: f64,
    pub achieved_rps: f64,
    pub latency: BellStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct BellReport {
    pub mode: String,
    pub peak_rps: f64,
    pub floor_rps: f64,
    pub duration_secs: u64,
    pub max_inflight: usize,
    pub offered_total: u64,
    pub completed_total: u64,
    pub overflow_total: u64,
    pub error_total: u64,
    pub overall: BellStats,
    pub per_minute: Vec<MinuteBucket>,
}

impl BellConfig {
    fn from_opts(opts: &ExperimentOptions) -> Self {
        Self {
            peak_rps: opts.bell_peak_rps,
            floor_rps: opts.bell_floor_rps,
            duration_secs: opts.bell_duration_secs,
            max_inflight: opts.bell_max_inflight,
            bucket_secs: opts.bell_bucket_secs.max(1),
        }
    }
}

/// Target RPS at second `sec` (Gaussian bell centered at duration/2).
pub fn target_rps_at_second(sec: u64, cfg: &BellConfig) -> f64 {
    let mu = cfg.duration_secs as f64 / 2.0;
    let sigma = cfg.duration_secs as f64 / 8.0;
    let t = sec as f64;
    let gauss = (-((t - mu).powi(2)) / (2.0 * sigma.powi(2))).exp();
    (cfg.floor_rps + (cfg.peak_rps - cfg.floor_rps) * gauss).max(cfg.floor_rps)
}

/// Scheduled arrival offsets from experiment start.
pub fn bell_arrivals(cfg: &BellConfig) -> Vec<Duration> {
    let mut arrivals = Vec::new();
    for sec in 0..cfg.duration_secs {
        let rps = target_rps_at_second(sec, cfg);
        let n = rps.round().max(cfg.floor_rps) as u64;
        for i in 0..n {
            let offset = if n <= 1 { 0.0 } else { i as f64 / n as f64 };
            arrivals.push(Duration::from_secs_f64(sec as f64 + offset));
        }
    }
    arrivals
}

fn stats_from_samples(samples: &[f64]) -> BellStats {
    if samples.is_empty() {
        return BellStats {
            count: 0,
            min: 0.0,
            p50: 0.0,
            p90: 0.0,
            p99: 0.0,
            max: 0.0,
        };
    }
    let s = Stats::summarize(samples.to_vec());
    BellStats {
        count: s.count,
        min: s.min,
        p50: s.p50,
        p90: experiments::percentile(samples, 0.90),
        p99: experiments::percentile(samples, 0.99),
        max: s.max,
    }
}

pub fn summarize_buckets(samples: &[Sample], cfg: &BellConfig) -> BellReport {
    let offered_total = samples.len() as u64;
    let overflow_total = samples.iter().filter(|s| s.overflow).count() as u64;
    let completed: Vec<_> = samples.iter().filter(|s| !s.overflow).collect();
    let error_total = completed.iter().filter(|s| !s.ok).count() as u64;
    let completed_total = completed.len() as u64;

    let ok_latencies: Vec<f64> = completed
        .iter()
        .filter(|s| s.ok)
        .map(|s| s.latency_ms)
        .collect();
    let overall = stats_from_samples(&ok_latencies);

    let num_buckets = cfg.duration_secs.div_ceil(cfg.bucket_secs);
    let mut per_minute = Vec::new();
    for b in 0..num_buckets {
        let bucket_samples: Vec<_> = samples.iter().filter(|s| s.bucket == b).collect();
        let offered = bucket_samples.len() as u64;
        let overflow = bucket_samples.iter().filter(|s| s.overflow).count() as u64;
        let completed_in_bucket: Vec<_> = bucket_samples
            .iter()
            .filter(|s| !s.overflow)
            .copied()
            .collect();
        let errors = completed_in_bucket.iter().filter(|s| !s.ok).count() as u64;
        let latencies: Vec<f64> = completed_in_bucket
            .iter()
            .filter(|s| s.ok)
            .map(|s| s.latency_ms)
            .collect();
        let achieved = latencies.len() as u64;
        let window_secs = cfg.bucket_secs as f64;
        per_minute.push(MinuteBucket {
            minute: b,
            offered,
            completed: achieved,
            overflow,
            errors,
            offered_rps: offered as f64 / window_secs,
            achieved_rps: achieved as f64 / window_secs,
            latency: stats_from_samples(&latencies),
        });
    }

    BellReport {
        mode: String::new(),
        peak_rps: cfg.peak_rps,
        floor_rps: cfg.floor_rps,
        duration_secs: cfg.duration_secs,
        max_inflight: cfg.max_inflight,
        offered_total,
        completed_total,
        overflow_total,
        error_total,
        overall,
        per_minute,
    }
}

pub fn print_report(report: &BellReport) {
    println!(
        "[counter-latency-bench] bell mode={} peak={} floor={} duration={}s max_inflight={}",
        report.mode, report.peak_rps, report.floor_rps, report.duration_secs, report.max_inflight
    );
    println!(
        "[counter-latency-bench]   offered={} completed={} overflow={} errors={}",
        report.offered_total, report.completed_total, report.overflow_total, report.error_total
    );
    if report.overall.count > 0 {
        println!(
            "[counter-latency-bench]   overall: min={:.1} p50={:.1} p90={:.1} p99={:.1} max={:.1} n={}",
            report.overall.min,
            report.overall.p50,
            report.overall.p90,
            report.overall.p99,
            report.overall.max,
            report.overall.count
        );
    } else {
        println!("[counter-latency-bench]   overall: (no successful samples)");
    }
    println!("[counter-latency-bench]   per-minute (offered_rps achieved_rps p50 p90 p99 overflow errors):");
    for b in &report.per_minute {
        if b.offered == 0 {
            continue;
        }
        println!(
            "[counter-latency-bench]     min={:>3} offered={:.1}/s achieved={:.1}/s p50={:.1} p90={:.1} p99={:.1} overflow={} errors={}",
            b.minute,
            b.offered_rps,
            b.achieved_rps,
            b.latency.p50,
            b.latency.p90,
            b.latency.p99,
            b.overflow,
            b.errors
        );
    }
}

async fn drive_open_loop_http(
    cfg: BellConfig,
    client: reqwest::Client,
    url: String,
) -> Result<Vec<Sample>> {
    let arrivals = bell_arrivals(&cfg);
    let semaphore = Arc::new(Semaphore::new(cfg.max_inflight));
    let start = Instant::now();
    let results = Arc::new(Mutex::new(Vec::with_capacity(arrivals.len())));
    let mut join_set = JoinSet::new();

    for arrival in arrivals {
        let bucket = arrival.as_secs() / cfg.bucket_secs;
        let target = start + arrival;
        tokio::time::sleep_until(target.into()).await;

        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                results.lock().await.push(Sample {
                    latency_ms: 0.0,
                    ok: false,
                    overflow: true,
                    bucket,
                });
                continue;
            }
        };

        let client = client.clone();
        let url = url.clone();
        let results = Arc::clone(&results);
        join_set.spawn(async move {
            let scheduled = target;
            let ok = match client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(r#"{"amount":1}"#)
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => true,
                _ => false,
            };
            let done = Instant::now();
            let latency_ms = done.duration_since(scheduled).as_secs_f64().max(0.0) * 1000.0;
            drop(permit);
            results.lock().await.push(Sample {
                latency_ms,
                ok,
                overflow: false,
                bucket,
            });
        });
    }

    while join_set.join_next().await.is_some() {}

    let samples = Arc::try_unwrap(results)
        .map_err(|_| anyhow::anyhow!("bell results still shared"))?
        .into_inner();
    Ok(samples)
}

async fn drive_open_loop_raw(cfg: BellConfig, db: Arc<SDb>) -> Result<Vec<Sample>> {
    let arrivals = bell_arrivals(&cfg);
    let semaphore = Arc::new(Semaphore::new(cfg.max_inflight));
    let start = Instant::now();
    let results = Arc::new(Mutex::new(Vec::with_capacity(arrivals.len())));
    let mut join_set = JoinSet::new();

    for arrival in arrivals {
        let bucket = arrival.as_secs() / cfg.bucket_secs;
        let target = start + arrival;
        tokio::time::sleep_until(target.into()).await;

        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                results.lock().await.push(Sample {
                    latency_ms: 0.0,
                    ok: false,
                    overflow: true,
                    bucket,
                });
                continue;
            }
        };

        let db = Arc::clone(&db);
        let results = Arc::clone(&results);
        join_set.spawn(async move {
            let scheduled = target;
            let ok = db
                .query("UPDATE bench_kv:singleton SET value += 1 RETURN NONE")
                .await
                .is_ok();
            let done = Instant::now();
            let latency_ms = done.duration_since(scheduled).as_secs_f64().max(0.0) * 1000.0;
            drop(permit);
            results.lock().await.push(Sample {
                latency_ms,
                ok,
                overflow: false,
                bucket,
            });
        });
    }

    while join_set.join_next().await.is_some() {}

    let samples = Arc::try_unwrap(results)
        .map_err(|_| anyhow::anyhow!("bell results still shared"))?
        .into_inner();
    Ok(samples)
}

/// M1: open-loop HTTP bell against live server.
pub async fn run_http(opts: &ExperimentOptions) -> Result<()> {
    let url = opts
        .http_url
        .as_ref()
        .context("--http-url required for load-bell experiment")?;
    let cfg = BellConfig::from_opts(opts);

    println!(
        "[counter-latency-bench] experiment=load-bell mode=http peak={} floor={} duration={}s max_inflight={}",
        cfg.peak_rps, cfg.floor_rps, cfg.duration_secs, cfg.max_inflight
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("build reqwest client")?;

    let samples = drive_open_loop_http(cfg, client, url.clone()).await?;

    let mut report = summarize_buckets(&samples, &cfg);
    report.mode = "http".into();
    print_report(&report);
    write_report(opts, &report)?;
    Ok(())
}

async fn open_raw_rocksdb(path: &Path) -> Result<SDb> {
    std::fs::create_dir_all(path).context("create raw floor dir")?;
    let lock = path.join("LOCK");
    if lock.exists() {
        let _ = std::fs::remove_file(&lock);
    }
    let db = SDb::init();
    db.connect::<RocksDb>(path.to_string_lossy().as_ref())
        .await
        .context("connect raw floor RocksDB")?;
    db.use_ns(BENCH_NS)
        .use_db(BENCH_NS)
        .await
        .context("use ns/db")?;
    Ok(db)
}

/// M2: single-connection raw Surreal->RocksDB RMW on the same bell schedule.
pub async fn run_raw_floor(opts: &ExperimentOptions) -> Result<()> {
    let mut cfg = BellConfig::from_opts(opts);
    // Single embedded connection: serialize writes (open-loop still schedules arrivals).
    cfg.max_inflight = 1;
    let db_path = opts.data_dir.join("raw-floor-rocksdb");

    println!(
        "[counter-latency-bench] experiment=raw-floor-bell mode=raw_surreal peak={} floor={} duration={}s max_inflight={} path={}",
        cfg.peak_rps,
        cfg.floor_rps,
        cfg.duration_secs,
        cfg.max_inflight,
        db_path.display()
    );

    let db = Arc::new(open_raw_rocksdb(&db_path).await?);
    db.query("DEFINE TABLE IF NOT EXISTS bench_kv SCHEMALESS")
        .await
        .context("define bench_kv")?;
    db.query("UPSERT bench_kv:singleton CONTENT { value: 0 }")
        .await
        .context("upsert bench_kv")?;

    let samples = drive_open_loop_raw(cfg, db).await?;

    let mut report = summarize_buckets(&samples, &cfg);
    report.mode = "raw_surreal_rmw".into();
    print_report(&report);
    write_report(opts, &report)?;
    Ok(())
}

fn write_report(opts: &ExperimentOptions, report: &BellReport) -> Result<()> {
    if let Some(path) = &opts.report {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create report dir")?;
        }
        std::fs::write(path, serde_json::to_string_pretty(report)?).context("write report")?;
        println!(
            "[counter-latency-bench] report written to {}",
            path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bell_peak_near_center() {
        let cfg = BellConfig {
            peak_rps: 1000.0,
            floor_rps: 1.0,
            duration_secs: 3600,
            max_inflight: 100,
            bucket_secs: 60,
        };
        let center = target_rps_at_second(1800, &cfg);
        assert!((center - 1000.0).abs() < 1.0);
        let edge = target_rps_at_second(0, &cfg);
        assert!(edge < 10.0);
    }

    #[test]
    fn arrivals_count_matches_schedule() {
        let cfg = BellConfig {
            peak_rps: 10.0,
            floor_rps: 1.0,
            duration_secs: 10,
            max_inflight: 100,
            bucket_secs: 60,
        };
        let arrivals = bell_arrivals(&cfg);
        assert!(arrivals.len() >= 10);
    }
}
