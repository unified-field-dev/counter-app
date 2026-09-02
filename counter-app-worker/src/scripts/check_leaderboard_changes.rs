//! Boson task that diffs leaderboard ranks after a `UserCounter` mutation.
//!
//! Teaching path: Valence [`crate::side_effects::LeaderboardNotifier`] calls
//! `CheckLeaderboardChanges::send_with(actor_json, params)` → Boson runs this
//! handler with `ExecutionContext` → `boson_valence_identity::valence_from_context`
//! → `UserCounter::query` + notification sends.

use anyhow::Result;
use boson_core::ExecutionContext;
use boson_valence_identity::valence_from_context;
use valence::{extract_id_from_record, RecordId, Valence};

use crate::generated::UserCounter;

/// Boson background task: compute leaderboard position changes and notify users.
///
/// Enqueued from [`crate::side_effects::LeaderboardNotifier`] with the mutating
/// user's id and old/new scores. Inside the task:
///
/// 1. `valence_from_context` builds a Valence handle from [`ExecutionContext`].
/// 2. `UserCounter::query` loads the current top 11 (one extra to detect
///    enter/leave of the top 10).
/// 3. Reconstructs the "before" board by substituting `old_value` for the
///    mutating user and re-sorting.
/// 4. Diffs before/after top-10 maps and sends a notification per changed rank.
///
/// Deletes and daily resets to zero are skipped in the notifier so this task
/// is not flooded.
#[boson_macros::task(
    // Pool/retry knobs are coordinator policy; name must match send_with registration.
    name = "check_leaderboard_changes",
    priority = 40,
    pool = "global",
    max_in_flight = 200,
    max_enqueue_per_second = 100,
    max_attempts = 3,
    base_delay_ms = 1000,
    backoff_multiplier = 2.0,
    max_delay_ms = 30_000
)]
pub async fn check_leaderboard_changes(
    ctx: Box<dyn ExecutionContext>,
    user_id: String,
    old_value: i64,
    new_value: i64,
) -> Result<()> {
    // Boson → Valence: actor JSON from enqueue is reconstituted here.
    let valence = valence_from_context(ctx.as_ref())?;
    check_leaderboard_changes_with_valence(&valence, &user_id, old_value, new_value).await
}

/// Valence-taking core of the `check_leaderboard_changes` Boson task (contract tests call this directly).
pub async fn check_leaderboard_changes_with_valence(
    valence: &Valence,
    user_id: &str,
    old_value: i64,
    new_value: i64,
) -> Result<()> {
    log::info!("[counter-app-worker] check_leaderboard_changes: old={old_value}, new={new_value}");

    // 1. Query the current top 11 after the mutation has been persisted.
    let current_top = UserCounter::query(valence)
        .order_by_value(valence::SortDirection::Desc)
        .limit(11)
        .await?;

    // Build the "after" list as (user_id_string, score) tuples.
    let after_list: Vec<(String, i64)> = current_top
        .iter()
        .map(|c| {
            let uid = extract_user_id(c.user());
            (uid, *c.value())
        })
        .collect();

    // 2. Reconstruct the "before" list by replacing the mutating user's score
    //    with old_value. If old_value == 0 the user wasn't meaningfully on the
    //    board before (first click or post-reset), so we exclude them entirely
    //    so they receive a "You entered the top 10 at #N!" notification.
    let mut before_list: Vec<(String, i64)> = after_list
        .iter()
        .filter_map(|(uid, score)| {
            if *uid == user_id {
                if old_value == 0 {
                    None
                } else {
                    Some((uid.clone(), old_value))
                }
            } else {
                Some((uid.clone(), *score))
            }
        })
        .collect();

    // If the mutating user is not in the current top 11 (their new score is
    // too low), we still need them in the before list if their old score
    // would have placed them in the top 10. Skip when old_value == 0 since
    // they weren't on the board.
    if old_value > 0 {
        let mutating_in_list = before_list.iter().any(|(uid, _)| *uid == user_id);
        if !mutating_in_list {
            before_list.push((user_id.to_string(), old_value));
        }
    }

    // Sort both lists: score descending, then user_id ascending for deterministic ties.
    let sort_fn = |a: &(String, i64), b: &(String, i64)| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0));

    before_list.sort_by(sort_fn);
    let mut after_sorted = after_list.clone();
    after_sorted.sort_by(sort_fn);

    // Trim to top 10.
    before_list.truncate(10);
    after_sorted.truncate(10);

    // 3. Build rank maps: user_id -> 1-based rank.
    let before_ranks: std::collections::HashMap<&str, usize> = before_list
        .iter()
        .enumerate()
        .map(|(i, (uid, _))| (uid.as_str(), i + 1))
        .collect();

    let after_ranks: std::collections::HashMap<&str, usize> = after_sorted
        .iter()
        .enumerate()
        .map(|(i, (uid, _))| (uid.as_str(), i + 1))
        .collect();

    // Collect all user IDs that appear in either list.
    let mut all_users: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (uid, _) in &before_list {
        all_users.insert(uid.as_str());
    }
    for (uid, _) in &after_sorted {
        all_users.insert(uid.as_str());
    }

    // 4. Diff and send notifications for each affected user.
    for uid in all_users {
        let old_rank = before_ranks.get(uid).copied();
        let new_rank = after_ranks.get(uid).copied();

        // Skip if rank didn't change.
        if old_rank == new_rank {
            continue;
        }

        let message = match (old_rank, new_rank) {
            (None, Some(nr)) => format!("You entered the top 10 at #{nr}!"),
            (Some(or), None) => format!("You dropped out of the top 10 (was #{or})."),
            (Some(or), Some(nr)) if nr < or => {
                format!("You moved up from #{or} to #{nr} on the leaderboard!")
            }
            (Some(or), Some(nr)) => {
                format!("You moved down from #{or} to #{nr} on the leaderboard.")
            }
            _ => continue,
        };

        notify_rank_change(valence, uid, message).await;
    }

    // Also log the mutating user in case they weren't caught above (e.g. outside top 10).
    log::info!("[counter-app-worker] check_leaderboard_changes: done");
    Ok(())
}

fn extract_user_id(rid: &RecordId) -> String {
    extract_id_from_record(rid).unwrap_or_default()
}

/// Send a leaderboard notification, logging (not failing) on delivery errors.
async fn notify_rank_change(valence: &valence::Valence, uid: &str, message: String) {
    log::info!("[counter-app-worker] check_leaderboard_changes: sending leaderboard notification");

    let user_record = RecordId::new("user", uid);

    if let Err(e) = uf_notifications_core::send_notification(
        uf_notifications_core::SendNotification {
            user_id: user_record,
            kind: "leaderboard".into(),
            title: "Leaderboard Update".into(),
            message,
            url: Some(crate::paths::HIGH_SCORES.into()),
            data_json: None,
        },
        valence,
    )
    .await
    {
        log::info!(
            "[counter-app-worker] check_leaderboard_changes: notification delivery failed: {e}"
        );
    }
}
