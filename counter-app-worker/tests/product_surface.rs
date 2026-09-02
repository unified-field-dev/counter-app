//! Product surface contracts for counter-app (sibling crate).
//!
//! Lives under `counter-app-worker` so CI can gate route/testid/auth needles
//! without compiling Orbital/turf UI when host pins churn. Pattern matches
//! gauge `gauge/tests/product_surface.rs` and lepton-uf-app
//! `lepton-shell/tests/product_surface.rs`.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read_app(rel: &str) -> String {
    let path = workspace_root().join("counter-app").join("src").join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn counter_routes_mount_happy_path() {
    let lib = read_app("lib.rs");
    for needle in [
        r#"path!("counter")"#,
        r#"path!("")"#,
        r#"path!("high-scores")"#,
        r#"path!("admin")"#,
        "CounterLayout",
        "id: \"counter\"",
        "route_path: \"/counter\"",
        "permission_manifest: permissions::CounterPermission",
    ] {
        assert!(
            lib.contains(needle),
            "CounterRoutes / uf_app missing `{needle}`"
        );
    }
}

#[test]
fn counter_routes_drop_leaf_sad_path() {
    let lib = read_app("lib.rs");
    for needle in [r#"path!("high-scores")"#, r#"path!("admin")"#] {
        assert!(
            lib.contains(needle),
            "removing `{needle}` drops a Counter funnel entry"
        );
    }
    assert!(
        !lib.contains("unimplemented!"),
        "CounterRoutes must not ship unimplemented placeholders"
    );
}

#[test]
fn uf_app_wrong_id_sad_path() {
    let lib = read_app("lib.rs");
    assert!(
        lib.contains("id: \"counter\""),
        "wrong uf_app id breaks Orbital host registration"
    );
    assert!(
        !lib.contains("id: \"counter-app\""),
        "uf_app id must stay `counter` (product route id), not crate name counter-app"
    );
}

#[test]
fn layout_nav_testids_happy_path() {
    let layout = read_app("counter/counter_example/layout.rs");
    for needle in [
        "nav-live-counter",
        "nav-high-scores",
        "nav-counter-admin",
        "AppBarUserMenu",
        "UnifiedFieldShellLayout",
        "Outlet",
    ] {
        assert!(
            layout.contains(needle),
            "CounterLayout missing contract `{needle}`"
        );
    }
}

#[test]
fn layout_missing_nav_sad_path() {
    let layout = read_app("counter/counter_example/layout.rs");
    for id in ["nav-live-counter", "nav-high-scores", "nav-counter-admin"] {
        assert!(
            layout.contains(id),
            "dropping `{id}` breaks operator left-nav contract"
        );
    }
}

#[test]
fn admin_route_auth_gate_happy_path() {
    let lazy = read_app("lazy_routes.rs");
    for needle in [
        "RequireAuthenticated",
        "requires_email_verification=true",
        "permission_name=\"CounterAdmin\"",
        "CounterAdminPage",
        "counter-verified-admin-page-root",
    ] {
        assert!(
            lazy.contains(needle),
            "CounterAdminRoute missing contract `{needle}`"
        );
    }
}

#[test]
fn admin_route_drop_auth_guard_sad_path() {
    let lazy = read_app("lazy_routes.rs");
    assert!(
        lazy.contains("RequireAuthenticated") && lazy.contains("CounterAdminPage"),
        "removing RequireAuthenticated opens /counter/admin to anonymous sessions"
    );
    assert!(
        lazy.contains("requires_email_verification=true"),
        "admin route must keep email-verification gate"
    );
    assert!(
        lazy.contains("permission_name=\"CounterAdmin\""),
        "admin route must require CounterAdmin permission"
    );
    assert!(
        !lazy.contains("unimplemented!"),
        "lazy routes must not ship unimplemented placeholders"
    );
}

#[test]
fn high_scores_page_limit_clamp_happy_path() {
    let types = read_app("counter/components/high_scores/types.rs");
    for needle in [
        "MAX_HIGH_SCORES_LIMIT",
        "fn clamp_high_scores_page",
        "limit.min(MAX_HIGH_SCORES_LIMIT)",
    ] {
        assert!(
            types.contains(needle),
            "high-scores clamp missing `{needle}`"
        );
    }
    let server = read_app("counter/components/high_scores/server.rs");
    assert!(
        server.contains("clamp_high_scores_page(offset, limit)"),
        "GetHighScoresPage must clamp before querying"
    );
}

#[test]
fn user_increment_forbidden_mapped_happy_path() {
    let server = read_app("counter/counter_example/server.rs");
    assert!(
        server.contains("CounterServiceError::Forbidden")
            && server.contains("CounterServerError::Forbidden"),
        "server must map worker Forbidden into UI Forbidden"
    );
    let service = fs::read_to_string(workspace_root().join("counter-app-worker/src/service.rs"))
        .expect("service.rs");
    assert!(
        service.contains("ensure_may_mutate_user_counter")
            && service.contains("CounterServiceError::Forbidden"),
        "increment_user must enforce actor/user_id match"
    );
}

#[test]
fn page_testid_and_server_bindings_happy_path() {
    let live = read_app("counter/counter_example/pages/live/mod.rs");
    for needle in [
        "counter-container",
        "counter_get",
        "user_counter_get",
        "IncrementCounter",
    ] {
        assert!(
            live.contains(needle),
            "CounterExamplePage missing `{needle}`"
        );
    }

    let high = read_app("counter/counter_example/pages/high_scores.rs");
    for needle in ["high-scores-page", "HighScoresTable"] {
        assert!(high.contains(needle), "HighScoresPage missing `{needle}`");
    }

    let admin = read_app("counter/counter_example/pages/admin/mod.rs");
    for needle in ["counter-admin-container", "counter_get", "SetCounter"] {
        assert!(
            admin.contains(needle),
            "CounterAdminPage missing `{needle}`"
        );
    }
}

#[test]
fn page_drop_testid_sad_path() {
    let live = read_app("counter/counter_example/pages/live/mod.rs");
    assert!(
        live.contains("data_testid=\"counter-container\""),
        "dropping counter-container breaks host / future Playwright parity"
    );
    let high = read_app("counter/counter_example/pages/high_scores.rs");
    assert!(
        high.contains("data_testid=\"high-scores-page\""),
        "dropping high-scores-page breaks host / future Playwright parity"
    );
    let admin = read_app("counter/counter_example/pages/admin/mod.rs");
    assert!(
        admin.contains("data_testid=\"counter-admin-container\""),
        "dropping counter-admin-container breaks host / future Playwright parity"
    );
}

#[test]
fn server_wrappers_call_worker_service_happy_path() {
    let server = read_app("counter/counter_example/server.rs");
    for needle in [
        "higgs::Higgs::from_request",
        "counter_service::get_global",
        "counter_service::increment_global",
        "counter_service::set_global",
        "counter_service::get_user",
        "counter_service::increment_user",
        "session_user_record_id",
        "validate_anon_increment",
        "validate_increment_amount",
    ] {
        assert!(
            server.contains(needle),
            "server wrappers missing contract `{needle}`"
        );
    }
}

#[test]
fn user_paths_require_session_sad_path() {
    let server = read_app("counter/counter_example/server.rs");
    for fn_name in [
        "pub async fn user_counter_get",
        "pub async fn user_counter_increment",
    ] {
        let start = server
            .find(fn_name)
            .unwrap_or_else(|| panic!("missing `{fn_name}`"));
        let body = &server[start..start + 500.min(server.len() - start)];
        assert!(
            body.contains("session_user_record_id"),
            "`{fn_name}` must resolve session user before service call"
        );
    }
}

#[test]
fn counter_set_requires_counter_admin_happy_path() {
    let server = read_app("counter/counter_example/server.rs");
    assert!(
        server.contains(r#"permission = "CounterAdmin""#)
            && server.contains("pub async fn counter_set"),
        "counter_set must gate on CounterAdmin via uf_product_macros::server"
    );
}

#[test]
fn permission_manifest_counter_admin_happy_path() {
    let perms = read_app("permissions.rs");
    for needle in [
        "domain_key = \"counter\"",
        "CounterAdmin",
        "UfPermissionManifest",
        "COUNTER_ADMIN_GROUP_ID",
        "counter_admin",
    ] {
        assert!(
            perms.contains(needle),
            "CounterPermission manifest missing `{needle}`"
        );
    }
}

#[test]
fn lazy_routes_wire_pages_happy_path() {
    let lazy = read_app("lazy_routes.rs");
    for needle in ["CounterExamplePage", "HighScoresPage", "CounterAdminPage"] {
        assert!(
            lazy.contains(needle),
            "lazy_routes missing page wire `{needle}`"
        );
    }
}

#[test]
fn local_counter_host_matches_uf_app_happy_path() {
    let host = fs::read_to_string(workspace_root().join("examples/local-counter-host/src/main.rs"))
        .expect("local-counter-host main.rs");
    for needle in [
        "\"app_id\": \"counter\"",
        "\"route_path\": \"/counter\"",
        "\"auth_gate\": \"RequireAuthenticated\"",
        "\"admin_permission\": \"CounterAdmin\"",
        "get_global",
        "increment_global",
        "set_global",
    ] {
        assert!(
            host.contains(needle),
            "local-counter-host missing contract `{needle}`"
        );
    }
    let lib = read_app("lib.rs");
    assert!(
        lib.contains("id: \"counter\"") && lib.contains("route_path: \"/counter\""),
        "host inventory must stay aligned with uf_app!"
    );
    let lazy = read_app("lazy_routes.rs");
    assert!(
        lazy.contains("RequireAuthenticated"),
        "host auth_gate must stay aligned with CounterAdminRoute guard"
    );
    let perms = read_app("permissions.rs");
    assert!(
        perms.contains("CounterAdmin"),
        "host admin_permission must stay aligned with CounterPermission"
    );
}
