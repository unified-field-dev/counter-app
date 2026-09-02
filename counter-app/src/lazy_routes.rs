//! Lazy-loaded route views for WASM code-splitting (`cargo leptos --split`).
//!
//! Each leaf under `/counter` implements Leptos [`LazyRoute`]: `data()` builds
//! the route state and `view()` renders the page component. [`crate::CounterRoutes`]
//! wraps these types in `Lazy::<…>::new()` so the host emits a separate WASM
//! chunk for the counter family instead of baking every page into the main
//! hydrate bundle.
//!
//! Call [`prefetch_family`] when you want the chunk warmed before navigation
//! (hover, sibling page mount). Admin wraps [`CounterAdminPage`] in
//! [`uf_product::routes::RequireAuthenticated`] with verified email and
//! [`CounterAdmin`](crate::permissions::CounterPermission::CounterAdmin).

use leptos::prelude::*;
use leptos_router::{lazy_route, LazyRoute};

use crate::counter::{CounterAdminPage, CounterExamplePage, HighScoresPage};

/// Prefetch the counter family WASM chunk (leaf pages share split modules).
///
/// Invoke from a parent shell or hover handler when you expect the user to open
/// `/counter` soon. Awaits [`CounterExampleRoute::preload`]; other leaves in
/// this family share the same split graph in typical `cargo leptos --split`
/// layouts.
pub async fn prefetch_family() {
    CounterExampleRoute::preload().await;
}

/// Lazy `/counter` live page (`CounterExamplePage`).
///
/// Registered as `Lazy::<CounterExampleRoute>` on the empty child path of
/// [`crate::CounterRoutes`]. Implements [`LazyRoute`] so the hydrate build can
/// code-split this view.
#[derive(Clone, Copy, Debug, Default)]
pub struct CounterExampleRoute;

#[lazy_route]
impl LazyRoute for CounterExampleRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <CounterExamplePage /> }.into_any()
    }
}

/// Lazy `/counter/high-scores` page (`HighScoresPage`).
///
/// Same [`LazyRoute`] pattern as [`CounterExampleRoute`]; path is `high-scores`
/// under the counter parent.
#[derive(Clone, Copy, Debug, Default)]
pub struct HighScoresRoute;

#[lazy_route]
impl LazyRoute for HighScoresRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <HighScoresPage /> }.into_any()
    }
}

/// Lazy `/counter/admin` page with verified email + `CounterAdmin` gate.
///
/// Wraps [`CounterAdminPage`] in [`uf_product::routes::RequireAuthenticated`]
/// with `requires_email_verification=true` and `permission_name=CounterAdmin`.
/// The server fn enforces the same permission (defense in depth).
#[derive(Clone, Copy, Debug, Default)]
pub struct CounterAdminRoute;

#[lazy_route]
impl LazyRoute for CounterAdminRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! {
            <div data-testid="counter-verified-admin-page-root">
                <uf_product::routes::RequireAuthenticated
                    requires_email_verification=true
                    permission_name="CounterAdmin"
                >
                    <CounterAdminPage />
                </uf_product::routes::RequireAuthenticated>
            </div>
        }
        .into_any()
    }
}
