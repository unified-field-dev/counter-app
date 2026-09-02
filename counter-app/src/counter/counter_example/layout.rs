//! Orbital shell for the `/counter` route tree.
//!
//! [`CounterLayout`] is the `ParentRoute` view for [`crate::CounterRoutes`]: app
//! bar, categorized left navigation, and a Leptos `Outlet` for the active leaf
//! page. Navigation links jump into platform operator UIs (Valence schemas,
//! Chronon jobs, Boson tasks, Photon topics) so newcomers can see how this demo
//! plugs into the rest of Unified Field from one shell.

use crate::paths;
use crate::AppMetadata;
use lepton_shell::AppBarUserMenu;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use orbital::components::{
    Navigation, NavigationBody, NavigationCategory, NavigationCategoryHeader, NavigationConfig,
    NavigationLink, NavigationMaterial, NavigationSubItemGroup, NavigationSubLink,
};
use uf_integrations::{
    ShellAppBar, ShellAuthMenu, ShellLeftNav, UnifiedFieldAppBar, UnifiedFieldShellLayout,
};

/// Counter app shell: Orbital app bar, left nav, and router `Outlet`.
///
/// Mounted as the parent view under `path!("counter")` by `uf_app!` /
/// [`crate::CounterRoutes`]. Use this pattern when a product app needs one
/// shared chrome around several leaf pages (live, high scores, admin) and
/// deep links into Valence / Chronon / Boson / Photon operator screens.
///
/// Auth chrome comes from `ShellAuthMenu` + `AppBarUserMenu`. Leaf pages still
/// call `use_auth_state` / `RequireAuthenticated` for their own gates.
#[component]
pub fn CounterLayout() -> impl IntoView {
    let app_name = AppMetadata::name().to_string();
    let selected_value = RwSignal::new(None::<String>);
    let open_categories = RwSignal::new(vec![
        "counter-app".to_string(),
        "data".to_string(),
        "scripts".to_string(),
        "boson".to_string(),
        "photon".to_string(),
    ]);

    view! {
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar
                    app_name=app_name
                    app_id=AppMetadata::id()
                    homepage_url="/".to_string()
                >
                    <ShellAuthMenu slot:auth_menu>
                        <AppBarUserMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ShellLeftNav slot>
                <Navigation config=NavigationConfig::new().with_selected_value(selected_value).with_open_categories(open_categories)>
                    <NavigationMaterial slot />
                    <NavigationBody slot>
                        <NavigationCategory value="counter-app">
                            <NavigationCategoryHeader slot icon=icondata::AiAppstoreOutlined>
                                "Counter App"
                            </NavigationCategoryHeader>
                            <NavigationSubItemGroup>
                                <NavigationSubLink path=paths::ROOT value=paths::ROOT icon=icondata::AiPlayCircleOutlined exact=true test_id="nav-live-counter">"Live Counter"</NavigationSubLink>
                                <NavigationSubLink path=paths::HIGH_SCORES value=paths::HIGH_SCORES icon=icondata::AiTeamOutlined test_id="nav-high-scores">"High Scores"</NavigationSubLink>
                                <NavigationSubLink path=paths::ADMIN value=paths::ADMIN icon=icondata::AiSettingOutlined test_id="nav-counter-admin">"Counter Admin"</NavigationSubLink>
                            </NavigationSubItemGroup>
                        </NavigationCategory>
                        <NavigationCategory value="data">
                            <NavigationCategoryHeader slot icon=icondata::AiDatabaseOutlined>
                                "Data"
                            </NavigationCategoryHeader>
                            <NavigationSubItemGroup>
                                <NavigationSubLink path="/valence/schema/counter" value="/valence/schema/counter" icon=icondata::AiAppstoreOutlined test_id="nav-counter-schema">"counter schema"</NavigationSubLink>
                                <NavigationSubLink path="/valence/schema/counter/id/singleton" value="/valence/schema/counter/id/singleton" icon=icondata::AiAppstoreOutlined test_id="nav-counter-data">"counter data"</NavigationSubLink>
                                <NavigationSubLink path="/valence/schema/user_counter" value="/valence/schema/user_counter" icon=icondata::AiAppstoreOutlined test_id="nav-user-counter-schema">"user_counter schema"</NavigationSubLink>
                                <NavigationSubLink path="/valence/schema/user" value="/valence/schema/user" icon=icondata::AiAppstoreOutlined test_id="nav-user-schema">"user"</NavigationSubLink>
                                <NavigationSubLink path="/valence/schema/notification" value="/valence/schema/notification" icon=icondata::AiAppstoreOutlined test_id="nav-notification-schema">"notification"</NavigationSubLink>
                            </NavigationSubItemGroup>
                        </NavigationCategory>
                        <NavigationCategory value="scripts">
                            <NavigationCategoryHeader slot icon=icondata::AiClockCircleOutlined>
                                "Scripts"
                            </NavigationCategoryHeader>
                            <NavigationSubItemGroup>
                                <NavigationSubLink path="/chronon/jobs/daily-highscores-reset" value="/chronon/jobs/daily-highscores-reset" icon=icondata::AiClockCircleOutlined test_id="nav-chronon-daily-reset">"daily-highscores-reset"</NavigationSubLink>
                                <NavigationSubLink path="/chronon/jobs/bot-score-bumper" value="/chronon/jobs/bot-score-bumper" icon=icondata::AiClockCircleOutlined test_id="nav-chronon-bot-bumper">"bot-score-bumper"</NavigationSubLink>
                                <NavigationSubLink path="/chronon/jobs/ensure-bot-users" value="/chronon/jobs/ensure-bot-users" icon=icondata::AiClockCircleOutlined test_id="nav-chronon-ensure-bot-user">"ensure-bot-users"</NavigationSubLink>
                            </NavigationSubItemGroup>
                        </NavigationCategory>
                        <NavigationCategory value="boson">
                            <NavigationCategoryHeader slot icon=icondata::AiThunderboltOutlined>
                                "Boson"
                            </NavigationCategoryHeader>
                            <NavigationSubItemGroup>
                                <NavigationSubLink path="/boson/tasks/check_leaderboard_changes" value="/boson/tasks/check_leaderboard_changes" icon=icondata::AiThunderboltOutlined test_id="nav-boson-leaderboard">"check_leaderboard_changes"</NavigationSubLink>
                            </NavigationSubItemGroup>
                        </NavigationCategory>
                        <NavigationCategory value="photon">
                            <NavigationCategoryHeader slot icon=icondata::AiWifiOutlined>
                                "Photon"
                            </NavigationCategoryHeader>
                            <NavigationSubItemGroup>
                                <NavigationSubLink path="/photon/topics/user.notifications" value="/photon/topics/user.notifications" icon=icondata::AiWifiOutlined test_id="nav-photon-notifications">"user.notifications"</NavigationSubLink>
                            </NavigationSubItemGroup>
                        </NavigationCategory>
                        <NavigationLink path="/orbital" value="/orbital" icon=icondata::AiAppstoreOutlined test_id="nav-component-library">"Component Library"</NavigationLink>
                    </NavigationBody>
                </Navigation>
            </ShellLeftNav>
            <Outlet />
        </UnifiedFieldShellLayout>
    }
}
