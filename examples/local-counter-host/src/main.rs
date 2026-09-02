//! Canonical local counter host: Valence sqlite-mem + `counter-app-worker` service
//! under a protected `/counter` path.
//!
//! Copy surfaces for product hosts: this package's `Cargo.toml` + `main.rs`,
//! plus the product-mount dependency / Leptos / Higgs
//! sketches in the host README. Oneshot path `/counter` matches Orbital app
//! id/path `counter` / `/counter` (see JSON `inventory`).
//!
//! Proves the domain happy path a real product host wires before mounting
//! [`counter_app::CounterRoutes`] (get → increment → set) without the full
//! Leptos SSR/WASM graph.
//!
//! ## When to use
//! First smoke of counter domain + auth-gated `/counter` in a local host.
//!
//! ## Command
//! ```bash
//! export CARGO_BUILD_JOBS=1
//! export CARGO_TARGET_DIR=target-counter-app
//! cargo run -p local-counter-host
//! ```
//!
//! ## Success
//! Stdout prints `local_counter_host: OK — /counter deny/allow + get/increment/set`.
//!
//! ## Look next
//! Mount `<CounterRoutes />` in an L4 embedded/remote host; register Chronon/Boson
//! pieces from `counter-app-worker`; resolve request Valence via `higgs::Higgs::from_request + valence`.

#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::extract::Extension;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use axum::Router;
use counter_app_worker::embedded_surreal::LOGICAL_NAME;
use counter_app_worker::service::{get_global, increment_global, set_global};
use http_body_util::BodyExt;
use tower::ServiceExt;
use valence::actor::Actor;
use valence::{
    register_backend_logical_names, router_key, DatabaseBackend, DatabaseRouter,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

#[derive(Clone)]
struct DemoSession {
    user_id: String,
}

#[derive(Clone)]
struct HostState {
    valence: Arc<Valence>,
}

async fn require_session(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    if req.extensions().get::<DemoSession>().is_some() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn inject_demo_session(mut req: Request<Body>, next: Next) -> Response {
    if let Some(user) = req
        .headers()
        .get("x-demo-user")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
    {
        req.extensions_mut().insert(DemoSession { user_id: user });
    }
    next.run(req).await
}

fn prepare_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: example process; OnceLock reads this before first ownership get.
        unsafe {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

async fn mem_valence() -> Valence {
    prepare_env();
    // In-memory SQLite backend + router — same shape hosts use without Surreal.
    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("connect sqlite"),
    );
    let mut router = DatabaseRouter::new();
    // LOGICAL_NAME comes from counter-app-worker embedded_surreal (schema database key).
    register_backend_logical_names(
        &mut router,
        backend,
        &[LOGICAL_NAME],
        RegisterBackendLogicalNamesOptions::default(),
    );
    Valence::builder()
        .database_router(Arc::new(router))
        .default_backend_key(router_key(LOGICAL_NAME, SQLITE_ENGINE_ID))
        // System actor: scripts/admin-style writes; UI paths use User via Higgs.
        .with_actor(Actor::System {
            operation: "local-counter-host".into(),
        })
        .build()
        .expect("valence build")
}

async fn counter_api(
    Extension(session): Extension<DemoSession>,
    Extension(state): Extension<HostState>,
) -> impl IntoResponse {
    let v = state.valence.as_ref();
    // Same three calls the UI server fns wrap: get → increment → set.
    let empty = get_global(v).await.expect("get empty");
    let after_incr = increment_global(7, v).await.expect("increment");
    let after_set = set_global(42, v).await.expect("set");
    Json(serde_json::json!({
        "path": "/counter",
        "user": session.user_id,
        "empty": empty.value,
        "after_increment": after_incr.value,
        "after_set": after_set.value,
        // Matches counter-app `uf_app!` / CounterPermission / admin lazy route.
        "inventory": {
            "app_id": "counter",
            "route_path": "/counter",
            "auth_gate": "RequireAuthenticated",
            "admin_permission": "CounterAdmin",
        },
    }))
}

fn app(state: HostState) -> Router {
    Router::new()
        .route("/counter", get(counter_api))
        // Teaching stand-in for host auth middleware (real hosts use Lepton/Higgs).
        .route_layer(from_fn(require_session))
        .layer(Extension(state))
        .layer(from_fn(inject_demo_session))
}

async fn status_for(app: Router, path: &str, user: Option<&str>) -> StatusCode {
    let mut builder = Request::builder().uri(path);
    if let Some(user) = user {
        builder = builder.header("x-demo-user", user);
    }
    app.oneshot(builder.body(Body::empty()).expect("req"))
        .await
        .expect("oneshot")
        .status()
}

#[tokio::main]
async fn main() {
    let valence = Arc::new(mem_valence().await);
    let state = HostState { valence };
    let denied = status_for(app(state.clone()), "/counter", None).await;
    assert_eq!(denied, StatusCode::UNAUTHORIZED);

    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/counter")
                .header("x-demo-user", "demo-ops")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(body["path"], "/counter");
    assert_eq!(body["user"], "demo-ops");
    assert_eq!(body["empty"], 0);
    assert_eq!(body["after_increment"], 7);
    assert_eq!(body["after_set"], 42);
    assert_eq!(body["inventory"]["app_id"], "counter");
    assert_eq!(body["inventory"]["route_path"], "/counter");
    assert_eq!(body["inventory"]["auth_gate"], "RequireAuthenticated");
    assert_eq!(body["inventory"]["admin_permission"], "CounterAdmin");

    println!("local_counter_host: OK — /counter deny/allow + get/increment/set");
}
