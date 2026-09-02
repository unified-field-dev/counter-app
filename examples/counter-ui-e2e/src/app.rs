//! Mount eager CounterRoutes for Playwright.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use uf_integrations::{
    provide_shell_auth_menu, HostAuthMenu, ShellAppBar, ShellAuthMenu, UnifiedFieldAppBar,
    UnifiedFieldShellLayout,
};
use uf_product::components::ContentContainer;
use uf_product::primitives::{Body1, Flex, FlexAlign, FlexGap, Link, Title3};
use uf_product::{orbital_shell, OrbitalTemplate};

use crate::counter_routes_eager::CounterRoutesEager;
use crate::gate_demos::E2eAuthProvider;
use crate::harness_auth_menu::HarnessAuthMenu;

/// SSR document shell.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    orbital_shell(options, || view! { <App/> })
}

/// Root: harness auth + eager counter routes (same pages as CounterRoutes).
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    #[cfg(feature = "ssr")]
    {
        provide_context(crate::e2e_higgs_config());
        wire_gauge_permissions_bridge();
    }
    provide_shell_auth_menu(|| view! { <HarnessAuthMenu /> });

    view! {
        <OrbitalTemplate>
            <Stylesheet id="leptos" href="/pkg/counter-ui-e2e.css"/>
            <Title text="counter-ui-e2e"/>
            <E2eAuthProvider>
                <Router>
                    <Routes fallback=|| view! { <p>"Not found"</p> }>
                        <Route path=path!("/") view=HomePage/>
                        <CounterRoutesEager />
                    </Routes>
                </Router>
            </E2eAuthProvider>
        </OrbitalTemplate>
    }
}

/// Wire Gauge as uf-product permission backend (same pattern as chronon-uf-app-e2e).
#[cfg(feature = "ssr")]
pub fn wire_gauge_permissions_bridge() {
    use std::sync::Arc;

    struct GaugePermissionBackend;

    #[async_trait::async_trait]
    impl uf_product::permissions::PermissionBackend for GaugePermissionBackend {
        async fn has_permission(&self, permission_name: &str) -> Result<bool, ServerFnError> {
            let ctx = higgs::Higgs::from_request().await?;
            let valence = ctx
                .valence()
                .map_err(|e| ServerFnError::new(format!("Failed to build Valence: {e}")))?;
            let _caller =
                gauge::instrumentation::PermissionCheckCallerGuard::new("permission_backend");
            gauge::service::actor_can(&valence, permission_name)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to check permission: {e}")))
        }
    }

    uf_product::permissions::provide_permission_backend(Arc::new(GaugePermissionBackend));
}

#[cfg(not(feature = "ssr"))]
pub fn wire_gauge_permissions_bridge() {}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar app_name="Counter e2e".to_string()>
                    <ShellAuthMenu slot:auth_menu>
                        <HostAuthMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ContentContainer max_width="900px" data_testid="counter-e2e-home">
                <Flex vertical=true gap=FlexGap::Medium align=FlexAlign::Start>
                    <Title3>"counter-ui-e2e"</Title3>
                    <Body1>"CounterRoutes host for Playwright."</Body1>
                    <Link href="/counter">"Open /counter"</Link>
                </Flex>
            </ContentContainer>
        </UnifiedFieldShellLayout>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
    uf_product::hide_boot_loader();
}
