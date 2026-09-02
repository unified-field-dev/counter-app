use serde::Serialize;

use crate::gates::{AmplificationVerdict, DeltaVerdict};
use crate::stats::Stats;

#[derive(Debug, Clone, Serialize)]
pub struct MetricStats {
    pub min: f64,
    pub p50: f64,
    pub p95: f64,
    pub max: f64,
    pub count: usize,
}

impl From<Stats> for MetricStats {
    fn from(s: Stats) -> Self {
        Self {
            min: s.min,
            p50: s.p50,
            p95: s.p95,
            max: s.max,
            count: s.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TierReport {
    pub tier: u8,
    pub run: usize,
    pub repeat_total: usize,
    pub git_sha: String,
    pub op: String,
    pub iterations: usize,
    pub warmup: usize,
    pub env: Vec<String>,
    pub increment_total: MetricStats,
    pub user_counter_get_ms: MetricStats,
    pub counter_get_ms: MetricStats,
    pub user_counter_commit_ms: MetricStats,
    pub counter_commit_ms: MetricStats,
    pub spectra_events_per_iter: MetricStats,
    pub spectra_gauges_per_iter: MetricStats,
    pub boson_queued: MetricStats,
    pub db_retry_count: MetricStats,
    pub db_retry_total_sleep_ms: MetricStats,
    pub prev_tier: Option<u8>,
    pub delta_increment_p95_ms: Option<f64>,
    pub delta_verdict: DeltaVerdict,
    pub amplification_verdict: AmplificationVerdict,
    pub overall_verdict: String,
    pub artifact_txt: Option<String>,
    pub artifact_json: Option<String>,
}

impl TierReport {
    pub fn write_json(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

pub fn git_sha_short() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn collect_env_flags() -> Vec<String> {
    [
        "SPECTRA_CONSOLE",
        "SPECTRA_COMPOSITE_PERSIST",
        "SPECTRA_SYNC_HOT_PATH",
        "CHRONON_DISABLE_WORKER",
    ]
    .iter()
    .filter_map(|k| std::env::var(k).ok().map(|v| format!("{k}={v}")))
    .chain([format!(
        "permission_cache={}",
        !std::env::args().any(|a| a == "--no-permission-cache")
    )])
    .collect()
}
