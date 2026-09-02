//! Chronon scripts and Boson tasks that keep the counter example's leaderboard alive:
//! a synthetic bot roster, periodic bot-score bumping, leaderboard-change
//! notifications, and daily/bootstrap resets.
//!
//! Each Chronon entry takes `Box<dyn chronon_core::ScriptContext>` and builds a
//! [`valence::Valence`] with `chronon_valence_identity::valence_from_context`.
//! The Boson task takes `Box<dyn boson_core::ExecutionContext>` and uses
//! `boson_valence_identity::valence_from_context`, then fans out notifications.
//! Seed and mutate paths call the same generated [`crate::generated::UserCounter`]
//! / identity models as the request-path service.
//!
//! ## Features
//!
//! - **Bot score bump** — [`bot_score_bumper::bot_score_bumper`] Chronon script
//!   (`ScriptContext` + `valence_from_context`) adjusts one top-tier bot per tick.
//! - **Daily / bootstrap reset** — [`daily_highscores_reset::daily_highscores_reset`]
//!   Chronon script zeroes real users and restores bot `reset_score` values.
//! - **Ensure bot users** — [`ensure_bot_users::ensure_bot_users`] Chronon
//!   `run_once` seed (also callable as [`ensure_bot_users::ensure_bot_users_seed`]).
//! - **Leaderboard rank diff** — [`check_leaderboard_changes`] Boson task
//!   (`ExecutionContext` + `send_with` from the Valence side effect).
//! - **Synthetic roster data** — [`bot_roster`] static defs consumed by seed,
//!   bump, and reset scripts.

/// The synthetic bot roster used to populate the leaderboard demo.
pub mod bot_roster;
/// Chronon script: periodically bumps one top-tier bot to stay above real users.
///
/// Entry: `#[chronon_coordinator_macros::script]` +
/// `chronon_valence_identity::valence_from_context`. Queries
/// [`crate::generated::UserCounter`] ordered by score, resolves emails via
/// Lepton identity, then mutates one bot with `get_mutable` / `commit`.
pub mod bot_score_bumper;
/// Boson task: diffs leaderboard rank changes and notifies affected users.
///
/// Entry: `#[boson_macros::task]` on [`check_leaderboard_changes`]. Enqueued from
/// [`crate::side_effects::LeaderboardNotifier`] via
/// `CheckLeaderboardChanges::send_with` after a `UserCounter` mutation.
pub mod check_leaderboard_changes;
/// Chronon script: resets all counters (real users to 0, bots to their tier score) daily.
///
/// Same `ScriptContext` → `valence_from_context` pattern as the bumper; walks every
/// `UserCounter` and sets values, then upserts the global singleton to first place.
pub mod daily_highscores_reset;
/// Idempotent seeding: ensures the bot roster exists as real `User` + `UserCounter` rows.
///
/// Chronon `run_once` default job plus [`ensure_bot_users::ensure_bot_users_seed`] for
/// server preflight. Creates Lepton identity rows, then upserts counters.
pub mod ensure_bot_users;
mod user_email;

pub use bot_score_bumper::bot_score_bumper_with_valence;
pub use check_leaderboard_changes::check_leaderboard_changes_with_valence;
pub use daily_highscores_reset::daily_highscores_reset_with_valence;
pub use ensure_bot_users::ensure_bot_users_seed;
