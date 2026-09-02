//! Permission manifest for the Counter demo app (CA-09).
//!
//! `uf_app!` in the crate root points at [`CounterPermission`] so Orbital /
//! Gauge hosts discover this product's permission domain at inventory time.
//! [`CounterPermission::CounterAdmin`] gates `counter_set` and `/counter/admin`
//! (server fn + route). Public increment paths stay open; see `SECURITY.md`.
//!
//! Hosts sync manifests with `gauge::manifest_sync::sync_permission_manifests`
//! (creates `manifest_counter_owners` automatically), then grant
//! [`CounterPermission::CounterAdmin`] to [`COUNTER_ADMIN_GROUP_ID`] and add
//! operator users as members of that group.

use uf_product_macros::UfPermissionManifest;

/// Stable Gauge group id for CounterAdmin operators.
///
/// Grant `CounterAdmin` to this group (not directly to users) so membership
/// controls admin access. E2E and production hosts upsert this row at boot.
pub const COUNTER_ADMIN_GROUP_ID: &str = "counter_admin";

/// Human-readable name for [`COUNTER_ADMIN_GROUP_ID`].
pub const COUNTER_ADMIN_GROUP_NAME: &str = "Counter Admin";

/// Counter app permission domain for the platform catalog.
///
/// Derive [`UfPermissionManifest`] with `domain_key = "counter"` and pass the
/// enum to `uf_app!` via `permission_manifest`. Hosts enumerate domains from
/// inventory and wire Gauge via `uf_product::permissions::provide_permission_backend`.
#[allow(clippy::expl_impl_clone_on_copy)]
#[derive(UfPermissionManifest)]
#[permission_manifest(
    domain_key = "counter",
    domain_name = "Counter",
    domain_description = "Counter demo application"
)]
pub enum CounterPermission {
    /// Gate for absolute global counter set and admin UI.
    #[permission(description = "Manage counter demo settings")]
    CounterAdmin,
}
