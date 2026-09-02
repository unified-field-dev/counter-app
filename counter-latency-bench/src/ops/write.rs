use std::time::Instant;

use anyhow::{Context, Result};
use counter_app_worker::generated::{Counter, UserCounter};
use valence::{Model, Valence};

#[derive(Debug, Clone, serde::Serialize)]
pub struct WriteStepMs {
    pub user_counter_commit_ms: f64,
    pub counter_commit_ms: f64,
    pub total_ms: f64,
}

pub async fn run_write_once(v: &Valence, user_pk: &str) -> Result<WriteStepMs> {
    let total_start = Instant::now();

    let t0 = Instant::now();
    let user_counter = UserCounter::get(user_pk, v)
        .await
        .context("UserCounter::get")?
        .context("user_counter row missing — seed failed")?;
    let updated_user = user_counter
        .get_mutable(v)
        .set_value(*user_counter.value() + 1)
        .context("UserCounter set_value")?
        .commit()
        .await
        .context("UserCounter commit")?;
    let _ = updated_user;
    let user_counter_commit_ms = elapsed_ms(t0);

    let t1 = Instant::now();
    let global = Counter::get("singleton", v)
        .await
        .context("Counter::get")?
        .context("counter singleton missing — seed failed")?;
    let updated_global = global
        .get_mutable(v)
        .set_value(*global.value() + 1)
        .context("Counter set_value")?
        .commit()
        .await
        .context("Counter commit")?;
    let _ = updated_global;
    let counter_commit_ms = elapsed_ms(t1);

    Ok(WriteStepMs {
        user_counter_commit_ms,
        counter_commit_ms,
        total_ms: elapsed_ms(total_start),
    })
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
