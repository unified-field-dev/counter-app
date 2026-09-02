#![allow(
    dead_code,
    unused_imports,
    missing_docs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
//! Valence models generated from `schemas/*.json` by `build.rs` (via `valence_codegen`).
//!
//! Public types such as [`Counter`] and `UserCounter` implement [`valence::Model`]
//! (`get` / `query` / `get_mutable` / `upsert`). Field-level semantics and privacy
//! policies live in the hand-written `valence_schema!` sources under `schemas/`;
//! this module is the compile output and is intentionally left without per-item
//! rustdoc.

use crate::side_effects::LeaderboardNotifier;
use valence::Model as _;

use valence::privacy_policies::common::{AUTHENTICATED, PUBLIC_READ, SYSTEM_ONLY};
use valence::privacy_policies::owner::{OWNER_BY_ID, OWNER_BY_USER_FIELD};

include!(concat!(env!("OUT_DIR"), "/generated_models.rs"));
