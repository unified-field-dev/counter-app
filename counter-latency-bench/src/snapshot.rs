use serde::Serialize;

use crate::stack::BenchRuntime;

#[derive(Debug, Clone, Default, Serialize)]
pub struct StackSnapshot {
    pub boson_queued: u32,
    pub spectra_events: u32,
    pub spectra_gauges: u32,
    pub spectra_counters: u32,
    pub db_retry_count: u32,
    pub db_retry_total_sleep_ms: f64,
}

pub async fn capture(runtime: &BenchRuntime) -> StackSnapshot {
    let mut snap = StackSnapshot::default();

    #[cfg(feature = "tier-boson")]
    if runtime.tier >= 1 {
        snap.boson_queued = crate::stack::tier1_boson::boson_queued_count(runtime).await;
    }

    #[cfg(feature = "tier-spectra")]
    if let Some(recording) = &runtime.spectra_recording {
        let (events, gauges, counters) = crate::stack::tier2_spectra::spectra_counts(recording);
        snap.spectra_events = events;
        snap.spectra_gauges = gauges;
        snap.spectra_counters = counters;
        let (retry_count, _) = crate::stack::tier2_spectra::retry_count_from_recording(recording);
        snap.db_retry_count = retry_count;
    }

    snap
}

pub async fn capture_iteration_delta(
    runtime: &BenchRuntime,
    events_before: u32,
    gauges_before: u32,
    retry_before: u32,
) -> StackSnapshot {
    let full = capture(runtime).await;
    StackSnapshot {
        boson_queued: full.boson_queued,
        spectra_events: full.spectra_events.saturating_sub(events_before),
        spectra_gauges: full.spectra_gauges.saturating_sub(gauges_before),
        spectra_counters: full.spectra_counters,
        db_retry_count: full.db_retry_count.saturating_sub(retry_before),
        db_retry_total_sleep_ms: full.db_retry_total_sleep_ms,
    }
}

#[cfg(feature = "tier-spectra")]
pub fn spectra_baselines(runtime: &BenchRuntime) -> (u32, u32, u32) {
    if let Some(recording) = &runtime.spectra_recording {
        crate::stack::tier2_spectra::spectra_counts(recording)
    } else {
        (0, 0, 0)
    }
}

#[cfg(not(feature = "tier-spectra"))]
pub fn spectra_baselines(_runtime: &BenchRuntime) -> (u32, u32, u32) {
    (0, 0, 0)
}

#[cfg(feature = "tier-spectra")]
pub fn retry_baseline(runtime: &BenchRuntime) -> u32 {
    runtime
        .spectra_recording
        .as_ref()
        .map(|r| crate::stack::tier2_spectra::retry_count_from_recording(r).0)
        .unwrap_or(0)
}

#[cfg(not(feature = "tier-spectra"))]
pub fn retry_baseline(_runtime: &BenchRuntime) -> u32 {
    0
}

#[cfg(feature = "tier-spectra")]
pub fn clear_spectra_recording(runtime: &BenchRuntime) {
    if let Some(recording) = &runtime.spectra_recording {
        crate::stack::tier2_spectra::clear_recording(recording);
    }
}

#[cfg(not(feature = "tier-spectra"))]
pub fn clear_spectra_recording(_runtime: &BenchRuntime) {}
