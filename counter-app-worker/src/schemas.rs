//! Product-local Valence schema sources for `Counter` and `UserCounter`.
//!
//! Each submodule `include!`s a `valence_schema!` / `valence_trait_schema!` file from
//! this crate's `schemas/` folder. The macros submit metadata through
//! `valence::inventory`; [`valence::SchemaRegistry`] picks them up when the crate is
//! linked — no manual registration call. Generated Rust models land in
//! [`crate::generated`] via `build.rs`.
//!
//! ## Features
//!
//! - **Counter singleton** — global demo row (`table: "counter"`, id `"singleton"`)
//!   with public read/update for anon increments.
//! - **`UserCounter`** — per-user scores with `OWNER_BY_USER_FIELD` writes, public
//!   leaderboard reads, and [`crate::side_effects::LeaderboardNotifier`].
//! - **`UserLinkedCounter` trait** — shared field/connection shape used by trait-detail
//!   E2E and the `UserCounter` schema `traits: […]` list.

mod counter_schema {
    include!("../schemas/counter_valence_schema.rs");
}

mod user_linked_counter_trait {
    include!("../schemas/user_linked_counter_valence_trait.rs");
}

mod user_counter_schema {
    include!("../schemas/user_counter_valence_schema.rs");
}
