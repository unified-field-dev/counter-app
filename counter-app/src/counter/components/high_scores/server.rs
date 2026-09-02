//! Paginated (and legacy) high-scores Higgs server functions.
//!
//! [`get_high_scores_page`] teaches the Valence list path behind Orbital
//! infinite scroll: resolve Higgs context, query `UserCounter`, resolve display
//! names under session Valence, and return an [`orbital_paging::Page`] with
//! `next_request_offset` so skipped rows do not stall pagination.
//!
//! Leaderboard rows use session Valence (`UserCounter` is `PUBLIC_READ`).
//! Profile/display names still load under the same viewer session; missing or
//! denied profiles fall back to a redacted label.

use leptos::prelude::*;
use orbital_paging::Page;

use super::types::{clamp_high_scores_page, HighScoreEntry, HighScoresResponse};

/// Paginated high-scores endpoint over Valence `UserCounter`.
///
/// Call from [`super::table::HighScoresTable`] (or any Orbital infinite-scroll
/// `fetch` closure). Flow under `ssr`:
/// 1. [`clamp_high_scores_page`] on client `offset` / `limit`.
/// 2. `Higgs::from_request` → system Valence for scores, session Valence for profiles.
/// 3. Query `UserCounter` ordered by value desc; over-fetch by one row for `has_more`.
/// 4. Build [`HighScoreEntry`] rows; set `Page::next_request_offset` from DB rows
///    fetched (including skipped), not filtered length.
///
/// # Errors
///
/// Returns [`ServerFnError`] when Higgs/Valence setup fails or the query errors.
/// Failures pass through [`into_server_error`](crate::into_server_error) so Spectra
/// `counter_server_errors` records the kind.
#[server(GetHighScoresPage)]
pub async fn get_high_scores_page(
    /// Zero-based index of the first `UserCounter` row to return.
    offset: u32,
    /// Maximum number of leaderboard rows to return.
    limit: u32,
) -> Result<Page<HighScoreEntry>, ServerFnError> {
    use crate::counter::counter_example::error::{
        count_to_usize, ctx_err, ctx_valence_err, into_server_error,
    };
    use crate::generated::UserCounter;
    use valence::extract_id_from_record;

    let (offset, limit) = clamp_high_scores_page(offset, limit);

    let ctx = higgs::Higgs::from_request().await?;
    // Leaderboard rows are PUBLIC_READ on `UserCounter`; session Valence is enough
    // for anonymous and authenticated viewers (no System elevation).
    let v_scores = ctx
        .valence()
        .map_err(|e| into_server_error(ctx_err("get_high_scores_page valence", e)))?;
    let v_viewer = v_scores.clone();

    // Over-fetch by 1 at the **UserCounter row** layer. Rows skipped below (missing
    // user, bad id) must still advance `offset` via [`Page::next_request_offset`],
    // otherwise the client would re-fetch the same DB slice and pagination breaks.
    let fetch_n = (limit.saturating_add(1)).max(1);
    // Total `UserCounter` rows (not filtered rows). Needed on **every** page so `has_more`
    // stays correct when the last DB window is full (`limit + 1` rows) but there is no next
    // slice — follow-up pages used to set `total_count: None` and used `db_rows_fetched > limit`,
    // which keeps `has_more` true after the final full fetch and hides the end-of-list UI.
    let total_rows = u64::try_from(
        UserCounter::query(&v_scores)
            .await
            .map_err(|e| into_server_error(ctx_valence_err("get_high_scores_page count", e)))?
            .len(),
    )
    .unwrap_or(u64::MAX);
    let total_count = Some(total_rows);

    let counters = UserCounter::query(&v_scores)
        .order_by_value(valence::SortDirection::Desc)
        .limit(fetch_n)
        .offset(offset)
        .await
        .map_err(|e| into_server_error(ctx_valence_err("get_high_scores_page", e)))?;

    let mut scores = Vec::with_capacity(counters.len());
    for counter in &counters {
        let Some(id_ref) = counter.id() else {
            continue;
        };
        let Ok(row_key) = extract_id_from_record(id_ref) else {
            continue;
        };
        let count = count_to_usize(*counter.value());
        let display_name = if let Ok(user) = counter.get_user(&v_scores).await {
            user.get_profile(&v_viewer)
                .await
                .unwrap_or_default()
                .into_iter()
                .next()
                .and_then(|p| {
                    let display = p.display_name().trim();
                    (!display.is_empty()).then(|| display.to_string())
                })
                .unwrap_or_else(|| redacted_player_label(&row_key))
        } else {
            redacted_player_label(&row_key)
        };
        scores.push(HighScoreEntry {
            row_key,
            display_name,
            count,
        });
    }

    let db_rows_fetched = u32::try_from(counters.len()).unwrap_or(u32::MAX);
    let next_request_offset = offset.saturating_add(db_rows_fetched);
    let limit_usize = limit as usize;
    let mut items = scores;
    if items.len() > limit_usize {
        items.truncate(limit_usize);
    }

    let has_more = u64::from(next_request_offset) < total_rows;

    Ok(Page {
        items,
        has_more,
        total_count,
        next_request_offset: Some(next_request_offset),
    })
}

/// Redact a user id into a stable, non-PII leaderboard label.
#[cfg(feature = "ssr")]
fn redacted_player_label(row_key: &str) -> String {
    if row_key.len() <= 4 {
        format!("Player {row_key}")
    } else {
        format!("Player …{}", &row_key[row_key.len() - 4..])
    }
}

/// Legacy non-paginated endpoint — first page of [`get_high_scores_page`].
///
/// Prefer [`get_high_scores_page`] for new UI. Kept so older callers still get
/// a flat [`HighScoresResponse`].
///
/// # Errors
///
/// Propagates errors from [`get_high_scores_page`].
#[server(GetHighScores)]
pub async fn get_high_scores() -> Result<HighScoresResponse, ServerFnError> {
    let page = get_high_scores_page(0, 10).await?;
    Ok(HighScoresResponse { scores: page.items })
}
