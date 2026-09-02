#![recursion_limit = "256"]
//! Counter example application for Unified Field hosts.
//!
//! Leptos UI under `/counter` that shows how an Orbital product app is registered,
//! routed, and composed with Valence, Chronon, Boson, Photon, and Spectra. Pages,
//! server functions, typed errors, and forensic logging helpers are intentionally
//! `pub` so you can read the call sites when wiring your own app.
//!
//! Domain get/increment/set, Valence schemas, Chronon scripts, Boson tasks, and
//! Photon topic definitions live in `counter-app-worker` (re-exported as [`worker`]
//! under `ssr`). Host binaries stay outside this repository.
//!
//! ## Features
//!
//! - **Counter routes** — Nested `/counter` route tree (live counter, high scores,
//!   admin) registered with `uf_app!` and mounted once when the host router starts.
//!   [Get started](#mount-counterroutes)
//! - **Counter server functions** — Higgs `#[server]` wrappers
//!   [`counter_get`], [`increment_counter`], and [`counter_set`] that resolve
//!   request context, call the worker service, and map errors to Spectra.
//!   [Get started](#counter-server-functions)
//! - **Live counter subscription** — Photon-leptos `#[synced]` on [`counter_get`]
//!   plus client `subscribe_counter_get` so the live page refetches when
//!   `CounterUpdated` publishes. [Get started](#photon-live-subscription)
//! - **Typed server errors** — [`CounterServerError`] and [`into_server_error`]
//!   turn domain failures into `ServerFnError` and record `counter_server_errors`.
//! - **Pages and layout** — [`CounterExamplePage`], [`HighScoresPage`],
//!   [`CounterAdminPage`], and [`CounterLayout`] compose Orbital chrome around the
//!   routes.
//!
//! Domain failures stay typed as [`CounterServerError`] / `worker::CounterServiceError`
//! until [`into_server_error`] maps them to Leptos `ServerFnError` and records Spectra
//! `error_kind` labels (`validation`, `rate_limited`, `not_authenticated`, …).
//!
//! ## Mount CounterRoutes
//!
//! [`CounterRoutes`] nests the `/counter` subtree inside a host Leptos `<Routes>`
//! tree and, via `uf_app!`, registers launcher metadata and the inventory entry.
//! Mount during host router setup at startup, alongside other product apps, so
//! `/counter`, `/counter/high-scores`, and `/counter/admin` resolve.
//!
//! **Prerequisites:** Host Leptos router; enable `ssr` and/or `hydrate` on this crate
//! as the host split requires; worker schemas registered in the host Valence router
//! when server functions run.
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use leptos_router::components::Routes;
//! use counter_app::CounterRoutes;
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     view! {
//!         <Routes fallback=|| "not found">
//!             <CounterRoutes />
//!         </Routes>
//!     }
//! }
//! ```
//!
//! On success `/counter` renders the live page, nested routes load their lazy WASM
//! chunks, and the product inventory lists `counter` / `/counter`. Missing host
//! Valence or Photon wiring surfaces as server-fn or subscription errors at runtime —
//! see root `SECURITY.md` for demo auth posture.
//!
//! ## Counter server functions
//!
//! Server functions are the UI boundary over the worker domain service. They teach
//! the Higgs request path: build context with `Higgs::from_request`, take a
//! Valence handle via `valence()`, call
//! `worker::service::{get_global,increment_global,set_global}`, then map failures
//! through [`into_server_error`]. Call these from Leptos resources and actions after
//! routes are mounted when the page needs the public global counter.
//!
//! **Prerequisites:** `ssr` feature; Higgs + Valence in request context; worker
//! crate linked; Spectra topics registered if you want error counters.
//!
//! ```rust,ignore
//! use counter_app::{counter_get, increment_counter, counter_set, CounterResponse};
//!
//! let before: CounterResponse = counter_get().await?;
//! let after = increment_counter(1).await?;
//! assert!(after.value >= before.value + 1);
//!
//! let set = counter_set(42).await?;
//! assert!(set.value == 42 || set.value > 0);
//! let value = set.value;
//! assert!(value > 0);
//! ```
//!
//! On success each call returns [`CounterResponse`] with the persisted `value`.
//! Validation, anon rate limits, and auth failures become [`CounterServerError`]
//! variants, then `ServerFnError` with Spectra `error_kind` labels. Next:
//! [Live counter subscription](#photon-live-subscription) for refetch-on-publish,
//! or the `counter-app-worker` crate-root **Counter service** section for the
//! headless API.
//!
//! ## Photon live subscription
//!
//! The live page stays current without polling by combining Photon-leptos
//! `#[synced]` on [`counter_get`] (topic `counter.updated`, WebSocket `/ws/counter`,
//! refetch strategy, demo `auth = "none"`) with the generated client helper
//! `subscribe_counter_get`. When a mutate path publishes `CounterUpdated`
//! (`new_value`), connected clients receive a `ws_trigger` signal and refetch.
//! Wire this on the client after mount when you want live invalidation.
//!
//! **Prerequisites:** Routes mounted; Photon WS route for `/ws/counter` on the host;
//! `ssr` for the server attribute; hydrate build for the client subscribe helper.
//! Treat `auth = "none"` as demo-only (payload is a public integer) — see `SECURITY.md`.
//!
//! ```rust,ignore
//! use counter_app::counter_get;
//! // Generated by #[photon_leptos::synced] on counter_get:
//! use counter_app::subscribe_counter_get;
//! use counter_app_worker::events::CounterUpdated;
//!
//! // Client: resource keyed on ws_trigger refetches when CounterUpdated publishes.
//! let ws_trigger = subscribe_counter_get(|| {});
//! let event = CounterUpdated { new_value: 7 };
//! assert_eq!(event.new_value, 7);
//! println!("subscribed; trigger={:?}", ws_trigger.get());
//! // After mutate, server publishes CounterUpdated; client refetches via counter_get.
//! ```
//!
//! On success `subscribe_counter_get` yields a signal that changes when the topic
//! fires, and the resource calling `counter_get` reloads. Publish failures log a
//! warning when `COUNTER_NOOP_PUBLISH` is unset. Next: the worker crate-root
//! **Photon counter topic** section for the publish path.
//!
//! ## Feature flags
//!
//! | Flag | Effect |
//! |------|--------|
//! | `ssr` | Server-side Leptos split; Higgs server fns; links `counter-app-worker`. |
//! | `hydrate` | Client hydration for routed pages, Orbital shell, and Photon subscribe helpers. |
//!
//! ## Examples
//!
//! Start with [Mount CounterRoutes](#mount-counterroutes). Domain get/increment/set
//! coverage: `cargo test -p counter-app-worker --test counter_workflow_contract`.
//! Runnable oneshot host: `examples/local-counter-host` (deny/allow + get/increment/set;
//! inventory `counter` / `/counter` — see its README).
//!
//! ## Where to look next
//!
//! - [`counter`] — pages, layout, server fns, and live Photon wiring.
//! - [`worker`] (`ssr`) — Valence schemas, service, Chronon, Boson, Photon topics.
//! - [`CounterServerError`] / [`into_server_error`] — typed UI errors → Spectra.
//! - Root `SECURITY.md` — intentional public-write and WS demo posture.

// `uf_app!` / `orbital_routes_extract` emit undocumented associated items.
#![allow(missing_docs)]
#![allow(clippy::unused_unit, unused_imports, unused_variables)]

use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path, Lazy,
};
use uf_product_macros::uf_app;

/// Counter example feature: pages, server logic, layout, and live Photon wiring.
pub mod counter;
mod lazy_routes;
pub mod permissions;

pub use counter::counter_example::{
    counter_get, counter_increment, counter_set, increment_counter, into_server_error,
    user_counter_get, user_counter_increment, CounterData, CounterResponse, CounterServerError,
    UserCounterResponse, MAX_INCREMENT_AMOUNT,
};
pub use counter::{CounterAdminPage, CounterExamplePage, CounterLayout, HighScoresPage};
pub use lazy_routes::{prefetch_family, CounterAdminRoute, CounterExampleRoute, HighScoresRoute};

#[cfg(feature = "ssr")]
pub use counter_app_worker as worker;
#[cfg(feature = "ssr")]
pub use counter_app_worker::{embedded_surreal, generated};

// Product inventory: launcher metadata + which routes component owns `/counter`.
// Hosts discover this via uf_product AppRegistration / Orbital inventory.
uf_app! {
    name: "Counter",
    id: "counter",
    description: "A simple counter application demonstrating Valence ORM integration",
    icon: "📊",
    version: "0.1.0",
    routes: CounterRoutes,
    route_path: "/counter",
    permission_manifest: permissions::CounterPermission,
}

/// Counter application routes: live counter, high scores, and admin views.
///
/// Mount inside the host `<Routes>` tree (see crate-root
/// [Mount CounterRoutes](index.html#mount-counterroutes)). Uses
/// `orbital_routes_extract` so Orbital inventory sees the nested paths, and
/// [`Lazy`] leaf routes so `cargo leptos --split` can emit a
/// separate WASM chunk for this family. `uf_app!` above registers launcher
/// metadata (`id: "counter"`, `route_path: "/counter"`).
#[allow(missing_docs)]
// Emits route metadata for Orbital / product inventory (sibling of `uf_app!`).
#[orbital_macros::orbital_routes_extract]
#[component(transparent)]
pub fn CounterRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        // Nested under `counter` so paths are `/counter`, `/counter/high-scores`, …
        <ParentRoute path=path!("counter") view=CounterLayout>
            // Lazy = separate WASM chunk per leaf when using cargo-leptos --split.
            <Route path=path!("") view={Lazy::<CounterExampleRoute>::new()} />
            <Route path=path!("high-scores") view={Lazy::<HighScoresRoute>::new()} />
            <Route path=path!("admin") view={Lazy::<CounterAdminRoute>::new()} />
        </ParentRoute>
    }
    .into_inner()
}
