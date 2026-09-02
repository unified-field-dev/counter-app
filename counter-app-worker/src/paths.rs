//! Stable URL paths for the counter app (subset used by worker notifications).
//!
//! Keep in sync with `counter-app` route constants from `orbital_macros::orbital_routes_extract`.
//! Leaderboard notification copy links here so deep-links match the UI router without
//! depending on the Leptos crate from this worker.

/// High scores page (matches `/counter` + `/high-scores`).
pub const HIGH_SCORES: &str = "/counter/high-scores";
