//! Page components for the counter example: live counter, high scores, and admin.
//!
//! Each page is an Orbital-composed Leptos view mounted through
//! the crate `lazy_routes` module under [`crate::CounterLayout`]. They call
//! Higgs server functions from the sibling `server` module and read Orbital
//! auth via `use_auth_state` / `use_auth_context`.

mod admin;
mod high_scores;
mod live;

pub use admin::CounterAdminPage;
pub use high_scores::HighScoresPage;
pub use live::CounterExamplePage;

use orbital::AuthSession;

/// Display label for headings ("Welcome back, …" / leaderboard title).
///
/// Thin wrapper over [`AuthSession::display_label`] so pages share one call site.
pub(crate) fn session_display_label(session: &AuthSession) -> String {
    session.display_label()
}
