//! Leaderboard DTOs shared by the Higgs endpoint and Orbital table UI.
//!
//! Keep pagination clamps ([`clamp_high_scores_page`]) next to the wire types so
//! server and client agree on page size and abuse limits before Valence runs.

use serde::{Deserialize, Serialize};

/// A single leaderboard row: display name and counter value.
///
/// Built in [`super::server::get_high_scores_page`] from Valence `UserCounter`
/// plus a profile display name (or redacted label). `row_key` is the record id
/// for Leptos list reconciliation, not shown in the table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HighScoreEntry {
    /// Stable key for list reconciliation (`UserCounter` id). Not shown in the table.
    #[serde(default)]
    pub row_key: String,
    /// Display name of the user this row belongs to.
    pub display_name: String,
    /// The user's counter value (their score).
    pub count: usize,
}

/// Response payload for the legacy non-paginated high-scores endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighScoresResponse {
    /// Leaderboard rows, ordered by score descending.
    pub scores: Vec<HighScoreEntry>,
}

/// Page size used by both server and Orbital infinite scroll.
pub const HIGH_SCORES_PAGE_SIZE: u32 = 10;

/// Hard ceiling for `get_high_scores_page` `limit` (abuse guard).
pub const MAX_HIGH_SCORES_LIMIT: u32 = 100;

/// Clamp leaderboard pagination inputs before querying Valence.
///
/// Call at the start of [`super::server::get_high_scores_page`] (and any other
/// entry that accepts client `offset`/`limit`). Returns `(offset, limit)` with
/// `limit` in `1..=MAX_HIGH_SCORES_LIMIT`.
#[must_use]
pub fn clamp_high_scores_page(offset: u32, limit: u32) -> (u32, u32) {
    let limit = if limit == 0 {
        1
    } else {
        limit.min(MAX_HIGH_SCORES_LIMIT)
    };
    (offset, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_accepts_normal_page_happy() {
        assert_eq!(
            clamp_high_scores_page(0, HIGH_SCORES_PAGE_SIZE),
            (0, HIGH_SCORES_PAGE_SIZE)
        );
    }

    #[test]
    fn clamp_caps_oversize_limit_sad() {
        assert_eq!(
            clamp_high_scores_page(5, MAX_HIGH_SCORES_LIMIT + 50),
            (5, MAX_HIGH_SCORES_LIMIT)
        );
    }

    #[test]
    fn clamp_zero_limit_becomes_one_sad() {
        assert_eq!(clamp_high_scores_page(0, 0), (0, 1));
    }
}
