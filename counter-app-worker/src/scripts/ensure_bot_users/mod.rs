//! Idempotent seeding: ensure the bot roster exists as real `User` + `UserCounter` rows.
//!
//! Chronon entry [`ensure_bot_users`] takes `ScriptContext` and calls
//! `chronon_valence_identity::valence_from_context`, then
//! `ensure_bot_users_seed`. Hosts can also call the seed helper directly at
//! preflight with an already-built [`valence::Valence`].

use anyhow::Result;
use valence::Valence;

use super::bot_roster::BOT_ROSTER;

mod counter;
mod create;

use counter::backfill_bot_counter;
use create::create_bot_user;

/// Ensure every [`super::bot_roster::BOT_ROSTER`] entry has identity + counter rows.
///
/// For each bot:
/// 1. Query `AccountEmail` by address; if present, resolve the owning `User` and
///    `backfill_bot_counter` when the counter is missing.
/// 2. Otherwise create `User` + `Account` + membership + email + profile
///    (Lepton identity models), then upsert a `UserCounter` at `reset_score`.
///
/// Safe to re-run: existing emails skip create. Prefer this helper from server
/// preflight; the Chronon wrapper is [`ensure_bot_users`].
pub async fn ensure_bot_users_seed(valence: &Valence) -> Result<()> {
    use lepton_identity::generated::{AccountEmail, User};
    use valence::{RecordPredicate, StringPredicate};

    log::info!(
        "[counter-app-worker] ensure_bot_users: Ensuring {} bot users exist",
        BOT_ROSTER.len()
    );

    let mut created = 0u32;

    for bot in BOT_ROSTER {
        let existing_email = AccountEmail::query(valence)
            .where_address(StringPredicate::Equals(bot.email.to_string()))
            .limit(1)
            .first()
            .await?;

        if let Some(email_row) = existing_email {
            let Some(email_id) = email_row.id().cloned() else {
                anyhow::bail!("Bot email {} missing id", bot.email);
            };
            let existing = User::query(valence)
                .where_primary_email(RecordPredicate::Equals(email_id))
                .limit(1)
                .first()
                .await?;
            if let Some(user) = existing {
                log::info!(
                    "[counter-app-worker] ensure_bot_users: {} already exists, skipping",
                    bot.email
                );
                backfill_bot_counter(valence, bot, &user).await?;
                continue;
            }
        }

        create_bot_user(valence, bot).await?;
        created += 1;
        log::info!(
            "[counter-app-worker] ensure_bot_users: Created {} (score {})",
            bot.email,
            bot.reset_score
        );
    }

    log::info!(
        "[counter-app-worker] ensure_bot_users: Done — created {} new bot(s), {} total in roster",
        created,
        BOT_ROSTER.len()
    );
    Ok(())
}

/// Chronon `run_once` entry that seeds the bot roster at coordinator boot.
///
/// Obtains Valence via `chronon_valence_identity::valence_from_context(&*ctx)`
/// and delegates to `ensure_bot_users_seed`. Default job: `ensure-bot-users`
/// with `run_once` (no cron).
#[chronon_coordinator_macros::script(
    name = "ensure_bot_users",
    default_job(job = "ensure-bot-users", run_once)
)]
pub async fn ensure_bot_users(ctx: Box<dyn chronon_core::ScriptContext>) -> Result<()> {
    let valence = chronon_valence_identity::valence_from_context(&*ctx)?;
    ensure_bot_users_seed(&valence).await
}
