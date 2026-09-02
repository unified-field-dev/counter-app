//! Chronon / Boson script-core contracts via `_with_valence` helpers.
//!
//! Seeds the bot roster with [`ensure_bot_users_seed`], then exercises bumper,
//! daily reset, and leaderboard-change notification cores without Chronon/Boson
//! context.
//!
//! Run: `cargo test -p counter-app-worker --test scripts_contract`

#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use chrono::Utc;
use common::{mem_router, system_valence, valence_for};
use counter_app_worker::generated::{Counter, UserCounter};
use counter_app_worker::scripts::bot_roster::{bot_reset_score, BOT_ROSTER};
use counter_app_worker::{
    bot_score_bumper_with_valence, check_leaderboard_changes_with_valence,
    daily_highscores_reset_with_valence, ensure_bot_users_seed, get_user,
};
use lepton_identity::generated::{Account, AccountEmail, User, UserStatus, UserUserType};
use uf_notifications_core::Notification;
use valence::actor::Actor;
use valence::{Model, RecordId, RecordPredicate, StringPredicate, Valence};

async fn seed_real_user_with_email(
    valence: &Valence,
    user_id: &str,
    email: &str,
    display_name: &str,
    score: i64,
) {
    let now = Utc::now();
    let user = User::new(
        Some(UserUserType::Person),
        Some("test-hash".into()),
        Some(UserStatus::Active),
        None,
        None,
        None,
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    let user_created = User::upsert(user_id, user, valence)
        .await
        .expect("upsert user");
    let user_thing = RecordId::new("user", user_id);

    let account = Account::new(
        display_name.to_string(),
        user_thing.clone(),
        None,
        None,
        None,
        None,
        now,
        now,
    )
    .expect("build account");
    let account_created = Account::create(account, valence)
        .await
        .expect("create account");
    let account_thing = account_created.id().cloned().expect("account id");

    let email_row = AccountEmail::new(account_thing, email.to_string(), Some(now), now, now)
        .expect("build email");
    let email_created = AccountEmail::create(email_row, valence)
        .await
        .expect("create email");
    let email_thing = email_created.id().cloned().expect("email id");

    account_created
        .get_mutable(valence)
        .set_primary_email(email_thing.clone())
        .expect("set account email")
        .set_updated_at(now)
        .expect("account updated_at")
        .commit()
        .await
        .expect("commit account");

    user_created
        .get_mutable(valence)
        .set_primary_email(email_thing)
        .expect("set user email")
        .set_updated_at(now)
        .expect("user updated_at")
        .commit()
        .await
        .expect("commit user");

    let counter = UserCounter::new(user_thing, score).expect("new counter");
    UserCounter::upsert(user_id, counter, valence)
        .await
        .expect("upsert counter");
}

async fn leaderboard_notifications_for(valence: &Valence, user_id: &str) -> Vec<Notification> {
    Notification::query(valence)
        .where_user(RecordPredicate::Equals(RecordId::new("user", user_id)))
        .where_kind(StringPredicate::Equals("leaderboard".into()))
        .await
        .expect("notification query")
}

async fn upsert_score(valence: &Valence, user_id: &str, score: i64) {
    let row = UserCounter::new(RecordId::new("user", user_id), score).expect("new");
    UserCounter::upsert(user_id, row, valence)
        .await
        .expect("upsert");
}

#[tokio::test]
async fn ensure_bot_users_seed_happy() {
    let v = system_valence().await;
    ensure_bot_users_seed(&v).await.expect("seed bots");

    for bot in BOT_ROSTER {
        let email = AccountEmail::query(&v)
            .where_address(StringPredicate::Equals(bot.email.to_string()))
            .limit(1)
            .first()
            .await
            .expect("email query");
        assert!(
            email.is_some(),
            "bot email {} must exist after seed",
            bot.email
        );
    }

    let counters = UserCounter::query(&v).await.expect("counters");
    assert_eq!(
        counters.len(),
        BOT_ROSTER.len(),
        "each bot should have a UserCounter"
    );
}

#[tokio::test]
async fn bot_score_bumper_with_valence_happy() {
    let router = mem_router().await;
    let v = valence_for(
        router,
        Actor::System {
            operation: "scripts-contract".into(),
        },
    );

    ensure_bot_users_seed(&v).await.expect("seed bots");

    // Real user above top1 reset_score (80) so every top-tier bot is out of order.
    let real_score = 90_i64;
    seed_real_user_with_email(&v, "real_alice", "alice@example.com", "Alice", real_score).await;

    bot_score_bumper_with_valence(&v)
        .await
        .expect("bumper should succeed");

    let counters = UserCounter::query(&v)
        .order_by_value(valence::SortDirection::Desc)
        .await
        .expect("query");

    let mut max_top_bot = 0_i64;
    for c in &counters {
        let uid = valence::extract_id_from_record(c.user()).unwrap_or_default();
        if uid == "real_alice" {
            continue;
        }
        // Bot counters use the generated user id (not email). Resolve via email walk.
        let record_id = valence::extract_id_from_record(c.user()).unwrap_or_default();
        if let Ok(Some(user)) = User::get(&record_id, &v).await {
            if let Some(email_id) = user.primary_email() {
                if let Ok(bare) = valence::extract_id_from_record(email_id) {
                    if let Ok(Some(email_row)) = AccountEmail::get(&bare, &v).await {
                        let addr = email_row.address();
                        if addr.starts_with("top") && addr.ends_with("@example.com") {
                            max_top_bot = max_top_bot.max(*c.value());
                        }
                    }
                }
            }
        }
    }

    assert!(
        max_top_bot > real_score,
        "top-tier bot should be bumped above real user ({max_top_bot} > {real_score})"
    );
}

#[tokio::test]
async fn bot_score_bumper_empty_sad() {
    let v = system_valence().await;
    bot_score_bumper_with_valence(&v)
        .await
        .expect("empty board must return Ok");
}

#[tokio::test]
async fn daily_highscores_reset_with_valence_happy() {
    let v = system_valence().await;

    ensure_bot_users_seed(&v).await.expect("seed bots");
    seed_real_user_with_email(&v, "real_bob", "bob@example.com", "Bob", 123).await;

    daily_highscores_reset_with_valence(&v)
        .await
        .expect("reset");

    let bob = get_user("real_bob", &v).await.expect("bob get");
    assert_eq!(bob.user_count, 0, "real user must reset to 0");

    for bot in BOT_ROSTER {
        let email = AccountEmail::query(&v)
            .where_address(StringPredicate::Equals(bot.email.to_string()))
            .limit(1)
            .first()
            .await
            .expect("email")
            .expect("bot email present");
        let email_id = email.id().cloned().expect("email id");
        let user = User::query(&v)
            .where_primary_email(RecordPredicate::Equals(email_id))
            .limit(1)
            .first()
            .await
            .expect("user query")
            .expect("bot user");
        let uid = valence::extract_id_from_record(user.id().expect("user id")).expect("bare id");
        let counter = UserCounter::get(&uid, &v)
            .await
            .expect("get")
            .expect("bot counter");
        let expected = bot_reset_score(bot.email).expect("reset score");
        assert_eq!(
            *counter.value(),
            expected,
            "bot {} should reset to {expected}",
            bot.email
        );
    }

    let max_bot_reset = BOT_ROSTER.iter().map(|b| b.reset_score).max().unwrap_or(0);
    let global = Counter::get("singleton", &v)
        .await
        .expect("global get")
        .expect("global exists");
    assert_eq!(
        *global.value(),
        max_bot_reset,
        "global singleton should match first-place bot reset"
    );
}

#[tokio::test]
async fn check_leaderboard_changes_with_valence_notify_happy() {
    let v = system_valence().await;

    // Fill top 9, then place challenger into top 10 at rank 1.
    for i in 1..=9 {
        upsert_score(&v, &format!("board_{i}"), i64::from(i) * 10).await;
    }
    upsert_score(&v, "challenger", 100).await;

    let before = leaderboard_notifications_for(&v, "challenger").await;
    assert!(before.is_empty(), "no prior notifications");

    check_leaderboard_changes_with_valence(&v, "challenger", 0, 100)
        .await
        .expect("notify path");

    let after = leaderboard_notifications_for(&v, "challenger").await;
    assert!(
        !after.is_empty(),
        "entering top 10 must create a leaderboard notification"
    );
    let msg = after[0].message();
    assert!(
        msg.contains("entered") || msg.contains("top 10") || msg.contains('#'),
        "expected enter/rank message, got {msg}"
    );
    assert_eq!(after[0].kind(), "leaderboard");
}

#[tokio::test]
async fn check_leaderboard_changes_skip_noop_sad() {
    let v = system_valence().await;

    for i in 1..=5 {
        upsert_score(&v, &format!("noop_{i}"), i64::from(i) * 10).await;
    }
    upsert_score(&v, "stable", 60).await;

    // old == new with matching persisted score → before/after ranks identical.
    // LeaderboardNotifier's new_value==0 enqueue skip is side-effect-layer only;
    // calling this helper with new_value==0 after a zero write can still notify.
    check_leaderboard_changes_with_valence(&v, "stable", 60, 60)
        .await
        .expect("noop ok");

    let notes = leaderboard_notifications_for(&v, "stable").await;
    assert!(
        notes.is_empty(),
        "old_value==new_value must not create notifications, got {}",
        notes.len()
    );
}
