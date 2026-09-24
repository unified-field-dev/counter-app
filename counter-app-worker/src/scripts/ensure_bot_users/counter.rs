//! Backfill `UserCounter` for bots created before counters were seeded.
//!
//! Teaching path for "row may already exist": `UserCounter::get`; if `None`,
//! `UserCounter::new` + `UserCounter::upsert` with the bot's `reset_score`.

use anyhow::Result;
use valence::Valence;

use super::super::bot_roster::BotDef;

/// Ensure an existing bot user has a `UserCounter` (may be missing if the user
/// was created by an earlier version of this script that did not seed counters).
///
/// Uses Valence [`Model`](valence::Model) get / upsert on
/// [`crate::generated::UserCounter`]. Idempotent when the counter already exists.
pub(super) async fn backfill_bot_counter(
    valence: &Valence,
    bot: &BotDef,
    user: &lepton_identity::generated::User,
) -> Result<()> {
    use crate::generated::UserCounter;
    use valence::Model;

    let user_record = user
        .id()
        .ok_or_else(|| anyhow::anyhow!("Bot user {} missing ID", bot.email))?
        .clone();
    let user_id_clean = valence::extract_id_from_record(&user_record)
        .map_err(|e| anyhow::anyhow!("Bot user {} invalid ID: {e}", bot.email))?;

    if UserCounter::get(&user_id_clean, valence, valence::use_!(r#"In **Counter app**, we **load User Counter** so the application can decide what to do next in this workflow. The result is used by **Counter app** logic—not necessarily displayed on a page unless that feature’s UI shows it."#)).await?.is_none() {
        let counter = UserCounter::new(user_record, bot.reset_score)
            .map_err(|e| anyhow::anyhow!("Failed to create UserCounter for {}: {e}", bot.email))?;
        UserCounter::upsert(&user_id_clean, counter, valence, valence::use_!(r#"When **Counter app** needs to persist work, we **save User Counter** so the next step in that feature can continue with the latest values. People and services allowed for **Counter app** use this data for that workflow—not as a general export of unrelated personal fields."#))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upsert UserCounter for {}: {e}", bot.email))?;
        log::info!(
            "[counter-app-worker] ensure_bot_users: Created missing UserCounter for {} (score {})",
            bot.email,
            bot.reset_score
        );
    }
    Ok(())
}
