//! Chronon script that resets personal and global counters once per day.
//!
//! Same platform shape as the bumper: `ScriptContext` →
//! `chronon_valence_identity::valence_from_context` → Valence `query` /
//! `get_mutable` / `upsert` on Counter models.

use anyhow::Result;
use lepton_identity::generated::User as GeneratedUser;
use valence::{extract_id_from_record, Model, Valence};

use crate::generated::{Counter, UserCounter};

use super::bot_roster::bot_reset_score;

/// Reset all user counter values and the global counter daily.
///
/// Builds Valence from Chronon `ScriptContext`, then:
///
/// 1. `UserCounter::query` loads every personal counter.
/// 2. For each row, resolves the user's email; bots get
///    `bot_reset_score`, real users get `0`.
/// 3. Writes via `get_mutable` → `set_value` → `commit`.
/// 4. Sets the global [`Counter`] singleton to the highest reset value (first
///    place), using mutate-or-upsert — not zero.
///
/// Default cron: midnight UTC (`0 0 * * *`).
#[chronon_coordinator_macros::script(
    name = "daily_highscores_reset",
    default_job(job = "daily-highscores-reset", cron = "0 0 * * *")
)]
pub async fn daily_highscores_reset(ctx: Box<dyn chronon_core::ScriptContext>) -> Result<()> {
    let valence = chronon_valence_identity::valence_from_context(&*ctx)?;
    daily_highscores_reset_with_valence(&valence).await
}

/// Valence-taking core of [`daily_highscores_reset`] (contract tests call this directly).
pub async fn daily_highscores_reset_with_valence(valence: &Valence) -> Result<()> {
    // Query all UserCounter records
    let counters = UserCounter::query(valence).await?;

    log::info!(
        "[counter-app-worker] daily_highscores_reset: Resetting {} user counters",
        counters.len()
    );

    let mut max_reset_value: i64 = 0;

    for counter in &counters {
        let counter_id = extract_id_from_record(
            counter
                .id()
                .ok_or_else(|| anyhow::anyhow!("Counter missing ID"))?,
        )?;

        // Resolve the user's email to determine whether this is a bot.
        let record_id = extract_id_from_record(counter.user())?;

        let reset_value = match GeneratedUser::get(&record_id, valence).await {
            Ok(Some(user)) => super::user_email::primary_email_address(&user, valence)
                .await
                .map_or(0, |email| bot_reset_score(&email).unwrap_or(0)),
            _ => 0,
        };

        if reset_value > max_reset_value {
            max_reset_value = reset_value;
        }

        counter
            .get_mutable(valence)
            .set_value(reset_value)
            .map_err(|e| anyhow::anyhow!("Failed to set value for counter {counter_id}: {e}"))?
            .commit()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update counter {counter_id}: {e}"))?;
    }

    // ── Set the global counter to the first-place score ──────────────
    let first_place_score = max_reset_value;

    let global_counter = Counter::get("singleton", valence).await?;

    if let Some(counter) = global_counter {
        counter
            .get_mutable(valence)
            .set_value(first_place_score)
            .map_err(|e| anyhow::anyhow!("Failed to set value for global counter: {e}"))?
            .commit()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update global counter: {e}"))?;
    } else {
        let new_counter = Counter::new(first_place_score)
            .map_err(|e| anyhow::anyhow!("Failed to create global counter: {e}"))?;

        Counter::upsert("singleton", new_counter, valence)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upsert global counter: {e}"))?;
    }

    log::info!(
        "[counter-app-worker] daily_highscores_reset: Global counter set to first-place value: {first_place_score}"
    );

    log::info!("[counter-app-worker] daily_highscores_reset: Completed successfully");
    Ok(())
}
