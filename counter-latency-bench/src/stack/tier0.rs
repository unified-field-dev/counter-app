use std::sync::Arc;

use anyhow::{Context, Result};
use counter_app_worker::generated::{Counter, UserCounter};
use valence::schema::SchemaRegistry;
use valence::{Actor, DatabaseRouter, Model, RecordId, SDb, SurrealMemBackend, Valence};

const BENCH_NS: &str = "counter_bench";

pub fn router_groups() -> &'static [&'static [&'static str]] {
    // Same embedded Surreal handle; distinct logical names for subsystem Valence stores.
    &[&["default"], &["boson"], &["chronon"], &["photon"]]
}

pub fn assert_schemas_linked() -> Result<()> {
    let reg = SchemaRegistry::global();
    anyhow::ensure!(
        reg.get_schema("counter").is_some(),
        "counter schema not linked — depend on counter-app-worker"
    );
    anyhow::ensure!(
        reg.get_schema("user_counter").is_some(),
        "user_counter schema not linked — depend on counter-app-worker"
    );
    Ok(())
}

pub async fn mem_db_and_router() -> Result<(SDb, Arc<DatabaseRouter>)> {
    let db = SDb::init();
    db.connect::<surrealdb::engine::local::Mem>(())
        .await
        .context("connect in-memory Surreal")?;
    db.use_ns(BENCH_NS)
        .use_db(BENCH_NS)
        .await
        .context("use ns/db")?;
    let mut router = DatabaseRouter::new();
    SurrealMemBackend::register_embedded_logical_names_slices(
        &mut router,
        db.clone(),
        router_groups(),
    );
    DatabaseRouter::set_global(router);
    Ok((db, DatabaseRouter::global()))
}

pub fn build_valence(router: Arc<DatabaseRouter>, actor: Actor, permission_cache: bool) -> Valence {
    let mut builder = Valence::new(router).with_actor(actor);
    if permission_cache {
        builder = builder.enable_permission_cache();
    }
    builder.build()
}

pub async fn seed_counters(
    router: Arc<DatabaseRouter>,
    user_id: &str,
    seed_value: i64,
    permission_cache: bool,
) -> Result<(Valence, String)> {
    let user_pk = user_id.to_string();
    let user_record = RecordId::new("user", user_id);

    let v_seed = build_valence(
        router.clone(),
        Actor::User {
            user_id: user_pk.clone(),
        },
        permission_cache,
    );

    Counter::upsert(
        "singleton",
        Counter::new(seed_value).context("Counter::new for seed")?,
        &v_seed,
    )
    .await
    .context("seed Counter singleton")?;

    UserCounter::upsert(
        &user_pk,
        UserCounter::new(user_record, seed_value).context("UserCounter::new for seed")?,
        &v_seed,
    )
    .await
    .context("seed UserCounter")?;

    let v = build_valence(
        router,
        Actor::User {
            user_id: user_pk.clone(),
        },
        permission_cache,
    );

    Ok((v, user_pk))
}

pub fn actor_label(user_id: &str) -> String {
    format!("User({user_id})")
}
