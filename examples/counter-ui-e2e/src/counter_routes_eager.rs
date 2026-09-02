//! Eager `/counter` routes for the Playwright host.
//!
//! Production [`counter_app::CounterRoutes`] wraps leaf pages in `Lazy` for
//! wasm-split. Nested `Lazy` under `ParentRoute` still panics on hydrate in
//! this Leptos pin, so the lab host mounts the same page components without
//! `Lazy`.

use counter_app::{CounterAdminPage, CounterExamplePage, CounterLayout, HighScoresPage};
use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path,
};

/// Same paths as [`counter_app::CounterRoutes`], without Lazy route views.
#[component(transparent)]
pub fn CounterRoutesEager() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("counter") view=CounterLayout>
            <Route path=path!("") view=CounterExamplePage />
            <Route path=path!("high-scores") view=HighScoresPage />
            <Route path=path!("admin") view=AdminVerifiedGate />
        </ParentRoute>
    }
    .into_inner()
}

/// Admin leaf wrapped in verified email + CounterAdmin gate (matches production lazy route).
#[component]
fn AdminVerifiedGate() -> impl IntoView {
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
}
