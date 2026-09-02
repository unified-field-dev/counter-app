//! Axum + Leptos host for counter-app CounterRoutes Playwright.

#![allow(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::unwrap_used,
    missing_docs
)]

use axum::extract::Extension;
use axum::middleware::from_fn;
use axum::routing::{get, post};
use axum::Router;
use leptos::config::get_configuration;
use leptos::prelude::provide_context;
use leptos_axum::{file_and_error_handler, generate_route_list, LeptosRoutes};
use photon_axum::ws_router;
use std::path::PathBuf;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use counter_ui_e2e::seed::seed_data;
use counter_ui_e2e::{
    build_photon, e2e_higgs_config, e2e_router, init_e2e_valence, inject_e2e_session_snapshot,
    shell, wire_gauge_permissions_bridge, App, AppState, E2ePhotonAuth,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_level(log::Level::Info).expect("logger");
    any_spawner::Executor::init_tokio().expect("tokio executor");

    // Link worker so Photon `CounterUpdated` inventory registers.
    let _ = counter_app_worker::events::CounterUpdated { new_value: 0 };

    // Lab default for rate-limit sad path (override with env if needed).
    if std::env::var_os("COUNTER_ANON_INCREMENTS_PER_MIN").is_none() {
        // SAFETY: e2e host boot only.
        unsafe {
            std::env::set_var("COUNTER_ANON_INCREMENTS_PER_MIN", "3");
        }
    }

    tokio::task::LocalSet::new().run_until(serve()).await
}

async fn serve() -> anyhow::Result<()> {
    init_e2e_valence().await;
    let photon = build_photon()?;

    let conf = get_configuration(None).expect("leptos config");
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let site_root = std::path::PathBuf::from(leptos_options.site_root.as_ref());
    let pkg_dir = site_root.join(leptos_options.site_pkg_dir.as_ref());
    let fonts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../L0-upstream-cores/orbital/public/fonts");

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_name("uf_counter_ui_e2e")
        .with_path("/");

    let leptos_options_for_routes = leptos_options.clone();
    let higgs = e2e_higgs_config();
    let router = e2e_router();

    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        photon,
    };
    let ctx_state = app_state.clone();

    let app = Router::new()
        .route("/health", get(|| async { axum::http::StatusCode::OK }))
        .route("/api/test/seed-data", post(seed_data))
        .nest_service("/pkg", ServeDir::new(pkg_dir))
        .nest_service("/fonts", ServeDir::new(fonts_dir))
        .leptos_routes_with_context(
            &app_state,
            routes,
            move || {
                provide_context::<std::sync::Arc<higgs::HiggsConfig>>(higgs.clone());
                provide_context(ctx_state.clone());
                wire_gauge_permissions_bridge();
            },
            move || shell(leptos_options_for_routes.clone()),
        )
        .fallback(file_and_error_handler::<AppState, _>(shell));

    let app = ws_router::<AppState, E2ePhotonAuth>(app)
        .layer(from_fn(inject_e2e_session_snapshot))
        .layer(Extension(router))
        .layer(session_layer)
        .with_state(app_state);

    log::info!("counter-ui-e2e listening on http://{addr} (Photon /ws/counter mounted)");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
