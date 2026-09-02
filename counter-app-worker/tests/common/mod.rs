//! Shared in-memory Valence helpers for counter-app-worker integration tests.

#![allow(dead_code)]

use std::sync::Arc;

use counter_app_worker::embedded_surreal::LOGICAL_NAME;
use valence::actor::Actor;
use valence::{
    register_backend_logical_names, router_key, DatabaseBackend, DatabaseRouter,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

fn prepare_test_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    // Drop process-wide point-get cache so prior tests cannot satisfy `get` on a fresh DB.
    valence::clear_for_test();

    // Unified ownership fetch emits Surreal-shaped RETURN SQL that SQLite rejects.
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: test harness only; OnceLock reads this before first ownership get.
        unsafe {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

/// Fresh in-memory `SQLite` backend registered under the counter logical name.
pub async fn mem_router() -> Arc<DatabaseRouter> {
    prepare_test_env();
    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("connect sqlite"),
    );
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        backend,
        &[LOGICAL_NAME],
        RegisterBackendLogicalNamesOptions::default(),
    );
    Arc::new(router)
}

pub fn valence_for(router: Arc<DatabaseRouter>, actor: Actor) -> Valence {
    Valence::builder()
        .database_router(router)
        .default_backend_key(router_key(LOGICAL_NAME, SQLITE_ENGINE_ID))
        .with_actor(actor)
        .build()
        .expect("valence build")
}

pub async fn system_valence() -> Valence {
    valence_for(
        mem_router().await,
        Actor::System {
            operation: "counter-test".into(),
        },
    )
}

pub async fn user_valence(user_id: &str) -> Valence {
    valence_for(
        mem_router().await,
        Actor::User {
            user_id: user_id.into(),
        },
    )
}
