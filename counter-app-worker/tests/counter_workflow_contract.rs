//! Named happy/sad contracts for product-local counter service.
//!
//! Covers the same domain surface as `counter-app` `#[server]` fns
//! (`counter_get` / `counter_increment` / `counter_set` /
//! `user_counter_get` / `user_counter_increment`), which are thin Higgs
//! wrappers over [`counter_app_worker::service`].
//!
//! Run: `cargo test -p counter-app-worker --test counter_workflow_contract`

#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{mem_router, system_valence, user_valence, valence_for};
use counter_app_worker::service::{
    get_global, get_user, increment_global, increment_user, set_global, validate_anon_increment,
    validate_increment_amount, MAX_ANON_INCREMENT_AMOUNT, MAX_INCREMENT_AMOUNT,
};
use valence::actor::Actor;

#[test]
fn validate_increment_amount_accepts_in_range_happy_path() {
    validate_increment_amount(1).expect("1 ok");
    validate_increment_amount(MAX_INCREMENT_AMOUNT).expect("max ok");
}

#[test]
fn validate_increment_amount_zero_rejected_sad() {
    let err = validate_increment_amount(0).expect_err("zero");
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got {msg}");
    assert!(msg.contains("greater than 0"), "got {msg}");
}

#[test]
fn validate_increment_amount_over_max_rejected_sad() {
    let err = validate_increment_amount(MAX_INCREMENT_AMOUNT + 1).expect_err("over max");
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got {msg}");
    assert!(msg.contains(&MAX_INCREMENT_AMOUNT.to_string()), "got {msg}");
}

#[test]
fn validate_anon_increment_accepts_in_range_happy_path() {
    validate_anon_increment(1).expect("1 ok");
    validate_anon_increment(MAX_ANON_INCREMENT_AMOUNT).expect("anon max ok");
}

#[test]
fn validate_anon_increment_over_anon_max_rejected_sad() {
    let err = validate_anon_increment(MAX_ANON_INCREMENT_AMOUNT + 1).expect_err("over anon max");
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got {msg}");
    assert!(
        msg.contains(&MAX_ANON_INCREMENT_AMOUNT.to_string()),
        "got {msg}"
    );
}

#[test]
fn rate_limited_variant_display_sad() {
    use counter_app_worker::CounterServiceError;
    let msg = CounterServiceError::RateLimited.to_string();
    assert!(msg.contains("rate limit"), "got {msg}");
    assert!(!msg.starts_with("validation failed"), "got {msg}");
}

/// Process-global env + shared `"anon-increment"` bucket — serialize this suite.
static ANON_RATE_LIMIT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn validate_anon_increment_rate_limit_exhaustion_sad() {
    use counter_app_worker::CounterServiceError;

    let _guard = ANON_RATE_LIMIT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let prev = std::env::var("COUNTER_ANON_INCREMENTS_PER_MIN").ok();
    // SAFETY: test-only; lock above serializes env + bucket mutation for this process.
    unsafe {
        std::env::set_var("COUNTER_ANON_INCREMENTS_PER_MIN", "3");
    }
    counter_app_worker::reset_for_tests();

    for i in 0..3 {
        validate_anon_increment(1).unwrap_or_else(|e| panic!("request {i} should pass: {e}"));
    }

    let err = validate_anon_increment(1).expect_err("4th request must be rate limited");
    assert!(
        matches!(err, CounterServiceError::RateLimited),
        "expected RateLimited, got {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("rate limit"), "got {msg}");
    assert!(!msg.starts_with("validation failed"), "got {msg}");

    // SAFETY: restore prior env for sibling tests in this process.
    unsafe {
        match prev {
            Some(v) => std::env::set_var("COUNTER_ANON_INCREMENTS_PER_MIN", v),
            None => std::env::remove_var("COUNTER_ANON_INCREMENTS_PER_MIN"),
        }
    }
    counter_app_worker::reset_for_tests();
}

#[tokio::test]
async fn get_global_missing_singleton_returns_zero_happy_path() {
    let v = system_valence().await;
    let got = get_global(&v).await.expect("get");
    assert_eq!(got.value, 0);
}

#[tokio::test]
async fn increment_global_create_then_update_happy_path() {
    let v = system_valence().await;
    let first = increment_global(3, &v).await.expect("create");
    assert_eq!(first.value, 3);
    let second = increment_global(2, &v).await.expect("update");
    assert_eq!(second.value, 5);
    let got = get_global(&v).await.expect("get");
    assert_eq!(got.value, 5);
}

#[tokio::test]
async fn set_global_overwrites_value_happy_path() {
    let v = system_valence().await;
    increment_global(10, &v).await.expect("seed");
    let set = set_global(42, &v).await.expect("set");
    assert_eq!(set.value, 42);
    assert_eq!(get_global(&v).await.expect("get").value, 42);
}

#[tokio::test]
async fn increment_global_zero_amount_rejected_sad() {
    let v = system_valence().await;
    let err = increment_global(0, &v).await.expect_err("zero");
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got {msg}");
    assert!(msg.contains("greater than 0"), "got {msg}");
}

#[tokio::test]
async fn increment_global_over_max_rejected_sad() {
    let v = system_valence().await;
    let err = increment_global(MAX_INCREMENT_AMOUNT + 1, &v)
        .await
        .expect_err("over max");
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got {msg}");
    assert!(msg.contains(&MAX_INCREMENT_AMOUNT.to_string()), "got {msg}");
}

#[tokio::test]
async fn user_counter_get_defaults_to_zero_happy_path() {
    let v = user_valence("alice").await;
    let got = get_user("alice", &v).await.expect("get");
    assert_eq!(got.user_count, 0);
    assert_eq!(got.global_count, 0);
}

#[tokio::test]
async fn user_counter_increment_updates_personal_and_global_happy_path() {
    let router = mem_router().await;
    let v = valence_for(
        router,
        Actor::User {
            user_id: "alice".into(),
        },
    );
    let first = increment_user("alice", 4, &v).await.expect("first");
    assert_eq!(first.user_count, 4);
    assert_eq!(first.global_count, 4);

    let second = increment_user("alice", 1, &v).await.expect("second");
    assert_eq!(second.user_count, 5);
    assert_eq!(second.global_count, 5);

    let got = get_user("alice", &v).await.expect("get");
    assert_eq!(got.user_count, 5);
    assert_eq!(got.global_count, 5);
}

#[tokio::test]
async fn user_counter_increment_zero_rejected_sad() {
    let v = user_valence("alice").await;
    let err = increment_user("alice", 0, &v).await.expect_err("zero");
    let msg = err.to_string();
    assert!(msg.contains("validation failed"), "got {msg}");
}

#[tokio::test]
async fn user_counter_non_owner_cannot_increment_other_user_sad() {
    let router = mem_router().await;
    let v_alice = valence_for(
        router.clone(),
        Actor::User {
            user_id: "alice".into(),
        },
    );
    increment_user("alice", 3, &v_alice)
        .await
        .expect("alice seed");

    let v_bob = valence_for(
        router,
        Actor::User {
            user_id: "bob".into(),
        },
    );
    let err = increment_user("alice", 1, &v_bob)
        .await
        .expect_err("bob must not mutate alice");
    let msg = err.to_string();
    assert!(
        msg.contains("not authorized"),
        "non-owner update must be Forbidden, got {msg}"
    );
}

#[tokio::test]
async fn user_counter_create_path_foreign_user_forbidden_sad() {
    // Create-path IDOR: no prior row for alice; bob must still be denied.
    let router = mem_router().await;
    let v_bob = valence_for(
        router,
        Actor::User {
            user_id: "bob".into(),
        },
    );
    let err = increment_user("alice", 1, &v_bob)
        .await
        .expect_err("bob must not create alice's counter");
    let msg = err.to_string();
    assert!(
        msg.contains("not authorized"),
        "create-path foreign user must be Forbidden, got {msg}"
    );
}

#[tokio::test]
async fn user_counter_valence_create_policy_denies_foreign_user_sad() {
    use counter_app_worker::generated::UserCounter;
    use valence::{Model, RecordId};

    let router = mem_router().await;
    let v_bob = valence_for(
        router,
        Actor::User {
            user_id: "bob".into(),
        },
    );
    let forged = UserCounter::new(RecordId::new("user", "alice"), 1).expect("new");
    let err = UserCounter::upsert("alice", forged, &v_bob)
        .await
        .expect_err("Valence create must deny foreign user field");
    assert!(
        !err.to_string().is_empty(),
        "privacy deny should surface a concrete error"
    );
}

/// Multi-step workflow covering the Layer 2 e2e waiver: get → increment → get →
/// set → get, plus authenticated personal/global increment.
#[tokio::test]
async fn counter_workflow_get_increment_set_user_happy_path() {
    let router = mem_router().await;
    let v_sys = valence_for(
        router.clone(),
        Actor::System {
            operation: "counter-workflow".into(),
        },
    );

    assert_eq!(get_global(&v_sys).await.expect("get empty").value, 0);
    assert_eq!(
        increment_global(7, &v_sys).await.expect("anon incr").value,
        7
    );
    assert_eq!(get_global(&v_sys).await.expect("get after incr").value, 7);
    assert_eq!(set_global(100, &v_sys).await.expect("admin set").value, 100);
    assert_eq!(get_global(&v_sys).await.expect("get after set").value, 100);

    let v_alice = valence_for(
        router,
        Actor::User {
            user_id: "alice".into(),
        },
    );
    let user = increment_user("alice", 5, &v_alice)
        .await
        .expect("user incr");
    assert_eq!(user.user_count, 5);
    assert_eq!(user.global_count, 105);

    let round_trip = get_user("alice", &v_alice).await.expect("user get");
    assert_eq!(round_trip.user_count, 5);
    assert_eq!(round_trip.global_count, 105);
}
