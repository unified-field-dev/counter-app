//! Valence [`SideEffect`](valence::SideEffect) hooks triggered by model mutations.
//!
//! Registered on [`crate::generated::UserCounter`] via the schema DSL
//! (`side_effects: [LeaderboardNotifier]`). After a successful write, the hook
//! runs in-process on the mutating Valence and may enqueue Boson work with
//! `send_with` so notifications stay off the request path.

/// The `UserCounter` side effect that enqueues leaderboard-change notifications.
pub mod leaderboard_notifier;

pub use leaderboard_notifier::{should_enqueue_leaderboard_check, LeaderboardNotifier};
