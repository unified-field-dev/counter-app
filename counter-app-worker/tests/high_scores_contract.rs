//! Valence-layer high-score ordering contracts (worker-only).
//!
//! `get_high_scores_page` lives in `counter-app` and needs Higgs request context
//! plus display-name resolution under session Valence. These tests cover the
//! `UserCounter` query the page wraps: ordered scores, empty board, and
//! offset/limit pagination. Display-name resolution stays a UI/server concern.
//!
//! Run: `cargo test -p counter-app-worker --test high_scores_contract`

#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::system_valence;
use counter_app_worker::generated::UserCounter;
use valence::{Model, RecordId};

async fn upsert_score(valence: &valence::Valence, user_id: &str, score: i64) {
    let row = UserCounter::new(RecordId::new("user", user_id), score).expect("new");
    UserCounter::upsert(user_id, row, valence)
        .await
        .expect("upsert");
}

#[tokio::test]
async fn high_scores_ordered_desc_happy() {
    let v = system_valence().await;
    upsert_score(&v, "hs_low", 10).await;
    upsert_score(&v, "hs_high", 50).await;
    upsert_score(&v, "hs_mid", 30).await;

    let rows = UserCounter::query(&v)
        .order_by_value(valence::SortDirection::Desc)
        .limit(10)
        .await
        .expect("query");

    let values: Vec<i64> = rows.iter().map(|c| *c.value()).collect();
    assert_eq!(
        values,
        vec![50, 30, 10],
        "expected desc order, got {values:?}"
    );
}

#[tokio::test]
async fn high_scores_empty_board_sad() {
    let v = system_valence().await;
    let rows = UserCounter::query(&v)
        .order_by_value(valence::SortDirection::Desc)
        .limit(10)
        .await
        .expect("query");
    assert!(rows.is_empty(), "empty board must return empty vec");
}

#[tokio::test]
async fn high_scores_offset_limit_middle_page_happy() {
    let v = system_valence().await;
    upsert_score(&v, "hs_low", 10).await;
    upsert_score(&v, "hs_high", 50).await;
    upsert_score(&v, "hs_mid", 30).await;

    let page = UserCounter::query(&v)
        .order_by_value(valence::SortDirection::Desc)
        .limit(1)
        .offset(1)
        .await
        .expect("query");

    assert_eq!(page.len(), 1, "limit=1 should return one row");
    assert_eq!(
        *page[0].value(),
        30,
        "offset=1 on desc board should be middle score"
    );
}
