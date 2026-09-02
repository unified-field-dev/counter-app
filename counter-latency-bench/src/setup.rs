//! Back-compat re-exports for tier-0 setup helpers.
use anyhow::Result;
use valence::Valence;

pub use crate::stack::tier0::actor_label;

pub async fn setup_bench(
    user_id: &str,
    seed_value: i64,
    permission_cache: bool,
) -> Result<(Valence, String)> {
    crate::stack::tier0::assert_schemas_linked()?;
    let (_, router) = crate::stack::tier0::mem_db_and_router().await?;
    crate::stack::tier0::seed_counters(router, user_id, seed_value, permission_cache).await
}
