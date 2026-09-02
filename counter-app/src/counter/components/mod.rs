//! Shared UI pieces reused across counter pages.
//!
//! Auth warning chrome and the high-scores leaderboard live here so pages under
//! `super::counter_example` stay thin. The leaderboard path teaches Higgs
//! pagination over Valence `UserCounter` plus Orbital infinite scroll.

/// Warning banner for unauthenticated call-outs.
pub mod auth_warning_banner;
/// High-scores types, server fns, and table UI.
pub mod high_scores;

pub use auth_warning_banner::AuthWarningBanner;
pub use high_scores::{
    clamp_high_scores_page, get_high_scores, get_high_scores_page, HighScoreEntry, HighScoresTable,
    HIGH_SCORES_PAGE_SIZE, MAX_HIGH_SCORES_LIMIT,
};
