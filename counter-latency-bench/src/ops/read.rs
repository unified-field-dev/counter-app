use std::time::Instant;

use anyhow::{Context, Result};
use counter_app_worker::generated::{Counter, UserCounter};
use valence::{Model, Valence};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadStepMs {
    pub user_counter_get_ms: f64,
    pub counter_get_ms: f64,
    pub total_ms: f64,
}

pub async fn run_read_once(v: &Valence, user_pk: &str) -> Result<ReadStepMs> {
    let total_start = Instant::now();

    let t0 = Instant::now();
    let _ = UserCounter::get(user_pk, v)
        .await
        .context("UserCounter::get")?
        .context("user_counter row missing — seed failed")?;
    let user_counter_get_ms = elapsed_ms(t0);

    let t1 = Instant::now();
    let _ = Counter::get("singleton", v)
        .await
        .context("Counter::get")?
        .context("counter singleton missing — seed failed")?;
    let counter_get_ms = elapsed_ms(t1);

    Ok(ReadStepMs {
        user_counter_get_ms,
        counter_get_ms,
        total_ms: elapsed_ms(total_start),
    })
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
