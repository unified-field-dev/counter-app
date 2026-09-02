//! In-process token bucket for anonymous counter increments (CA-05).
//!
//! Used by [`crate::service::validate_anon_increment`] before Valence writes.
//! Budget defaults to 60 requests/minute; override with
//! `COUNTER_ANON_INCREMENTS_PER_MIN`. Capacity `0` fails closed (deny all).
//! Process-local only — not shared across hosts.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const DEFAULT_REQUESTS_PER_MIN: u32 = 60;
const CLEANUP_THRESHOLD: usize = 1_000;
const STALE_BUCKET_TTL: Duration = Duration::from_secs(3600);

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Default)]
struct Limiter {
    buckets: Mutex<HashMap<String, Bucket>>,
}

fn limiter() -> &'static Limiter {
    static INSTANCE: OnceLock<Limiter> = OnceLock::new();
    INSTANCE.get_or_init(Limiter::default)
}

fn requests_per_min() -> u32 {
    std::env::var("COUNTER_ANON_INCREMENTS_PER_MIN")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_REQUESTS_PER_MIN)
}

/// Returns true when an anonymous increment request is allowed under the per-minute budget.
///
/// Called from [`crate::service::validate_anon_increment`]. A `false` result maps
/// to [`crate::CounterServiceError::RateLimited`].
pub fn allow_request() -> bool {
    allow("anon-increment", requests_per_min())
}

/// Clears all in-process buckets (lab / unit seams only).
///
/// Playwright and contract tests call this before rate-limit sad paths so prior
/// suites cannot leave the shared `"anon-increment"` bucket exhausted.
pub fn reset_for_tests() {
    limiter()
        .buckets
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}

fn allow(key: &str, capacity: u32) -> bool {
    // Fail closed: `0` means deny all (misconfig / intentional kill switch),
    // not "unlimited". Use a large positive budget to raise the ceiling.
    if capacity == 0 {
        return false;
    }
    let capacity = f64::from(capacity);
    let refill_per_sec = capacity / 60.0;
    let now = Instant::now();

    let mut buckets = limiter()
        .buckets
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if buckets.len() > CLEANUP_THRESHOLD {
        buckets.retain(|_, b| now.duration_since(b.last_refill) < STALE_BUCKET_TTL);
    }

    let bucket = buckets.entry(key.to_string()).or_insert_with(|| Bucket {
        tokens: capacity,
        last_refill: now,
    });
    let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
    bucket.tokens = elapsed.mul_add(refill_per_sec, bucket.tokens).min(capacity);
    bucket.last_refill = now;

    let allowed = if bucket.tokens >= 1.0 {
        bucket.tokens -= 1.0;
        true
    } else {
        false
    };
    drop(buckets);
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_capacity_then_blocks() {
        let key = "test-allows-up-to-capacity";
        for i in 0..5 {
            assert!(allow(key, 5), "request {i} should be allowed");
        }
        assert!(!allow(key, 5), "6th request should be blocked");
    }

    #[test]
    fn zero_capacity_denies_all_sad() {
        let key = "test-zero-capacity-deny";
        for _ in 0..5 {
            assert!(!allow(key, 0), "capacity 0 must fail closed");
        }
    }
}
