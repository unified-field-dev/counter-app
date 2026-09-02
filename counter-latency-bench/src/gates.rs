use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DeltaVerdict {
    Baseline,
    Conclusive,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AmplificationVerdict {
    NotApplicable,
    Correlated,
    InconclusiveMechanism,
}

/// Tier delta is conclusive when Δp95 ≥ max(5ms, 2× prev p95) OR Δp95 ≥ 50ms.
pub fn delta_conclusive(prev_p95: f64, curr_p95: f64) -> bool {
    if prev_p95 <= 0.0 {
        return false;
    }
    let delta = curr_p95 - prev_p95;
    delta >= 50.0 || delta >= prev_p95.max(5.0) * 2.0
}

pub fn classify_delta(prev_p95: Option<f64>, curr_p95: f64) -> DeltaVerdict {
    match prev_p95 {
        None => DeltaVerdict::Baseline,
        Some(prev) if delta_conclusive(prev, curr_p95) => DeltaVerdict::Conclusive,
        Some(_) => DeltaVerdict::Inconclusive,
    }
}

pub fn delta_ms(prev_p95: Option<f64>, curr_p95: f64) -> Option<f64> {
    prev_p95.map(|p| curr_p95 - p)
}

/// Amplification correlates when at least one counter meets plan thresholds alongside conclusive delta.
pub fn amplification_correlates(
    delta: DeltaVerdict,
    prev_events_p95: Option<f64>,
    curr_events_p95: f64,
    prev_gauges_p95: Option<f64>,
    curr_gauges_p95: f64,
    prev_boson_p95: Option<f64>,
    curr_boson_p95: f64,
    prev_retry_p95: Option<f64>,
    curr_retry_p95: f64,
    curr_retry_sleep_p95: f64,
) -> AmplificationVerdict {
    if delta != DeltaVerdict::Conclusive {
        return AmplificationVerdict::NotApplicable;
    }

    let events_ok = prev_events_p95
        .map(|p| p > 0.0 && curr_events_p95 >= 2.0 * p)
        .unwrap_or(false);
    let gauges_ok = prev_gauges_p95
        .map(|p| p > 0.0 && curr_gauges_p95 >= 2.0 * p)
        .unwrap_or(false);
    let boson_ok =
        curr_boson_p95 >= 1.0 && prev_boson_p95.map(|p| curr_boson_p95 > p).unwrap_or(true);
    let retry_ok = curr_retry_p95 >= 1.0 && prev_retry_p95.map(|p| p == 0.0).unwrap_or(true);
    let retry_sleep_ok = curr_retry_sleep_p95 >= 10.0;

    if events_ok || gauges_ok || boson_ok || retry_ok || retry_sleep_ok {
        AmplificationVerdict::Correlated
    } else {
        AmplificationVerdict::InconclusiveMechanism
    }
}

pub fn overall_verdict_label(delta: DeltaVerdict, amp: AmplificationVerdict) -> &'static str {
    match (delta, amp) {
        (DeltaVerdict::Baseline, _) => "baseline (evidence)",
        (DeltaVerdict::Inconclusive, _) => "inconclusive",
        (DeltaVerdict::Conclusive, AmplificationVerdict::Correlated) => "conclusive",
        (DeltaVerdict::Conclusive, AmplificationVerdict::InconclusiveMechanism) => {
            "conclusive latency, inconclusive mechanism"
        }
        (DeltaVerdict::Conclusive, AmplificationVerdict::NotApplicable) => "conclusive",
        _ => "inconclusive",
    }
}

pub fn budget_breached(p95: f64, budget_ms: f64, full_stack_anchor_ms: f64) -> bool {
    p95 >= budget_ms || p95 >= full_stack_anchor_ms * 0.2
}
