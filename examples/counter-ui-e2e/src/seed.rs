//! Harness-only seed endpoint for Playwright.
//!
//! `POST /api/test/seed-data` body:
//! `{ "auth": "anonymous"|"owner"|"member"|"unverified", "seed_scores"?: bool, "reset_rate_limit"?: bool }`
//!
//! Naming matches tag-ui-e2e (`/api/test/seed-data`) rather than product-only variants.

use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::e2e_valence::{
    read_global_value, refresh_owner_counter_admin_membership, reset_global_counter,
    reset_user_counter, seed_leaderboard_scores,
};
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// `anonymous` | `owner` | `member` | `unverified`
    #[serde(default = "default_auth")]
    pub auth: String,
    /// When true, upsert UserCounter rows for the leaderboard scenario.
    #[serde(default)]
    pub seed_scores: bool,
    /// When true (default), clear in-process anon increment buckets.
    #[serde(default = "default_reset_rate_limit")]
    pub reset_rate_limit: bool,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

fn default_reset_rate_limit() -> bool {
    true
}

pub async fn seed_data(
    session: tower_sessions::Session,
    Json(body): Json<SeedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = E2eAuthKind::parse(&body.auth);
    write_e2e_auth_kind(&session, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if body.reset_rate_limit {
        counter_app_worker::reset_for_tests();
    }

    reset_global_counter().await.map_err(|e| {
        log::error!("reset global counter failed: {e:#}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(user_id) = match kind {
        E2eAuthKind::Owner => Some("owner"),
        E2eAuthKind::Member => Some("alice"),
        _ => None,
    } {
        reset_user_counter(user_id).await.map_err(|e| {
            log::error!("reset user counter failed for {user_id}: {e:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    if kind == E2eAuthKind::Owner {
        refresh_owner_counter_admin_membership()
            .await
            .map_err(|e| {
                log::error!("refresh owner CounterAdmin group membership failed: {e:#}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    let mut scores_json = serde_json::Value::Null;
    if body.seed_scores {
        let scores = seed_leaderboard_scores().await.map_err(|e| {
            log::error!("seed_scores failed: {e:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        scores_json = serde_json::json!(scores
            .into_iter()
            .map(|(id, score)| serde_json::json!({ "user_id": id, "score": score }))
            .collect::<Vec<_>>());
    }

    let global_value = read_global_value().await.ok();

    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "fixtures": {
            "global_value": global_value,
            "scores": scores_json,
        }
    })))
}
