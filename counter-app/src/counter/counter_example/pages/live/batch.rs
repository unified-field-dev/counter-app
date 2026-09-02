//! Pure helpers for pending-click flush timing (1s idle / 5s max).
//!
//! `IncrementButton` arms browser timers from these constants. Unit tests call
//! [`should_flush`] so the policy stays documented without spinning Leptos.

/// Flush after this much idle time since the last click in the batch.
pub const IDLE_FLUSH_MS: u64 = 1_000;

/// Flush at latest this long after the first click in the batch.
pub const MAX_FLUSH_MS: u64 = 5_000;

/// Whether a pending batch should flush given elapsed times since last click
/// and since the batch started.
///
/// Returns `true` when idle time reached [`IDLE_FLUSH_MS`] or batch age reached
/// [`MAX_FLUSH_MS`]. Kept `const` so tests and docs share one policy function.
#[must_use]
#[allow(dead_code)] // Exercised by unit tests; kept as the documented flush policy.
pub const fn should_flush(idle_elapsed_ms: u64, batch_elapsed_ms: u64) -> bool {
    idle_elapsed_ms >= IDLE_FLUSH_MS || batch_elapsed_ms >= MAX_FLUSH_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flushes_on_idle() {
        assert!(should_flush(1_000, 500));
        assert!(!should_flush(999, 500));
    }

    #[test]
    fn flushes_on_max_batch_age() {
        assert!(should_flush(0, 5_000));
        assert!(!should_flush(0, 4_999));
    }
}
