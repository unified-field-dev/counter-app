//! Integration tests: generated [`Counter`] CRUD + `valence::ownership` hooks.
//!
//! Run: `cargo test -p counter-app --test ownership_model_integration --features ssr`

#![cfg(feature = "ssr")]
#![allow(missing_docs)]
// Integration-test helpers fall outside clippy's `allow-expect-in-tests` scope.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use counter_app::embedded_surreal::LOGICAL_NAME;
use counter_app::generated::Counter;
use std::sync::Arc;
use valence::actor::Actor;
use valence::owner_ref::OwnerRef;
use valence::ownership::{normalize_record_id_for_ownership, OwnershipService};
use valence::schema::SchemaRegistry;

use valence::{
    register_backend_logical_names_slices, router_key, BatchCreatable, DatabaseBackend,
    DatabaseRouter, Model, RegisterBackendLogicalNamesOptions, SqliteBackend, Valence,
    SQLITE_ENGINE_ID,
};

const fn router_groups() -> &'static [&'static [&'static str]] {
    // `counter` rows route to the app embedded DB; ownership tables use `default`.
    &[&["default"], &[LOGICAL_NAME]]
}

fn assert_registry_has_counter() {
    assert!(
        SchemaRegistry::global().get_schema("counter").is_some(),
        "counter schema must be linked (counter-app `schemas` module)"
    );
}

/// Fresh in-memory `SQLite` backend registered under every logical name.
///
/// Each call opens an isolated database, so tests don't need namespaces.
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

#[tokio::test]
async fn counter_create_writes_ownership_from_actor() {
    assert_registry_has_counter();
    let v = valence_for(
        mem_router().await,
        Actor::User {
            user_id: "alice".into(),
        },
    );

    let created = Counter::create(Counter::new(5).expect("new"), &v)
        .await
        .expect("create");
    let bare = normalize_record_id_for_ownership(created.id().expect("id").id());
    let own = OwnershipService::get_ownership_json("counter", &bare, &v)
        .await
        .expect("get ownership")
        .expect("ownership row");
    assert_eq!(own.get("owner_id").and_then(|x| x.as_str()), Some("alice"));
    assert_eq!(own.get("owner_type").and_then(|x| x.as_str()), Some("user"));
    assert_eq!(own.get("status").and_then(|x| x.as_str()), Some("active"));
}

#[tokio::test]
async fn counter_upsert_update_leaves_owner_unchanged() {
    assert_registry_has_counter();
    let router = mem_router().await;

    let v_alice = valence_for(
        router.clone(),
        Actor::User {
            user_id: "alice".into(),
        },
    );
    let created = Counter::create(Counter::new(1).expect("new"), &v_alice)
        .await
        .expect("create");
    let bare = normalize_record_id_for_ownership(created.id().expect("id").id());

    let v_bob = valence_for(
        router,
        Actor::User {
            user_id: "bob".into(),
        },
    );
    Counter::upsert(&bare, Counter::new(99).expect("new"), &v_bob)
        .await
        .expect("upsert update");

    let own = OwnershipService::get_ownership_json("counter", &bare, &v_bob)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(own.get("owner_id").and_then(|x| x.as_str()), Some("alice"));
}

#[tokio::test]
async fn counter_upsert_create_path_writes_ownership() {
    assert_registry_has_counter();
    let v = valence_for(
        mem_router().await,
        Actor::User {
            user_id: "carol".into(),
        },
    );

    let id = "upsert-create-only-1";
    let upserted = Counter::upsert(id, Counter::new(2).expect("new"), &v)
        .await
        .expect("upsert insert");
    let bare = normalize_record_id_for_ownership(upserted.id().expect("id").id());
    let own = OwnershipService::get_ownership_json("counter", &bare, &v)
        .await
        .unwrap()
        .expect("ownership");
    assert_eq!(own.get("owner_id").and_then(|x| x.as_str()), Some("carol"));
}

#[tokio::test]
async fn counter_delete_marks_ownership_pending_deletion() {
    assert_registry_has_counter();
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    let router = mem_router().await;

    let v_alice = valence_for(
        router.clone(),
        Actor::User {
            user_id: "alice".into(),
        },
    );
    let created = Counter::create(Counter::new(3).expect("new"), &v_alice)
        .await
        .expect("create");
    let bare = normalize_record_id_for_ownership(created.id().expect("id").id());

    let v_system = valence_for(
        router,
        Actor::System {
            operation: "test-delete".into(),
        },
    );
    Counter::delete(&bare, &v_system).await.expect("delete");

    let own = OwnershipService::get_ownership_json("counter", &bare, &v_system)
        .await
        .unwrap()
        .expect("ownership row survives delete");
    assert_eq!(
        own.get("status").and_then(|x| x.as_str()),
        Some("pending_deletion")
    );
}

#[tokio::test]
async fn counter_create_owner_override_wins_over_actor() {
    assert_registry_has_counter();
    let v = valence_for(
        mem_router().await,
        Actor::User {
            user_id: "alice".into(),
        },
    );
    let v = v.with_owner_override(OwnerRef {
        owner_id: "app-99".into(),
        owner_kind: valence::OwnerKind::Application,
    });

    let created = Counter::create(Counter::new(4).expect("new"), &v)
        .await
        .expect("create");
    let bare = normalize_record_id_for_ownership(created.id().expect("id").id());
    let own = OwnershipService::get_ownership_json("counter", &bare, &v)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(own.get("owner_id").and_then(|x| x.as_str()), Some("app-99"));
    assert_eq!(
        own.get("owner_type").and_then(|x| x.as_str()),
        Some("application")
    );
}

/// The batch runtime is host-owned on valence main; this exercises the codegen
/// batch-create ownership hook the way a host batch executor would.
#[tokio::test]
async fn counter_batch_create_writes_ownership_from_actor() {
    assert_registry_has_counter();
    let v = valence_for(
        mem_router().await,
        Actor::User {
            user_id: "carol".into(),
        },
    );

    let backend = v
        .backend_for_table(<Counter as BatchCreatable>::table_name())
        .expect("backend for counter");
    let data = serde_json::to_value(Counter::new(9).expect("new")).expect("serialize");
    let row = backend
        .create_record(<Counter as BatchCreatable>::table_name(), data)
        .await
        .expect("batch create");
    Counter::ensure_ownership_after_batch_create(row.clone(), &v)
        .await
        .expect("ownership hook");

    let rid: valence::RecordId =
        serde_json::from_value(row.get("id").cloned().expect("id")).expect("record id");
    let bare = normalize_record_id_for_ownership(rid.id());
    let own = OwnershipService::get_ownership_json("counter", &bare, &v)
        .await
        .expect("get ownership")
        .expect("ownership row");
    assert_eq!(own.get("owner_id").and_then(|x| x.as_str()), Some("carol"));
    assert_eq!(own.get("owner_type").and_then(|x| x.as_str()), Some("user"));
    assert_eq!(own.get("status").and_then(|x| x.as_str()), Some("active"));
}

#[tokio::test]
async fn user_counter_create_writes_ownership_from_actor() {
    assert!(
        SchemaRegistry::global()
            .get_schema("user_counter")
            .is_some(),
        "user_counter schema must be linked"
    );
    let v = valence_for(
        mem_router().await,
        Actor::User {
            user_id: "alice".into(),
        },
    );

    let created = counter_app::generated::UserCounter::create(
        counter_app::generated::UserCounter::new(valence::RecordId::new("user", "alice"), 1)
            .expect("new"),
        &v,
    )
    .await
    .expect("create");
    let bare = normalize_record_id_for_ownership(created.id().expect("id").id());
    let own = OwnershipService::get_ownership_json("user_counter", &bare, &v)
        .await
        .expect("get ownership")
        .expect("ownership row");
    assert_eq!(own.get("owner_id").and_then(|x| x.as_str()), Some("alice"));
    assert_eq!(own.get("owner_type").and_then(|x| x.as_str()), Some("user"));
    assert_eq!(own.get("status").and_then(|x| x.as_str()), Some("active"));
}

#[tokio::test]
async fn user_counter_update_denied_for_non_owner() {
    assert!(
        SchemaRegistry::global()
            .get_schema("user_counter")
            .is_some(),
        "user_counter schema must be linked"
    );
    let router = mem_router().await;
    let v_alice = valence_for(
        router.clone(),
        Actor::User {
            user_id: "alice".into(),
        },
    );
    let created = counter_app::generated::UserCounter::create(
        counter_app::generated::UserCounter::new(valence::RecordId::new("user", "alice"), 3)
            .expect("new"),
        &v_alice,
    )
    .await
    .expect("create");

    let v_bob = valence_for(
        router,
        Actor::User {
            user_id: "bob".into(),
        },
    );
    let err = created
        .get_mutable(&v_bob)
        .set_value(99)
        .expect("set_value")
        .commit()
        .await;
    assert!(
        err.is_err(),
        "non-owner update should be denied by OWNER_BY_USER_FIELD"
    );
}
