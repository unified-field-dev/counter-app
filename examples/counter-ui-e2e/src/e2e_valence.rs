//! Process-wide Valence + Higgs for Playwright (counter schemas).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, OnceLock};

use chrono::Utc;
use counter_app::permissions::{COUNTER_ADMIN_GROUP_ID, COUNTER_ADMIN_GROUP_NAME};
use counter_app_worker::embedded_surreal::LOGICAL_NAME;
use counter_app_worker::generated::UserCounter;
use gauge::manifest_sync::{
    sync_permission_manifests, PermissionDomainInput, PermissionInput, PermissionManifestInput,
};
use gauge::service;
use gauge::super_user::SUPER_USER_GROUP_NAME;
use higgs::actor_policy::external_actor_json_policy;
use higgs::{HiggsConfig, HiggsValenceFactory};
use lepton_identity::generated::{User, UserStatus, UserUserType};
use valence::{
    register_backend_logical_names, router_key, Actor, DatabaseBackend, DatabaseRouter, Model,
    RecordId, RegisterBackendLogicalNamesOptions, RouterValenceFactory, RouterValenceFactoryConfig,
    SqliteBackend, Valence, ValenceFactory, SQLITE_ENGINE_ID,
};

struct E2eState {
    router: Arc<DatabaseRouter>,
    higgs: Arc<HiggsConfig>,
    default_backend_key: String,
}

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

struct HiggsFactory(RouterValenceFactory);

impl HiggsValenceFactory for HiggsFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        self.0.build(actor_json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

fn prepare_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    // SAFETY: host boot only.
    unsafe {
        if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

async fn seed_user(id: &str, email_verified: bool, valence: &Valence) {
    let now = Utc::now();
    let confirmed_at = email_verified.then_some(now);
    let user = User::new(
        Some(UserUserType::Person),
        Some("e2e-password-hash".to_string()),
        Some(UserStatus::Active),
        None,
        None,
        confirmed_at,
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    User::upsert(id, user, valence).await.expect("upsert user");
}

/// Build shared Valence/Higgs once and seed baseline users.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    prepare_env();

    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("SqliteBackend::connect_memory"),
    );
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        counter_app_worker::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions::default(),
    );
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        gauge::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions::default(),
    );
    let router = Arc::new(router);
    let default_key = router_key(LOGICAL_NAME, SQLITE_ENGINE_ID);

    let system = Valence::builder()
        .database_router(Arc::clone(&router))
        .default_backend_key(default_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_counter_host".into(),
        })
        .build()
        .expect("e2e Valence");

    seed_user("owner", true, &system).await;
    seed_user("unverified", false, &system).await;
    seed_user("alice", true, &system).await;
    seed_user("bob", true, &system).await;
    seed_user("carol", true, &system).await;
    // Verified member sad-path tests need a Gauge principal without CounterAdmin.
    ensure_user_gauge_principal(&system, "alice").await;

    seed_super_user_with_member(&system, "owner").await;
    sync_permission_manifests(&system, &[counter_admin_manifest()])
        .await
        .expect("sync CounterAdmin manifest");
    ensure_counter_admin_group(&system).await;
    let owner_ctx = system.with_actor(Actor::User {
        user_id: "owner".to_string(),
    });
    wire_counter_admin_group(&owner_ctx, &system, "owner").await;
    demote_owner_from_super_user(&system).await;

    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(HiggsFactory(RouterValenceFactory::new(
        Arc::clone(&router),
        RouterValenceFactoryConfig::new(default_key.clone())
            .actor_json_policy(external_actor_json_policy()),
    )));
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        router,
        higgs,
        default_backend_key: default_key,
    });
    let _ = E2E_STATE.set(state);
}

fn state() -> Arc<E2eState> {
    E2E_STATE
        .get()
        .expect("init_e2e_valence must run first")
        .clone()
}

pub fn e2e_router() -> Arc<DatabaseRouter> {
    Arc::clone(&state().router)
}

pub fn e2e_higgs_config() -> Arc<HiggsConfig> {
    Arc::clone(&state().higgs)
}

pub fn e2e_system_valence() -> Valence {
    Valence::builder()
        .database_router(e2e_router())
        .default_backend_key(state().default_backend_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_seed".into(),
        })
        .build()
        .expect("system valence")
}

/// Seed leaderboard rows (ordered high → low) under System Valence.
pub async fn seed_leaderboard_scores() -> anyhow::Result<Vec<(String, i64)>> {
    let system = e2e_system_valence();
    let rows = [("alice", 30_i64), ("bob", 20_i64), ("carol", 10_i64)];
    let mut out = Vec::with_capacity(rows.len());
    for (id, score) in rows {
        let user_thing = RecordId::new("user", id);
        let counter = UserCounter::new(user_thing, score)?;
        UserCounter::upsert(id, counter, &system).await?;
        out.push((id.to_string(), score));
    }
    Ok(out)
}

/// Read current global singleton value (missing → 0).
pub async fn read_global_value() -> anyhow::Result<usize> {
    let system = e2e_system_valence();
    let got = counter_app_worker::get_global(&system).await?;
    Ok(got.value)
}

/// Reset global counter to zero for Playwright isolation (System actor seed op).
pub async fn reset_global_counter() -> anyhow::Result<()> {
    let system = e2e_system_valence();
    let _ = counter_app_worker::set_global(0, &system).await?;
    Ok(())
}

/// Reset one user's personal counter for Playwright isolation.
pub async fn reset_user_counter(user_id: &str) -> anyhow::Result<()> {
    let system = e2e_system_valence();
    let user_thing = RecordId::new("user", user_id);
    let counter = UserCounter::new(user_thing, 0)?;
    UserCounter::upsert(user_id, counter, &system).await?;
    Ok(())
}

fn counter_admin_manifest() -> PermissionManifestInput {
    PermissionManifestInput {
        app_id: "counter".into(),
        domains: vec![PermissionDomainInput {
            key: "counter".into(),
            name: "Counter".into(),
            description: "Counter demo application".into(),
            permissions: vec![PermissionInput {
                name: "CounterAdmin".into(),
                description: "Manage counter demo settings".into(),
            }],
        }],
    }
}

async fn ensure_counter_admin_group(system: &Valence) {
    let now = Utc::now();
    let group = gauge::generated::PermissionGroup::new(
        COUNTER_ADMIN_GROUP_NAME.to_string(),
        Some("Operators who may set the global counter and use /counter/admin".to_string()),
        now,
        now,
    )
    .expect("build counter admin group");
    gauge::generated::PermissionGroup::upsert(COUNTER_ADMIN_GROUP_ID, group, system)
        .await
        .expect("upsert counter admin group");
}

async fn ensure_user_gauge_principal(system: &Valence, user_id: &str) {
    use lepton_identity::generated::User;

    let user = User::get(user_id, system)
        .await
        .expect("get seed user")
        .expect("seed user exists");
    gauge::generated::PermissionUserPrincipal::upsert(
        &format!("user:{user_id}"),
        gauge::generated::PermissionUserPrincipal::new(
            user.id().expect("user id").clone(),
            user_id.to_string(),
        )
        .expect("user principal"),
        system,
    )
    .await
    .expect("upsert user principal");
}

async fn wire_counter_admin_group(grant_actor: &Valence, system: &Valence, user_id: &str) {
    let perms = service::list_permissions(grant_actor, None)
        .await
        .expect("list permissions");
    let counter_admin = perms
        .into_iter()
        .find(|p| p.name == "CounterAdmin")
        .expect("CounterAdmin after sync");
    service::grant_permission_to_group(&counter_admin.id, COUNTER_ADMIN_GROUP_ID, grant_actor)
        .await
        .expect("grant CounterAdmin to counter_admin group");

    let group = gauge::generated::PermissionGroup::get(COUNTER_ADMIN_GROUP_ID, system)
        .await
        .expect("get counter_admin group")
        .expect("counter_admin group exists");
    ensure_user_gauge_principal(system, user_id).await;
    let principal =
        gauge::generated::PermissionUserPrincipal::get(&format!("user:{user_id}"), system)
            .await
            .expect("get user principal")
            .expect("user principal exists");
    group
        .relate_to_member_record(principal.id().expect("principal id"), system)
        .await
        .expect("relate counter_admin member");
}

/// Idempotent: ensure owner holds CounterAdmin via the `counter_admin` group.
pub async fn refresh_owner_counter_admin_membership() -> anyhow::Result<()> {
    let system = e2e_system_valence();
    ensure_counter_admin_group(&system).await;
    seed_super_user_with_member(&system, "owner").await;
    let grant_ctx = system.with_actor(Actor::User {
        user_id: "owner".to_string(),
    });
    wire_counter_admin_group(&grant_ctx, &system, "owner").await;
    demote_owner_from_super_user(&system).await;
    Ok(())
}

async fn seed_super_user_with_member(system: &Valence, member_user_id: &str) {
    let super_group = gauge::generated::PermissionGroup::new(
        SUPER_USER_GROUP_NAME.to_string(),
        Some("super users".to_string()),
        Utc::now(),
        Utc::now(),
    )
    .expect("build super user group");
    let created =
        gauge::generated::PermissionGroup::upsert("super_user_group", super_group, system)
            .await
            .expect("upsert super user group");

    let member = User::get(member_user_id, system)
        .await
        .expect("query member")
        .expect("member exists");
    let principal = gauge::generated::PermissionUserPrincipal::upsert(
        &format!("user:{member_user_id}"),
        gauge::generated::PermissionUserPrincipal::new(
            member.id().expect("member id").clone(),
            member_user_id.to_string(),
        )
        .expect("new principal"),
        system,
    )
    .await
    .expect("upsert principal");
    created
        .relate_to_owner_record(principal.id().expect("principal id"), system)
        .await
        .expect("relate super owner");
    created
        .relate_to_member_record(principal.id().expect("principal id"), system)
        .await
        .expect("relate super member");
}

async fn demote_owner_from_super_user(system: &Valence) {
    let Some(super_group) = gauge::generated::PermissionGroup::get("super_user_group", system)
        .await
        .expect("get super user group")
    else {
        return;
    };
    let Some(principal) = gauge::generated::PermissionUserPrincipal::get("user:owner", system)
        .await
        .expect("get owner principal")
    else {
        return;
    };
    let pid = principal.id().expect("principal id").clone();
    let _ = super_group.unrelate_from_member_record(&pid, system).await;
    let _ = super_group.unrelate_from_owner_record(&pid, system).await;
}
