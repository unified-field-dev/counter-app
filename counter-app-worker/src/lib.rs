//! Headless counter-example worker: Valence models, Chronon scripts, Boson tasks, and Photon topics.
//!
//! Linked by `server`, `chronon-server`, `boson-server`, and `photon-server` so inventory-based
//! registration (schemas, scripts, tasks, topics) is present without pulling Leptos. The UI crate
//! `counter-app` re-exports this package as `worker` under its `ssr` feature for host convenience.
//!
//! ## Features
//!
//! - **Counter domain service** — Typed get / increment / set over Valence for the global
//!   counter and per-user scores via [`get_global`], [`increment_global`], and [`set_global`].
//!   [Get started](#counter-service)
//! - **Chronon leaderboard scripts** — Scheduled scripts such as [`bot_score_bumper`] that
//!   keep demo bots on the board. [Get started](#chronon-scripts)
//! - **Boson leaderboard task** — Async
//!   [`scripts::check_leaderboard_changes`] task enqueued from
//!   [`side_effects::LeaderboardNotifier`] after `UserCounter` mutations.
//!   [Get started](#boson-leaderboard-task)
//! - **Photon counter topic** — [`events::CounterUpdated`] (`#[photon::topic]`) published when
//!   the global counter changes so live UIs can refetch.
//!   [Get started](#photon-counter-topic)
//! - **Valence schemas** — `Counter` / `UserCounter` models under [`generated`] and
//!   [`embedded_surreal`], authored in the `schemas` sources.
//! - **Side-effect notifications** — [`side_effects::LeaderboardNotifier`] bridges Valence
//!   mutations to Boson enqueue.
//!
//! Library callers inspect [`CounterServiceError`]. Chronon / Boson script entry points
//! aggregate with `anyhow::Result` at the framework boundary. Spectra counters for server-fn
//! failures live in `counter-app` (`into_server_error`).
//!
//! ## Counter service
//!
//! The domain service is the headless API for the global counter and per-user scores.
//! Callers pass an already-built `valence::Valence` (from Higgs in UI server fns, or from
//! Chronon / Boson identity helpers in jobs). Use this layer when you need get / increment /
//! set without Leptos — contract tests and teaching hosts call it directly.
//!
//! **Prerequisites:** Valence router with Counter schemas registered; actor appropriate for
//! the privacy policy (anon vs authenticated user vs system in scripts).
//!
//! ```rust,ignore
//! use counter_app_worker::{get_global, increment_global, set_global};
//!
//! let before = get_global(&valence).await?;
//! let after = increment_global(1, &valence).await?;
//! assert_eq!(after.value, before.value + 1);
//!
//! let set = set_global(10, &valence).await?;
//! assert_eq!(set.value, 10);
//! let value = set.value;
//! assert_eq!(value, 10);
//! ```
//!
//! On success responses carry the persisted `value` (and user+global pair for user APIs).
//! [`CounterServiceError::Validation`], `Forbidden`, `RateLimited`, and `Valence` cover
//! the failure modes UI maps through `into_server_error`. Next:
//! [Chronon leaderboard scripts](#chronon-scripts) or the UI crate server-fn guide.
//!
//! ## Chronon scripts
//!
//! Chronon scripts keep the demo leaderboard lively on a schedule. [`bot_score_bumper`]
//! runs under `#[chronon_coordinator_macros::script]` with a default cron job, resolves
//! Valence via `chronon_valence_identity::valence_from_context`, and adjusts one top-tier
//! bot per tick. Register scripts with the Chronon coordinator at worker boot when you
//! want scheduled upkeep alongside ensure-bot-users and daily reset.
//!
//! **Prerequisites:** Chronon coordinator; Valence identity bridge; `Counter` / `UserCounter`
//! schemas; bot roster emails seeded.
//!
//! ```rust,ignore
//! use chronon_coordinator_macros::script;
//! use counter_app_worker::bot_score_bumper;
//!
//! // Shipped entry uses #[chronon_coordinator_macros::script(
//! //     name = "bot_score_bumper",
//! //     default_job(job = "bot-score-bumper", cron = "0,30 * * * * *")
//! // )]
//! async fn run_once(ctx: Box<dyn chronon_core::ScriptContext>) -> anyhow::Result<()> {
//!     bot_score_bumper(ctx).await?;
//!     Ok(())
//! }
//! ```
//!
//! On success the script returns `Ok(())` after at most one bot score write (or early exit
//! when the board is empty). Valence and identity errors propagate as `anyhow::Error`.
//! Related: [`daily_highscores_reset`], [`ensure_bot_users`]. Next:
//! [Boson leaderboard task](#boson-leaderboard-task).
//!
//! ## Boson leaderboard task
//!
//! After a `UserCounter` mutation, [`side_effects::LeaderboardNotifier`] enqueues
//! [`scripts::check_leaderboard_changes`] (`#[boson_macros::task]`) via
//! `CheckLeaderboardChanges::send_with` so rank diffs and notifications run off the
//! request path. Use this pattern when a Valence write should fan out to async work
//! without blocking the server function.
//!
//! **Prerequisites:** Boson coordinator and pool config; Valence side effects registered
//! for `UserCounter`; notification pipeline available for affected users.
//!
//! ```rust,ignore
//! use boson_macros::task;
//! use counter_app_worker::scripts::check_leaderboard_changes::{
//!     check_leaderboard_changes, CheckLeaderboardChanges, CheckLeaderboardChangesParams,
//! };
//!
//! // Shipped handler: #[boson_macros::task(name = "check_leaderboard_changes", …)]
//! // pub async fn check_leaderboard_changes(...) -> Result<()>
//! async fn enqueue_example(actor_json: serde_json::Value) -> anyhow::Result<()> {
//!     CheckLeaderboardChanges::send_with(
//!         actor_json,
//!         CheckLeaderboardChangesParams {
//!             user_id: "user:1".into(),
//!             old_value: 3,
//!             new_value: 4,
//!         },
//!     )
//!     .await?;
//!     Ok(())
//! }
//! ```
//!
//! On success the task diffs top-10 membership and sends notifications for rank changes.
//! Deletes and daily resets to zero are skipped in the notifier to avoid storms. Next:
//! [Photon counter topic](#photon-counter-topic) for live UI invalidation of the global counter.
//!
//! ## Photon counter topic
//!
//! [`events::CounterUpdated`] is the unkeyed Photon topic (`#[photon::topic(name = "counter.updated")]`)
//! whose `new_value` field carries the global counter after increment or set. UI server
//! fns call `publish()` after a successful write so photon-leptos subscribers refetch.
//! Define topics in the worker (no Leptos) and consume them from the UI crate.
//!
//! **Prerequisites:** Photon runtime on the host; topic registered in inventory; UI
//! `#[synced]` / `subscribe_counter_get` wired for live pages.
//!
//! ```rust,ignore
//! use photon::topic;
//! use counter_app_worker::events::CounterUpdated;
//!
//! // Shipped type: #[photon::topic(name = "counter.updated")]
//! // pub struct CounterUpdated { pub new_value: usize }
//!
//! async fn publish_example(new_value: usize) -> Result<(), Box<dyn std::error::Error>> {
//!     CounterUpdated { new_value }.publish().await?;
//!     Ok(())
//! }
//! ```
//!
//! On success connected clients on `/ws/counter` observe the event and refetch
//! `counter_get`. Set `COUNTER_NOOP_PUBLISH` in tests to skip publish. Next: UI crate
//! **Photon live subscription** section for the client subscribe helper.
//!
//! ## Feature flags
//!
//! | Flag | Effect |
//! |------|--------|
//! | `db-sqlite` (default) | Valence engine selection for SQLite-backed local/dev stores. |
//! | `db-hybrid` | Alternate Valence engine selection for hybrid deployments. |
//!
//! ## Examples
//!
//! Start with [Counter service](#counter-service). Integration coverage:
//! `cargo test -p counter-app-worker --test counter_workflow_contract`.
//! Workspace example `examples/local-counter-host` covers auth plus get/increment/set
//! (see its README). UI pages and server fns are in `counter-app`.
//!
//! ## Where to look next
//!
//! - [`service`] — get / increment / set and [`CounterServiceError`].
//! - [`scripts`] — Chronon scripts and the Boson leaderboard task module.
//! - [`events`] — Photon topic definitions.
//! - [`side_effects`] — Valence → Boson bridge.
//! - `counter-app` — Leptos pages, Higgs server fns, Photon client subscription.

#![allow(missing_docs)]
#![recursion_limit = "256"]

mod anon_rate_limit;

/// Lab/unit seam: clear the anonymous increment token buckets.
pub use anon_rate_limit::reset_for_tests;
pub mod embedded_surreal;
pub mod generated;
mod schemas;

pub mod events;
pub mod paths;
pub mod scripts;
pub mod service;
pub mod side_effects;

pub use side_effects::should_enqueue_leaderboard_check;

pub use service::{
    get_global, get_user, increment_global, increment_user, set_global, validate_anon_increment,
    validate_increment_amount, CounterResponse, CounterServiceError, UserCounterResponse,
    MAX_ANON_INCREMENT_AMOUNT, MAX_INCREMENT_AMOUNT,
};

pub use scripts::bot_score_bumper::{bot_score_bumper, bot_score_bumper_with_valence};
pub use scripts::check_leaderboard_changes::check_leaderboard_changes_with_valence;
pub use scripts::daily_highscores_reset::{
    daily_highscores_reset, daily_highscores_reset_with_valence,
};
pub use scripts::ensure_bot_users::{ensure_bot_users, ensure_bot_users_seed};
// Boson `#[task]` expands the entry fn into `CheckLeaderboardChanges` — discover via
// [`scripts::check_leaderboard_changes`].
