//! High-scores leaderboard: types, paginated Higgs endpoint, and Orbital table.
//!
//! [`get_high_scores_page`] is the teaching path for Valence list queries behind
//! a Leptos server fn. [`HighScoresTable`] consumes pages through
//! [`orbital::components::OrbitalInfiniteScroll`].

mod server;
mod table;
mod types;

pub use server::{get_high_scores, get_high_scores_page};
pub use table::HighScoresTable;
pub use types::{
    clamp_high_scores_page, HighScoreEntry, HighScoresResponse, HIGH_SCORES_PAGE_SIZE,
    MAX_HIGH_SCORES_LIMIT,
};
