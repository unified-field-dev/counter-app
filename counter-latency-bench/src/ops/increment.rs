use std::time::Instant;

use anyhow::Result;
use serde::Serialize;
use valence::Valence;

use super::read::{run_read_once, ReadStepMs};
use super::write::{run_write_once, WriteStepMs};

#[derive(Debug, Clone, Serialize)]
pub struct IncrementStepMs {
    pub user_counter_get_ms: f64,
    pub counter_get_ms: f64,
    pub user_counter_commit_ms: f64,
    pub counter_commit_ms: f64,
    pub increment_total_ms: f64,
}

pub async fn run_increment_once(v: &Valence, user_pk: &str) -> Result<IncrementStepMs> {
    let total_start = Instant::now();

    let read = run_read_once(v, user_pk).await?;
    let write = run_write_once(v, user_pk).await?;

    Ok(IncrementStepMs {
        user_counter_get_ms: read.user_counter_get_ms,
        counter_get_ms: read.counter_get_ms,
        user_counter_commit_ms: write.user_counter_commit_ms,
        counter_commit_ms: write.counter_commit_ms,
        increment_total_ms: elapsed_ms(total_start),
    })
}

impl From<ReadStepMs> for IncrementStepMs {
    fn from(read: ReadStepMs) -> Self {
        Self {
            user_counter_get_ms: read.user_counter_get_ms,
            counter_get_ms: read.counter_get_ms,
            user_counter_commit_ms: 0.0,
            counter_commit_ms: 0.0,
            increment_total_ms: read.total_ms,
        }
    }
}

impl From<WriteStepMs> for IncrementStepMs {
    fn from(write: WriteStepMs) -> Self {
        Self {
            user_counter_get_ms: 0.0,
            counter_get_ms: 0.0,
            user_counter_commit_ms: write.user_counter_commit_ms,
            counter_commit_ms: write.counter_commit_ms,
            increment_total_ms: write.total_ms,
        }
    }
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
