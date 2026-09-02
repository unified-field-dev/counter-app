//! Server functions backing the counter example.
//!
//! Domain logic lives in `counter_app_worker::service`. This module teaches the
//! Higgs request path: `Higgs::from_request` → `valence()` → worker service →
//! [`into_server_error`] / Spectra, then Photon `CounterUpdated::publish` on writes.
//! Live reads also use `#[photon_leptos::synced]` so the client can subscribe.

use leptos::prelude::*;

#[cfg(feature = "ssr")]
use super::error::{ctx_err, into_server_error, CounterServerError};
#[cfg(feature = "ssr")]
use crate::worker::service as counter_service;

pub use super::types::{CounterData, CounterResponse, UserCounterResponse, MAX_INCREMENT_AMOUNT};

/// Map worker service errors into the UI crate's [`CounterServerError`].
#[cfg(feature = "ssr")]
fn map_service_err(e: crate::worker::CounterServiceError) -> CounterServerError {
    match e {
        crate::worker::CounterServiceError::Validation(msg) => CounterServerError::Validation(msg),
        crate::worker::CounterServiceError::Forbidden => CounterServerError::Forbidden,
        crate::worker::CounterServiceError::RateLimited => CounterServerError::RateLimited,
        crate::worker::CounterServiceError::Valence(err) => CounterServerError::Valence(err),
    }
}

#[cfg(feature = "ssr")]
fn to_ui_counter(r: crate::worker::CounterResponse) -> CounterResponse {
    CounterResponse { value: r.value }
}

#[cfg(feature = "ssr")]
fn to_ui_user(r: crate::worker::UserCounterResponse) -> UserCounterResponse {
    UserCounterResponse {
        user_count: r.user_count,
        global_count: r.global_count,
    }
}

/// Strip a `table:` prefix from the session user id so Valence lookups use the bare record id.
///
/// Higgs `session_user_id()` may return a display form (`user:<id>`). Valence
/// `UserCounter` lookups expect the bare record id —
/// `valence::extract_id_from_record_display` peels the table prefix when present.
#[cfg(feature = "ssr")]
fn session_user_record_id(ctx: &higgs::Higgs) -> Result<String, ServerFnError> {
    let raw = ctx
        .session_user_id()
        .ok_or_else(|| into_server_error(CounterServerError::NotAuthenticated))?;
    Ok(valence::extract_id_from_record_display(raw).unwrap_or_else(|_| raw.to_string()))
}

/// Fire-and-forget publish of [`super::events::CounterUpdated`] on the current Tokio runtime.
///
/// Skips when `COUNTER_NOOP_PUBLISH` is truthy (contract tests). Uses
/// `Handle::try_current().spawn` so the server fn can return without awaiting Photon IO.
#[cfg(feature = "ssr")]
fn spawn_counter_updated(new_value: usize) {
    // Contract tests set COUNTER_NOOP_PUBLISH to skip Photon without a runtime.
    if std::env::var("COUNTER_NOOP_PUBLISH").ok().is_some_and(|v| {
        let v = v.trim().to_ascii_lowercase();
        matches!(v.as_str(), "1" | "true" | "yes" | "on")
    }) {
        return;
    }
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // Fire-and-forget: `#[photon::topic]` type gets `.publish()` from Photon.
        handle.spawn(async move {
            if let Err(e) = (super::events::CounterUpdated { new_value })
                .publish()
                .await
            {
                log::warn!("[counter-app] Failed to publish CounterUpdated: {e}");
            }
        });
    }
}

/// Read the global counter, live-synced over `/ws/counter` via Photon-leptos.
///
/// Call from a Leptos resource on the live page (or any UI that should track the
/// public integer). Teaching walk of the platform calls:
///
/// 1. `#[photon_leptos::synced(topic = "counter.updated", ws = "/ws/counter",
///    strategy = "refetch", auth = "none")]` — generates `subscribe_counter_get` on
///    the client and ties refetch to the worker Photon topic. Demo `auth = "none"`
///    is intentional (payload is only the public count); do not copy to private data.
/// 2. `#[uf_product_macros::server]` — registers a Higgs-aware Leptos server function.
/// 3. `Higgs::from_request().await` — loads session + platform handles for this request.
/// 4. `ctx.valence()` — borrows the request-scoped Valence; failures go through
///    [`ctx_err`] → [`into_server_error`].
/// 5. Spectra `record_get_request` — low-cardinality `auth` label (`user` | `anon`).
/// 6. `worker::service::get_global` — Valence read of the singleton Counter row.
///
/// # Errors
///
/// Returns [`ServerFnError`] when Higgs/Valence setup fails or the worker service
/// returns a domain error (mapped and recorded to Spectra).
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app::{counter_get, CounterResponse};
///
/// let response: CounterResponse = counter_get().await?;
/// assert!(response.value > 0 || response.value == 0);
/// ```
#[photon_leptos::synced(
    // Photon-leptos: client gets `subscribe_counter_get`; server refetches this fn
    // when topic `counter.updated` fires on WebSocket `/ws/counter`.
    // `auth = "none"` is demo-only (payload is a public integer) — see SECURITY.md.
    topic = "counter.updated",
    ws = "/ws/counter",
    strategy = "refetch",
    auth = "none"
)]
// `uf_product_macros::server` = Leptos `#[server]` + Higgs request wiring.
#[uf_product_macros::server]
pub async fn counter_get() -> Result<CounterResponse, ServerFnError> {
    use super::logging::{log_request_step, record_get_request};

    // Spectra UC3: structured step row (optional forensics; safe labels only).
    log_request_step("counter_get", "starting", "", "");

    // Higgs: per-request session + platform handles (Valence, identity, …).
    let ctx = higgs::Higgs::from_request().await?;
    // Borrow the request-scoped Valence; map setup failures through Spectra.
    let v = ctx
        .valence()
        .map_err(|e| into_server_error(ctx_err("counter_get valence", e)))?;

    // Spectra UC1: low-cardinality traffic counter (`auth` = user | anon).
    record_get_request(if ctx.session_user_id().is_some() {
        "user"
    } else {
        "anon"
    });

    // Domain read lives in the worker (no Leptos) — Valence `Counter::get("singleton")`.
    let response = counter_service::get_global(&v)
        .await
        .map_err(|e| into_server_error(map_service_err(e)))?;
    let response = to_ui_counter(response);
    log_request_step(
        "counter_get",
        "returning value",
        "",
        &response.value.to_string(),
    );
    Ok(response)
}

/// Increment for the current session by `amount` (batched client flush).
///
/// Entry point used by the live Increment button. After Higgs context loads, it
/// branches: authenticated sessions call [`user_counter_increment`] (personal +
/// global); anonymous sessions call [`counter_increment`] (global only, with anon
/// rate limit). Optional rootcause helpers time Higgs and Spectra deltas when
/// enabled — useful when learning where request time goes, not required for product
/// copies.
///
/// # Errors
///
/// Propagates validation from `validate_increment_amount` and nested increment paths
/// as [`ServerFnError`].
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app::{increment_counter, CounterData};
///
/// let data = increment_counter(1).await?;
/// match data {
///     CounterData::Global(r) => assert!(r.value >= 1),
///     CounterData::User(r) => assert!(r.user_count >= 1),
/// }
/// ```
#[uf_product_macros::server]
#[server(IncrementCounter)]
pub async fn increment_counter(
    /// Number of clicks to apply in this flush (batched client-side).
    amount: usize,
) -> Result<CounterData, ServerFnError> {
    // Shared abuse guard (worker) before any Valence IO.
    counter_service::validate_increment_amount(amount)
        .map_err(|e| into_server_error(map_service_err(e)))?;

    #[cfg(feature = "ssr")]
    let spectra_before = super::rootcause::spectra_snapshot_before();
    #[cfg(feature = "ssr")]
    let total_start = super::rootcause::enabled().then(std::time::Instant::now);

    // Same Higgs entry as `counter_get`; rootcause timing is optional teaching aid.
    let ctx = super::rootcause::timed("increment_counter.higgs_from_request", || {
        higgs::Higgs::from_request()
    })
    .await?;

    // Branch on session: personal+global vs global-only (anon rate limit).
    let result = if ctx.session_user_id().is_some() {
        user_counter_increment(amount).await.map(CounterData::User)
    } else {
        counter_increment(amount).await.map(CounterData::Global)
    };

    #[cfg(feature = "ssr")]
    match &result {
        Ok(_) => {
            if let Some(start) = total_start {
                super::rootcause::log_span(
                    "increment_counter.total",
                    super::rootcause::elapsed_ms(start),
                    "",
                );
            }
            super::rootcause::log_spectra_per_increment("increment_counter", spectra_before);
        }
        Err(_) => {
            super::rootcause::log_spectra_per_increment("increment_counter_err", spectra_before);
        }
    }

    result
}

/// Increment the anonymous/global counter by `amount` and publish `CounterUpdated`.
///
/// Teaching walk:
///
/// 1. `Higgs::from_request` + `valence()` — same request context pattern as [`counter_get`].
/// 2. `validate_anon_increment` — worker abuse guard + in-process rate limit.
/// 3. `get_global` then `increment_global` — Valence read/modify/write on the Counter model.
/// 4. Spectra `record_increment_request("anon", "ok")` on success.
/// 5. `spawn_counter_updated` — Photon publish so `subscribe_counter_get` clients refetch.
///
/// # Errors
///
/// Validation, rate limit, Valence, or Higgs failures via [`into_server_error`].
#[uf_product_macros::server]
pub async fn counter_increment(amount: usize) -> Result<CounterResponse, ServerFnError> {
    use super::logging::{log_request_step, record_increment_request};

    log_request_step("counter_increment", "starting", "", "");
    let ctx = super::rootcause::timed("counter_increment.higgs_from_request", || {
        higgs::Higgs::from_request()
    })
    .await?;
    // Anon path: tighter per-request cap + in-process rate limit (CA-05).
    counter_service::validate_anon_increment(amount)
        .map_err(|e| into_server_error(map_service_err(e)))?;
    let v = ctx
        .valence()
        .map_err(|e| into_server_error(ctx_err("counter_increment valence", e)))?;

    let before = counter_service::get_global(&v)
        .await
        .map(|r| r.value.to_string())
        .unwrap_or_else(|_| "0".to_string());
    log_request_step("counter_increment", "loaded counter", &before, &before);

    // Valence read-modify-write on `Counter` id `"singleton"` (see worker service).
    let response = super::rootcause::timed("counter_increment.service", || {
        counter_service::increment_global(amount, &v)
    })
    .await
    .map_err(|e| into_server_error(map_service_err(e)))?;
    let response = to_ui_counter(response);

    record_increment_request("anon", "ok");
    log_request_step(
        "counter_increment",
        "success",
        &before,
        &response.value.to_string(),
    );

    // Photon: notify live subscribers without blocking the HTTP response.
    super::rootcause::timed("counter_increment.publish", || async {
        spawn_counter_updated(response.value);
    })
    .await;

    Ok(response)
}

/// Set the global counter to an explicit value (admin write).
///
/// Requires Gauge [`CounterAdmin`](crate::permissions::CounterPermission::CounterAdmin)
/// via `#[uf_product_macros::server(permission = …)]`. Session Valence performs the
/// Valence write (no System elevation). Hosts must wire `PermissionBackend` and
/// grant the permission — see root `SECURITY.md`.
///
/// # Errors
///
/// Higgs/Valence/service failures mapped through [`into_server_error`].
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app::counter_set;
///
/// let response = counter_set(42).await?;
/// assert_eq!(response.value, 42);
/// ```
#[uf_product_macros::server(permission = "CounterAdmin")]
#[server(SetCounter)]
pub async fn counter_set(
    /// New value to set the global counter to.
    value: usize,
) -> Result<CounterResponse, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let v = ctx
        .valence()
        .map_err(|e| into_server_error(ctx_err("counter_set valence", e)))?;

    // Absolute write: `Counter::upsert("singleton", …)` in the worker.
    let response = counter_service::set_global(value, &v)
        .await
        .map_err(|e| into_server_error(map_service_err(e)))?;
    let response = to_ui_counter(response);

    spawn_counter_updated(response.value);

    Ok(response)
}

/// Read the caller's personal counter plus the global counter.
///
/// Requires an authenticated Higgs session. Resolves the bare user record id via
/// session helpers, then `worker::service::get_user`. Use on pages that show both
/// scores after login.
///
/// # Errors
///
/// [`CounterServerError::NotAuthenticated`] when no session; Valence/service errors
/// otherwise — all via [`into_server_error`].
#[uf_product_macros::server]
#[server(UserCounterGet)]
pub async fn user_counter_get() -> Result<UserCounterResponse, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    // Session id may be `user:<id>`; Valence keys use the bare id.
    let user_id = session_user_record_id(&ctx)?;
    let v = ctx
        .valence()
        .map_err(|e| into_server_error(ctx_err("user_counter_get valence", e)))?;

    // Two Valence reads: `UserCounter` + global `Counter` singleton.
    let response = counter_service::get_user(&user_id, &v)
        .await
        .map_err(|e| into_server_error(map_service_err(e)))?;
    Ok(to_ui_user(response))
}

/// Increment the caller's personal counter and the global counter by `amount`.
///
/// Authenticated path used by [`increment_counter`]. After Valence
/// `increment_user`, publishes `CounterUpdated` with the new **global** count so
/// anonymous live viewers still refetch.
///
/// # Errors
///
/// Not authenticated, forbidden cross-user writes, validation, or Valence failures
/// via [`into_server_error`].
#[uf_product_macros::server]
#[server(UserCounterIncrement)]
pub async fn user_counter_increment(amount: usize) -> Result<UserCounterResponse, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let user_id = session_user_record_id(&ctx)?;

    let v = ctx
        .valence()
        .map_err(|e| into_server_error(ctx_err("user_counter_increment valence", e)))?;

    // Personal + global Valence writes; may fire LeaderboardNotifier → Boson.
    let response = super::rootcause::timed("user_counter_increment.service", || {
        counter_service::increment_user(&user_id, amount, &v)
    })
    .await
    .map_err(|e| into_server_error(map_service_err(e)))?;
    let response = to_ui_user(response);

    // Publish the *global* count so anonymous live viewers refetch too.
    super::rootcause::timed("user_counter_increment.publish", || async {
        spawn_counter_updated(response.global_count);
    })
    .await;

    Ok(response)
}
