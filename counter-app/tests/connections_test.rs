//! Connection navigation integration tests.
//!
//! Verifies that `UserCounter`'s generated connection methods (`get_user`, `get_from_user`,
//! `get_from_user_id`, `user_thing` / user) and `IdHolder` trait compile and work correctly.
//!
//! Run with: `cargo test -p counter-app --test connections_test --features ssr`

#![cfg(feature = "ssr")]
#![allow(missing_docs)]
// Integration-test helpers fall outside clippy's `allow-expect-in-tests` scope.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use counter_app::embedded_surreal::LOGICAL_NAME;
use counter_app::generated::UserCounter;
use std::sync::Arc;
use valence::actor::Actor;
use valence::connection::IdHolder;
use valence::schema::SchemaRegistry;
use valence::{
    register_backend_logical_names_slices, router_key, DatabaseBackend, DatabaseRouter, Model,
    RecordId, RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

const fn router_groups() -> &'static [&'static [&'static str]] {
    &[&["default"], &[LOGICAL_NAME]]
}

/// Fresh in-memory `SQLite` backend registered under every logical name.
async fn mem_router() -> Arc<DatabaseRouter> {
    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("connect sqlite"),
    );
    let mut router = DatabaseRouter::new();
    register_backend_logical_names_slices(
        &mut router,
        backend,
        router_groups(),
        RegisterBackendLogicalNamesOptions::default(),
    );
    Arc::new(router)
}

fn valence_for(router: Arc<DatabaseRouter>, actor: Actor) -> Valence {
    Valence::builder()
        .database_router(router)
        .default_backend_key(router_key(LOGICAL_NAME, SQLITE_ENGINE_ID))
        .with_actor(actor)
        .build()
        .expect("valence build")
}

#[test]
fn test_user_counter_implements_id_holder() {
    fn assert_id_holder<T: IdHolder>() {}
    assert_id_holder::<UserCounter>();
}

#[test]
fn test_user_counter_has_user_thing() {
    let user = RecordId::new("user", "test123");
    let counter = UserCounter::new(user, 0).expect("new should succeed");
    let t = counter.user_thing();
    assert_eq!(t.id(), "test123");
}

#[test]
fn test_connection_method_pointers_exist() {
    let _ = UserCounter::get_from_user_id;
    let _ = UserCounter::get_from_user;
    let _ = UserCounter::get_user;
}

#[test]
fn test_user_counter_schema_registered_with_connections() {
    let meta = SchemaRegistry::global()
        .get_schema("user_counter")
        .expect("user_counter schema must be linked");
    assert!(
        !meta.schema.connections.is_empty(),
        "user_counter should declare at least one connection"
    );
}

#[tokio::test]
async fn get_from_user_id_returns_seeded_row() {
    let v = valence_for(
        mem_router().await,
        Actor::User {
            user_id: "alice".into(),
        },
    );

    let created = UserCounter::create(
        UserCounter::new(RecordId::new("user", "alice"), 7).expect("new"),
        &v,
    )
    .await
    .expect("create");

    let found = UserCounter::get_from_user_id("alice", &v)
        .await
        .expect("get_from_user_id");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].user_thing().id(), "alice");
    assert_eq!(
        found[0].id().map(|id| id.id().to_string()),
        created.id().map(|id| id.id().to_string())
    );
}
