//! Logical database name/storage constants for counter example Valence schemas.
//!
//! `valence_schema!` blocks in `schemas/` reference [`DEFAULT_STORAGE`] so generated
//! models bind to the same logical DB the host provisions. Engine id is selected by
//! Cargo feature: `db-sqlite` (default) or `db-hybrid`.

use valence::{Database, DatabaseFromEngine};

/// Logical name of the single embedded database used by this example.
pub const LOGICAL_NAME: &str = "default";

#[cfg(feature = "db-hybrid")]
const ENGINE_ID: &str = valence::HYBRID_ENGINE_ID;

#[cfg(not(feature = "db-hybrid"))]
const ENGINE_ID: &str = valence::SQLITE_ENGINE_ID;

/// [`LOGICAL_NAME`] bound to the active storage engine, for host registration.
pub const DEFAULT_STORAGE: DatabaseFromEngine = Database::from_engine(LOGICAL_NAME, ENGINE_ID);

/// All logical database names this crate expects the host to provision.
pub const EMBEDDED_SURREAL_LOGICAL_NAMES: &[&str] = &[LOGICAL_NAME];
