//! Chronon script that keeps one top-tier bot ahead of real users on the board.
//!
//! Teaching path: `#[chronon_coordinator_macros::script]` → `ScriptContext` →
//! `chronon_valence_identity::valence_from_context` → `UserCounter::query` /
//! `Model::get_mutable`. Inventory registration happens when this crate links
//! into a Chronon host.

use anyhow::Result;
use lepton_identity::generated::User as GeneratedUser;
use valence::{extract_id_from_record, Model, RecordId, Valence};

use crate::generated::{Counter, UserCounter};

use super::bot_roster::{is_bot_email, top_tier_bots};

struct LeaderboardEntry {
    email: String,
    score: i64,
    counter_id: String,
    user_record: RecordId,
}

/// Periodically adjust one top-tier bot to stay above real users.
///
/// Resolves Valence from Chronon via `chronon_valence_identity::valence_from_context`,
/// then:
///
/// 1. `UserCounter::query` ordered by score descending (full leaderboard).
/// 2. Resolves every entry's email (`User::get` + primary email) to tell bots from
///    real users.
/// 3. Starting from the **lowest-ranked** top-tier bot and working upward, finds
///    the first bot whose score is <= any real user directly above it.
/// 4. Sets that bot's score with `get_mutable` → `set_value` → `commit` to
///    `real_user_score + 1` plus a small per-rank offset.
/// 5. Exits after at most one bot write so each tick stays small.
///
/// Bottom-tier bots are never touched; they sit as background filler.
/// Default cron: every 30 seconds (`0,30 * * * * *`).
#[chronon_coordinator_macros::script(
    // Registers with Chronon inventory; default_job installs a cron on first boot.
    name = "bot_score_bumper",
    default_job(job = "bot-score-bumper", cron = "0,30 * * * * *")
)]
pub async fn bot_score_bumper(ctx: Box<dyn chronon_core::ScriptContext>) -> Result<()> {
    // Chronon → Valence bridge (same idea as Higgs::valence() on HTTP requests).
    let valence = chronon_valence_identity::valence_from_context(&*ctx)?;
    bot_score_bumper_with_valence(&valence).await
}

/// Valence-taking core of [`bot_score_bumper`] (contract tests call this directly).
#[allow(clippy::too_many_lines)] // leaderboard bump + notify paths stay in one body
pub async fn bot_score_bumper_with_valence(valence: &Valence) -> Result<()> {
    log::info!("[counter-app-worker] bot_score_bumper: Starting");

    // ── 1. Fetch the full leaderboard ───────────────────────────────────
    let counters = UserCounter::query(valence)
        .order_by_value(valence::SortDirection::Desc)
        .await?;

    if counters.is_empty() {
        log::info!("[counter-app-worker] bot_score_bumper: No user counters yet, nothing to do");
        return Ok(());
    }

    // ── 2. Build a resolved leaderboard with emails ─────────────────────
    let mut board: Vec<LeaderboardEntry> = Vec::with_capacity(counters.len());

    for counter in &counters {
        let user_record = counter.user().clone();
        let record_id = extract_id_from_record(counter.user()).unwrap_or_default();

        let email = match GeneratedUser::get(&record_id, valence).await {
            Ok(Some(user)) => {
                match super::user_email::primary_email_address(&user, valence).await {
                    Some(address) => address,
                    None => continue,
                }
            }
            _ => continue, // orphaned counter — skip
        };

        let counter_id = extract_id_from_record(
            counter
                .id()
                .ok_or_else(|| anyhow::anyhow!("Counter missing ID"))?,
        )?;

        board.push(LeaderboardEntry {
            email,
            score: *counter.value(),
            counter_id,
            user_record,
        });
    }

    // ── 3. Find highest real-user score above each top-tier bot ─────────
    // Walk the board (already sorted desc). Track the highest real user
    // score seen so far.
    let mut max_real_score: Option<i64> = None;

    // Collect top-tier bot emails for fast lookup.
    let top_emails: Vec<&str> = top_tier_bots().map(|b| b.email).collect();

    // We want to process bots from lowest-ranked upward. Since the board is
    // sorted desc, the lowest-ranked top-tier bot appears last. Scan from
    // top to bottom to accumulate `max_real_score`, then pick the first
    // out-of-order bot.
    //
    // "Out of order" means a top-tier bot whose current score is <= the
    // highest real user score seen above it in the leaderboard.
    let mut target: Option<(usize, i64)> = None; // (board index, real score above)

    for (idx, entry) in board.iter().enumerate() {
        let is_top_bot = top_emails.contains(&entry.email.as_str());
        let is_bot = is_bot_email(&entry.email);

        if !is_bot {
            // Real user — update running max.
            max_real_score = Some(max_real_score.map_or(entry.score, |m: i64| m.max(entry.score)));
        }

        if is_top_bot {
            if let Some(real_above) = max_real_score {
                if entry.score <= real_above {
                    // This bot is out of order — a real user is above it.
                    target = Some((idx, real_above));
                    // Don't break — we want the *lowest-ranked* out-of-order
                    // bot, so keep scanning.
                }
            }
        }
    }

    // ── 4. Bump the target bot ──────────────────────────────────────────
    let (target_idx, new_score) = if let Some((target_idx, real_score_above)) = target {
        let entry = &board[target_idx];
        let rank_offset = top_emails
            .iter()
            .position(|&e| e == entry.email)
            .and_then(|p| i64::try_from(p).ok())
            .unwrap_or(0);
        let new_score = real_score_above + 1 + rank_offset;
        (target_idx, new_score)
    } else {
        // No real user is ahead of a top-tier bot. Still nudge the lowest
        // top-tier bot so the leaderboard keeps moving for spectators.
        let Some((idx, entry)) = board
            .iter()
            .enumerate()
            .rev()
            .find(|(_, e)| top_emails.contains(&e.email.as_str()))
        else {
            log::info!(
                "[counter-app-worker] bot_score_bumper: No top-tier bots on the board, nothing to do"
            );
            return Ok(());
        };
        log::info!(
            "[counter-app-worker] bot_score_bumper: Idle nudge for {} (score {})",
            entry.email,
            entry.score
        );
        (idx, entry.score + 1)
    };

    let entry = &board[target_idx];

    log::info!(
        "[counter-app-worker] bot_score_bumper: Bumping {} from {} to {}",
        entry.email,
        entry.score,
        new_score
    );

    update_bot_counter(valence, entry, new_score).await?;

    log::info!(
        "[counter-app-worker] bot_score_bumper: {} bumped to {}",
        entry.email,
        new_score
    );

    // ── 5. Bump the global counter and publish the event ─────────────
    let global_val = bump_global_counter(valence, new_score - entry.score).await?;

    if let Err(e) = (crate::events::CounterUpdated {
        new_value: global_val,
    })
    .publish()
    .await
    {
        log::info!(
            "[counter-app-worker] bot_score_bumper: WARN: Failed to publish CounterUpdated: {e}"
        );
    }

    log::info!("[counter-app-worker] bot_score_bumper: Done. Global counter now {global_val}");
    Ok(())
}

/// Set (or create) the bot's user counter to `new_score`.
async fn update_bot_counter(
    valence: &Valence,
    entry: &LeaderboardEntry,
    new_score: i64,
) -> Result<()> {
    let bot_counter = UserCounter::get(&entry.counter_id, valence).await?;

    if let Some(counter) = bot_counter {
        counter
            .get_mutable(valence)
            .set_value(new_score)
            .map_err(|e| anyhow::anyhow!("Failed to set bot score: {e}"))?
            .commit()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update bot counter: {e}"))?;
    } else {
        let new_counter = UserCounter::new(entry.user_record.clone(), new_score)
            .map_err(|e| anyhow::anyhow!("Failed to create bot counter: {e}"))?;
        UserCounter::upsert(&entry.counter_id, new_counter, valence)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upsert bot counter: {e}"))?;
    }
    Ok(())
}

/// Add `score_delta` to the global singleton counter, returning its new value.
async fn bump_global_counter(valence: &Valence, score_delta: i64) -> Result<usize> {
    let global_counter = Counter::get("singleton", valence).await?;
    let global_val = if let Some(counter) = global_counter {
        let updated_val = *counter.value() + score_delta;
        *counter
            .get_mutable(valence)
            .set_value(updated_val)
            .map_err(|e| anyhow::anyhow!("Failed to set global counter: {e}"))?
            .commit()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update global counter: {e}"))?
            .value()
    } else {
        let new_counter = Counter::new(score_delta.max(0))
            .map_err(|e| anyhow::anyhow!("Failed to create global counter: {e}"))?;
        *Counter::upsert("singleton", new_counter, valence)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upsert global counter: {e}"))?
            .value()
    };
    Ok(usize::try_from(global_val).unwrap_or(0))
}
